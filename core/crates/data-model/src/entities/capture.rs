//! Fate of a captured place (lot TW2-T1: occupy, ransom, sack, raze),
//! mirroring `data/schemas/capture_rules.schema.json`
//! (`data/rules/capture.json`). Spec `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T1.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::entities::settlement::SettlementKind;
use crate::ids::{FactionId, SettlementId};

/// Contents of `data/rules/capture.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureRules {
    pub occupy: OccupyRules,
    pub raid: RaidRules,
    pub siege: SiegeRules,
    pub ransom: OutcomeRules,
    pub sack: OutcomeRules,
    pub raze: RazeRules,
    /// Scale (0-1) of the gold, population, devastation and extra unrest of
    /// an outcome by kind of place (the city of the province is 1).
    pub place_share: BTreeMap<SettlementKind, f64>,
    /// Kinds of place that can never be razed (province cities).
    pub raze_forbidden_kinds: Vec<SettlementKind>,
    /// Emblematic places that can never be razed (royal abbeys, shrines).
    pub raze_forbidden_settlements: Vec<SettlementId>,
    pub ai: CaptureAiRules,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// « Occuper »: the place changes hands, nothing more.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OccupyRules {
    /// Unrest added to the province when its city is taken.
    pub unrest_city: u8,
    /// Unrest added when another place of the province is taken.
    pub unrest_place: u8,
}

/// A chevauchée: what one turn of raiding does to a province.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RaidRules {
    /// Devastation added per turn.
    pub devastation: u8,
    /// Unrest added per turn.
    pub unrest: u8,
    /// Share of the province's seasonal tax base taken as loot.
    pub loot_share: f64,
}

/// Campaign siege duration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SiegeRules {
    /// Base duration in turns, added to the fortification level.
    pub base_turns: u32,
}

/// Extra effects of ransom and sack, on top of the occupation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutcomeRules {
    /// Gold taken = seasonal tax base of the province × this share × place share.
    pub gold_income_share: f64,
    /// Floor of the gold taken (livres, before the place share).
    pub min_gold: i64,
    /// Unrest added on top of the occupation (× place share).
    pub extra_unrest: u8,
    /// Population change of the province, percent (× place share).
    #[serde(default)]
    pub population_percent: i32,
    /// Devastation added to the province (× place share).
    #[serde(default)]
    pub devastation: u8,
    /// Buildings of the place destroyed (the most recent first).
    #[serde(default)]
    pub buildings_destroyed: u32,
    /// Experience (0-10 scale) gained by every unit of the capturing armies.
    #[serde(default)]
    pub unit_experience: u8,
    /// Piety of the capturing ruler.
    #[serde(default)]
    pub ruler_piety: i32,
    /// Papal favour of the capturing faction.
    #[serde(default)]
    pub papal_favor: i32,
    /// Opinion of the former holder of the place towards the captor.
    #[serde(default)]
    pub victim_opinion: i32,
    /// Opinion of every other living faction towards the captor.
    #[serde(default)]
    pub others_opinion: i32,
    /// Duration (turns) of the opinion modifiers.
    #[serde(default)]
    pub opinion_turns: u32,
}

/// « Raser / brûler »: the extra effects plus the loss of the place.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RazeRules {
    /// Effects shared with ransom and sack.
    pub effects: OutcomeRules,
    /// Fortification levels lost; a place left at level 0 is a ruin.
    pub fortification_loss: u8,
    /// Turns a ruin stays unusable (no recruitment, no construction).
    pub ruin_turns: u32,
}

/// Scores of the AI's choice: the highest total wins (ties: occupy, ransom,
/// sack, raze in that order). No randomness, so that adding the rule leaves
/// the campaign draws alone.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureAiRules {
    /// Base scores of every faction.
    pub default: OutcomeScores,
    /// Doctrine of a faction (replaces the default base scores).
    #[serde(default)]
    pub factions: BTreeMap<FactionId, OutcomeScores>,
    /// Treasury (livres) under which the faction is poor.
    pub poor_treasury: i64,
    /// Added when the faction is poor.
    pub poor_bonus: OutcomeScores,
    /// Added when the province's culture differs from the faction's.
    pub foreign_culture_bonus: OutcomeScores,
    /// Added when the faction is the de jure owner of the place (reconquest).
    pub own_land_bonus: OutcomeScores,
    /// Added for a place (not the city) whose province city stays in enemy
    /// hands: it cannot be held.
    pub exposed_bonus: OutcomeScores,
}

/// One score per outcome.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutcomeScores {
    #[serde(default)]
    pub occupy: i32,
    #[serde(default)]
    pub ransom: i32,
    #[serde(default)]
    pub sack: i32,
    #[serde(default)]
    pub raze: i32,
}

crate::bundled_rules!(CaptureRules, "rules/capture.json", default);
