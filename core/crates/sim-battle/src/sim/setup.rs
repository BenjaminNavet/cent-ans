//! Battle construction and initial deployment.

use super::*;

impl BattleSim {
    /// at the scale of the setup's head count ([`BattleScale::for_setup`]).
    pub fn new(setup: BattleSetup, seed: u64) -> Result<Self, SetupError> {
        let scale = BattleScale::for_setup(&setup);
        Self::new_scaled(setup, seed, scale)
    }

    /// [`Self::new`] at a given scale (EP1: forced tier; sieges always use
    /// the standard field, whatever `scale` says about the field).
    pub fn new_scaled(
        setup: BattleSetup,
        seed: u64,
        scale: BattleScale,
    ) -> Result<Self, SetupError> {
        Self::new_scaled_weather(setup, seed, scale, None)
    }

    /// [`Self::new_scaled`] with the weather forced (NT2: custom battle).
    /// The draw still consumes its roll so the field of a given seed stays
    /// the same whatever the weather.
    pub fn new_scaled_weather(
        setup: BattleSetup,
        seed: u64,
        mut scale: BattleScale,
        forced_weather: Option<Weather>,
    ) -> Result<Self, SetupError> {
        if setup.siege.is_some() {
            scale.field = crate::scale::FieldSize::STANDARD;
        }
        for side in SideId::BOTH {
            if setup.side(side).units.iter().all(|u| u.soldiers == 0) {
                return Err(SetupError::EmptySide(side));
            }
        }
        let mut rng = BattleRng::from_seed(seed);
        let drawn = Weather::draw(setup.season, &mut rng);
        let weather = forced_weather.unwrap_or(drawn);
        let is_siege = setup.siege.is_some();
        let mut field = Battlefield::generate_site_crossing(
            &setup.field_site(),
            setup.field_crossing(),
            scale.field,
            weather,
            &mut rng,
        );
        if !is_siege {
            // EP6: countryside and camps (derived stream), then the hand-made
            // decor of a historical map.
            // A bare field (`bare_field`: labs, tests) keeps its
            // camps only.
            if setup.bare_field {
                field.lay_camps(&rng);
            } else {
                field.lay_decor(&setup.province, &rng);
            }
            if let Some(plan) = &setup.decor_plan {
                field.apply_decor_plan(plan);
            }
        }
        let mut siege = setup.siege.as_ref().map(|s| {
            SiegeWorks::for_battle(
                s.fortification,
                s.breach,
                setup.siege_layout.as_ref(),
                &mut rng,
            )
        });
        let mut fire = fire::FireSystem::new(seed, is_siege);
        if let Some(works) = siege.as_mut() {
            fire.prepare(works);
            // BR3: props of the suburbs too (deterministic, no draw).
            works.lay_props();
        }
        if let Some(works) = siege.as_ref() {
            field.prepare_for_siege_around(if works.landmark.is_some() {
                works.outer_radius()
            } else {
                crate::siege::RING_RADIUS
            });
        }
        let mut units = Vec::new();
        for side in SideId::BOTH {
            let side_setup = setup.side(side);
            let general = side_setup.general.as_ref();
            for (index, unit_setup) in side_setup.units.iter().enumerate() {
                if unit_setup.soldiers == 0 {
                    continue;
                }
                let mut unit = Unit::from_setup(units.len() as u32, side, index, unit_setup);
                if let Some(general) = general {
                    unit.morale_cap = (unit.morale_cap + general.morale_bonus).clamp(0.0, 100.0);
                    unit.morale = (unit.morale + general.morale_bonus).clamp(0.0, 100.0);
                    unit.is_general = general.unit_index == index;
                }
                units.push(unit);
            }
        }
        let dismount_order = setup
            .orders
            .iter()
            .find(|o| o.kind == data_model::BattleOrderKind::Dismount)
            .cloned();
        if is_siege {
            // Men-at-arms who can fight on foot (`dismount`) leave their
            // horses for the assault, as at every medieval escalade (same
            // rule as the "pied à terre" order, without its armour bonus).
            let speed = dismount_order
                .as_ref()
                .and_then(|o| o.effects.speed_max)
                .unwrap_or(ASSAULT_DISMOUNT_SPEED);
            for unit in units
                .iter_mut()
                .filter(|u| u.side == SideId::Attacker && u.mounted && u.has(Ability::Dismount))
            {
                unit.dismount(speed, 0);
            }
            let siege = setup.siege.as_ref().expect("siege battle");
            // NT5 (N7): the ram only when built (always without campaign
            // engines), and the siege towers built on the spot.
            if siege.has_ram() {
                let mut ram = Unit::from_setup(
                    units.len() as u32,
                    SideId::Attacker,
                    usize::MAX,
                    &ram_setup(),
                );
                ram.synthetic = true;
                ram.ram = true;
                units.push(ram);
            }
            for tower in siege.built_towers() {
                let mut unit =
                    Unit::from_setup(units.len() as u32, SideId::Attacker, usize::MAX, tower);
                unit.synthetic = true;
                units.push(unit);
            }
        }
        let ai_enabled = match setup.player_side {
            Some(SideId::Attacker) => [false, true],
            Some(SideId::Defender) => [true, false],
            None => [true, true],
        };
        let general_alive = [
            units
                .iter()
                .any(|u| u.side == SideId::Attacker && u.is_general),
            units
                .iter()
                .any(|u| u.side == SideId::Defender && u.is_general),
        ];
        let count = units.len();
        let standard_rng = rng.derive(standards::STANDARD_SALT);
        let standard_rules = Arc::new(setup.standards.clone().unwrap_or_default());
        let mut sim = BattleSim {
            setup: Arc::new(setup),
            field: Arc::new(field),
            weather,
            units,
            rng,
            elapsed: 0.0,
            ticks: 0,
            accumulator: 0.0,
            ai_enabled,
            hold: [false; 2],
            end_conditions: true,
            finished: false,
            winner: None,
            general_alive,
            general_killed: [false; 2],
            general_captured: [false; 2],
            events: Vec::new(),
            events_read: 0,
            alerts: Vec::new(),
            alerts_read: 0,
            flanked_alerted: vec![false; count],
            shots: std::collections::VecDeque::new(),
            charge_announced: vec![false; count],
            impacts: std::collections::VecDeque::new(),
            siege,
            square_announced: false,
            square_threatened: false,
            order_uses: Default::default(),
            no_quarter: [false; 2],
            deploying: false,
            path_cache: Default::default(),
            obstacle_cache: Default::default(),
            relief_map: Default::default(),
            fire,
            assault: Default::default(),
            scale,
            standard_rules,
            push_rules: crate::push::PushRules::bundled().clone(),
            standard_rng,
            standard_rout_seen: vec![false; count],
            trophies: Vec::new(),
            crossings: Default::default(),
            road_index: Default::default(),
            drown_announced: Vec::new(),
            camp_states: Default::default(),
            decor_grid: Default::default(),
            start_hour: crate::time_of_day::TimeOfDayRules::bundled().default_hour,
            day_phase: None,
            decision: crate::decision::DecisionRules::bundled().clone(),
            duel: crate::duel::DuelRules::bundled().clone(),
            rout: crate::rout::RoutRules::bundled().clone(),
            pace: crate::pace::PaceRules::bundled().clone(),
            approach_gap: 0.0,
            clock: Default::default(),
            end: None,
            scenario: None,
            ambush: None,
        };
        sim.hold_reserves();
        if sim.siege.is_some() {
            sim.deploy_siege();
        } else {
            sim.deploy();
            sim.apply_opening();
        }
        let text = match sim.weather {
            Weather::Clear => "Le ciel est dégagé sur le champ de bataille.".to_owned(),
            Weather::Rain => "Il pleut : les cordes des arcs se détendent.".to_owned(),
            Weather::Fog => "Un épais brouillard couvre le champ de bataille.".to_owned(),
            Weather::Snow => "La neige tombe sur le champ de bataille.".to_owned(),
        };
        sim.log(text, None);
        sim.log_opening();
        if let Some(works) = &sim.siege {
            let open = works.openings().len();
            let text = if open > 0 {
                format!("Siège : {open} brèche(s) déjà ouverte(s) dans l'enceinte.")
            } else {
                "Siège : les murailles sont intactes ; échelles, tours et bélier sont prêts."
                    .to_owned()
            };
            sim.log(text, None);
            let dismounted = sim
                .units
                .iter()
                .any(|u| u.side == SideId::Attacker && u.dismounted);
            if dismounted {
                let of = of_faction(&sim.setup.attacker.faction_name);
                let text = match dismount_order.and_then(|o| o.journal_assault) {
                    Some(template) => template.replace("{of_faction}", &of),
                    None => format!("Les chevaliers {of} mettent pied à terre pour l'assaut."),
                };
                sim.log(text, Some(SideId::Attacker));
            }
        }
        Ok(sim)
    }

