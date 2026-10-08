//! Contacts, speed, advance, climbing, siege works and unit movement.

use super::*;

/// Regiments further apart than this (centre to centre) never touch (metres).
const CONTACT_RANGE: f64 = 250.0;

impl BattleSim {
    /// For each unit, the enemy units in contact with it (in id order).
    pub(super) fn contacts(&self) -> Vec<Vec<usize>> {
        let mut result = vec![Vec::new(); self.units.len()];
        for (i, j) in self.near_pairs(CONTACT_RANGE, Unit::present) {
            let (a, b) = (&self.units[i], &self.units[j]);
            if a.side == b.side {
                continue;
            }
            let (dx, dz) = (b.x - a.x, b.z - a.z);
            let dist = (dx * dx + dz * dz).sqrt();
            let dir = if dist > 1e-6 {
                (dx / dist, dz / dist)
            } else {
                (0.0, 1.0)
            };
            let gap_ab = a.distance_to_rect(b.x, b.z) - b.support(dir);
            let gap_ba = b.distance_to_rect(a.x, a.z) - a.support(dir);
            if gap_ab.max(gap_ba) < CONTACT_GAP && !self.wall_between(a, b) {
                result[i].push(j);
                result[j].push(i);
            }
        }
        result
    }

    /// An intact wall separates the two regiments for melee: only climbers
    /// fight the defenders above them, and regiments on the wall walk fight
    /// each other.
    pub(super) fn wall_between(&self, a: &Unit, b: &Unit) -> bool {
        let Some(works) = &self.siege else {
            return false;
        };
        if works.crosses_intact((a.x, a.z), (b.x, b.z)).is_none() {
            return false;
        }
        let bridged = (a.climbing.is_some() && b.on_wall)
            || (b.climbing.is_some() && a.on_wall)
            || (a.on_wall && b.on_wall);
        !bridged
    }

    pub(super) fn nearest_enemy(&self, index: usize, able_only: bool) -> Option<(usize, f64)> {
        let unit = &self.units[index];
        self.units
            .iter()
            .enumerate()
            .filter(|(_, e)| e.side != unit.side && e.present() && (!able_only || e.able()))
            .map(|(j, e)| (j, ((e.x - unit.x).powi(2) + (e.z - unit.z).powi(2)).sqrt()))
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
    }

    /// Movement speed of `unit` heading along `dir`, in m/s.
    pub(super) fn speed(&self, unit: &Unit, dir: (f64, f64)) -> f64 {
        let mut speed = f64::from(unit.stats.speed) * 0.04;
        let pace = self.pace();
        if unit.state != UnitState::Routing
            && !unit.running
            && unit.state != UnitState::Charging
            && pace.move_speed_factor != 1.0
            && self.approach_gap > pace.approach_range_m
        {
            speed *= pace.move_speed_factor;
        }
        if unit.siege_tower() {
            // Pushed along by the assault troops.
            speed = speed.max(0.55);
        }
        let routing = unit.state == UnitState::Routing;
        if unit.running || routing {
            if !routing {
                speed *= self.pace().run_speed_factor;
            }
            speed *= if unit.is_cavalry() && unit.state == UnitState::Charging {
                2.5
            } else {
                2.0
            };
            // CB2: a run under the run mode (1 in the bundled data).
            speed *= unit.run_mode_speed();
        }
        // RJ-a: formation pace, slower while reforming.
        speed *= unit.formation_speed();
        if self.field.in_forest(unit.x, unit.z) {
            speed *= if unit.mounted { 0.4 } else { 0.65 };
        }
        if self.field.in_mud(unit.x, unit.z) {
            speed *= if self.weather == Weather::Rain {
                0.45
            } else {
                0.55
            };
        }
        // EP3: fords, deep water, streams, banks, bridges, roads.
        speed *= self.water_speed(unit);
        if self.weather == Weather::Snow {
            speed *= 0.8;
        }
        speed *= self
            .field
            .site_speed_factor(unit.x, unit.z, unit.mounted, self.weather);
        let here = self.field.height(unit.x, unit.z);
        let ahead = self
            .field
            .height(unit.x + dir.0 * 3.0, unit.z + dir.1 * 3.0);
        let grade = (ahead - here) / 3.0;
        speed *= if grade > 0.0 {
            1.0 / (1.0 + grade * 6.0)
        } else {
            1.0 + (-grade).min(0.1)
        };
        let speed = speed * (1.0 - unit.fatigue / 200.0);
        // CB4: close ranks walk slower.
        let speed = speed * self.ability_effects(unit).map_or(1.0, |e| e.speed_factor);
        // CB1: a `match_speed` group keeps the pace of its slowest regiment.
        if unit.match_speed {
            speed * self.group_pace_factor(unit)
        } else {
            speed
        }
    }

