//! Cover of the campaign map at the resolution of the navigation grid (lot
//! CV3-1, ambushes: spec `docs/design/2026-09-27-campagne-vivante.md` § 1.2).
//!
//! Two rasters of `data/map/`, both optional and decoded lazily once per
//! process (like [`crate::navgrid`]):
//!
//! - the forest cover named by `forest_cover.json` (`cover.file` /
//!   `cover.channel`: `splat.png`, channel B, forests about 1340, lot R1);
//! - `wetlands.png` (lot R1, RGB: marshes, ponds, wet meadows), the raster of
//!   the zones of `wetlands.json`.
//!
//! Both are sampled at the centre of every grid cell. Bocage has no raster:
//! it comes from the province terrain. Without rasters, the class falls back
//! on the terrain of the province (`Forest`, `Bocage`, `Marsh`).

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::entities::posture::CoverRules;
use crate::entities::province::Terrain;
use crate::load::GameData;

/// Cover class of a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverClass {
    Open,
    Forest,
    Bocage,
    Marsh,
}

impl CoverClass {
    pub fn key(self) -> &'static str {
        match self {
            CoverClass::Open => "open",
            CoverClass::Forest => "forest",
            CoverClass::Bocage => "bocage",
            CoverClass::Marsh => "marsh",
        }
    }

    /// French label ("forêt").
    pub fn label_fr(self) -> &'static str {
        match self {
            CoverClass::Open => "terrain découvert",
            CoverClass::Forest => "forêt",
            CoverClass::Bocage => "bocage",
            CoverClass::Marsh => "marais",
        }
    }

    pub fn is_covered(self) -> bool {
        self != CoverClass::Open
    }

    /// The class a province terrain gives on its own.
    pub fn of_terrain(terrain: Terrain) -> CoverClass {
        match terrain {
            Terrain::Forest => CoverClass::Forest,
            Terrain::Bocage => CoverClass::Bocage,
            Terrain::Marsh => CoverClass::Marsh,
            _ => CoverClass::Open,
        }
    }
}

/// Forest and wetland densities (0-255) of every cell of the grid.
#[derive(Clone, PartialEq)]
pub struct CoverMap {
    pub width: u32,
    pub height: u32,
    /// Map pixels per cell side (same as the navigation grid).
    pub scale: u32,
    /// Row-major forest cover, 255 = full forest.
    pub forest: Vec<u8>,
    /// Row-major wetland density, 255 = marsh.
    pub wetland: Vec<u8>,
}

impl fmt::Debug for CoverMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CoverMap")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("scale", &self.scale)
            .finish_non_exhaustive()
    }
}

impl CoverMap {
    /// An open map (no forest, no wetland) of `width × height` cells.
    pub fn open(width: u32, height: u32, scale: u32) -> Self {
        let cells = (width as usize) * (height as usize);
        CoverMap {
            width,
            height,
            scale: scale.max(1),
            forest: vec![0; cells],
            wetland: vec![0; cells],
        }
    }

    fn index_of(&self, point: [f32; 2]) -> Option<usize> {
        let scale = self.scale as f32;
        if !(point[0] >= 0.0 && point[1] >= 0.0) {
            return None;
        }
        let (x, y) = ((point[0] / scale) as u32, (point[1] / scale) as u32);
        (x < self.width && y < self.height)
            .then(|| (y as usize) * (self.width as usize) + x as usize)
    }

    /// Sets the densities of the cell holding map pixel `point` (tests).
    pub fn set(&mut self, point: [f32; 2], forest: u8, wetland: u8) {
        if let Some(index) = self.index_of(point) {
            self.forest[index] = forest;
            self.wetland[index] = wetland;
        }
    }

    /// `(forest, wetland)` densities (0-1) at map pixel `point`.
    pub fn densities(&self, point: [f32; 2]) -> (f64, f64) {
        self.index_of(point).map_or((0.0, 0.0), |i| {
            (
                f64::from(self.forest[i]) / 255.0,
                f64::from(self.wetland[i]) / 255.0,
            )
        })
    }
}

/// Lazily loaded [`CoverMap`], shared by the clones of a [`GameData`].
#[derive(Clone, Default)]
pub struct CoverHandle {
    source: Option<CoverSource>,
    cell: Arc<OnceLock<Option<Arc<CoverMap>>>>,
}

#[derive(Clone)]
struct CoverSource {
    map_dir: PathBuf,
    forest_file: String,
    forest_channel: usize,
}

impl fmt::Debug for CoverHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CoverHandle")
            .field("map_dir", &self.source.as_ref().map(|s| &s.map_dir))
            .field("loaded", &self.cell.get().is_some())
            .finish()
    }
}

impl CoverHandle {
    /// A handle holding `map` (tests).
    pub fn with(map: Option<CoverMap>) -> Self {
        let cell = OnceLock::new();
        let _ = cell.set(map.map(Arc::new));
        CoverHandle {
            source: None,
            cell: Arc::new(cell),
        }
    }
}

/// Process-wide cache per map directory.
fn cached_load(source: &CoverSource, grid: (u32, u32, u32)) -> Option<Arc<CoverMap>> {
    type Cache = Mutex<HashMap<PathBuf, Option<Arc<CoverMap>>>>;
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = cache.lock().unwrap_or_else(|poison| poison.into_inner());
    if let Some(map) = guard.get(&source.map_dir) {
        return map.clone();
    }
    let map = load_cover(source, grid).map(Arc::new);
    guard.insert(source.map_dir.clone(), map.clone());
    map
}

