//! Loading of the water and road data of `data/map/` (formats read by the Godot loaders).
//!
//! * `map.json` -> size (via `data_model::MapMeta`), `coastline.geojson`, `roads.geojson`,
//!   `rivers_render.json` (via `data_model::map_geo`);
//! * `lakes.json` (`polygon_px`, `level_m`, `center_px`), `crossings_px.json` (snapped
//!   bridges/fords/ferries), `river_display.json`, `river_names.json`, `sea_basins.json`,
//!   `freshwater_px.json` (counts only for now).
//!
//! Every file is optional (test data): a missing file gives an empty collection, a parse error
//! is logged and treated the same way.

use std::collections::HashMap;
use std::path::Path;

use bevy::prelude::*;
use data_model::map::MapMeta;
use data_model::map_geo;
use serde::Deserialize;

/// A lake polygon of `lakes.json`.
#[derive(Debug, Clone)]
pub struct Lake {
    pub id: String,
    pub name: String,
    pub level_m: f32,
    pub center: Vec2,
    pub polygon: Vec<Vec2>,
}

/// A river piece with one width per point (`rivers_render.json`).
#[derive(Debug, Clone)]
pub struct River {
    pub name: String,
    pub importance: i64,
    pub points: Vec<Vec2>,
    pub widths: Vec<f32>,
}

/// Emblematic-town zone where the generic river is not drawn.
#[derive(Debug, Clone)]
pub struct Zone {
    pub id: String,
    pub px: Vec2,
    pub radius_px: f32,
}

/// A road line; `main` roads get the two-tone line at the medium view.
#[derive(Debug, Clone)]
pub struct Road {
    pub main: bool,
    pub points: Vec<Vec2>,
}

/// A snapped historical crossing (bridge, ford, ferry).
#[derive(Debug, Clone)]
pub struct Crossing {
    pub id: String,
    pub name: String,
    pub structure: String,
    pub px: Vec2,
    pub dir: Vec2,
    pub width: f32,
}

/// Look of one sea basin (`sea_basins.json`, neutral defaults completed).
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct BasinLook {
    pub tint: [f32; 3],
    pub clarity: f32,
    pub swell: f32,
    pub long_swell: f32,
    pub foam: f32,
    pub whitecaps: f32,
    pub grey_scale: f32,
}

impl Default for BasinLook {
    fn default() -> Self {
        Self {
            tint: [1.0; 3],
            clarity: 1.0,
            swell: 1.0,
            long_swell: 0.0,
            foam: 1.0,
            whitecaps: 0.0,
            grey_scale: 1.0,
        }
    }
}

/// A sea basin polygon with its look.
#[derive(Debug, Clone, Deserialize)]
pub struct Basin {
    pub id: String,
    #[serde(default)]
    pub look: BasinLook,
    #[serde(default)]
    pub polygon_px: Vec<[f32; 2]>,
}

/// `sea_basins.json`.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SeaBasins {
    pub cell_px: u32,
    pub blur_cells: u32,
    pub default: BasinLook,
    pub basins: Vec<Basin>,
}

impl Default for SeaBasins {
    fn default() -> Self {
        Self {
            cell_px: 32,
            blur_cells: 4,
            default: BasinLook::default(),
            basins: Vec::new(),
        }
    }
}

/// Maximum number of basins (one weight channel each).
pub const MAX_BASINS: usize = 4;

/// Subset of `river_display.json` used by the materials and labels.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RiverDisplay {
    pub major_min_px: f32,
    pub minor_min_px: f32,
    pub importance_min_px_scale: Vec<f32>,
    pub thin_fade: f32,
    pub minor_max_distance: f32,
    pub minor_fade: [f32; 2],
    pub minor_far_alpha: f32,
    pub shallow_color: [f32; 3],
    pub deep_color: [f32; 3],
    pub bank_ink_color: [f32; 3],
    pub bank_ink_strength: f32,
    pub bank_ink_px: f32,
    pub edge_soft_px: f32,
    pub sky_reflect: f32,
    pub sky_color: [f32; 3],
    pub labels: LabelConfig,
}

impl Default for RiverDisplay {
    fn default() -> Self {
        Self {
            major_min_px: 1.6,
            minor_min_px: 1.0,
            importance_min_px_scale: vec![1.0; 7],
            thin_fade: 0.7,
            minor_max_distance: 1500.0 / 7080.0,
            minor_fade: [0.0, 0.0],
            minor_far_alpha: 1.0,
            shallow_color: [0.30, 0.40, 0.36],
            deep_color: [0.13, 0.26, 0.31],
            bank_ink_color: [0.05, 0.13, 0.20],
            bank_ink_strength: 0.0,
            bank_ink_px: 0.8,
            edge_soft_px: 0.7,
            sky_reflect: 0.55,
            sky_color: [0.66, 0.72, 0.76],
            labels: LabelConfig::default(),
        }
    }
}

