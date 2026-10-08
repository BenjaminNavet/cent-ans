//! Melee entry, charge impact, damage and unit/general deaths.

use super::*;

impl BattleSim {
    /// A unit touching enemies fights; a charging unit delivers its impact.
    pub(super) fn enter_melee(&mut self, i: usize, contacts: &[usize]) {
        let primary = self.primary_opponent(i, contacts);
        let was = self.units[i].state;
        let charging = was == UnitState::Charging
            || (self.units[i].running && self.units[i].target.is_some() && was != UnitState::Melee);
        self.units[i].state = UnitState::Melee;
        self.units[i].destination = None;
        self.units[i].still_time = 0.0;
        if let Some(p) = primary {
            // Reforming under attack is slow (3°/s on foot): a flank attack
            // keeps its bonus for a good while. With several opponents the
            // unit only turns towards its own target.
            if contacts.len() == 1 || self.units[i].target == Some(p as u32) {
                let (px, pz) = (self.units[p].x, self.units[p].z);
                let unit = &mut self.units[i];
                unit.facing = turn_towards(
                    unit.facing,
                    angle_to(px - unit.x, pz - unit.z),
                    Self::turn_rate(unit) * 0.2,
                );
            }
            if charging && was != UnitState::Melee {
                self.charge_impact(i, p);
            }
        }
    }

    pub(super) fn charge_impact(&mut self, i: usize, p: usize) {
        let angle = attack_angle(&self.units[p], self.units[i].x, self.units[i].z);
        let cavalry = self.units[i].is_cavalry();
        let mass = impact::charge_mass(&self.units[i]);
        let heading = angle_to(
            self.units[p].x - self.units[i].x,
            self.units[p].z - self.units[i].z,
        );
        let point = self.contact_point(i, p);
        let base = ImpactEvent {
            time: self.elapsed,
            attacker: self.units[i].id,
            defender: self.units[p].id,
            kind: ImpactKind::Shock,
            point,
            heading,
            mass,
            knocked: 0,
            unhorsed: 0,
            depth: 0.0,
            cohesion: 0.0,
        };
        if cavalry && angle == 0 && self.units[p].stakes_planted {
            let defender_id = self.units[p].id;
            let unit = &mut self.units[i];
            let loss = unit.hp * 0.12;
            unit.hp -= loss;
            unit.tick_losses += loss;
            unit.morale -= 15.0;
            unit.charge_timer = 0.0;
            unit.loss_cause = LossCause::Stakes;
            unit.loss_by = Some(defender_id);
            self.log_unit(i, |label| format!("Les {label} s'empalent sur les pieux !"));
            self.record_impact(ImpactEvent {
                kind: ImpactKind::Stakes,
                unhorsed: loss.round() as u32,
                ..base
            });
            return;
        }
        // BV2: levelled pikes stop the horses and unhorse the first riders
        // (CB4: also pikes planted against a frontal charge).
        if impact::pikes_stop(&self.units[i], &self.units[p], angle)
            || self.ability_stops_charge(&self.units[i], &self.units[p], angle)
        {
            let defender_id = self.units[p].id;
            let unit = &mut self.units[i];
            let loss = unit.hp * impact::PIKE_STOP_LOSS;
            unit.hp -= loss;
            unit.tick_losses += loss;
            unit.morale -= 10.0;
            unit.charge_timer = 0.0;
            unit.loss_cause = LossCause::Pikes;
            unit.loss_by = Some(defender_id);
            let text = format!(
                "La charge des {} se brise sur les piques des {} !",
                self.unit_label(i),
                self.unit_label(p)
            );
            let side = self.units[i].side;
            self.log(text, Some(side));
            self.record_impact(ImpactEvent {
                kind: ImpactKind::Pikes,
                unhorsed: loss.round() as u32,
                ..base
            });
            return;
        }
        // B5: a hedge or a ditch in front of the target, or the lanes of a
        // village, break the impact of horsemen.
        let (from, to) = (
            (self.units[i].x, self.units[i].z),
            (self.units[p].x, self.units[p].z),
        );
        let water = if cavalry {
            self.water_breaks_charge(from, to)
        } else {
            None
        };
        let decor = if cavalry {
            self.field.decor_breaks_charge(to.0, to.1)
        } else {
            None
        };
        if cavalry
            && (self.field.breaks_charge(from, to)
                || self.field.in_village(to.0, to.1)
                || water.is_some()
                || decor.is_some())
        {
            self.units[i].charge_timer = 0.0;
            self.units[i].morale -= 5.0;
            let text = if let Some(how) = water {
                format!("La charge des {} {how}.", self.unit_label(i))
            } else if self.field.in_village(to.0, to.1) {
                format!(
                    "La charge des {} se brise dans le village.",
                    self.unit_label(i)
                )
            } else if let Some(place) = decor {
                format!("La charge des {} se brise {place}.", self.unit_label(i))
            } else {
                format!("La charge des {} se brise sur la haie.", self.unit_label(i))
            };
            let side = self.units[i].side;
            self.log(text, Some(side));
            self.record_impact(ImpactEvent {
                kind: ImpactKind::Broken,
                ..base
            });
            return;
        }
        self.units[i].charge_timer = CHARGE_IMPACT;
        // Loss of cohesion: the shock of the horses (B-rules, unchanged).
        // CB4: close ranks take the shock better.
        let braced = self.ability_charge_taken(&self.units[p]);
        let cohesion = if cavalry && !self.units[p].braced() {
            impact::shock_morale(angle) * braced
        } else {
            0.0
        };
        self.units[p].morale -= cohesion;
        // BV2: the horses knock men down; they stop fighting until they are
        // back on their feet.
        let knocked = if cavalry && self.units[i].mounted {
            let count = impact::knocked_count(&self.units[i], &self.units[p], angle);
            if braced == 1.0 {
                count
            } else {
                (f64::from(count) * braced).round() as u32
            }
        } else {
            0
        };
        let depth = impact::drive_depth(mass, knocked, &self.units[p]);
        if knocked > 0 {
            let target = &mut self.units[p];
            target.knocked = f64::from(knocked);
            target.knocked_timer = impact::KNOCKDOWN_TIME;
        }
        self.record_impact(ImpactEvent {
            knocked,
            depth,
            cohesion,
            ..base
        });
    }

