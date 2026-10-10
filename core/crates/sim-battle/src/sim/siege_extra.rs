//! Siege additions (F5a § 4; values in `data/rules/siege_works.json`, ADR 0329): shooting from the towers of the ring and the
//! garrison's sortie.
//!
//! - Every tower still joined to an intact stretch of wall shoots at the
//!   nearest besieger outside the walls within `tower.range_m`, once every
//!   `tower.reload_s` seconds (staggered by tower), while the garrison has a
//!   regiment able to man it.
//! - When the besiegers falter (their fighting value below
//!   `sortie.ratio` × the garrison's, after `sortie.delay_s` seconds) an
//!   AI garrison opens its gate and sallies out (`SiegeWorks::sortie`).

use super::{armor_factor, BattleSim, DT};
use crate::setup::SideId;

impl BattleSim {
    pub(super) fn tower_fire(&mut self) {
        let Some(works) = &self.siege else {
            return;
        };
        let manned = self
            .units
            .iter()
            .any(|u| u.side == SideId::Defender && u.able() && !u.synthetic);
        if !manned {
            return;
        }
        let rules = crate::siege::SiegeWorkRules::bundled();
        let (tower_rules, counter) = (&rules.tower, &rules.counter_battery);
        let range = tower_rules.range(works.fortification);
        let shots = tower_rules.shots(works.fortification);
        let period = (tower_rules.reload_s / DT).round() as u64;
        let mut volleys: Vec<(usize, usize, f64)> = Vec::new();
        for (k, tower) in works.towers.iter().enumerate() {
            if !(self.ticks + k as u64 * 7).is_multiple_of(period) {
                continue;
            }
            let joined = works.pieces.iter().any(|p| {
                p.intact()
                    && [p.a, p.b]
                        .iter()
                        .any(|e| (e.0 - tower.x).powi(2) + (e.1 - tower.z).powi(2) < 4.0)
            });
            // T4: a gate taken by the attacker silences the towers around it.
            if !joined || self.tower_silenced(tower.x, tower.z) {
                continue;
            }
            let target = (0..self.units.len())
                .filter(|&j| {
                    let u = &self.units[j];
                    // Roofed machines (ram, siege towers) take a bolt only
                    // when `counter_battery.tower_engine_factor` allows it.
                    u.side == SideId::Attacker
                        && u.present()
                        && (counter.tower_engine_factor > 0.0 || (!u.ram && !u.siege_tower()))
                        && !works.inside(u.x, u.z)
                })
                .map(|j| {
                    let u = &self.units[j];
                    (
                        j,
                        ((u.x - tower.x).powi(2) + (u.z - tower.z).powi(2)).sqrt(),
                    )
                })
                .filter(|&(_, d)| d <= range)
                // Men first: a roofed machine only draws a bolt when no
                // regiment is within reach.
                .min_by(|a, b| {
                    let roofed = |j: usize| self.units[j].ram || self.units[j].siege_tower();
                    roofed(a.0)
                        .cmp(&roofed(b.0))
                        .then(a.1.total_cmp(&b.1))
                        .then(a.0.cmp(&b.0))
                });
            if let Some((j, d)) = target {
                volleys.push((k, j, d));
            }
        }
        for (tower, j, d) in volleys {
            let target_id = self.units[j].id;
            self.push_fx(crate::siege_fx::SiegeFxKind::TowerVolley {
                tower,
                target: target_id,
            });
            let target = &self.units[j];
            let accuracy = tower_rules.accuracy
                * (1.0 - tower_rules.accuracy_range_loss * d / range)
                * self.weather.range_factor();
            let roofed = target.ram || target.siege_tower();
            let mut kills = shots
                * accuracy
                * tower_rules.lethality
                * armor_factor(self.defense_points(target))
                * self.pace().ranged_rate;
            if roofed {
                kills *= counter.tower_engine_factor;
            }
            let kills = kills.min(self.units[j].hp);
            self.units[j].hp -= kills;
            self.units[j].tick_losses += kills;
            self.units[j].missile_timer = 0.0;
            if kills > 0.0 {
                self.units[j].loss_cause = crate::impact::LossCause::Arrow;
                self.units[j].loss_by = None;
            }
            if self.units[j].hp <= 0.0 {
                self.unit_destroyed(j);
            }
        }
    }

    /// An AI garrison sallies out when the besiegers falter.
    pub(super) fn check_sortie(&mut self) {
        let ready = self.siege.as_ref().is_some_and(|w| !w.sortie)
            && self.ai_enabled[SideId::Defender.index()]
            && self.elapsed >= crate::siege::SiegeWorkRules::bundled().sortie.delay_s;
        if !ready {
            return;
        }
        let power = |side: SideId| -> f64 {
            self.units
                .iter()
                .filter(|u| u.side == side && u.able() && !u.synthetic)
                .map(crate::ai::unit_power)
                .sum()
        };
        let (attackers, garrison) = (power(SideId::Attacker), power(SideId::Defender));
        if garrison > 0.0
            && attackers < crate::siege::SiegeWorkRules::bundled().sortie.ratio * garrison
        {
            if let Some(works) = self.siege.as_mut() {
                works.sortie = true;
            }
            self.log(
                "La garnison ouvre ses portes et fait une sortie !".to_owned(),
                Some(SideId::Defender),
            );
        }
    }
}
