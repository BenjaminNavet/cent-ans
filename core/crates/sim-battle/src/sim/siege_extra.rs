//! Siege additions (F5a § 4): shooting from the towers of the ring and the
//! garrison's sortie.
//!
//! - Every tower still joined to an intact stretch of wall shoots at the
//!   nearest besieger outside the walls within [`TOWER_RANGE`], once every
//!   [`TOWER_RELOAD`] seconds (staggered by tower), while the garrison has a
//!   regiment able to man it.
//! - When the besiegers falter (their fighting value below
//!   [`SORTIE_RATIO`] × the garrison's, after [`SORTIE_DELAY`] seconds) an
//!   AI garrison opens its gate and sallies out (`SiegeWorks::sortie`).

use super::{armor_factor, BattleSim, DT};
use crate::setup::SideId;

/// Reach of the tower crossbows (metres).
pub const TOWER_RANGE: f64 = 180.0;
/// Seconds between two volleys of one tower.
pub const TOWER_RELOAD: f64 = 8.0;
/// Shooters per tower.
pub const TOWER_SHOTS: f64 = 5.0;
/// The garrison sallies once the besiegers weigh less than this share of it.
pub const SORTIE_RATIO: f64 = 0.5;
/// No sortie in the first minutes of the assault.
pub const SORTIE_DELAY: f64 = 120.0;

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
        let period = (TOWER_RELOAD / DT).round() as u64;
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
                    // Roofed machines (ram, siege towers) are not worth a bolt.
                    u.side == SideId::Attacker
                        && u.present()
                        && !u.ram
                        && !u.siege_tower()
                        && !works.inside(u.x, u.z)
                })
                .map(|j| {
                    let u = &self.units[j];
                    (
                        j,
                        ((u.x - tower.x).powi(2) + (u.z - tower.z).powi(2)).sqrt(),
                    )
                })
                .filter(|&(_, d)| d <= TOWER_RANGE)
                .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
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
            let accuracy = 0.3 * (1.0 - 0.5 * d / TOWER_RANGE) * self.weather.range_factor();
            let kills = TOWER_SHOTS
                * accuracy
                * 0.6
                * armor_factor(self.defense_points(target))
                * self.pace().ranged_rate;
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
            && self.elapsed >= SORTIE_DELAY;
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
        if garrison > 0.0 && attackers < SORTIE_RATIO * garrison {
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
