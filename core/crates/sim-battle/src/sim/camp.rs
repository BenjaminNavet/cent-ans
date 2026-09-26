//! Camps and baggage (lot EP6): each army leaves its camp behind its
//! deployment zone ([`crate::decor::Camp`]). An able enemy regiment standing
//! in an unguarded camp loots it little by little
//! ([`crate::decor::CampRules::loot_seconds`]); a friendly able regiment
//! within the guard radius stops the looting, which slowly wanes. The first
//! enemy in the camp spreads alarm (a small morale loss); a looted camp costs
//! every regiment of its army morale, once, as the loss of the baggage at
//! Agincourt, and the looters tire under their booty. The battle result
//! records the lost baggage.

use super::{BattleSim, DT};
use crate::decor::DecorRules;
use crate::setup::SideId;

/// State of the camp of one side.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CampState {
    /// Looting done, 0-1.
    pub progress: f64,
    pub looted: bool,
    /// The enemy has entered the camp at least once.
    pub alarmed: bool,
    /// Able enemy regiments in the camp at the last step.
    pub looters: u32,
    /// Able friendly regiments guarding it at the last step.
    pub guards: u32,
}

impl BattleSim {
    /// State of the camp of `side` (`None` without a camp).
    pub fn camp_state(&self, side: SideId) -> Option<CampState> {
        self.field.decor.camp(side)?;
        Some(self.camp_states[side.index()])
    }

    /// One step of the camps (after the melee, before morale).
    pub(super) fn resolve_camps(&mut self) {
        if self.field.decor.camps.is_empty() {
            return;
        }
        let rules = &DecorRules::bundled().camp;
        for side in SideId::BOTH {
            let Some(camp) = self.field.decor.camp(side) else {
                continue;
            };
            if self.camp_states[side.index()].looted {
                continue;
            }
            let fp = camp.area.footprint();
            let mut looters = Vec::new();
            let mut guards = 0;
            for (i, u) in self.units.iter().enumerate() {
                if !u.able() || u.synthetic {
                    continue;
                }
                if u.side != side && fp.contains(u.x, u.z, 5.0) {
                    looters.push(i);
                } else if u.side == side && fp.signed_distance(u.x, u.z) <= rules.guard_radius_m {
                    guards += 1;
                }
            }
            let state = &mut self.camp_states[side.index()];
            state.looters = looters.len() as u32;
            state.guards = guards;
            if looters.is_empty() || guards > 0 {
                state.progress = (state.progress - rules.decay_per_second * DT).max(0.0);
                continue;
            }
            let alarm = !state.alarmed;
            state.alarmed = true;
            // More looters strip the camp faster (up to twice as fast).
            let pace = (1.0 + 0.25 * (looters.len() as f64 - 1.0)).min(2.0);
            state.progress += DT / rules.loot_seconds * pace;
            let looted = state.progress >= 1.0;
            if looted {
                state.progress = 1.0;
                state.looted = true;
            }
            if !alarm && !looted {
                continue;
            }
            let name = self.setup.side(side).faction_name.clone();
            if alarm {
                self.shake_side(side, rules.alarm_morale);
                self.log(
                    format!(
                        "L'ennemi est entré dans le camp ({name}) : les bagages sont menacés !"
                    ),
                    Some(side),
                );
            }
            if looted {
                self.shake_side(side, rules.looted_morale);
                for &i in &looters {
                    let u = &mut self.units[i];
                    u.fatigue = (u.fatigue + rules.looter_fatigue).min(100.0);
                }
                self.log(
                    format!("Le camp et les bagages ({name}) sont pillés : l'armée perd courage."),
                    Some(side),
                );
            }
        }
    }

    /// Every regiment of `side` on the field loses `morale`.
    fn shake_side(&mut self, side: SideId, morale: f64) {
        for u in self
            .units
            .iter_mut()
            .filter(|u| u.side == side && u.present())
        {
            u.morale = (u.morale - morale).max(0.0);
        }
    }
}
