//! Missile fire at units and walls.

use super::*;

impl BattleSim {
    pub(super) fn resolve_shooting(&mut self, contacts: &[Vec<usize>]) {
        for i in 0..self.units.len() {
            let unit = &self.units[i];
            if !unit.present()
                || !unit.shoots()
                || unit.ammo == 0
                || !contacts[i].is_empty()
                || matches!(
                    unit.state,
                    UnitState::Routing | UnitState::Melee | UnitState::Charging
                )
                || unit.withdrawing
            {
                continue;
            }
            let moving = unit.state == UnitState::Marching;
            if moving && !unit.shoots_on_move() {
                continue;
            }
            if self.units[i].reload > 0.0 {
                self.units[i].reload -= DT;
                continue;
            }
            if let Some(piece) = self.pick_wall_target(i) {
                self.fire_at_wall(i, piece);
                continue;
            }
            // CB2: an engine battering walls never shoots men.
            if self.units[i].breach {
                continue;
            }
            let Some(target) = self.pick_shooting_target(i) else {
                if self.units[i].state == UnitState::Shooting && self.units[i].target.is_none() {
                    self.units[i].state = UnitState::Idle;
                }
                continue;
            };
            self.fire(i, target);
        }
    }

    /// CB3: does any present, standing regiment of `side` see `target`? Reuses the missile-arc
    /// spotter range (`data/rules/missile_arc.json`, `spotter_range_m`, already the distance at
    /// which a friend directs an indirect volley) and the forest/wall/line-of-sight checks of
    /// [`Self::visible`]. Purely derived from the current state (not a field of [`Unit`]): it
    /// never enters `state_digest` or the replay format. Feeds the tactical view's fog of war
    /// (`spotted` in `get_units`); the normal view is unaffected.
    pub fn spotted_by(&self, target: &Unit, side: SideId) -> bool {
        if target.side == side {
            return true;
        }
        let rules = crate::missile_arc::MissileArcRules::bundled();
        let reach = rules.spotter_range_m * self.range_factor();
        self.units.iter().any(|u| {
            u.side == side && u.present() && !u.synthetic && u.state != UnitState::Routing && {
                let dist = (u.x - target.x).hypot(u.z - target.z);
                dist <= reach && self.visible(u, target, dist)
            }
        })
    }

    pub(crate) fn visible(&self, shooter: &Unit, target: &Unit, dist: f64) -> bool {
        if self.field.in_forest(target.x, target.z) && dist > 60.0 {
            return false;
        }
        if let Some(works) = &self.siege {
            // Walls hide what is behind them, except from (or of) the wall
            // walk; engines lob over them.
            if shooter.category != UnitCategory::Siege
                && !shooter.on_wall
                && !target.on_wall
                && target.climbing.is_none()
                && works
                    .crosses_intact((shooter.x, shooter.z), (target.x, target.z))
                    .is_some()
            {
                return false;
            }
        }
        // R4: direct at a target in sight, lobbed over a crest otherwise.
        self.fire_mode(shooter, target).is_some()
    }

    pub(super) fn pick_shooting_target(&self, i: usize) -> Option<usize> {
        let unit = &self.units[i];
        let in_range = |j: usize| -> Option<f64> {
            let t = &self.units[j];
            if t.side == unit.side || !t.present() || t.state == UnitState::Melee {
                return None;
            }
            let dist = ((t.x - unit.x).powi(2) + (t.z - unit.z).powi(2)).sqrt();
            (dist <= self.effective_range(unit, t.x, t.z) && self.visible(unit, t, dist))
                .then_some(dist)
        };
        if let Some(t) = unit.target {
            if in_range(t as usize).is_some() {
                return Some(t as usize);
            }
            // The ordered target cannot be shot (hidden, in a melee): at
            // will, the nearest enemy that can be.
            if !unit.fire_at_will {
                return None;
            }
        } else if !unit.fire_at_will {
            return None;
        }
        (0..self.units.len())
            .filter_map(|j| in_range(j).map(|d| (j, d)))
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
            .map(|(j, _)| j)
    }

