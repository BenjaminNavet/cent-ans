//! H4 « Médecine et herboristerie » (`docs/design/2026-09-23-histoire-et-savoir.md` § 3).
//!
//! The medicine tree is an ordinary technology branch (`TechBranch::Medicine`)
//! fed by the faction's single research pool (`ResearchPoints`): there is no
//! per-branch research, so no `ResearchMedicine` effect (see
//! `docs/design/h3-h4-api.md`). This module holds the two new effects:
//!
//! - `PlagueResistance` (percent points; buildings of the province plus the
//!   controller's technologies, capped at [`MAX_PLAGUE_RESISTANCE`]): each
//!   plague may spare the province and, when it strikes, is milder. It
//!   applies to the local plague of `population` (health collapse), to the
//!   Black Death wave of `chronicle` and to the local epidemic events
//!   ([`EPIDEMIC_EVENTS`]).
//! - `WoundRecovery` (percent; the faction's technologies, capped at
//!   [`MAX_WOUND_RECOVERY`]): after a field battle (auto-resolved or fought
//!   in 3D), every unit that survives gets back that share of its losses as
//!   tended wounded ([`recovered_wounded`], applied by
//!   `movement::apply_outcome`).

use data_model::{FactionId, GameData, ProvinceId};

use crate::state::CampaignState;

/// Ceiling of the plague resistance (percent points).
pub const MAX_PLAGUE_RESISTANCE: f64 = 50.0;
/// Ceiling of the wound recovery (percent of losses).
pub const MAX_WOUND_RECOVERY: f64 = 50.0;
/// Share of the plague resistance that turns into a chance of being spared
/// by the Black Death (it spares less than local plagues).
pub const BLACK_DEATH_SPARE_PERCENT: f64 = 50.0;
/// Chronicle events treated as local epidemics.
pub const EPIDEMIC_EVENTS: &[&str] = &["evt_epidemie_locale"];

/// `true` for a local epidemic event.
pub fn is_epidemic(event: &data_model::EventId) -> bool {
    EPIDEMIC_EVENTS.contains(&event.as_str())
}

/// Plague resistance of `province` as a fraction (0 to 0.5): its buildings
/// plus its controller's technologies.
pub fn plague_resistance(state: &CampaignState, data: &GameData, province: &ProvinceId) -> f64 {
    let Some(p) = state.provinces.get(province) else {
        return 0.0;
    };
    let buildings = crate::buildings::effects_of(data, &p.buildings).plague_resistance;
    let techs = crate::research::faction_tech_effects(state, data, &p.controller).plague_resistance;
    let total = buildings.flat + buildings.percent + techs.flat + techs.percent;
    total.clamp(0.0, MAX_PLAGUE_RESISTANCE) / 100.0
}

/// Wound recovery of `faction` as a fraction (0 to 0.5).
pub fn wound_recovery(state: &CampaignState, data: &GameData, faction: &FactionId) -> f64 {
    let value = crate::research::faction_tech_effects(state, data, faction).wound_recovery;
    (value.flat + value.percent).clamp(0.0, MAX_WOUND_RECOVERY) / 100.0
}

/// Wounded a surviving unit gets back out of `losses` at `recovery`
/// (fraction), rounded down.
pub fn recovered_wounded(losses: u32, recovery: f64) -> u32 {
    (f64::from(losses) * recovery.clamp(0.0, 1.0)).floor() as u32
}

/// Scales a harmful amount (negative) by `1 - resistance`, rounded towards
/// zero; beneficial amounts are left alone.
pub fn mitigated(amount: i32, resistance: f64) -> i32 {
    if amount >= 0 || resistance <= 0.0 {
        return amount;
    }
    (f64::from(amount) * (1.0 - resistance)).trunc() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mitigation_only_softens_harm() {
        assert_eq!(mitigated(-10, 0.5), -5);
        assert_eq!(mitigated(-15, 0.3), -10);
        assert_eq!(mitigated(3, 0.5), 3);
        assert_eq!(mitigated(-8, 0.0), -8);
    }

    #[test]
    fn wounded_are_rounded_down() {
        assert_eq!(recovered_wounded(100, 0.25), 25);
        assert_eq!(recovered_wounded(7, 0.25), 1);
        assert_eq!(recovered_wounded(0, 0.5), 0);
    }
}
