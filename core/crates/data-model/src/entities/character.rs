//! Character: a historical or generated person (`character.schema.json`).

use serde::{Deserialize, Serialize};

use crate::common::{HistoricalDate, LocalizedName, Percent, Sources};
use crate::ids::{CharacterId, FactionId, ProvinceId, TraitId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sex {
    Male,
    Female,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Ruler,
    Consort,
    Heir,
    Prince,
    Commander,
    Noble,
    Prelate,
    Burgher,
    Exile,
    Claimant,
    Regent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CharacterStatus {
    AtCourt,
    InExile,
    Captive,
    OnCampaign,
    Minor,
}

/// A dated title held by the character.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Title {
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<HistoricalDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<HistoricalDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Level 0-10 in each of the three skill branches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Skills {
    pub command: u8,
    pub governance: u8,
    pub court: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Family {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub father: Option<CharacterId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mother: Option<CharacterId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spouses: Vec<CharacterId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<CharacterId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub siblings: Vec<CharacterId>,
}

impl Family {
    /// Every character referenced by this family record, with the field name.
    pub fn references(&self) -> impl Iterator<Item = (&'static str, &CharacterId)> {
        self.father
            .iter()
            .map(|id| ("family.father", id))
            .chain(self.mother.iter().map(|id| ("family.mother", id)))
            .chain(self.spouses.iter().map(|id| ("family.spouses", id)))
            .chain(self.children.iter().map(|id| ("family.children", id)))
            .chain(self.siblings.iter().map(|id| ("family.siblings", id)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Character {
    pub id: CharacterId,
    pub name: LocalizedName,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epithet: Option<String>,
    pub sex: Sex,
    pub house: String,
    pub faction: FactionId,
    pub role: Role,
    pub birth: HistoricalDate,
    /// Historical death; the simulation may bring it forward or delay it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub death: Option<HistoricalDate>,
    #[serde(default = "default_true")]
    pub historical: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub titles: Vec<Title>,
    pub skills: Skills,
    pub traits: Vec<TraitId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub piety: Option<Percent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<Family>,
    /// Province where the character stands in spring 1337.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_location: Option<ProvinceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<CharacterStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}

fn default_true() -> bool {
    true
}