    /// Point where regiment `i` strikes regiment `p`: on `p`'s face, along
    /// the line between the two centres.
    pub(super) fn contact_point(&self, i: usize, p: usize) -> (f64, f64) {
        let (a, b) = (&self.units[i], &self.units[p]);
        let (dx, dz) = (a.x - b.x, a.z - b.z);
        let len = (dx * dx + dz * dz).sqrt();
        if len < 1e-6 {
            return (b.x, b.z);
        }
        let dir = (dx / len, dz / len);
        let reach = b.support(dir).min(len);
        (b.x + dir.0 * reach, b.z + dir.1 * reach)
    }

    /// The enemy `i` strikes among `contacts`: its target, else the nearest.
    /// The contacts are those of the start of the step: regiments killed
    /// since (missiles, fire) are skipped.
    pub(super) fn primary_opponent(&self, i: usize, contacts: &[usize]) -> Option<usize> {
        let alive = |j: usize| self.units[j].present();
        if let Some(t) = self.units[i].target {
            if contacts.contains(&(t as usize)) && alive(t as usize) {
                return Some(t as usize);
            }
        }
        let unit = &self.units[i];
        contacts
            .iter()
            .copied()
            .filter(|&j| alive(j))
            .min_by(|&a, &b| {
                let da = (self.units[a].x - unit.x).powi(2) + (self.units[a].z - unit.z).powi(2);
                let db = (self.units[b].x - unit.x).powi(2) + (self.units[b].z - unit.z).powi(2);
                da.total_cmp(&db).then(a.cmp(&b))
            })
    }

    pub(super) fn general_bonus(&self, side: SideId) -> Option<&crate::setup::GeneralSetup> {
        if self.general_alive[side.index()] {
            self.setup.side(side).general.as_ref()
        } else {
            None
        }
    }