/// Decodes `path` (8-bit, any of gray / RGB / RGBA) as `(bytes, channels,
/// width, height)`.
fn decode_any(path: &Path) -> Option<(Vec<u8>, usize, u32, u32)> {
    for (color, channels) in [
        (png::ColorType::Rgba, 4),
        (png::ColorType::Rgb, 3),
        (png::ColorType::Grayscale, 1),
    ] {
        if let Ok((bytes, width, height)) = crate::navgrid::decode(path, color) {
            return Some((bytes, channels, width, height));
        }
    }
    None
}

/// Samples `channel` of a decoded raster at the centre of every grid cell,
/// taking the maximum over `channels` when `channel` is `None`.
fn sample(
    raster: &(Vec<u8>, usize, u32, u32),
    channel: Option<usize>,
    (width, height, scale): (u32, u32, u32),
) -> Vec<u8> {
    let (bytes, channels, rw, rh) = raster;
    let map_w = f64::from(width * scale);
    let map_h = f64::from(height * scale);
    let mut out = vec![0u8; (width as usize) * (height as usize)];
    for cy in 0..height {
        let py = ((f64::from(cy) + 0.5) * f64::from(scale) / map_h * f64::from(*rh)) as u32;
        let py = py.min(rh - 1) as usize;
        for cx in 0..width {
            let px = ((f64::from(cx) + 0.5) * f64::from(scale) / map_w * f64::from(*rw)) as u32;
            let px = px.min(rw - 1) as usize;
            let base = (py * (*rw as usize) + px) * channels;
            let value = match channel {
                Some(c) => bytes[base + c.min(channels - 1)],
                None => bytes[base..base + (*channels).min(3)]
                    .iter()
                    .copied()
                    .max()
                    .unwrap_or(0),
            };
            out[(cy as usize) * (width as usize) + cx as usize] = value;
        }
    }
    out
}

fn load_cover(source: &CoverSource, grid: (u32, u32, u32)) -> Option<CoverMap> {
    let forest = decode_any(&source.map_dir.join(&source.forest_file));
    let wetland = decode_any(&source.map_dir.join("wetlands.png"));
    if forest.is_none() && wetland.is_none() {
        return None;
    }
    let (width, height, scale) = grid;
    let mut map = CoverMap::open(width, height, scale);
    if let Some(raster) = &forest {
        map.forest = sample(raster, Some(source.forest_channel), grid);
    }
    if let Some(raster) = &wetland {
        map.wetland = sample(raster, None, grid);
    }
    Some(map)
}

fn channel_index(name: &str) -> usize {
    match name {
        "g" => 1,
        "b" => 2,
        "a" => 3,
        _ => 0,
    }
}

impl GameData {
    /// Prepares the lazy cover map of `map_dir` (called by [`GameData::load`]).
    pub(crate) fn prepare_cover(&mut self, map_dir: &Path) {
        let config: Option<serde_json::Value> =
            std::fs::read_to_string(map_dir.join("forest_cover.json"))
                .ok()
                .and_then(|text| serde_json::from_str(&text).ok());
        let cover = config.as_ref().and_then(|c| c.get("cover"));
        let forest_file = cover
            .and_then(|c| c.get("file"))
            .and_then(|v| v.as_str())
            .unwrap_or("splat.png")
            .to_owned();
        let forest_channel = channel_index(
            cover
                .and_then(|c| c.get("channel"))
                .and_then(|v| v.as_str())
                .unwrap_or("b"),
        );
        self.cover = CoverHandle {
            source: Some(CoverSource {
                map_dir: map_dir.to_path_buf(),
                forest_file,
                forest_channel,
            }),
            cell: Arc::new(OnceLock::new()),
        };
    }

    /// The cover map (decoded on first use); `None` without rasters.
    pub fn cover_map(&self) -> Option<&CoverMap> {
        self.cover
            .cell
            .get_or_init(|| {
                let source = self.cover.source.as_ref()?;
                let grid = self.navgrid();
                cached_load(source, (grid.width, grid.height, grid.scale))
            })
            .as_deref()
    }

    /// Replaces the cover map (tests; `None`: province terrain only).
    pub fn set_cover_map(&mut self, map: Option<CoverMap>) {
        self.cover = CoverHandle::with(map);
    }

    /// Cover class at map pixel `point`: forest or marsh from the rasters
    /// (thresholds of `rules`), else bocage from the province terrain; the
    /// province terrain alone without rasters.
    pub fn cover_class_at(&self, point: [f32; 2], rules: &CoverRules) -> CoverClass {
        let terrain = self
            .province_at_point(point[0], point[1])
            .and_then(|id| self.provinces.get(id))
            .map(|p| p.terrain);
        let Some(map) = self.cover_map() else {
            return terrain.map_or(CoverClass::Open, CoverClass::of_terrain);
        };
        let (forest, wetland) = map.densities(point);
        if forest >= rules.forest_threshold {
            CoverClass::Forest
        } else if wetland >= rules.wetland_threshold {
            CoverClass::Marsh
        } else if terrain == Some(Terrain::Bocage) {
            CoverClass::Bocage
        } else {
            CoverClass::Open
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn densities_follow_the_cells() {
        let mut map = CoverMap::open(4, 4, 2);
        map.set([3.0, 5.0], 255, 0);
        assert_eq!(map.densities([2.5, 4.5]), (1.0, 0.0));
        assert_eq!(map.densities([0.5, 0.5]), (0.0, 0.0));
        assert_eq!(map.densities([-1.0, 0.5]), (0.0, 0.0));
    }

    #[test]
    fn terrain_classes() {
        assert_eq!(CoverClass::of_terrain(Terrain::Forest), CoverClass::Forest);
        assert_eq!(CoverClass::of_terrain(Terrain::Plains), CoverClass::Open);
        assert!(!CoverClass::Open.is_covered());
    }
}
