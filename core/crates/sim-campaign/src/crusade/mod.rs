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

use data_model::{CrusadeRules, FactionId, GameData, ProvinceId, SettlementId, UnitTypeId};

use serde::{Deserialize, Serialize};

use crate::events::{EventKind, GameEvent};

use crate::orders::Order;

use crate::religion::{self, FaithRelation};

use crate::rng::CampaignRng;

use crate::state::{CampaignState, Unit};

mod fervor;
mod passage;
mod preach;
mod view;

pub use fervor::{alms, on_battle, on_settlement_taken, on_war_declared, zeal_morale};
use fervor::{alms_at, change, floor, held_ports, target_name, zeal_at};
pub(crate) use passage::resolve_crusade;
use passage::sync_target;
pub use preach::{
    ai_passage_reserve, ai_preach, passage_cost, preach_blocker, preach_passage,
    spawn_units_at_settlement,
};
pub use view::crusade_view;

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
    /// JR4b: turns before the master of a besieged place of the Holy Land
    /// can call its defence again.
    #[serde(default)]
    pub relief_cooldown: u32,
    /// JR4b: places whose current siege by the crusade was already relieved.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relieved: Vec<SettlementId>,
    /// JR5: the target was delivered at least once (its fervour and
    /// prestige are won only the first time).
    #[serde(default)]
    pub target_ever_taken: bool,
    /// JR5: capital before the deliverance, restored when the target is lost.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub former_capital: Option<ProvinceId>,
    /// JR5: places of the Holy Land whose capture already lifted the fervour.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub counted_places: Vec<SettlementId>,
}

/// Why `preach_passage` was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CrusadeError {
    #[error("Seule la faction croisée peut prêcher le passage.")]
    NotCrusaders,
    #[error("Aucun port tenu pour accueillir les volontaires.")]
    NoPort,
    #[error("Trésor insuffisant : {needed} livres nécessaires, {available} disponibles.")]
    InsufficientFunds { needed: i64, available: i64 },
    #[error("Passage déjà prêché : nouvel appel dans {}.", count_noun(*.0, "tour", "tours"))]
    Cooldown(u32),
}

/// `n` and the noun agreed with it (« 1 tour », « 8 tours »).
pub(super) fn count_noun(n: u32, singular: &str, plural: &str) -> String {
    format!("{n} {}", if n > 1 { plural } else { singular })
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
    /// The thresholds of the rules, for the tooltip: zeal bonus at or above
    /// `zeal_high_threshold`, malus below `zeal_low_threshold`, desertion
    /// of `desertion_men_percent` % a turn below `desertion_threshold`.
    pub zeal_high_threshold: u8,
    pub zeal_low_threshold: u8,
    pub zeal_high_morale: i32,
    pub zeal_low_morale: i32,
    pub desertion_threshold: u8,
    pub desertion_men_percent: u32,
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
pub(crate) fn init_crusade(state: &mut CampaignState, data: &GameData) {
    let Some(rules) = &data.crusade_rules else {
        return;
    };
    if state.factions.contains_key(&rules.faction) {
        state.crusade = Some(CrusadeState {
            fervor: rules.fervor.start.min(100),
            ..CrusadeState::default()
        });
    }
}

/// Position and units of the starting army of `faction` when the crusade
/// rules give it one (its base settlement instead of the capital's city).
pub(crate) fn starting_army(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Option<(SettlementId, Vec<Unit>)> {
    let rules = data.crusade_rules.as_ref()?;
    if &rules.faction != faction || !state.settlements.contains_key(&rules.base_settlement) {
        return None;
    }
    let units = rules
        .starting_army
        .iter()
        .filter_map(|id| data.unit_types.get(id))
        .map(Unit::fresh)
        .collect();
    Some((rules.base_settlement.clone(), units))
}

/// The rules of the open crusade (`None`: no rules, no state, or the
/// crusader faction is dead — its state is then frozen).
pub(super) fn active<'a>(state: &CampaignState, data: &'a GameData) -> Option<&'a CrusadeRules> {
    let rules = data.crusade_rules.as_ref()?;
    state.crusade.as_ref()?;
    state
        .factions
        .get(&rules.faction)
        .is_some_and(|f| f.alive)
        .then_some(rules)
}

/// `true` when `faction` is the living crusader faction of an open crusade.
pub fn is_crusader(state: &CampaignState, data: &GameData, faction: &FactionId) -> bool {
    active(state, data).is_some_and(|rules| &rules.faction == faction)
}

/// The vow binds the AI: the crusader faction, when the player does not
/// lead it, neither offers nor signs a peace with `other` while `other`
/// holds the target province. (The player stays free to treat, and pays
/// for it in fervour each turn.)
pub fn ai_vow_forbids_peace(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    other: &FactionId,
) -> bool {
    faction != &state.player_faction
        && active(state, data).is_some_and(|rules| {
            &rules.faction == faction
                && state.province_controller(&rules.target_province) == Some(other)
        })
}

#[cfg(test)]
mod tests;