/// `river_display.json` -> `labels`.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LabelConfig {
    pub enabled: bool,
    pub min_segment_px: f32,
    pub spacing_px: Vec<f32>,
    pub max_distance_by_importance: Vec<f32>,
    pub min_importance: i64,
    pub clear_px_from_settlements: f32,
    pub max_labels: usize,
    pub major_max_distance: f32,
    pub major_names: Vec<String>,
}

impl Default for LabelConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            min_segment_px: 40.0,
            spacing_px: Vec::new(),
            max_distance_by_importance: Vec::new(),
            min_importance: 0,
            clear_px_from_settlements: 4.0,
            max_labels: 900,
            major_max_distance: 0.0,
            major_names: Vec::new(),
        }
    }
}

/// Everything the water plugin draws.
#[derive(Debug, Clone, Default)]
pub struct WaterData {
    pub map_size: Vec2,
    pub meters_per_px: f32,
    pub coastlines: Vec<Vec<Vec2>>,
    pub lakes: Vec<Lake>,
    pub rivers: Vec<River>,
    pub zones: Vec<Zone>,
    pub bank_px: f32,
    pub display: RiverDisplay,
    pub river_names: HashMap<String, String>,
    pub roads: Vec<Road>,
    pub crossings: Vec<Crossing>,
    pub basins: SeaBasins,
    /// Counts of `freshwater_px.json` lists (ponds, salt pans, waterfalls, fords, torrents).
    pub freshwater_counts: HashMap<String, usize>,
}

fn read(dir: &Path, name: &str) -> Option<Vec<u8>> {
    std::fs::read(dir.join(name)).ok()
}

fn json<T: for<'de> Deserialize<'de>>(dir: &Path, name: &str) -> Option<T> {
    let bytes = read(dir, name)?;
    match serde_json::from_slice(&bytes) {
        Ok(value) => Some(value),
        Err(error) => {
            warn!("water: {name}: {error}");
            None
        }
    }
}

fn to_vec2(points: &[[f32; 2]]) -> Vec<Vec2> {
    points.iter().map(|p| Vec2::new(p[0], p[1])).collect()
}

#[derive(Deserialize)]
struct RawLakes {
    #[serde(default)]
    lakes: Vec<RawLake>,
}

#[derive(Deserialize)]
struct RawLake {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    level_m: f32,
    #[serde(default)]
    center_px: [f32; 2],
    #[serde(default)]
    polygon_px: Vec<[f32; 2]>,
}

#[derive(Deserialize)]
struct RawCrossings {
    #[serde(default)]
    crossings: Vec<RawCrossing>,
}

#[derive(Deserialize)]
struct RawCrossing {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default = "stone")]
    structure: String,
    #[serde(default)]
    px: [f32; 2],
    #[serde(default = "east")]
    dir: [f32; 2],
    #[serde(default = "half")]
    width: f32,
    #[serde(default)]
    snapped: bool,
    #[serde(default)]
    in_custom_zone: bool,
}

fn stone() -> String {
    "stone".into()
}
fn east() -> [f32; 2] {
    [1.0, 0.0]
}
fn half() -> f32 {
    0.5
}

#[derive(Deserialize)]
struct RawNames {
    #[serde(default)]
    names: HashMap<String, String>,
}