    pub(super) fn turn_rate(unit: &Unit) -> f64 {
        let degrees: f64 = if unit.mounted { 40.0 } else { 15.0 };
        degrees.to_radians() * DT
    }

    /// Moves `index` towards `(tx, tz)`; returns the remaining distance.
    pub(super) fn advance(&mut self, index: usize, tx: f64, tz: f64, may_leave: bool) -> f64 {
        let unit = &self.units[index];
        let (dx, dz) = (tx - unit.x, tz - unit.z);
        let dist = (dx * dx + dz * dz).sqrt();
        if dist < 1e-6 {
            return 0.0;
        }
        let dir = (dx / dist, dz / dist);
        let step = (self.speed(unit, dir) * DT).min(dist);
        let heading = angle_to(dx, dz);
        let (width, depth) = (self.field.width, self.field.depth);
        let from = (unit.x, unit.z);
        let to = (unit.x + dir.0 * step, unit.z + dir.1 * step);
        let blocked = self.wall_block(index, from, to);
        let in_house = blocked.is_none()
            && (self.house_block(index, from, to)
                || (!may_leave && self.water_blocks(index, from, to)));
        let unit = &mut self.units[index];
        unit.blocked_by = blocked;
        if blocked.is_some() || in_house {
            unit.facing = turn_towards(unit.facing, heading, Self::turn_rate(unit) * 4.0);
            return dist;
        }
        unit.x = to.0;
        unit.z = to.1;
        if !may_leave {
            unit.x = unit.x.clamp(1.0, width - 1.0);
            unit.z = unit.z.clamp(1.0, depth - 1.0);
        }
        let rate = if unit.state == UnitState::Routing {
            std::f64::consts::PI
        } else {
            Self::turn_rate(unit) * 4.0
        };
        unit.facing = turn_towards(unit.facing, heading, rate);
        unit.still_time = 0.0;
        unit.stakes_planted = false;
        dist - step
    }

    /// The intact wall piece that stops a move from `from` to `to`, if any.
    /// Routing regiments slip through posterns and are never stopped;
    /// defenders step onto the wall walk from inside but not beyond it;
    /// attackers on the wall walk go where they please.
    pub(super) fn wall_block(
        &self,
        index: usize,
        from: (f64, f64),
        to: (f64, f64),
    ) -> Option<usize> {
        let works = self.siege.as_ref()?;
        let unit = &self.units[index];
        if unit.state == UnitState::Routing || unit.left_field {
            return None;
        }
        let defender_inside =
            unit.side == SideId::Defender && (unit.on_wall || works.inside(from.0, from.1));
        let band = works.band();
        for (k, piece) in works.pieces.iter().enumerate() {
            if !piece.intact() {
                continue;
            }
            // Sortie (F5a): the garrison opens its gate for its own regiments.
            if works.sortie && unit.side == SideId::Defender && piece.kind == PieceKind::Gate {
                continue;
            }
            if defender_inside {
                // Never further out than the middle of the wall walk.
                let out_to = piece.outside_offset(to.0, to.1);
                if piece.distance(to.0, to.1) < band
                    && out_to > 0.0
                    && out_to > piece.outside_offset(from.0, from.1)
                {
                    return Some(k);
                }
                continue;
            }
            if unit.on_wall {
                continue;
            }
            // Stopped only when closing in on the wall face (sliding along
            // it or backing away is free).
            let d_to = piece.distance(to.0, to.1);
            // Distance to the stretch itself (F5a): rounding the jamb of a
            // breach or gate is free.
            let closing = d_to < piece.distance(from.0, from.1) - 1e-9;
            if piece.crossed_by(from, to) || (d_to < band && closing) {
                return Some(k);
            }
        }
        None
    }