    pub(super) fn fire(&mut self, i: usize, t: usize) {
        let shooter = &self.units[i];
        let target = &self.units[t];
        let dist = ((target.x - shooter.x).powi(2) + (target.z - shooter.z).powi(2)).sqrt();
        let range = self.effective_range(shooter, target.x, target.z).max(1.0);
        let engine = (shooter.category == UnitCategory::Siege)
            .then(|| &siege::SiegeWorkRules::bundled().engine);
        let mut accuracy = 0.3 * (1.0 - 0.5 * dist / range);
        if shooter.has(Ability::RainPenalty) {
            accuracy *= self.weather.bow_factor();
        }
        accuracy *= 1.0 + f64::from(shooter.experience) / 20.0;
        if let Some(general) = self.general_bonus(shooter.side) {
            accuracy *= 1.0 + general.ranged_percent / 100.0;
        }
        if shooter.on_wall {
            accuracy *= 1.25;
        }
        // CB4: the aimed shot aims better, above all at horses.
        let shooter_ability = self.ability_effects(shooter);
        if let Some(e) = shooter_ability {
            accuracy *= e.accuracy_factor;
        }
        // An engine's stone ploughs a file of men: a set toll per shot, the
        // armour barely counting; a volley of arrows scales with the archers.
        let mut kills = match engine {
            Some(rules) => {
                let crew = shooter.hp / f64::from(shooter.initial_soldiers.max(1));
                f64::from(shooter.stats.siege_attack.unwrap_or(0))
                    * rules.kills_per_siege_attack
                    * crew
                    * accuracy
                    / 0.3
                    * armor_factor(self.defense_points(target) * rules.armor_weight)
            }
            None => {
                shooter.hp * accuracy * f64::from(shooter.stats.ranged) / 100.0
                    * armor_factor(self.defense_points(target))
                    * self.pace().ranged_rate
            }
        };
        if let Some(e) = shooter_ability.filter(|_| target.mounted) {
            kills *= e.vs_mounted_factor;
        }
        if self.field.in_forest(target.x, target.z) {
            kills *= 0.5;
        }
        if self.field.in_village(target.x, target.z) {
            kills *= crate::site::VILLAGE_COVER;
        } else {
            // EP6: hamlets, churchyards, manors, orchards, vineyards, camps.
            kills *= self.field.decor_cover(target.x, target.z);
            if self
                .field
                .hedge_between((shooter.x, shooter.z), (target.x, target.z))
            {
                kills *= crate::site::HEDGE_COVER;
            }
        }
        kills *= self.missile_cover(target, attack_angle(target, shooter.x, shooter.z));
        // RJ-a: formation and reform (a schiltron is a dense target).
        kills *= target.missile_taken_factor() * shooter.formation_shooting();
        if target.on_wall && !shooter.on_wall {
            kills *= 0.5; // merlons
        }
        if self.on_ladders(target) {
            kills *= 1.5;
        }
        if target.ram || target.siege_tower() {
            kills *= 0.1; // roofed and hung with wet hides
        }
        if attack_angle(target, shooter.x, shooter.z) == 2 {
            kills *= 1.3;
        }
        kills *= self.smoke_factor(shooter, target);
        // R4: an indirect volley scatters over ground nobody aims at.
        let mode = self.fire_mode(shooter, target);
        kills *= mode.map_or(1.0, |m| {
            m.accuracy(crate::missile_arc::MissileArcRules::bundled())
        });
        let indirect = mode.is_some_and(|m| m.indirect());
        let aim = (target.x, target.z);
        let reload = shooter.reload_period() * shooter_ability.map_or(1.0, |e| e.reload_factor);
        let heading = angle_to(target.x - shooter.x, target.z - shooter.z);
        let kills = kills.min(self.units[t].hp);
        let cover = if target.on_wall {
            ShotCover::Wall
        } else if target.pavise.is_some()
            || (target.has(Ability::Pavise) && target.state != UnitState::Marching)
        {
            ShotCover::Pavise
        } else if target.stakes_planted {
            ShotCover::Stakes
        } else {
            ShotCover::None
        };
        let shot = ShotEvent {
            time: self.elapsed,
            shooter: shooter.id,
            target: Some(target.id),
            from: (shooter.x, shooter.z),
            aim: (target.x, target.z),
            missiles: Self::missiles(shooter),
            kills,
            kind: Self::missile_kind(shooter),
            incendiary: self.shoots_fire(i),
            cover,
            indirect,
        };
        self.record_shot(shot);
        if mode.is_some_and(|m| m != crate::missile_arc::FireMode::Remembered) {
            self.units[t].seen_at = self.elapsed;
        }
        let cause = Self::missile_cause(&self.units[i]);
        let shooter_id = self.units[i].id;
        self.units[t].hp -= kills;
        self.units[t].tick_losses += kills;
        self.units[i].kills += kills;
        if !self.units[t].synthetic {
            self.clock.missile_losses[self.units[t].side.index()] += kills;
        }
        // ADR 0052: the arrows wound the horses too, and they panic, unless
        // the riders are already locked in a melee.
        if kills > 0.0 {
            self.units[t].morale -= engine.map_or(0.0, |rules| rules.morale_shock);
        }
        let target = &self.units[t];
        self.units[t].morale -= crate::missile_morale::MissileMoraleRules::bundled().panic(
            target.mounted && target.state != UnitState::Melee,
            kills,
            target.max_soldiers,
        );
        if kills > 0.0 {
            self.units[t].missile_timer = 0.0;
            self.units[t].loss_cause = cause;
            self.units[t].loss_by = Some(shooter_id);
        }
        let shooter = &mut self.units[i];
        shooter.reload = reload;
        shooter.ammo -= 1;
        shooter.facing = turn_towards(shooter.facing, heading, 0.5);
        if shooter.state != UnitState::Marching {
            shooter.state = UnitState::Shooting;
        }
        if shooter.ammo == 0 {
            let missiles = if shooter.category == UnitCategory::Siege {
                "projectiles"
            } else if shooter.has(Ability::Pavise) {
                "carreaux"
            } else {
                "flèches"
            };
            self.log_unit(i, |label| {
                format!("Les {label} sont à court de {missiles}.")
            });
            let (x, z, id) = (self.units[i].x, self.units[i].z, self.units[i].id);
            let side = self.units[i].side;
            self.alert(
                crate::alerts::AlertKind::AmmoOut,
                x,
                z,
                Some(side),
                Some(id),
            );
            if self.units[i].state == UnitState::Shooting {
                self.units[i].state = UnitState::Idle;
            }
        }
        if self.units[t].hp <= 0.0 {
            self.unit_destroyed(t);
        }
        self.incendiary_volley(i, aim);
    }

