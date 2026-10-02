//! Lot JR1 « Ferveur » (ADR 0165): the crusader faction lives on the zeal of
//! its vow instead of land.
//!
//! Everything is read from `data/rules/crusade.json`
//! ([`data_model::CrusadeRules`]): the faction, its base, the province of
//! the vow and the whole scale. Without the file, or without the faction,
//! [`CampaignState::crusade`] stays `None` and every function here is a
//! no-op.
//!
//! Spec `docs/superpowers/specs/2026-10-02-jr-croises-jerusalem-design.md`.

use data_model::{FactionId, GameData, SettlementId, UnitTypeId};
use serde::{Deserialize, Serialize};

use crate::events::GameEvent;
use crate::orders::Order;
use crate::state::CampaignState;

/// A contingent of volunteers at sea.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingPassage {
    /// Turn whose end lands the contingent.
    pub arrival_turn: u32,
    /// Port it was preached for.
    pub port: SettlementId,
    pub units: u32,
}

/// Dynamic state of the crusade (absent from older saves and when the
/// rules or the faction are missing).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrusadeState {
    /// 0-100.
    pub fervor: u8,
    /// Turns before the passage can be preached again.
    #[serde(default)]
    pub preach_cooldown: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_passages: Vec<PendingPassage>,
    /// The target province has been taken and is still held (floor).
    #[serde(default)]
    pub target_taken: bool,
    /// Causes (French) and points of the latest changes, for the tooltip.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub last_changes: Vec<(String, i32)>,
    /// Turn `last_changes` belongs to.
    #[serde(default)]
    pub changes_turn: u32,
    /// Alms paid at the last end of turn.
    #[serde(default)]
    pub alms_last_turn: i64,
}

/// Why `preach_passage` was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CrusadeError {
    #[error("seule la faction croisée peut prêcher le passage")]
    NotCrusaders,
    #[error("il faut tenir un port pour accueillir les volontaires")]
    NoPort,
    #[error("trésor insuffisant : {needed} livres nécessaires, {available} disponibles")]
    InsufficientFunds { needed: i64, available: i64 },
    #[error("le passage a déjà été prêché : encore {0} tour(s) avant un nouvel appel")]
    Cooldown(u32),
}

/// One line of the fervour tooltip.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FervorChange {
    pub cause: String,
    pub delta: i32,
}

/// A contingent at sea, for the interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PendingPassageView {
    pub turns_left: u32,
    pub port: SettlementId,
    pub port_name: String,
    pub units: u32,
}

/// The « Ferveur » section of the faction panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CrusadeView {
    pub fervor: u8,
    /// Level fervour cannot fall below (0 unless the target is held).
    pub floor: u8,
    /// Alms the next end of turn pays at the current fervour.
    pub alms: i64,
    pub alms_last_turn: i64,
    pub changes: Vec<FervorChange>,
    /// Morale points the faction's armies get from the current fervour.
    pub zeal_morale: i32,
    /// Percent of the men who leave each turn at the current fervour.
    pub desertion_percent: u32,
    pub target_taken: bool,
    pub target_name: String,
    pub passage_cost: i64,
    pub passage_cooldown: u32,
    pub passage_available: bool,
    /// French reason the passage cannot be preached now (empty: it can).
    pub passage_blocker: String,
    /// Units a contingent preached now would bring.
    pub passage_units: u32,
    pub passage_delay: u32,
    pub pending: Vec<PendingPassageView>,
}

/// Campaign setup: opens the crusade when the rules and the faction exist.
pub(crate) fn init_crusade(_state: &mut CampaignState, _data: &GameData) {}

/// `true` when `faction` is the living crusader faction of an open crusade.
pub fn is_crusader(_state: &CampaignState, _data: &GameData, _faction: &FactionId) -> bool {
    false
}

/// End of turn: wear of the vow, truce with the target's holder, alms,
/// landings, desertion, floor.
pub(crate) fn resolve_crusade(
    _state: &mut CampaignState,
    _data: &GameData,
    _events: &mut Vec<GameEvent>,
) {
}

/// A field battle or assault between `winner` and `loser` was decided.
pub fn on_battle(
    _state: &mut CampaignState,
    _data: &GameData,
    _winner: &FactionId,
    _loser: &FactionId,
) {
}

/// `taker` took `settlement` from `previous`.
pub fn on_settlement_taken(
    _state: &mut CampaignState,
    _data: &GameData,
    _taker: &FactionId,
    _settlement: &SettlementId,
    _events: &mut Vec<GameEvent>,
) {
}

/// `aggressor` declared war on `target`.
pub fn on_war_declared(
    _state: &mut CampaignState,
    _data: &GameData,
    _aggressor: &FactionId,
    _target: &FactionId,
) {
}

/// Morale points (0-100 scale) the armies of `faction` get from fervour.
pub fn zeal_morale(_state: &CampaignState, _data: &GameData, _faction: &FactionId) -> i32 {
    0
}

/// Why `faction` cannot preach the passage now (`None`: it can).
pub fn preach_blocker(
    _state: &CampaignState,
    _data: &GameData,
    _faction: &FactionId,
) -> Option<CrusadeError> {
    Some(CrusadeError::NotCrusaders)
}

/// `preach_passage`: pays, raises fervour, books a contingent.
pub fn preach_passage(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Result<(), CrusadeError> {
    match preach_blocker(state, data, faction) {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// Adds fresh units of `units` to the garrison of `settlement`; returns how
/// many were added (unknown unit types are skipped).
pub fn spawn_units_at_settlement(
    _state: &mut CampaignState,
    _data: &GameData,
    _settlement: &SettlementId,
    _units: &[UnitTypeId],
) -> u32 {
    0
}

/// The fervour panel of `faction` (`None` unless it is the crusaders).
pub fn crusade_view(
    _state: &CampaignState,
    _data: &GameData,
    _faction: &FactionId,
) -> Option<CrusadeView> {
    None
}

/// AI: preaches as soon as the action is available and affordable.
pub fn ai_preach(_state: &CampaignState, _data: &GameData, _faction: &FactionId) -> Vec<Order> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "JR1 skeleton"]
    fn fervor_wears_off_and_stays_within_bounds() {}

    #[test]
    #[ignore = "JR1 skeleton"]
    fn target_floor_holds_while_the_target_is_held() {}

    #[test]
    #[ignore = "JR1 skeleton"]
    fn alms_follow_fervor() {}

    #[test]
    #[ignore = "JR1 skeleton"]
    fn preach_is_refused_with_a_reason() {}

    #[test]
    #[ignore = "JR1 skeleton"]
    fn contingent_size_and_landing() {}

    #[test]
    #[ignore = "JR1 skeleton"]
    fn desertion_below_the_threshold() {}
}