    /// A regiment stopped by an intact wall: foot soldiers of the attacker
    /// raise their ladders (or cross from a docked siege tower).
    pub(super) fn start_climb(&mut self, i: usize) {
        let Some(piece) = self.units[i].blocked_by else {
            return;
        };
        let unit = &self.units[i];
        if unit.side != SideId::Attacker || !unit.can_climb() || unit.climbing.is_some() {
            return;
        }
        let Some(works) = &self.siege else {
            return;
        };
        let tower = works.pieces[piece].docked_tower.is_some_and(|t| {
            let t = &self.units[t as usize];
            (t.x - unit.x).powi(2) + (t.z - unit.z).powi(2) < 40.0 * 40.0
        });
        // NT5 (N7): without ladders built, only a docked tower lets them up.
        if !tower && !self.setup.siege.as_ref().is_none_or(|s| s.has_ladders()) {
            return;
        }
        let unit = &mut self.units[i];
        unit.climbing = Some(piece);
        unit.climb_progress = 0.0;
        unit.state = UnitState::Climbing;
        let text = if tower {
            format!(
                "Les {} s'élancent de la tour de siège sur le rempart.",
                self.unit_label(i)
            )
        } else {
            format!(
                "Les {} dressent leurs échelles contre la muraille.",
                self.unit_label(i)
            )
        };
        let side = self.units[i].side;
        self.log(text, Some(side));
        if !tower {
            let unit = self.units[i].id;
            self.push_fx(crate::siege_fx::SiegeFxKind::LaddersRaised { unit, piece });
        }
    }

    /// One step of climbing; on the top the regiment stands on the wall walk
    /// and resumes its order.
    pub(super) fn progress_climb(&mut self, i: usize, piece: usize, engaged: bool) {
        let Some(works) = &self.siege else {
            return;
        };
        let p = &works.pieces[piece];
        if !p.intact() {
            // The wall came down under them: walk through the breach.
            let unit = &mut self.units[i];
            unit.climbing = None;
            unit.climb_progress = 0.0;
            unit.state = UnitState::Marching;
            return;
        }
        let unit = &self.units[i];
        let tower = p.docked_tower.is_some_and(|t| {
            let t = &self.units[t as usize];
            (t.x - unit.x).powi(2) + (t.z - unit.z).powi(2) < 40.0 * 40.0
        });
        let duration = if tower {
            siege::TOWER_CLIMB_TIME
        } else {
            siege::LADDER_TIME
        };
        let mut rate = DT / duration * (1.0 - unit.fatigue / 200.0);
        if engaged {
            rate *= 0.25;
        }
        let (cx, cz) = p.closest_point(unit.x, unit.z);
        let (nx, nz) = p.outward();
        let inset = works.thickness * 0.25;
        let unit = &mut self.units[i];
        unit.climb_progress += rate;
        if unit.climb_progress >= 1.0 {
            unit.climbing = None;
            unit.climb_progress = 0.0;
            unit.on_wall = true;
            unit.x = cx - nx * inset;
            unit.z = cz - nz * inset;
            unit.state = UnitState::Marching;
            self.log_unit(i, |label| {
                format!("Les {label} prennent pied sur le rempart !")
            });
            let unit = self.units[i].id;
            self.push_fx(crate::siege_fx::SiegeFxKind::OnWall { unit, piece });
        }
    }

