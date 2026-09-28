//! Faction: a state or power on the campaign map (`faction.schema.json`).

use serde::{Deserialize, Serialize};

use crate::common::{HistoricalDate, LocalizedName, Percent, Sources};
use crate::ids::{
    CharacterId, CultureId, FactionId, ProvinceId, ReligionId, TechnologyId, TitleId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Government {
    Kingdom,
    Duchy,
    County,
    Principality,
    Republic,
    Theocracy,
    Empire,
    Lordship,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuccessionLaw {
    Salic,
    MalePreferencePrimogeniture,
    CognaticPrimogeniture,
    Elective,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationStatus {
    War,
    Peace,
    Alliance,
    Truce,
    Embargo,
    Vassal,
    Overlord,
    MarriageTie,
}

/// Coat of arms: French blazon plus UI colours (`#RRGGBB`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Heraldry {
    pub blazon: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub primary_color: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary_color: Option<String>,
    #[serde(default)]
    pub uncertain: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Starting diplomatic relation with another faction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Relation {
    pub faction: FactionId,
    pub status: RelationStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<HistoricalDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<HistoricalDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimKind {
    /// Claim on the throne of `faction`.
    Throne,
    /// Claim on `province`.
    Province,
}

/// A starting claim (casus belli), M5.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimData {
    pub kind: ClaimKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faction: Option<FactionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub province: Option<ProvinceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// One condition of a campaign objective (M10 victory).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ObjectiveCondition {
    /// The faction controls every listed province.
    ControlAll { provinces: Vec<ProvinceId> },
    /// The faction controls at least `count` of the listed provinces.
    ControlCount {
        provinces: Vec<ProvinceId>,
        count: u32,
    },
    /// `faction` controls none of the listed provinces.
    NoForeignControl {
        faction: FactionId,
        provinces: Vec<ProvinceId>,
    },
    /// The faction has no suzerain.
    Independent,
    /// `faction` is our vassal, or has disappeared.
    Subjugate { faction: FactionId },
}

/// A campaign objective shown to the player.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Objective {
    pub id: String,
    pub title: String,
    pub description: String,
    pub condition: ObjectiveCondition,
}

/// Victory conditions of a playable faction: every objective met before
/// `end_year` wins the campaign.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VictoryConditions {
    pub end_year: i32,
    pub objectives: Vec<Objective>,
    /// Seasons all objectives must hold in a row before victory (F9);
    /// absent = immediate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hold_turns: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiPersonality {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aggression: Option<Percent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diplomacy: Option<Percent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trade: Option<Percent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub piety: Option<Percent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Faction {
    pub id: FactionId,
    pub name: LocalizedName,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adjective: Option<String>,
    pub government: Government,
    #[serde(default)]
    pub playable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ruler: Option<CharacterId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heir: Option<CharacterId>,
    pub capital: ProvinceId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capital_city: Option<String>,
    pub religion: ReligionId,
    pub culture: CultureId,
    pub succession_law: SuccessionLaw,
    /// Faction this one is a vassal of at campaign start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suzerain: Option<FactionId>,
    /// Main title of the faction (lot FE, ADR 0098): its rank and *de jure*
    /// liege place the faction in the feudal tree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primary_title: Option<TitleId>,
    pub heraldry: Heraldry,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub titles: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub treasury: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prestige: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub starting_technologies: Vec<TechnologyId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relations: Vec<Relation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub claims: Vec<ClaimData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub victory: Option<VictoryConditions>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ai_personality: Option<AiPersonality>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}

impl Faction {
    /// UI-friendly short name, falling back to the display name.
    pub fn short_or_display_name(&self) -> &str {
        self.short_name.as_deref().unwrap_or(&self.name.display)
    }
}
