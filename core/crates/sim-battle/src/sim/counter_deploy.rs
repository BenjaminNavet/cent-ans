//! TW ai-deploy (ADR 0331): the AI answers the player's deployment. When
//! the player presses « Commencer », the AI side (which laid itself out by
//! roles before the player placed anything) re-reads the enemy line and
//! adjusts it, within its own zone:
//!
//! - **spears against horse**: each enemy cavalry regiment gets an
//!   anti-cavalry regiment (spear wall, pikes) swapped into the rank facing it;
//! - **shooters against slow foot**: the shooters shift sideways, within
//!   `counter_shooter_shift`, to stand behind the enemy's slow infantry;
//! - **refused flank**: against a front wider than `counter_wide_ratio`
//!   times its own, the flank the enemy overlaps most steps back and tucks in.
//!
//! Numbers in `data/rules/battle_ai.json`; only field battles against a
//! human player (two AIs place simultaneously); deterministic (no random).

use data_model::{Ability, UnitCategory};

use super::BattleSim;
use crate::ai::tuning;
use crate::setup::SideId;
use crate::unit::Unit;

fn is_horse(u: &Unit) -> bool {
    u.mounted || u.category == UnitCategory::Cavalry
}

fn anti_cavalry(u: &Unit) -> bool {
    !u.mounted
        && u.category == UnitCategory::Infantry
        && (u.has(Ability::SpearWall) || u.has(Ability::PikeSquare))
}

fn shooter(u: &Unit) -> bool {
    u.can_shoot() && u.ammo > 0 && !u.mounted && u.category != UnitCategory::Siege
}

impl BattleSim {
    /// Counter-deployment of the AI `side` against the human's layout (see
    /// the module documentation). Nothing in sieges, ambushes, or for a
    /// side that cannot deploy.
    pub(super) fn ai_counter_deploy(&mut self, side: SideId) {
        let rules = tuning();
        if self.siege.is_some() || !self.can_deploy(side) || self.ambush.is_some() {
            return;
        }
        let zone = self.deployment_zone(side);
        let back = if side == SideId::Attacker { -1.0 } else { 1.0 };
        let foes = self.side_units(side.other(), |u| {
            u.present() && !u.synthetic && u.category != UnitCategory::Siege
        });
        if foes.is_empty() {
            return;
        }
        let own = self.side_units(side, |u| u.present() && !u.on_wall && !u.synthetic);

        // 1. Spears against horse.
        let mut horse_x: Vec<f64> = foes
            .iter()
            .filter(|&&j| is_horse(&self.units[j]))
            .map(|&j| self.units[j].x)
            .collect();
        horse_x.sort_by(f64::total_cmp);
        // The rank in front of the horse, or the end of our line nearest it.
        let line = own
            .iter()
            .filter(|&&i| !is_horse(&self.units[i]) && !shooter(&self.units[i]))
            .map(|&i| self.units[i].x)
            .fold((f64::MAX, f64::MIN), |(a, b), x| (a.min(x), b.max(x)));
        let mut placed: Vec<usize> = Vec::new();
        for hx in horse_x.into_iter().take(rules.counter_max_pairs as usize) {
            let target_x = if line.0 <= line.1 {
                hx.clamp(line.0, line.1)
            } else {
                hx
            };
            let Some(spear) = own
                .iter()
                .copied()
                .filter(|i| !placed.contains(i) && anti_cavalry(&self.units[*i]))
                .min_by(|&a, &b| {
                    (self.units[a].x - target_x)
                        .abs()
                        .total_cmp(&(self.units[b].x - target_x).abs())
                        .then(a.cmp(&b))
                })
            else {
                break;
            };
            let spear_z = self.units[spear].z;
            // The regiment now in that rank (same line of foot), swapped.
            let occupant = own
                .iter()
                .copied()
                .filter(|&i| {
                    i != spear
                        && !placed.contains(&i)
                        && !self.units[i].is_general
                        && !is_horse(&self.units[i])
                        && (self.units[i].z - spear_z).abs() < rules.counter_rank_depth
                })
                .min_by(|&a, &b| {
                    (self.units[a].x - target_x)
                        .abs()
                        .total_cmp(&(self.units[b].x - target_x).abs())
                        .then(a.cmp(&b))
                });
            if let Some(other) = occupant {
                let (sx, sz) = (self.units[spear].x, self.units[spear].z);
                let (ox, oz) = (self.units[other].x, self.units[other].z);
                self.units[spear].x = ox;
                self.units[spear].z = oz;
                self.units[other].x = sx;
                self.units[other].z = sz;
                placed.push(spear);
                placed.push(other);
            }
        }

        // 2. Shooters against slow foot.
        let slow: Vec<f64> = foes
            .iter()
            .filter(|&&j| {
                let u = &self.units[j];
                u.category == UnitCategory::Infantry
                    && !u.mounted
                    && !shooter(u)
                    && f64::from(u.stats.speed) <= rules.counter_slow_speed
            })
            .map(|&j| self.units[j].x)
            .collect();
        let shooters: Vec<usize> = own
            .iter()
            .copied()
            .filter(|&i| shooter(&self.units[i]))
            .collect();
        if !slow.is_empty() && !shooters.is_empty() {
            let target = slow.iter().sum::<f64>() / slow.len() as f64;
            let centre =
                shooters.iter().map(|&i| self.units[i].x).sum::<f64>() / shooters.len() as f64;
            let lo = shooters
                .iter()
                .map(|&i| self.units[i].x)
                .fold(f64::MAX, f64::min);
            let hi = shooters
                .iter()
                .map(|&i| self.units[i].x)
                .fold(f64::MIN, f64::max);
            let wanted =
                (target - centre).clamp(-rules.counter_shooter_shift, rules.counter_shooter_shift);
            let shift = wanted.clamp(zone.x0 - lo, zone.x1 - hi);
            for &i in &shooters {
                self.units[i].x += shift;
            }
        }

        // 3. Refused flank against a wider front.
        let extent = |list: &[usize], units: &[Unit]| -> Option<(f64, f64)> {
            list.iter()
                .map(|&i| units[i].x)
                .fold(None, |acc, x| match acc {
                    None => Some((x, x)),
                    Some((a, b)) => Some((a.min(x), b.max(x))),
                })
        };
        if let (Some((fa, fb)), Some((oa, ob))) =
            (extent(&foes, &self.units), extent(&own, &self.units))
        {
            let own_width = (ob - oa).max(1.0);
            if (fb - fa) > rules.counter_wide_ratio * own_width {
                let left = oa - fa;
                let right = fb - ob;
                let (edge, inward) = if left >= right { (oa, 1.0) } else { (ob, -1.0) };
                if left.max(right) > 0.0 {
                    for &i in &own {
                        let u = &self.units[i];
                        if u.is_general
                            || shooter(u)
                            || (u.x - edge).abs() > rules.counter_flank_span
                        {
                            continue;
                        }
                        let (x, z) = zone.clamp(
                            self.units[i].x + inward * rules.counter_flank_tuck,
                            self.units[i].z + back * rules.counter_flank_pull,
                        );
                        self.units[i].x = x;
                        self.units[i].z = z;
                    }
                }
            }
        }

        // Never in deep water, whatever the shifts (F5d).
        let edge = if back < 0.0 { zone.z0 } else { zone.z1 };
        for &i in &own {
            let (x, z) = (self.units[i].x, self.units[i].z);
            let dry = crate::ai::dry_z(&self.field, x, z, edge, back);
            self.units[i].z = dry.clamp(zone.z0, zone.z1);
        }
    }
}