    /// Siege battles, after movement: wall-walk status, docked towers, the
    /// ram against the gate, and the hold of the central square.
    pub(super) fn resolve_siege_works(&mut self) {
        let Some(works) = self.siege.as_mut() else {
            return;
        };
        let band = works.band();
        let mut logs: Vec<(String, Option<SideId>)> = Vec::new();
        // Wall walk.
        for unit in self.units.iter_mut() {
            if !unit.present() {
                continue;
            }
            if unit.state == UnitState::Routing {
                unit.on_wall = false;
                continue;
            }
            let near = works.nearest_intact(unit.x, unit.z);
            unit.on_wall = match unit.side {
                SideId::Defender => near.is_some_and(|(p, d)| {
                    d < band + 1.0 && works.pieces[p].outside_offset(unit.x, unit.z) <= 0.5
                }),
                SideId::Attacker => unit.on_wall && near.is_some_and(|(_, d)| d < band + 3.0),
            };
        }
        // Siege towers dock against the wall they touch; SB: the "under
        // attack" marks wear off.
        for piece in works.pieces.iter_mut() {
            piece.docked_tower = None;
            piece.attacked_for = (piece.attacked_for - DT).max(0.0);
        }
        for unit in self.units.iter() {
            if !unit.siege_tower() || !unit.able() {
                continue;
            }
            if let Some((p, d)) = works.nearest_intact(unit.x, unit.z) {
                if d < band + 4.0 && works.pieces[p].kind == PieceKind::Wall {
                    works.pieces[p].docked_tower = Some(unit.id);
                }
            }
        }
        // The ram batters the gate, one blow every `RAM_PERIOD` seconds (SG1).
        let gate = works.gate;
        let mut blows: Vec<(u32, bool)> = Vec::new();
        for (index, unit) in self.units.iter_mut().enumerate() {
            if !unit.ram {
                continue;
            }
            let at_gate = unit.able()
                && works.pieces[gate].intact()
                && works.pieces[gate].distance(unit.x, unit.z) < band + 4.0;
            if at_gate {
                works.pieces[gate].mark_attacked(siege::UNDER_ATTACK_CONTACT_S);
            }
            let Some(seconds) = Self::ram_blow(&mut self.assault.ram_timers, index, at_gate) else {
                continue;
            };
            {
                let crew = unit.hp / f64::from(unit.initial_soldiers.max(1));
                works.pieces[gate].hp -=
                    siege::SiegeWorkRules::bundled().ram.damage_per_s * crew * seconds;
                blows.push((unit.id, works.pieces[gate].hp <= 0.0));
                if works.pieces[gate].hp <= 0.0 {
                    works.pieces[gate].hp = 0.0;
                    logs.push((
                        "La porte cède sous les coups du bélier !".to_owned(),
                        Some(SideId::Attacker),
                    ));
                }
            }
        }
        for (text, side) in logs {
            self.log(text, side);
        }
        for (unit, breached) in blows {
            self.push_fx(crate::siege_fx::SiegeFxKind::RamStrike {
                unit,
                piece: gate,
                breached,
            });
        }
    }

