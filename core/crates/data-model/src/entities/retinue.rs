//! General's retinue (lot C7), mirroring
//! `data/schemas/retinue.schema.json` (`data/retinue.json`).
//!
//! The rules that read it (acquisition, inheritance, transfer, effects)
//! live in `sim-campaign::retinue`.

use serde::{Deserialize, Serialize};

use crate::common::{Effect, LocalizedName, Sources};
use crate::ids::{BuildingId, CompanionId, FactionId, TraitId};

/// Contents of `data/retinue.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Retinue {
    /// Most companions one character may keep.
    pub max_per_character: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Catalogue order is the roll order of the acquisition rule.
    pub companions: Vec<Companion>,
}

impl Retinue {
    /// The companion `id`, if defined.
    pub fn companion(&self, id: &CompanionId) -> Option<&Companion> {
        self.companions.iter().find(|c| &c.id == id)
    }
}

/// Family of a companion (colour of its vignette).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompanionCategory {
    Military,
    Court,
    Faith,
    Learning,
    Commerce,
    Intrigue,
}

impl CompanionCategory {
    /// Stable key for the UI.
    pub fn key(self) -> &'static str {
        match self {
            CompanionCategory::Military => "military",
            CompanionCategory::Court => "court",
            CompanionCategory::Faith => "faith",
            CompanionCategory::Learning => "learning",
            CompanionCategory::Commerce => "commerce",
            CompanionCategory::Intrigue => "intrigue",
        }
    }
}

/// One kind of companion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Companion {
    pub id: CompanionId,
    pub name: LocalizedName,
    pub category: CompanionCategory,
    /// Letter or symbol drawn on the vignette.
    pub glyph: String,
    /// Added to the holder's trait and skill effects.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<Effect>,
    pub acquisition: Vec<Acquisition>,
    #[serde(default)]
    pub conditions: CompanionConditions,
    /// Passes to the holder's heir at his death.
    #[serde(default)]
    pub inheritable: bool,
    pub description: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}

/// Occasion on which a companion may join.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcquisitionTrigger {
    /// The general of the winning side of a battle.
    BattleWon,
    /// Any general of a battle, won or lost.
    BattleFought,
    /// The besieging general of a siege won.
    SiegeWon,
    /// The general of a chevauchée.
    RaidLed,
    /// A season spent by the general's army in a friendly settlement whose
    /// province has `building`.
    SeasonInSettlement,
    /// The ruler of a faction receiving a ransom.
    RansomReceived,
}

impl AcquisitionTrigger {
    /// Stable key (also salt of the deterministic roll).
    pub fn key(self) -> &'static str {
        match self {
            AcquisitionTrigger::BattleWon => "battle_won",
            AcquisitionTrigger::BattleFought => "battle_fought",
            AcquisitionTrigger::SiegeWon => "siege_won",
            AcquisitionTrigger::RaidLed => "raid_led",
            AcquisitionTrigger::SeasonInSettlement => "season_in_settlement",
            AcquisitionTrigger::RansomReceived => "ransom_received",
        }
    }
}

/// One acquisition rule: on `trigger`, joins with `chance_permille`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Acquisition {
    pub trigger: AcquisitionTrigger,
    pub chance_permille: u32,
    /// Required building for `season_in_settlement`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub building: Option<BuildingId>,
}

/// Who may gain a companion.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompanionConditions {
    /// Empty: every faction.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub factions: Vec<FactionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_command: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_battles: Option<u32>,
    /// At least one of them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requires_traits: Vec<TraitId>,
    /// None of them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub excludes_traits: Vec<TraitId>,
}
