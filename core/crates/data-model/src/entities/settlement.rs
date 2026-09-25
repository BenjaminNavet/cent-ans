//! Settlement: a capturable place inside a province (`settlement.schema.json`),
//! plus the tuning file `data/settlements/rules.json` and the movement graph
//! `data/map/settlement_graph.json`. See `docs/design/2026-09-24-echelle-colonies.md`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::common::{LocalizedName, Sources};
use crate::entities::province::Province;
use crate::ids::{BuildingId, FactionId, ProvinceId, SeaZoneId, SettlementId, UnitTypeId};

/// Type of settlement; fixes allowed buildings and the siege model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettlementKind {
    /// Capital city of the province (exactly one per province).
    City,
    /// Walled town or bastide.
    Town,
    /// Castle or fortress.
    Castle,
    /// Abbey, fortified priory or commandery.
    Abbey,
    /// Unfortified borough or village.
    Village,
}

impl SettlementKind {
    /// Every kind, in schema order.
    pub const ALL: [SettlementKind; 5] = [
        SettlementKind::City,
        SettlementKind::Town,
        SettlementKind::Castle,
        SettlementKind::Abbey,
        SettlementKind::Village,
    ];

    /// The `snake_case` key used in JSON files.
    pub fn key(self) -> &'static str {
        match self {
            SettlementKind::City => "city",
            SettlementKind::Town => "town",
            SettlementKind::Castle => "castle",
            SettlementKind::Abbey => "abbey",
            SettlementKind::Village => "village",
        }
    }
}

/// Static data of one settlement (one entry of `data/settlements/<province>.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settlement {
    pub id: SettlementId,
    pub province: ProvinceId,
    pub kind: SettlementKind,
    pub name: LocalizedName,
    /// `[lon, lat]`, WGS84 decimal degrees.
    pub lonlat: [f64; 2],
    /// Share of the province (tax, recruitment), 1-100, normalised per province.
    pub weight: u8,
    /// Owner in 1337 when it differs from the province owner (enclave).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<FactionId>,
    /// 0 for a village, otherwise 1-4.
    pub fortification_level: u8,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub buildings: Vec<BuildingId>,
    #[serde(default)]
    pub port: bool,
    /// Sea the port opens onto, when its province touches several (Calais
    /// and Dover on the Channel, not the North Sea): the sea of a crossing
    /// and of its interception (lot NV2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sea_zone: Option<SeaZoneId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Sources,
}

impl Settlement {
    /// The fallback city generated from `Province::capital_city` for a
    /// province that has no settlement file (lot C1). `id` is chosen by the
    /// caller (see [`city_id_for`]).
    pub fn fallback_city(province: &Province, id: SettlementId) -> Settlement {
        let city = &province.capital_city;
        let lonlat = match (city.lon, city.lat, &province.geo) {
            (Some(lon), Some(lat), _) => [lon, lat],
            (_, _, Some(geo)) => geo.capital_lonlat,
            _ => [0.0, 0.0],
        };
        Settlement {
            id,
            province: province.id.clone(),
            kind: SettlementKind::City,
            name: city.name.clone(),
            lonlat,
            weight: 100,
            owner: None,
            fortification_level: province.fortification_level.unwrap_or(0),
            buildings: province.buildings.clone(),
            port: province.coastal && province.has_port(),
            sea_zone: None,
            description: None,
            sources: Vec::new(),
        }
    }
}

/// `set_<slug>` for a display name: ASCII-folded, lowercase, runs of other
/// characters collapsed to `_`. Returns `None` when nothing usable remains.
pub fn city_id_for(display_name: &str) -> Option<SettlementId> {
    let slug = slugify(display_name);
    if slug.is_empty() {
        return None;
    }
    SettlementId::new(format!("{}{slug}", SettlementId::PREFIX)).ok()
}

