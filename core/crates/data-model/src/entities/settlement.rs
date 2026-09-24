//! Settlement: a capturable place inside a province (`settlement.schema.json`),
//! plus the tuning file `data/settlements/rules.json` and the movement graph
//! `data/map/settlement_graph.json`. See `docs/design/2026-09-24-echelle-colonies.md`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::common::{LocalizedName, Sources};
use crate::entities::province::Province;
use crate::ids::{BuildingId, FactionId, ProvinceId, SettlementId, UnitTypeId};

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettlementRules {
    /// Starting garrison of a settlement by kind (unit type ids, may repeat).
    pub starting_garrison: BTreeMap<SettlementKind, Vec<UnitTypeId>>,
    pub full_province_bonus: FullProvinceBonus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
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