    pub(super) fn resolve_movement(&mut self, contacts: &[Vec<usize>]) {
        for i in 0..self.units.len() {
            if !self.units[i].present() {
                continue;
            }
            let state = self.units[i].state;
            // Routing: flee towards the own edge. EP10: in a field battle,
            // straight to the rear of the army, swerving round the enemies
            // and the friends on the way; in a siege, also away from the
            // nearest enemy (unchanged).
            if state == UnitState::Routing {
                let (fx, fz) = self.flight_direction(i);
                let (x, z) = (self.units[i].x, self.units[i].z);
                self.advance(i, x + fx * 50.0, z + fz * 50.0, true);
                self.check_left_field(i);
                continue;
            }
            if self.units[i].withdrawing {
                if let Some((tx, tz)) = self.units[i].destination {
                    // In a siege, round the walls and the houses (F5a
                    // pathing; no ladders on the way out, and the wall
                    // walk is left straight inwards as before).
                    let (gx, gz) = if self.siege.is_some() && !self.units[i].on_wall {
                        self.grid_route(i, tx, tz).unwrap_or((tx, tz))
                    } else {
                        (tx, tz)
                    };
                    self.advance(i, gx, gz, true);
                }
                self.check_left_field(i);
                continue;
            }
            if self.units[i].rally_timer > 0.0 {
                self.units[i].rally_timer -= DT;
                if self.units[i].rally_timer <= 0.0 && self.units[i].state == UnitState::Rallied {
                    self.units[i].state = UnitState::Idle;
                }
            }
            let in_contact = !contacts[i].is_empty();
            if let Some(piece) = self.units[i].climbing {
                self.progress_climb(i, piece, in_contact);
                continue;
            }
            if in_contact && !self.units[i].disengaging {
                self.enter_melee(i, &contacts[i]);
                continue;
            }
            if !in_contact {
                self.units[i].disengaging = false;
            }
            if let Some(target) = self.units[i].target {
                let t = target as usize;
                // CB-M3: a target gone ends the attack; with orders queued,
                // so does a target in flight (the next order starts).
                let fleeing = self.units[t].state == UnitState::Routing
                    && !self.units[i].order_queue.is_empty();
                // CB2: a regiment on guard does not pursue.
                if !self.units[t].present() || fleeing || self.guard_releases(i, t, in_contact) {
                    self.units[i].target = None;
                    self.units[i].state = UnitState::Idle;
                    self.next_queued(i);
                    continue;
                }
                let (tx, tz) = (self.units[t].x, self.units[t].z);
                let unit = &self.units[i];
                let dist = ((tx - unit.x).powi(2) + (tz - unit.z).powi(2)).sqrt();
                // A target hidden (forest, walls, crest) is closed in on,
                // not waited for within range.
                if unit.shoots()
                    && unit.ammo > 0
                    && dist <= self.effective_range(unit, tx, tz)
                    && self.visible(unit, &self.units[t], dist)
                {
                    let unit = &mut self.units[i];
                    unit.state = UnitState::Shooting;
                    unit.facing = turn_towards(
                        unit.facing,
                        angle_to(tx - unit.x, tz - unit.z),
                        Self::turn_rate(unit) * 2.0,
                    );
                    continue;
                }
                if unit.pavise.is_some()
                    && unit.shoots()
                    && unit.ammo > 0
                    && dist <= self.effective_range(unit, tx, tz)
                {
                    // RS-J: within bowshot but out of sight (the shooting
                    // case returned above): the pavises come down and the
                    // crossbowmen close in until they see it, like the
                    // other shooters.
                    self.units[i].pavise = None;
                } else if self.units[i].pavise.is_some() {
                    // Behind the pavises: wait for the target to come in range.
                    if self.units[i].state != UnitState::Shooting {
                        self.units[i].state = UnitState::Idle;
                    }
                    continue;
                }
                let unit = &self.units[i];
                let charge_distance = if unit.is_cavalry() { 120.0 } else { 40.0 };
                let charging = unit.running && dist < charge_distance && !unit.shoots();
                if charging && self.units[i].state != UnitState::Charging {
                    self.units[i].state = UnitState::Charging;
                    if self.units[i].is_cavalry() && !self.charge_announced[i] {
                        self.charge_announced[i] = true;
                        self.log_unit(i, |label| format!("Les {label} chargent !"));
                    }
                } else if !charging {
                    self.units[i].state = UnitState::Marching;
                }
                let (gx, gz) = self.route(i, tx, tz);
                self.advance(i, gx, gz, false);
                self.start_climb(i);
                continue;
            }
            if let Some((tx, tz)) = self.units[i].destination {
                let (gx, gz) = self.route(i, tx, tz);
                self.advance(i, gx, gz, false);
                self.start_climb(i);
                let unit = &self.units[i];
                let remaining = ((tx - unit.x).powi(2) + (tz - unit.z).powi(2)).sqrt();
                if self.units[i].climbing.is_some() {
                    continue;
                }
                if remaining < 1.5 {
                    let unit = &mut self.units[i];
                    unit.destination = None;
                    unit.running = false;
                    unit.state = UnitState::Idle;
                    if let Some(facing) = unit.destination_facing.take() {
                        unit.facing = facing;
                    }
                    self.next_queued(i);
                } else if !self.units[i].disengaging {
                    // Out of contact after breaking off a melee: marching
                    // again (still `Melee` while pulling out under blows).
                    self.units[i].state = UnitState::Marching;
                }
                continue;
            }
            // CB-M3: an order cut short (a melee halts the march) leaves
            // the queue: the next order starts.
            if !self.units[i].order_queue.is_empty() && self.next_queued(i) {
                continue;
            }
            // Standing still: idle (shooting is decided later).
            let unit = &mut self.units[i];
            if matches!(
                unit.state,
                UnitState::Marching | UnitState::Charging | UnitState::Melee
            ) {
                unit.state = UnitState::Idle;
            }
            unit.still_time += DT;
            if unit.has(Ability::Stakes) && !unit.stakes_planted && unit.still_time >= STAKES_DELAY
            {
                unit.stakes_planted = true;
                self.log_unit(i, |label| format!("Les {label} plantent leurs pieux."));
            }
        }
    }

