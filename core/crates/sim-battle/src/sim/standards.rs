//! Regimental standards (lot EP5, ADR 0034).
//!
//! Every regiment (not the siege engines) carries a standard. Once the
//! regiment has suffered heavy losses, each step's casualties may bring the
//! standard down; a regiment that routs in contact may drop it. While it is
//! down the regiment loses morale and cohesion (its blows land weaker) and
//! the enemy close by takes heart. After [`BattleStandardRules::raise_seconds`]
//! the standard is raised again if the regiment still holds; if an able enemy
//! regiment holds the spot instead, it is taken: a trophy counted in the
//! battle result. A standard left on the ground by a regiment that fled or
//! perished goes to the victor at the end of the battle.
//!
//! Draws come from their own stream ([`BattleSim::standard_rng`], derived
//! from the seed): the other draws of a battle are unchanged by this lot.

use data_model::BattleStandardRules;

use super::BattleSim;
use crate::outcome::StandardTrophy;
use crate::setup::SideId;
use crate::unit::{StandardState, UnitState};

/// Salt of the standards' random stream.
pub(super) const STANDARD_SALT: u64 = 0x57A4_DA2D_E7A5_0001;

impl BattleSim {
    /// Rules of the standards of this battle.
    pub fn standard_rules(&self) -> &BattleStandardRules {
        &self.standard_rules
    }

    /// Standards taken so far (trophies), in the order they were taken.
    pub fn trophies(&self) -> &[StandardTrophy] {
        &self.trophies
    }

    /// Standards `bearers` drawn for regiment `index` (rendering: 1 or 2).
    pub fn standard_bearers(&self, index: usize) -> u32 {
        let unit = &self.units[index];
        if !unit.has_standard() {
            0
        } else if unit.max_soldiers >= self.standard_rules.two_bearers_from_soldiers {
            2
        } else {
            1
        }
    }

    fn morale_factor(&self, index: usize) -> f64 {
        if self.units[index].is_general {
            self.standard_rules.general_factor
        } else {
            1.0
        }
    }

    /// One step of the standards: falls, raising, capture (after the
    /// casualties of the step, before morale).
    pub(super) fn resolve_standards(&mut self, contacts: &[Vec<usize>]) {
        let rules = self.standard_rules.clone();
        let n = self.units.len();
        if self.standard_rout_seen.len() != n {
            self.standard_rout_seen.resize(n, false);
        }
        for i in 0..n {
            if !self.units[i].has_standard() || self.units[i].reserve {
                continue;
            }
            let routing = self.units[i].state == UnitState::Routing;
            let newly_routed = routing && !self.standard_rout_seen[i];
            self.standard_rout_seen[i] = routing;
            match self.units[i].standard {
                StandardState::Carried => {
                    let unit = &self.units[i];
                    if unit.left_field {
                        continue;
                    }
                    if unit.hp <= 0.0 {
                        // The last man falls with it.
                        let (x, z) = unit.standard_point();
                        self.standard_falls(i, x, z, &rules);
                        continue;
                    }
                    let mut chance = 0.0;
                    let strength = unit.hp / f64::from(unit.max_soldiers);
                    if unit.tick_losses > 0.0 && strength < rules.heavy_losses_below {
                        chance += rules.fall_chance_per_loss_percent * unit.tick_losses
                            / f64::from(unit.max_soldiers)
                            * 100.0;
                    }
                    if newly_routed && !contacts[i].is_empty() {
                        chance = chance.max(rules.rout_drop_chance);
                    }
                    if chance > 0.0 && self.standard_rng.unit() < chance {
                        let (x, z) = unit.standard_point();
                        self.standard_falls(i, x, z, &rules);
                    }
                }
                StandardState::Fallen { x, z, timer } => {
                    let factor = self.morale_factor(i);
                    let unit = &mut self.units[i];
                    if unit.present() {
                        unit.morale = (unit.morale
                            - rules.fallen_morale_per_second * factor * super::DT)
                            .max(0.0);
                    }
                    let timer = timer - super::DT;
                    if timer > 0.0 {
                        unit.standard = StandardState::Fallen { x, z, timer };
                        continue;
                    }
                    if let Some(captor) = self.captor_near(i, x, z, &rules) {
                        self.standard_taken(i, captor, &rules);
                    } else if self.units[i].able() {
                        self.units[i].standard = StandardState::Carried;
                        let text = format!("L'étendard des {} est relevé.", self.unit_label(i));
                        let side = self.units[i].side;
                        self.log(text, Some(side));
                    } else if !self.units[i].present() {
                        self.units[i].standard = StandardState::Lost { x, z };
                    } else {
                        // Routing but nobody on the spot: wait.
                        self.units[i].standard = StandardState::Fallen {
                            x,
                            z,
                            timer: rules.raise_seconds * 0.5,
                        };
                    }
                }
                StandardState::Lost { x, z } => {
                    // An enemy passing over the spot picks it up.
                    if let Some(captor) = self.captor_near(i, x, z, &rules) {
                        self.standard_taken(i, captor, &rules);
                    }
                }
                StandardState::Captured { .. } => {}
            }
        }
    }