    pub(crate) fn defense_points(&self, unit: &Unit) -> f64 {
        f64::from(unit.stats.armor)
            + self
                .general_bonus(unit.side)
                .map_or(0.0, |g| g.defense_percent)
    }

    pub(super) fn melee_damage(&self, attacker: &Unit, defender: &Unit) -> f64 {
        let mut damage = attacker.fighting_soldiers() * f64::from(attacker.stats.melee) / 100.0
            * armor_factor(self.defense_points(defender))
            * self.pace().melee_rate
            * DT
            // EP6: walls, hedges and houses of the decor shelter the defender.
            / self.field.decor_defense(defender.x, defender.z);
        if attacker.charge_timer > 0.0 {
            let charge = attacker.charge_points();
            let lance = if attacker.has(Ability::ChargeLance) {
                1.5
            } else {
                1.0
            };
            let wedge = attacker.formation_charge();
            let general = self
                .general_bonus(attacker.side)
                .map_or(0.0, |g| g.charge_percent);
            damage *= 1.0 + charge / 100.0 * lance * wedge * (1.0 + general / 100.0);
        }
        let angle = attack_angle(defender, attacker.x, attacker.z);
        damage *= match angle {
            0 => 1.0,
            1 => 1.5,
            _ => 2.0,
        };
        // CB4: close ranks, planted pikes.
        damage *= self.ability_melee_factor(attacker, defender, angle);
        // RJ-a: the defender's formation, and a change of formation under way.
        damage *= defender.melee_taken_factor();
        if defender.state == UnitState::Routing {
            damage *= 1.5;
        }
        // Matchup of the types (CB-M2: also quoted by the hover comparison).
        damage *= horse_against_foot(attacker, defender);
        damage *= pikes_against_horse(attacker, defender);
        // Siege: ladders are a poor place to fight from.
        if self.on_ladders(attacker) {
            damage *= 0.3;
        } else if attacker.climbing.is_some() {
            damage *= 0.9;
        }
        if self.on_ladders(defender) {
            damage *= 2.0;
        }
        // EP3: fords, streams, deep water, bridges and bridgeheads.
        damage *= self.water_melee_factor(attacker, defender);
        // SG4: downhill strikes harder, uphill weaker (`battle_crest.json`).
        damage *= crate::crest::CrestRules::bundled().melee_factor(
            self.field.height(attacker.x, attacker.z) - self.field.height(defender.x, defender.z),
        );
        damage *= 1.0 - attacker.fatigue / 250.0;
        damage *= 1.0 + f64::from(attacker.experience) / 20.0;
        damage *= 0.6 + attacker.morale.max(0.0) / 250.0;
        // EP5: without its standard the regiment loses its rallying point.
        if matches!(attacker.standard, crate::unit::StandardState::Fallen { .. }) {
            damage *= self.standard_rules.fallen_melee_factor;
        }
        // CV3-2: the palisade of an entrenched camp shelters its defenders.
        damage /= self.palisade_defense(attacker, defender);
        // EP11: compression and wrapping files (`sim/push.rs`).
        damage * self.push_melee_factor(attacker, defender)
    }

