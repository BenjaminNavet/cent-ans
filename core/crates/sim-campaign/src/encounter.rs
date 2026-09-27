//! Map encounters (lot CV3-3, spec `docs/design/2026-09-27-campagne-vivante.md`
//! § 2).
//!
//! Encounter sites (`data/encounters/enc_*.json`) appear at the start of each
//! season on passable land of a province, away from settlements and from one
//! another, and expire after a drawn number of seasons. An army whose march
//! ends within `trigger_radius_km` of a site meets it: the player answers
//! with the order `choose_encounter_option` (unanswered encounters take their
//! default option at the end of the turn), the AI draws an option weighted by
//! `ai_weight` at once. An option applies chronicle effects, army effects and
//! at most one special outcome: a battle against a `fac_rebels` troop raised
//! beside the army (fought through `movement::fight`, 3D or auto, whose
//! result applies `on_win` / `on_loss`), or regiments joining the army.

use data_model::{EncounterId, FactionId, GameData, ProvinceId};
use serde::{Deserialize, Serialize};

use crate::battle_auto::Winner;
use crate::events::GameEvent;
use crate::navigation::Cell;
use crate::state::{ArmyId, CampaignState};

/// Encounter part of the campaign state (`#[serde(default)]`: older saves
/// load without it).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EncounterState {
    /// Sites on the map, in creation order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sites: Vec<EncounterSite>,
    /// Player encounters awaiting a choice.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending: Vec<PendingEncounter>,
    /// Encounter battles not yet resolved (link from the site to the battle).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub battles: Vec<EncounterBattle>,
    #[serde(default)]
    pub next_site_id: u32,
}

/// An encounter waiting on the map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncounterSite {
    pub id: u32,
    pub encounter: EncounterId,
    pub cell: Cell,
    pub province: ProvinceId,
    /// The site disappears at the start of this turn.
    pub expires_turn: u32,
    /// Player army that met it and whose choice is pending.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claimed_by: Option<ArmyId>,
}

/// An encounter met by a player army, awaiting the player's choice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingEncounter {
    pub site: u32,
    pub encounter: EncounterId,
    pub army: ArmyId,
    pub faction: FactionId,
    pub province: ProvinceId,
    /// Turn it was met.
    pub turn: u32,
}

/// A battle started by an encounter option, until it is resolved.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncounterBattle {
    pub site: u32,
    pub encounter: EncounterId,
    pub option: usize,
    /// The army that met the encounter.
    pub army: ArmyId,
    pub faction: FactionId,
    pub province: ProvinceId,
    /// The `fac_rebels` troop raised for the battle.
    pub rebels: ArmyId,
    /// The war between `faction` and the rebels was declared for this
    /// battle (and ends with it).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub declared_war: bool,
}

/// Why a `choose_encounter_option` order was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EncounterError {
    #[error("aucune rencontre en attente pour cette armée sur le site n°{0}")]
    NoPendingEncounter(u32),
    #[error("cette rencontre ne concerne pas la faction {0}")]
    NotYours(FactionId),
    #[error("rencontre inconnue : {0}")]
    UnknownEncounter(EncounterId),
    #[error("choix invalide : {0}")]
    InvalidOption(usize),
    #[error("choix impossible : {0}")]
    OptionUnavailable(String),
}

/// A site shown on the map.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EncounterSiteView {
    pub id: u32,
    pub encounter: EncounterId,
    pub title: String,
    /// Map-pixel centre of the site's cell.
    pub point: [f32; 2],
    pub province: ProvinceId,
    pub province_name: String,
    pub expires_turn: u32,
    /// Seasons left before it disappears (1 = gone next season).
    pub expires_in: u32,
    /// A player army met it and the choice is pending.
    pub claimed: bool,
}

/// One option of a pending encounter, for the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncounterOptionView {
    pub index: usize,
    pub label: String,
    pub available: bool,
    /// French reason when `available` is `false`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// French summary of the effects, one per line.
    pub effects_text: String,
    /// `""`, `"battle"` or `"join"`.
    pub outcome: String,
    pub default: bool,
}

/// A pending encounter, for the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingEncounterView {
    pub site: u32,
    pub encounter: EncounterId,
    pub army: ArmyId,
    pub army_name: String,
    pub title: String,
    pub text: String,
    pub province: ProvinceId,
    pub province_name: String,
    pub options: Vec<EncounterOptionView>,
}

/// Start of a season: sites expire, then new ones appear.
pub fn start_season(state: &mut CampaignState, data: &GameData, events: &mut Vec<GameEvent>) {
    let _ = (state, data, events);
}

/// End of the player's turn: encounters left unanswered take their default
/// option.
pub(crate) fn resolve_unanswered(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let _ = (state, data, events);
}

/// End of a march of `army_id`: meets the nearest untouched site within
/// `trigger_radius_km`.
pub(crate) fn on_march_end(
    state: &mut CampaignState,
    data: &GameData,
    army_id: &ArmyId,
    events: &mut Vec<GameEvent>,
) {
    let _ = (state, data, army_id, events);
}

/// Order `choose_encounter_option`.
pub fn choose_option(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    army: &ArmyId,
    site: u32,
    option: usize,
    events: &mut Vec<GameEvent>,
) -> Result<(), EncounterError> {
    let _ = (state, data, faction, army, option, events);
    Err(EncounterError::NoPendingEncounter(site))
}

/// End of a battle (`movement::apply_battle_result`): applies `on_win` /
/// `on_loss` when it was an encounter battle and removes the troop.
pub(crate) fn after_battle(
    state: &mut CampaignState,
    data: &GameData,
    attackers: &[ArmyId],
    defenders: &[ArmyId],
    winner: Winner,
    events: &mut Vec<GameEvent>,
) {
    let _ = (state, data, attackers, defenders, winner, events);
}

impl CampaignState {
    /// Sites `faction` sees (their cell inside its vision).
    pub fn encounter_site_views(
        &self,
        data: &GameData,
        faction: &FactionId,
    ) -> Vec<EncounterSiteView> {
        let _ = (data, faction);
        Vec::new()
    }

    /// Encounters of `faction` awaiting a choice.
    pub fn pending_encounter_views(
        &self,
        data: &GameData,
        faction: &FactionId,
    ) -> Vec<PendingEncounterView> {
        let _ = (data, faction);
        Vec::new()
    }
}
