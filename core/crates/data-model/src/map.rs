//! Map metadata and province geometry written by the `tools/geo` pipeline
//! into `data/map/` (see `docs/design/m1-campaign-map.md`, section 2).
//!
//! These files are produced by a tool rather than edited by hand, so unknown
//! keys are kept in `extra` instead of being rejected.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ids::ProvinceId;

/// Contents of `data/map/map.json`: projection and raster geometry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapMeta {
    /// Coordinate reference system, e.g. `"EPSG:3035"`.
    pub crs: String,
    /// `[minx, miny, maxx, maxy]` of the projected extent.
    pub bounds_projected: [f64; 4],
    /// `[width, height]` of the rasters in pixels.
    pub size_px: [u32; 2],
    pub meters_per_px: f64,
    /// Altitude encoded by a heightmap value of 0.
    pub height_min_m: f64,
    /// Altitude encoded by the maximum heightmap value.
    pub height_max_m: f64,
    /// Any other key the pipeline may add.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// One feature of `data/map/provinces.geojson`: the polygon of a province in
/// map coordinates (pixels) plus precomputed properties.
#[derive(Debug, Clone, PartialEq)]
pub struct ProvinceGeometry {
    pub id: ProvinceId,
    /// `[x, y]` centroid in map pixels.
    pub centroid: [f64; 2],
    /// Adjacent provinces as computed from shared edges and sea links.
    pub neighbors: Vec<ProvinceId>,
    /// `[x, y]` position of the capital marker in map pixels.
    pub capital_px: [f64; 2],
    /// Raw GeoJSON `geometry` member (Polygon or MultiPolygon), kept as-is.
    pub geometry: Value,
    /// Any other property the pipeline may add.
    pub extra: BTreeMap<String, Value>,
}

/// Serialized shape of a GeoJSON `FeatureCollection` of provinces.
#[derive(Debug, Deserialize)]
pub(crate) struct ProvinceFeatureCollection {
    #[serde(rename = "type")]
    pub(crate) kind: String,
    pub(crate) features: Vec<ProvinceFeature>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProvinceFeature {
    #[serde(rename = "type")]
    pub(crate) kind: String,
    pub(crate) properties: ProvinceProperties,
    pub(crate) geometry: Value,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProvinceProperties {
    pub(crate) id: ProvinceId,
    pub(crate) centroid: [f64; 2],
    #[serde(default)]
    pub(crate) neighbors: Vec<ProvinceId>,
    pub(crate) capital_px: [f64; 2],
    #[serde(flatten)]
    pub(crate) extra: BTreeMap<String, Value>,
}

impl From<ProvinceFeature> for ProvinceGeometry {
    fn from(feature: ProvinceFeature) -> Self {
        ProvinceGeometry {
            id: feature.properties.id,
            centroid: feature.properties.centroid,
            neighbors: feature.properties.neighbors,
            capital_px: feature.properties.capital_px,
            geometry: feature.geometry,
            extra: feature.properties.extra,
        }
    }
}