    pub(super) fn resolve_melee(&mut self, contacts: &[Vec<usize>]) {
        let n = self.units.len();
        let mut damage = vec![0.0; n];
        let mut flanked = vec![0u8; n];
        // CB5: rising edge of `flanked` (0 -> non-zero), collected here and
        // turned into alerts once the loop below has released its `&mut`
        // borrow of `self.units`.
        let mut newly_flanked: Vec<(f64, f64, SideId, u32)> = Vec::new();
        // BV2: heaviest blow per defender this tick (cause of its deaths).
        let mut heaviest: Vec<Option<(f64, usize)>> = vec![None; n];
        // UB1: (striker, victim, damage) to credit the kills once capped.
        let mut credit: Vec<(usize, usize, f64)> = Vec::new();
        let mut dealt_ratio = vec![0.0; n];
        for i in 0..n {
            let unit = &self.units[i];
            if !unit.present() || contacts[i].is_empty() || unit.state == UnitState::Routing {
                continue;
            }
            if unit.withdrawing || unit.disengaging {
                continue;
            }
            let Some(p) = self.primary_opponent(i, &contacts[i]) else {
                continue;
            };
            let defender = &self.units[p];
            let dealt = self.melee_damage(unit, defender);
            damage[p] += dealt;
            if heaviest[p].is_none_or(|(d, _)| dealt > d) {
                heaviest[p] = Some((dealt, i));
            }
            credit.push((i, p, dealt));
            match attack_angle(defender, unit.x, unit.z) {
                1 => flanked[p] |= 1,
                2 => flanked[p] |= 2,
                _ => {}
            }
        }
        let causes: Vec<Option<(LossCause, u32)>> = heaviest
            .iter()
            .map(|h| {
                h.map(|(_, a)| {
                    let attacker = &self.units[a];
                    let cause = if attacker.charge_timer > 0.0 {
                        LossCause::Charge
                    } else {
                        LossCause::Melee
                    };
                    (cause, attacker.id)
                })
            })
            .collect();
        for i in 0..n {
            let unit = &mut self.units[i];
            if unit.charge_timer > 0.0 {
                unit.charge_timer -= DT;
            }
            if unit.knocked_timer > 0.0 {
                unit.knocked_timer -= DT;
                if unit.knocked_timer <= 0.0 {
                    unit.knocked = 0.0;
                }
            }
            if flanked[i] != 0 {
                if !self.flanked_alerted[i] {
                    self.flanked_alerted[i] = true;
                    newly_flanked.push((unit.x, unit.z, unit.side, unit.id));
                }
            } else {
                self.flanked_alerted[i] = false;
            }
            unit.flanked = flanked[i];
            if damage[i] <= 0.0 || !unit.present() {
                continue;
            }
            let before = unit.hp;
            let dealt = damage[i].min(unit.hp);
            dealt_ratio[i] = dealt / damage[i];
            unit.hp -= dealt;
            unit.tick_losses += dealt;
            if let Some((cause, by)) = causes[i] {
                unit.loss_cause = cause;
                unit.loss_by = Some(by);
            }
            if unit.is_general && self.general_alive[unit.side.index()] {
                let chance = dealt / before.max(1.0) * 0.4;
                if self.rng.unit() < chance || self.units[i].hp <= 0.0 {
                    self.kill_general(self.units[i].side);
                }
            }
            if self.units[i].hp <= 0.0 {
                self.unit_destroyed(i);
            }
        }
        for (striker, victim, blow) in credit {
            self.units[striker].kills += blow * dealt_ratio[victim];
        }
        for (x, z, side, id) in newly_flanked {
            self.alert(
                crate::alerts::AlertKind::Flanked,
                x,
                z,
                Some(side),
                Some(id),
            );
        }
    }

    pub(super) fn unit_destroyed(&mut self, i: usize) {
        self.units[i].hp = 0.0;
        if self.units[i].is_general && self.general_alive[self.units[i].side.index()] {
            self.kill_general(self.units[i].side);
        }
        self.log_unit(i, |label| format!("Les {label} sont anéantis."));
    }

    pub(super) fn kill_general(&mut self, side: SideId) {
        self.general_alive[side.index()] = false;
        self.general_killed[side.index()] = true;
        let name = self
            .setup
            .side(side)
            .general
            .as_ref()
            .map_or_else(|| "Le général".to_owned(), |g| g.name.clone());
        self.log(format!("{name} est tombé au combat !"), Some(side));
        let general_pos = self
            .units
            .iter()
            .find(|u| u.side == side && u.is_general)
            .map(|u| (u.x, u.z, u.id));
        if let Some((x, z, id)) = general_pos {
            self.alert(
                crate::alerts::AlertKind::GeneralDown,
                x,
                z,
                Some(side),
                Some(id),
            );
        }
        for unit in self.units.iter_mut().filter(|u| u.side == side) {
            unit.morale -= 25.0;
        }
    }
}