    /// Wall piece an engine batters this volley: its ordered piece, else
    /// (fire at will, no unit target) the nearest wall stretch in range
    /// facing it.
    pub(super) fn pick_wall_target(&self, i: usize) -> Option<usize> {
        let works = self.siege.as_ref()?;
        let unit = &self.units[i];
        if !unit.wall_breaker() || (unit.target.is_some() && !unit.breach) {
            return None;
        }
        let range = f64::from(unit.stats.range) * self.range_factor();
        let in_range = |p: usize| {
            let piece = &works.pieces[p];
            piece.intact()
                && piece.distance(unit.x, unit.z) <= range
                && piece.outside_offset(unit.x, unit.z) > 0.0
        };
        if let Some(p) = unit.wall_target.filter(|&p| in_range(p)) {
            return Some(p);
        }
        if unit.breach {
            return self.breach_piece(i, range);
        }
        if !unit.fire_at_will {
            return None;
        }
        (0..works.pieces.len())
            .filter(|&p| works.pieces[p].kind == PieceKind::Wall && in_range(p))
            .min_by(|&a, &b| {
                let da = works.pieces[a].distance(unit.x, unit.z);
                let db = works.pieces[b].distance(unit.x, unit.z);
                da.total_cmp(&db).then(a.cmp(&b))
            })
    }