impl WaterData {
    /// Reads the map directory (`<root>/data/map`).
    pub fn load(map_dir: &Path) -> WaterData {
        let mut data = WaterData::default();
        match MapMeta::load(map_dir) {
            Ok(meta) => {
                data.map_size = Vec2::new(meta.size_px[0] as f32, meta.size_px[1] as f32);
                data.meters_per_px = meta.meters_per_px as f32;
            }
            Err(error) => {
                warn!("water: map.json: {error}");
                data.map_size = Vec2::new(7080.0, 6104.0);
                data.meters_per_px = 719.0;
            }
        }
        data.bank_px = 1.1;
        if let Some(bytes) = read(map_dir, "coastline.geojson") {
            match map_geo::parse_coastline(&bytes) {
                Ok(lines) => data.coastlines = lines.iter().map(|l| to_vec2(l)).collect(),
                Err(error) => warn!("water: coastline: {error}"),
            }
        }
        if let Some(bytes) = read(map_dir, "rivers_render.json") {
            match map_geo::parse_rendered_rivers(&bytes) {
                Ok(rendered) => {
                    data.bank_px = rendered.bank_px;
                    data.zones = rendered
                        .zones
                        .into_iter()
                        .map(|z| Zone {
                            id: z.id,
                            px: Vec2::from(z.px),
                            radius_px: z.radius_px,
                        })
                        .collect();
                    data.rivers = rendered
                        .rivers
                        .into_iter()
                        .map(|r| River {
                            name: r.name,
                            importance: r.importance,
                            points: to_vec2(&r.points),
                            widths: r.widths,
                        })
                        .collect();
                }
                Err(error) => warn!("water: rivers_render: {error}"),
            }
        }
        if let Some(bytes) = read(map_dir, "roads.geojson") {
            match map_geo::parse_roads(&bytes) {
                Ok(roads) => {
                    data.roads = roads
                        .into_iter()
                        .map(|r| Road {
                            main: r.kind == "main",
                            points: to_vec2(&r.points),
                        })
                        .collect();
                }
                Err(error) => warn!("water: roads: {error}"),
            }
        }
        if let Some(raw) = json::<RawLakes>(map_dir, "lakes.json") {
            data.lakes = raw
                .lakes
                .into_iter()
                .filter(|l| l.polygon_px.len() >= 3)
                .map(|l| Lake {
                    id: l.id,
                    name: l.name,
                    level_m: l.level_m,
                    center: Vec2::from(l.center_px),
                    polygon: to_vec2(&l.polygon_px),
                })
                .collect();
        }
        if let Some(raw) = json::<RawCrossings>(map_dir, "crossings_px.json") {
            let zones = data.zones.clone();
            data.crossings = raw
                .crossings
                .into_iter()
                .filter(|c| c.snapped && !c.in_custom_zone)
                .filter(|c| {
                    let p = Vec2::from(c.px);
                    !zones.iter().any(|z| p.distance(z.px) < z.radius_px + 0.5)
                })
                .map(|c| Crossing {
                    id: c.id,
                    name: c.name,
                    structure: c.structure,
                    px: Vec2::from(c.px),
                    dir: Vec2::from(c.dir).normalize_or(Vec2::X),
                    width: c.width,
                })
                .collect();
        }
        data.display = json::<RiverDisplay>(map_dir, "river_display.json").unwrap_or_default();
        data.river_names = json::<RawNames>(map_dir, "river_names.json")
            .map(|n| n.names)
            .unwrap_or_default();
        data.basins = json::<SeaBasins>(map_dir, "sea_basins.json").unwrap_or_default();
        data.basins.basins.truncate(MAX_BASINS);
        if let Some(value) = json::<serde_json::Value>(map_dir, "freshwater_px.json")
            && let Some(object) = value.as_object()
        {
            for (key, entry) in object {
                if let Some(list) = entry.as_array() {
                    data.freshwater_counts.insert(key.clone(), list.len());
                }
            }
        }
        data
    }

    /// Display name of a river source name (`river_names.json`; empty value hides the label).
    pub fn display_name(&self, source: &str) -> String {
        self.river_names
            .get(source)
            .map_or(source, String::as_str)
            .trim()
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_directory_gives_empty_data() {
        let data = WaterData::load(Path::new("/nonexistent/map"));
        assert!(data.lakes.is_empty() && data.rivers.is_empty() && data.roads.is_empty());
        assert_eq!(data.map_size, Vec2::new(7080.0, 6104.0));
    }

    #[test]
    fn display_name_uses_table_and_empty_hides() {
        let mut data = WaterData::default();
        data.river_names
            .insert("Byarezina".into(), "Bérézina".into());
        data.river_names.insert("Hidden".into(), String::new());
        assert_eq!(data.display_name("Byarezina"), "Bérézina");
        assert_eq!(data.display_name("Hidden"), "");
        assert_eq!(data.display_name("Loire"), "Loire");
    }

    #[test]
    fn real_data_loads_when_present() {
        let Ok(root) = crate::paths::find_root() else {
            return;
        };
        let data = WaterData::load(&root.join("data/map"));
        assert!(data.map_size.x > 1000.0);
        assert!(!data.lakes.is_empty());
        assert!(!data.rivers.is_empty());
        assert!(!data.roads.is_empty());
        assert!(data.basins.basins.len() <= MAX_BASINS);
    }
}