    /// The nearest able enemy regiment of `index` holding the spot `(x, z)`.
    fn captor_near(
        &self,
        index: usize,
        x: f64,
        z: f64,
        rules: &BattleStandardRules,
    ) -> Option<usize> {
        let side = self.units[index].side;
        self.units
            .iter()
            .enumerate()
            .filter(|(_, u)| u.side != side && u.able() && !u.synthetic && u.has_standard())
            .map(|(j, u)| (j, u.distance_to_rect(x, z)))
            .filter(|&(_, d)| d <= rules.capture_radius_m)
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
            .map(|(j, _)| j)
    }

    fn standard_falls(&mut self, i: usize, x: f64, z: f64, rules: &BattleStandardRules) {
        let factor = self.morale_factor(i);
        self.units[i].standard = StandardState::Fallen {
            x,
            z,
            timer: rules.raise_seconds,
        };
        let unit = &mut self.units[i];
        unit.morale = (unit.morale - rules.fall_morale_shock * factor).max(0.0);
        let side = unit.side;
        let r2 = rules.enemy_bonus_radius_m * rules.enemy_bonus_radius_m;
        for other in self
            .units
            .iter_mut()
            .filter(|u| u.side != side && u.able() && u.has_standard())
        {
            if (other.x - x).powi(2) + (other.z - z).powi(2) <= r2 {
                other.morale = (other.morale + rules.enemy_morale_bonus * factor).min(100.0);
            }
        }
        let text = format!("L'étendard des {} est tombé !", self.unit_label(i));
        self.log(text, Some(side));
    }

    fn standard_taken(&mut self, i: usize, captor: usize, rules: &BattleStandardRules) {
        let factor = self.morale_factor(i);
        let (x, z) = self.units[i]
            .standard
            .ground()
            .unwrap_or_else(|| self.units[i].standard_point());
        self.units[i].standard = StandardState::Captured {
            by: self.units[captor].id,
        };
        let side = self.units[i].side;
        let r2 = rules.enemy_bonus_radius_m * rules.enemy_bonus_radius_m;
        for (j, other) in self.units.iter_mut().enumerate() {
            if other.side != side || !other.present() {
                continue;
            }
            let shock = if j == i {
                2.0 * rules.capture_morale_shock * factor
            } else if (other.x - x).powi(2) + (other.z - z).powi(2) <= r2 {
                rules.capture_morale_shock * factor
            } else {
                continue;
            };
            other.morale = (other.morale - shock).max(0.0);
        }
        let unit = &mut self.units[i];
        unit.morale_cap = (unit.morale_cap - rules.lost_morale_cap).max(0.0);
        let captor_unit = &mut self.units[captor];
        captor_unit.morale = (captor_unit.morale + rules.captor_morale_bonus * factor).min(100.0);
        let trophy = self.trophy_of(i, Some(captor));
        self.trophies.push(trophy);
        let text = format!(
            "Les {} prennent l'étendard des {} !",
            self.unit_label(captor),
            self.unit_label(i)
        );
        let captor_side = self.units[captor].side;
        self.log(text, Some(captor_side));
    }

    fn trophy_of(&self, i: usize, captor: Option<usize>) -> StandardTrophy {
        let unit = &self.units[i];
        StandardTrophy {
            unit_type: unit.unit_type.clone(),
            unit_name: unit.name.clone(),
            faction: self.setup.side(unit.side).faction.clone(),
            general: unit.is_general,
            captor: captor.map(|c| self.units[c].name.clone()),
            time: self.elapsed,
            taken_by: unit.side.other(),
        }
    }

    /// End of the battle: the victor holds the field and collects the
    /// standards the vanquished left on the ground.
    pub(super) fn collect_field_standards(&mut self, winner: SideId) {
        let loser = winner.other();
        for i in 0..self.units.len() {
            let unit = &self.units[i];
            if unit.side != loser || unit.standard.ground().is_none() {
                continue;
            }
            self.units[i].standard = StandardState::Captured { by: u32::MAX };
            let trophy = self.trophy_of(i, None);
            self.trophies.push(trophy);
        }
    }

    /// Standards taken by `side`.
    pub(super) fn trophies_of(&self, side: SideId) -> Vec<StandardTrophy> {
        self.trophies
            .iter()
            .filter(|t| t.taken_by == side)
            .cloned()
            .collect()
    }
}