    /// An engine's shot at a wall piece; the defenders standing on it suffer
    /// a little, and all of them fall off when it comes down.
    pub(super) fn fire_at_wall(&mut self, i: usize, piece: usize) {
        let unit = &self.units[i];
        let crew = unit.hp / f64::from(unit.initial_soldiers.max(1));
        let damage = f64::from(unit.stats.siege_attack.unwrap_or(0))
            * siege::SiegeWorkRules::bundled()
                .engine
                .wall_damage_per_siege_attack
            * crew
            // CB2: battering in breach (1 otherwise).
            * unit.breach_wall_damage();
        let heading = {
            let (mx, mz) = self.siege.as_ref().expect("siege").pieces[piece].midpoint();
            angle_to(mx - unit.x, mz - unit.z)
        };
        let aim = self.siege.as_ref().expect("siege").pieces[piece].midpoint();
        let shot = ShotEvent {
            time: self.elapsed,
            shooter: unit.id,
            target: None,
            from: (unit.x, unit.z),
            aim,
            missiles: Self::missiles(unit),
            kills: 0.0,
            kind: Self::missile_kind(unit),
            incendiary: self.shoots_fire(i),
            cover: ShotCover::Wall,
            indirect: false,
        };
        self.record_shot(shot);
        let shooter = &mut self.units[i];
        shooter.reload = crate::shot::ENGINE_RELOAD * shooter.breach_reload();
        shooter.ammo = shooter.ammo.saturating_sub(1);
        shooter.facing = turn_towards(shooter.facing, heading, 0.5);
        if shooter.state != UnitState::Marching {
            shooter.state = UnitState::Shooting;
        }
        let works = self.siege.as_mut().expect("siege");
        let band = works.band();
        let p = &mut works.pieces[piece];
        p.hp = (p.hp - damage).max(0.0);
        p.mark_attacked(siege::UNDER_ATTACK_SHOT_S);
        let breached = p.hp <= 0.0;
        let kind = p.kind;
        let p = works.pieces[piece].clone();
        let mut fell = Vec::new();
        for (j, u) in self.units.iter_mut().enumerate() {
            if !u.present() || !u.on_wall || p.distance(u.x, u.z) > band + 2.0 {
                continue;
            }
            let loss = if breached { u.hp * 0.12 } else { u.hp * 0.01 };
            u.hp -= loss;
            u.tick_losses += loss;
            if breached {
                u.on_wall = false;
                u.morale -= 10.0;
                fell.push(j);
            }
        }
        if breached {
            let text = match kind {
                PieceKind::Gate => "La porte vole en éclats !".to_owned(),
                PieceKind::Wall => {
                    "Un pan de muraille s'effondre : la brèche est ouverte !".to_owned()
                }
            };
            let side = self.units[i].side;
            self.log(text, Some(side));
        }
        for j in fell {
            if self.units[j].hp <= 0.0 {
                self.unit_destroyed(j);
            }
        }
        // SG1: where the stone struck (deterministic hash, no random draw).
        let id = self.units[i].id;
        let along = 0.15 + 0.7 * crate::siege_fx::hash01(self.ticks, u64::from(id));
        let height = 0.25 + 0.6 * crate::siege_fx::hash01(self.ticks ^ 0x5eed, u64::from(id));
        self.push_fx(crate::siege_fx::SiegeFxKind::EngineShot {
            unit: id,
            piece,
            x: p.a.0 + (p.b.0 - p.a.0) * along,
            z: p.a.1 + (p.b.1 - p.a.1) * along,
            height,
            breached,
        });
        self.incendiary_volley(i, p.midpoint());
    }
}
