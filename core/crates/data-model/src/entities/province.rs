//! Province: a region of the campaign map (`province.schema.json`).

use serde::{Deserialize, Serialize};

use crate::common::{LocalizedName, Percent, SocialClass, Sources};
use crate::ids::{BuildingId, CultureId, FactionId, ProvinceId, ReligionId, ResourceId, SeaZoneId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Terrain {
    Plains,
    Hills,
    Mountains,
    Forest,
    Marsh,
    Heath,
    Bocage,
    /// Dry open grassland (Pontic steppe): moves like plains, thin forage, cavalry country.
    Steppe,
    /// Arid land (Sahara fringe, Syrian desert): slow, very thin forage, summer attrition.
    Desert,
}

impl Terrain {
    /// The `snake_case` key used in JSON files.
    pub fn key(self) -> &'static str {
        match self {
            Terrain::Plains => "plains",
            Terrain::Hills => "hills",
            Terrain::Mountains => "mountains",
            Terrain::Forest => "forest",
            Terrain::Marsh => "marsh",
            Terrain::Heath => "heath",
            Terrain::Bocage => "bocage",
            Terrain::Steppe => "steppe",
            Terrain::Desert => "desert",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Climate {
    Oceanic,
    Continental,
    Mediterranean,
    Mountain,
    /// Hot and dry (North Africa, Levant, Mesopotamia).
    Arid,
    /// Dry continental grassland (Pontic-Caspian steppe).
    Steppe,
}

/// Capital city with its localized name and optional WGS84 position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapitalCity {
    pub name: LocalizedName,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lat: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lon: Option<f64>,
}

/// One social class of a province: head count and 0-100 gauges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PopulationClass {
    pub count: u64,
    pub unrest: Percent,
    pub health: Percent,
    pub wealth: Percent,
    pub goods_satisfaction: Percent,
}

/// Population split by social class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PopulationClasses {
    pub peasants: PopulationClass,
    pub burghers: PopulationClass,
    pub clergy: PopulationClass,
    pub nobility: PopulationClass,
}

impl PopulationClasses {
    /// The entry for a given class.
    pub fn get(&self, class: SocialClass) -> &PopulationClass {
        match class {
            SocialClass::Peasants => &self.peasants,
            SocialClass::Burghers => &self.burghers,
            SocialClass::Clergy => &self.clergy,
            SocialClass::Nobility => &self.nobility,
        }
    }

    /// Iterates over `(class, entry)` in schema order.
    pub fn iter(&self) -> impl Iterator<Item = (SocialClass, &PopulationClass)> {
        SocialClass::ALL
            .into_iter()
            .map(|class| (class, self.get(class)))
    }

    /// Iterates mutably over the four entries in schema order.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut PopulationClass> {
        [
            &mut self.peasants,
            &mut self.burghers,
            &mut self.clergy,
            &mut self.nobility,
        ]
        .into_iter()
    }

    /// Total head count across all classes.
    pub fn total(&self) -> u64 {
        self.iter().map(|(_, entry)| entry.count).sum()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Population {
    /// Pre-plague populations are orders of magnitude; defaults to `true`.
    #[serde(default = "default_true")]
    pub uncertain: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub classes: PopulationClasses,
}

fn default_true() -> bool {
    true
}

/// Geographic seed data written by the `tools/geo` pipeline (see `m1-campaign-map.md`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProvinceGeo {
    /// `[lon, lat]` of the capital city.
    pub capital_lonlat: [f64; 2],
    /// `[lon, lat]` of the Voronoi seed.
    pub seed_lonlat: [f64; 2],
    /// Relative weight of the seed in the weighted Voronoi diagram.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voronoi_weight: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Province {
    pub id: ProvinceId,
    pub name: LocalizedName,
    pub region: String,
    pub terrain: Terrain,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub climate: Option<Climate>,
    /// Land neighbours.
    pub neighbors: Vec<ProvinceId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rivers: Vec<String>,
    pub coastal: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sea_zones: Vec<SeaZoneId>,
    pub resources: Vec<ResourceId>,
    pub capital_city: CapitalCity,
    /// Faction controlling the province in spring 1337.
    pub owner: FactionId,
    pub culture: CultureId,
    pub religion: ReligionId,
    pub population: Population,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub buildings: Vec<BuildingId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fortification_level: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_supply: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geo: Option<ProvinceGeo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}

impl Province {
    /// Whether the province has at least one usable port.
    pub fn has_port(&self) -> bool {
        !self.ports.is_empty()
    }
}