    /// Initial field deployment: each side in « Ligne de bataille » (CB6,
    /// `data/rules/group_formations.json`), rows centred on the field with
    /// +x as the lateral axis on both sides (the placement before CB6).
    pub(super) fn deploy(&mut self) {
        let rules = crate::group_formation::GroupFormationRules::bundled();
        let preset = rules.battle_line();
        for side in SideId::BOTH {
            let (line_z, facing, back) = match side {
                SideId::Attacker => (self.field.attacker_line_z(), 0.0, -1.0),
                SideId::Defender => (self.field.defender_line_z(), std::f64::consts::PI, 1.0),
            };
            let ids: Vec<usize> = (0..self.units.len())
                .filter(|&i| self.units[i].side == side && !self.units[i].reserve)
                .collect();
            let frame = crate::group_formation::Frame::with_axes(
                self.field.size.center_x(),
                line_z,
                (1.0, 0.0),
                (0.0, -back),
                facing,
            );
            let placed = crate::group_formation::layout(
                rules,
                preset,
                &self.units,
                &ids,
                frame,
                crate::group_formation::LayoutOptions {
                    field_width: self.field.width,
                    separate_general: false,
                    wings: crate::group_formation::WingFill::Alternate,
                },
            );
            for p in placed {
                let unit = &mut self.units[p.index];
                unit.x = p.x;
                unit.z = p.z;
                unit.facing = p.facing;
            }
        }
    }

