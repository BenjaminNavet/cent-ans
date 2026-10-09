//! Shared plumbing of the per-province policies (SC CC8): the diet of
//! « La Table » (`table.rs`) and the regional edicts (`edicts.rs`) are both a
//! choice made by the controller of a province, which lapses when the
//! controller changes, can be changed once per turn and is journaled for the
//! player only. This module holds what they have in common; each policy keeps
//! its own rules (costs and requirements for diets, activation delay for
//! edicts).

use data_model::{FactionId, ProvinceId};

use crate::events::{EventKind, GameEvent};
use crate::state::CampaignState;

/// A choice stored on a province by the faction controlling it
/// (`ProvinceState::diet`, `ProvinceState::edict`).
pub trait ProvincePolicy {
    /// Faction that made the choice; it lapses when the controller changes.
    fn faction(&self) -> &FactionId;
    /// Turn of the last change.
    fn turn(&self) -> u32;
}

/// `choice` if it was made by the current controller of `province`.
pub fn controller_choice<'a, P: ProvincePolicy>(
    state: &CampaignState,
    province: &ProvinceId,
    choice: Option<&'a P>,
) -> Option<&'a P> {
    choice.filter(|c| state.province_controller(province) == Some(c.faction()))
}

/// `true` if `faction` already changed the policy this turn (one change per
/// province and per turn).
pub fn changed_this_turn<P: ProvincePolicy>(
    state: &CampaignState,
    choice: Option<&P>,
    faction: &FactionId,
) -> bool {
    choice.is_some_and(|c| c.turn() == state.turn && c.faction() == faction)
}

/// Journals a policy event, only for the player.
pub fn push_player_event(
    state: &CampaignState,
    kind: EventKind,
    faction: &FactionId,
    province: Option<&ProvinceId>,
    text: String,
    events: &mut Vec<GameEvent>,
) {
    if faction != &state.player_faction {
        return;
    }
    let mut event = GameEvent::new(kind, text).faction(faction);
    if let Some(province) = province {
        event = event.province(province);
    }
    events.push(event);
}
