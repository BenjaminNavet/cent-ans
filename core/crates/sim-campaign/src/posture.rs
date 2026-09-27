//! Army stances of the living campaign (lot CV3-1, spec
//! `docs/design/2026-09-27-campagne-vivante.md` § 1): ambush, forced march,
//! entrenched camp. Numbers in `data/rules/postures.json`
//! ([`data_model::PostureRules`]).

use data_model::{FactionId, GameData};

use crate::orders::OrderError;
use crate::state::{Army, ArmyId, CampaignState, Stance};

/// Checks that `army` may switch to `stance` now.
pub fn validate_stance_change(
    _state: &CampaignState,
    _data: &GameData,
    _army: &ArmyId,
    _stance: Stance,
) -> Result<(), OrderError> {
    Ok(())
}

/// Every stance with `Ok` when `army` may take it now, or the reason why not.
pub fn stance_options(
    state: &CampaignState,
    data: &GameData,
    army: &ArmyId,
) -> Vec<(Stance, Result<(), OrderError>)> {
    Stance::ALL
        .iter()
        .map(|stance| (*stance, validate_stance_change(state, data, army, *stance)))
        .collect()
}

/// `true` when `army` is hidden (ambush) from `faction`.
pub fn is_hidden_from(
    _state: &CampaignState,
    _data: &GameData,
    _army: &Army,
    _faction: &FactionId,
) -> bool {
    false
}

/// Success chance (0-1) of the ambush of `ambusher` on `victim`.
pub fn ambush_chance(
    _state: &CampaignState,
    _data: &GameData,
    _ambusher: &ArmyId,
    _victim: &ArmyId,
) -> f64 {
    0.0
}