    /// Cavalry on the wings of a front `front_width` wide, alternating right
    /// and left.
    pub(super) fn place_wings(&mut self, cavalry: &[usize], front_width: f64, z: f64) {
        let center = self.field.size.center_x();
        let mut left = center - front_width * 0.5 - 20.0;
        let mut right = center + front_width * 0.5 + 20.0;
        for (k, &i) in cavalry.iter().enumerate() {
            let (w, _) = self.units[i].extent();
            let x = if k % 2 == 0 {
                right += w * 0.5;
                let x = right;
                right += w * 0.5 + 12.0;
                x
            } else {
                left -= w * 0.5;
                let x = left;
                left -= w * 0.5 + 12.0;
                x
            };
            let unit = &mut self.units[i];
            unit.x = x.clamp(30.0, self.field.width - 30.0);
            unit.z = z;
        }
    }

    pub(super) fn side_units(&self, side: SideId, keep: impl Fn(&Unit) -> bool) -> Vec<usize> {
        (0..self.units.len())
            .filter(|&i| {
                self.units[i].side == side && !self.units[i].reserve && keep(&self.units[i])
            })
            .collect()
    }

    /// Siege deployment (M8 § 2): the besiegers south of the walls (shooters
    /// ahead of the infantry, towers and ram in front, engines behind); the
    /// garrison on the wall walk facing out (shooters first), a guard behind
    /// the gate and a reserve in the central square.
    pub(super) fn deploy_siege(&mut self) {
        let works = self.siege.clone().expect("siege battle");
        let front_z = works
            .vertices
            .iter()
            .map(|v| v.1)
            .fold(f64::INFINITY, f64::min);
        // Besiegers.
        let a = SideId::Attacker;
        for i in self.side_units(a, |_| true) {
            self.units[i].facing = 0.0;
        }
        let infantry = self.side_units(a, |u| u.category == UnitCategory::Infantry);
        let shooters = self.side_units(a, |u| u.category == UnitCategory::Ranged && !u.mounted);
        let cavalry = self.side_units(a, |u| {
            u.category == UnitCategory::Cavalry || (u.mounted && u.category == UnitCategory::Ranged)
        });
        let towers = self.side_units(a, Unit::siege_tower);
        let engines = self.side_units(a, |u| {
            u.category == UnitCategory::Siege && !u.siege_tower() && !u.ram
        });
        let rams = self.side_units(a, |u| u.ram);
        // Out of bowshot of the wall walk; towers and ram closer in.
        let width = self.place_row(&infantry, front_z - 265.0, -1.0);
        self.place_row(&shooters, front_z - 215.0, -1.0);
        self.place_wings(&cavalry, width.max(200.0), front_z - 280.0);
        self.place_row(&engines, front_z - 240.0, -1.0);
        let front = works.front_walls();
        for (k, &i) in towers.iter().enumerate() {
            let (mx, mz) = works.pieces[front[k % front.len()]].midpoint();
            let unit = &mut self.units[i];
            unit.x = mx + 15.0 * (k / front.len()) as f64;
            unit.z = mz - 70.0;
        }
        let (gx, gz) = works.pieces[works.gate].midpoint();
        for &i in &rams {
            self.units[i].x = gx;
            self.units[i].z = gz - 110.0;
        }
        // Garrison.
        let d = SideId::Defender;
        for i in self.side_units(d, |_| true) {
            self.units[i].facing = std::f64::consts::PI;
        }
        let wall_shooters =
            self.side_units(d, |u| u.category == UnitCategory::Ranged && !u.mounted);
        let foot = self.side_units(d, |u| u.category == UnitCategory::Infantry && !u.mounted);
        let others = self.side_units(d, |u| {
            u.mounted || matches!(u.category, UnitCategory::Cavalry | UnitCategory::Siege)
        });
        let reserve_count = if foot.len() >= 2 {
            foot.len().div_ceil(3)
        } else {
            0
        };
        let (reserve, wall_foot) = foot.split_at(reserve_count);
        let mut on_walls = wall_shooters;
        on_walls.extend_from_slice(wall_foot);
        let mut order = front.clone();
        let mut rest: Vec<usize> = (0..works.pieces.len())
            .filter(|p| works.pieces[*p].kind == PieceKind::Wall && !front.contains(p))
            .collect();
        rest.sort_by(|&x, &y| {
            works.pieces[x]
                .midpoint()
                .1
                .total_cmp(&works.pieces[y].midpoint().1)
                .then(x.cmp(&y))
        });
        order.extend(rest);
        let mut used = vec![4.0; works.pieces.len()];
        let mut leftover: Vec<usize> = reserve.to_vec();
        for i in on_walls {
            let (w, _) = self.units[i].extent();
            let slot = order.iter().copied().find(|&p| {
                works.pieces[p].intact() && used[p] + w + 4.0 <= works.pieces[p].length()
            });
            let Some(p) = slot else {
                leftover.push(i);
                continue;
            };
            let piece = &works.pieces[p];
            let (tx, tz) = piece.tangent();
            let (nx, nz) = piece.outward();
            let along = used[p] + w * 0.5;
            used[p] += w + 6.0;
            let inset = works.thickness * 0.25;
            let unit = &mut self.units[i];
            unit.x = piece.a.0 + tx * along - nx * inset;
            unit.z = piece.a.1 + tz * along - nz * inset;
            unit.facing = angle_to(nx, nz);
            unit.on_wall = true;
        }
        // A guard behind the gate, the rest in the square.
        leftover.sort_unstable();
        let (nx, nz) = works.pieces[works.gate].outward();
        let mut square: Vec<usize> = Vec::new();
        for (k, &i) in leftover.iter().enumerate() {
            if k == 0 && self.units[i].category == UnitCategory::Infantry {
                let unit = &mut self.units[i];
                unit.x = gx - nx * 30.0;
                unit.z = gz - nz * 30.0;
                unit.facing = angle_to(nx, nz);
            } else {
                square.push(i);
            }
        }
        let (_, cz) = works.center;
        self.place_row(&square, cz - 15.0, 1.0);
        self.place_row(&others, cz + 30.0, 1.0);
    }