/// ASCII slug: accents folded, lowercase `[a-z0-9]` runs joined by `_`.
pub fn slugify(text: &str) -> String {
    let mut folded = String::with_capacity(text.len());
    for ch in text.chars() {
        match fold_char(ch) {
            Some(ascii) => folded.push_str(ascii),
            None if ch.is_ascii_alphanumeric() => folded.push(ch.to_ascii_lowercase()),
            None => folded.push('_'),
        }
    }
    folded
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

/// Latin letters with diacritics and ligatures folded to lowercase ASCII.
fn fold_char(ch: char) -> Option<&'static str> {
    Some(match ch {
        'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' => "a",
        'ç' | 'Ç' => "c",
        'è' | 'é' | 'ê' | 'ë' | 'È' | 'É' | 'Ê' | 'Ë' => "e",
        'ì' | 'í' | 'î' | 'ï' | 'Ì' | 'Í' | 'Î' | 'Ï' => "i",
        'ñ' | 'Ñ' => "n",
        'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' => "o",
        'ù' | 'ú' | 'û' | 'ü' | 'Ù' | 'Ú' | 'Û' | 'Ü' => "u",
        'ý' | 'ÿ' | 'Ý' => "y",
        'æ' | 'Æ' => "ae",
        'œ' | 'Œ' => "oe",
        'ß' => "ss",
        'ð' | 'Ð' => "d",
        'þ' | 'Þ' => "th",
        _ => return None,
    })
}

/// Bonus for holding every settlement of a province (spec § 4.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FullProvinceBonus {
    /// Extra province income, in percent.
    pub income_percent: i32,
    /// Unrest change per season (negative lowers unrest).
    pub unrest_per_season: i32,
}

/// Contents of `data/settlements/rules.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettlementRules {
    /// Starting garrison of a settlement by kind (unit type ids, may repeat).
    pub starting_garrison: BTreeMap<SettlementKind, Vec<UnitTypeId>>,
    pub full_province_bonus: FullProvinceBonus,
    /// Movement on the settlement graph (lot C4).
    #[serde(default)]
    pub movement: MovementRules,
    /// Share of a garrison's unit upkeep paid by the controller, in per
    /// cent, by settlement kind (lot C4). Kinds left out pay the campaign
    /// default (`GARRISON_UPKEEP_PERCENT` of the simulation).
    #[serde(default)]
    pub garrison_upkeep_percent: BTreeMap<SettlementKind, i64>,
    /// Share of the upkeep of a settlement's buildings paid by its
    /// controller, in per cent, by settlement kind (lot C7a); kinds left out
    /// pay in full.
    #[serde(default)]
    pub building_upkeep_percent: BTreeMap<SettlementKind, i64>,
    /// Most units an army can leave as a settlement's garrison, by kind
    /// (lot C7a, `Order::GarrisonUnits`); kinds left out have no cap.
    #[serde(default)]
    pub garrison_cap: BTreeMap<SettlementKind, usize>,
    /// Where the loser of a battle falls back (lot C7a).
    #[serde(default)]
    pub retreat: RetreatRules,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Retreat of a beaten army (lot C7a, `rules.json` § `retreat`). Radii are
/// in v1 province steps (times `MovementRules::points_per_step`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetreatRules {
    /// Radius searched for a friendly settlement free of enemy armies,
    /// along a path that crosses no enemy place.
    pub friendly_radius_steps: f64,
    /// Failing that, radius searched for a settlement no enemy holds.
    pub neutral_radius_steps: f64,
    /// Stragglers lost on a retreat to a neutral settlement, per cent of
    /// each unit.
    pub neutral_loss_percent: u32,
    /// Extra losses of an army with nowhere to go (rout), per cent of each
    /// unit.
    pub rout_loss_percent: u32,
    /// After a rout, the army disperses below this share (per cent) of its
    /// maximum strength; otherwise the survivors rally at the nearest
    /// reachable friendly settlement (or disperse if there is none).
    pub rout_dissolve_below_percent: u32,
    /// Lot M5b: a beaten army that lost at least this share (per cent) of
    /// its men in the battle and has no friendly place within reach cannot
    /// fall back in order: it routs. 100 disables the rule.
    #[serde(default = "default_heavy_defeat_losses_percent")]
    pub heavy_defeat_losses_percent: u32,
}

