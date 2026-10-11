//! Navigable rivers and mountain passes of the campaign map (RG9, ADR 0343).
//!
//! - `data/movement/rivers.json` (schema `navigable_rivers.schema.json`): the
//!   rivers an army may sail between two riverside places.
//! - the `pass` entries of `data/map/crossings.json`: the polylines of the
//!   mountain passes, whose grid cells cost less than the mountains around.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::common::LocalizedName;
use crate::ids::SettlementId;

/// A riverside place of a navigable river.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RiverPlace {
    pub settlement: SettlementId,
    /// Position along the course, kilometres from the source.
    pub km: f64,
}

/// A navigable river: places ordered from the source (upstream) to the mouth.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavigableRiver {
    pub id: String,
    pub name: LocalizedName,
    pub places: Vec<RiverPlace>,
}

/// Contents of `data/movement/rivers.json`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NavigableRivers {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub rivers: Vec<NavigableRiver>,
}

/// A mountain pass of `data/map/crossings.json` (other fields ignored).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapPass {
    pub id: String,
    #[serde(default)]
    pub name: String,
    /// Longitude/latitude polyline of the road over the pass.
    #[serde(default)]
    pub route: Vec<[f64; 2]>,
}

#[derive(Deserialize)]
struct CrossingsFile {
    crossings: Vec<RawCrossing>,
}

#[derive(Deserialize)]
struct RawCrossing {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    route: Vec<[f64; 2]>,
}

/// Reads the `pass` entries of `crossings.json`; empty when absent or malformed.
pub(crate) fn read_passes(path: &Path) -> Vec<MapPass> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str::<CrossingsFile>(&text).ok())
        .map(|file| {
            file.crossings
                .into_iter()
                .filter(|c| c.kind == "pass")
                .map(|c| MapPass {
                    id: c.id,
                    name: c.name,
                    route: c.route,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Reads `rivers.json`; empty when absent.
pub(crate) fn read_rivers(path: &Path) -> Result<NavigableRivers, crate::load::DataError> {
    if !path.is_file() {
        return Ok(NavigableRivers::default());
    }
    crate::load::read_json(path)
}
