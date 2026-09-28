//! Feudal title (lot FE, ADR 0098), mirroring `data/schemas/title.schema.json`
//! (`data/titles/<id>.json`).
//!
//! Titles sit above factions: they carry the *de jure* hierarchy (kingdom,
//! duchy, county) and the provinces; a faction holds one or more titles.
//! Suzerains, vassals and province allegiances are deduced from who holds
//! which title (`sim_campaign::feudal`), never stored.

use serde::{Deserialize, Serialize};

use crate::common::{LocalizedName, Sources};
use crate::entities::faction::Heraldry;
use crate::ids::{FactionId, ProvinceId, TitleId};

/// Rank of a title; at most three levels. The Empire is a `Kingdom`.
///
/// The derived order runs from lowest to highest: `County < Duchy < Kingdom`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TitleRank {
    County,
    Duchy,
    Kingdom,
}

impl TitleRank {
    /// Snake-case key, as written in the data.
    pub fn key(self) -> &'static str {
        match self {
            TitleRank::County => "county",
            TitleRank::Duchy => "duchy",
            TitleRank::Kingdom => "kingdom",
        }
    }
}

/// Holder of a title in spring 1337.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TitleHolder {
    pub faction: FactionId,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub uncertain: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}

/// Condition of a historical objective carried by a faction's primary title
/// (spec § 4.8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TitleObjectiveCondition {
    /// The faction holds `title`.
    HoldTitle { title: TitleId },
    /// The faction owns every listed province.
    HoldProvinces { provinces: Vec<ProvinceId> },
    /// The faction has no suzerain.
    BeIndependent,
    /// `faction` is a direct vassal of ours.
    BeLiegeOf { faction: FactionId },
    /// The faction holds the kingdom-rank `title` (a crown).
    HoldCrown { title: TitleId },
}

/// One historical objective (2-3 per primary title).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TitleObjective {
    pub id: String,
    pub title: String,
    pub description: String,
    pub condition: TitleObjectiveCondition,
}

/// A feudal title (`data/titles/<id>.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeudalTitle {
    pub id: TitleId,
    pub rank: TitleRank,
    pub name: LocalizedName,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heraldry: Option<Heraldry>,
    /// Title this one depends on, of strictly higher rank; absent for a
    /// sovereign kingdom.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub de_jure_liege: Option<TitleId>,
    /// Provinces of the title's own domain (its vassal titles' provinces
    /// are not repeated here).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub de_jure_provinces: Vec<ProvinceId>,
    pub holder_1337: TitleHolder,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub objectives: Vec<TitleObjective>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranks_are_ordered_from_county_to_kingdom() {
        assert!(TitleRank::County < TitleRank::Duchy);
        assert!(TitleRank::Duchy < TitleRank::Kingdom);
    }

    #[test]
    fn parses_a_title() {
        let json = r#"{"id":"tit_guyenne","rank":"duchy","name":{"display":"Duché de Guyenne"},
            "de_jure_liege":"tit_france","de_jure_provinces":["prov_guyenne"],
            "holder_1337":{"faction":"fac_england","sources":["x"]},
            "objectives":[{"id":"o1","title":"t","description":"d","condition":{"kind":"be_independent"}}]}"#;
        let title: FeudalTitle = serde_json::from_str(json).expect("valid title");
        assert_eq!(title.rank, TitleRank::Duchy);
        assert_eq!(title.holder_1337.faction.as_str(), "fac_england");
        assert_eq!(
            title.objectives[0].condition,
            TitleObjectiveCondition::BeIndependent
        );
    }
}