    /// Places `row` side by side, centred on the field, wrapping into extra rows
    /// behind when wider than the field. Returns the width of the first row.
    pub(super) fn place_row(&mut self, row: &[usize], z: f64, back: f64) -> f64 {
        let gap = 12.0;
        let max_width = self.field.width - 100.0;
        let mut lines: Vec<Vec<usize>> = vec![Vec::new()];
        let mut width = 0.0;
        for &i in row {
            let (w, _) = self.units[i].extent();
            if width + w > max_width && !lines.last().is_some_and(Vec::is_empty) {
                lines.push(Vec::new());
                width = 0.0;
            }
            width += w + gap;
            lines.last_mut().expect("non-empty").push(i);
        }
        let mut first_width = 0.0;
        for (line_index, line) in lines.iter().enumerate() {
            let total: f64 = line
                .iter()
                .map(|&i| self.units[i].extent().0 + gap)
                .sum::<f64>()
                - gap;
            if line_index == 0 {
                first_width = total.max(0.0);
            }
            let mut x = self.field.size.center_x() - total * 0.5;
            for &i in line {
                let (w, _) = self.units[i].extent();
                let unit = &mut self.units[i];
                unit.x = x + w * 0.5;
                unit.z = z + back * 30.0 * line_index as f64;
                x += w + gap;
            }
        }
        first_width
    }
}
