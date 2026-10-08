//! Armies' movement and battles: the settlement graph, landings, the
//! allowance of a turn, field battles and the retreat of the beaten.
//!
//! Armies march freely on the navigation grid (`march.rs`,
//! `navigation.rs`, `docs/design/2026-09-24-mouvement-libre.md`). The
//! settlement graph (`GameData::movement_graph`) remains the skeleton of
//! the AI's strategic planning, of the agents' walks and of the sea
//! crossings (port to port). Battles are fought at once when an army
//! attacks another within `engage_radius_km`; the loser falls back on the
//! grid.

mod allowance;
mod battle;
mod path;
mod retreat;
mod sea;

pub use battle::*;
pub use path::*;
pub use retreat::*;
pub(crate) use sea::land_on_hostile_shore;

use std::cmp::Reverse;

use data_model::GameData;

use crate::state::{ArmyId, CampaignState};

pub(crate) fn strongest(state: &CampaignState, ids: &[ArmyId]) -> Option<ArmyId> {
    ids.iter()
        .max_by_key(|id| (state.armies[*id].total_strength(), Reverse((*id).clone())))
        .cloned()
}

/// Keeps the province of an army's general in step with the army.
pub(crate) fn move_general(state: &mut CampaignState, data: &GameData, army_id: &ArmyId) {
    let Some(army) = state.armies.get(army_id) else {
        return;
    };
    let Some(location) = state.army_province(data, army) else {
        return;
    };
    if let Some(general) = army.general.clone() {
        if let Some(character) = state.characters.get_mut(&general) {
            character.location = Some(location);
        }
    }
}