fn default_heavy_defeat_losses_percent() -> u32 {
    100
}

impl Default for RetreatRules {
    fn default() -> Self {
        RetreatRules {
            friendly_radius_steps: 2.0,
            neutral_radius_steps: 1.0,
            neutral_loss_percent: 10,
            rout_loss_percent: 50,
            rout_dissolve_below_percent: 30,
            heavy_defeat_losses_percent: default_heavy_defeat_losses_percent(),
        }
    }
}

/// Movement tuning of `rules.json` (lot C4). The unit of edge costs and
/// movement points is the kilometre of plain: great-circle distance times
/// the terrain cost, halved on a road.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MovementRules {
    /// Movement points worth one v1 province step (mean distance between
    /// the cities of neighbouring provinces).
    pub points_per_step: f64,
    /// Cost of a port-to-port crossing in steps (fallback graph).
    pub sea_crossing_steps: f64,
    /// Fallback graph: closest settlement pairs linked between two
    /// neighbouring provinces, on top of the city-to-city link.
    pub fallback_links_per_neighbor: u32,
    /// Share of the v1 seasonal steps (3, 2 in winter) an army marches on
    /// the settlement graph (lot C7a); flat and percent `Movement` effects
    /// are scaled with it.
    #[serde(default = "one")]
    pub season_scale: f64,
    /// Cost of a road edge relative to the same distance off-road (lot
    /// C7a). The C3 graph bakes a factor of 0.5 into its road edges; the
    /// loader rescales them to this factor.
    #[serde(default = "baked_road_factor")]
    pub road_cost_factor: f64,
}

fn one() -> f64 {
    1.0
}

/// Road factor baked into the costs of `settlement_graph.json` (lot C3).
pub const BAKED_ROAD_COST_FACTOR: f64 = 0.5;

fn baked_road_factor() -> f64 {
    BAKED_ROAD_COST_FACTOR
}

impl Default for MovementRules {
    fn default() -> Self {
        MovementRules {
            points_per_step: 140.0,
            sea_crossing_steps: 2.0,
            fallback_links_per_neighbor: 2,
            season_scale: 1.0,
            road_cost_factor: BAKED_ROAD_COST_FACTOR,
        }
    }
}

/// One edge of the settlement movement graph (spec § 4.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettlementEdge {
    pub from: SettlementId,
    pub to: SettlementId,
    /// Movement cost of the edge (terrain-weighted distance, halved on roads).
    pub cost: f64,
    #[serde(default)]
    pub road: bool,
    #[serde(default)]
    pub sea: bool,
}

/// Contents of `data/map/settlement_graph.json` (produced by `tools/geo`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettlementGraph {
    pub edges: Vec<SettlementEdge>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_folds_accents_and_separators() {
        assert_eq!(slugify("Édimbourg"), "edimbourg");
        assert_eq!(slugify("Bar-le-Duc"), "bar_le_duc");
        assert_eq!(slugify("Saint Andrews"), "saint_andrews");
        assert_eq!(slugify("Lödöse"), "lodose");
        assert_eq!(slugify("L'Isle-Jourdain"), "l_isle_jourdain");
        assert_eq!(city_id_for("Málaga").unwrap().as_str(), "set_malaga");
        assert!(city_id_for("---").is_none());
    }

    #[test]
    fn kind_keys_round_trip() {
        for kind in SettlementKind::ALL {
            let json = serde_json::to_string(&kind).unwrap();
            assert_eq!(json, format!("\"{}\"", kind.key()));
        }
    }
}