    /// EP10: unit vector from the front to the rear of `side`'s army (the
    /// attacker holds the low-z edge, the defender the high-z one, on the
    /// generated fields as on the historical maps).
    pub(super) fn rear_of(side: SideId) -> (f64, f64) {
        match side {
            SideId::Attacker => (0.0, -1.0),
            SideId::Defender => (0.0, 1.0),
        }
    }

    /// Direction of flight of the routing regiment `i` (not normalised in
    /// a siege, as before EP10).
    pub(super) fn flight_direction(&self, i: usize) -> (f64, f64) {
        let unit = &self.units[i];
        let rear = Self::rear_of(unit.side);
        if self.siege.is_some() {
            let (mut fx, mut fz) = rear;
            if let Some((j, d)) = self.nearest_enemy(i, false) {
                if d > 1e-6 {
                    let e = &self.units[j];
                    fx += (unit.x - e.x) / d;
                    fz += (unit.z - e.z) / d;
                }
            }
            return (fx, fz);
        }
        let reach = self
            .rout
            .flight
            .lookahead_m
            .max(self.rout.flight.friend_lookahead_m);
        let near = |u: &Unit| (u.x - unit.x).abs() < reach && (u.z - unit.z).abs() < reach;
        let enemies = self
            .units
            .iter()
            .filter(|u| u.side != unit.side && u.able() && near(u))
            .map(|u| (u.x, u.z));
        let friends = self
            .units
            .iter()
            .enumerate()
            .filter(|&(j, u)| j != i && u.side == unit.side && u.able() && near(u))
            .map(|(_, u)| (u.x, u.z));
        self.rout
            .flight
            .direction((unit.x, unit.z), rear, enemies, friends)
    }

    pub(super) fn check_left_field(&mut self, i: usize) {
        let unit = &self.units[i];
        let margin = 5.0;
        if unit.x < -margin
            || unit.z < -margin
            || unit.x > self.field.width + margin
            || unit.z > self.field.depth + margin
        {
            self.units[i].left_field = true;
            self.log_unit(i, |label| {
                format!("Les {label} quittent le champ de bataille.")
            });
        }
    }
}
