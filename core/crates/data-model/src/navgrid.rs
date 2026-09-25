//! Navigation grid and province raster of the campaign map (lot M2, spec
//! `docs/design/2026-09-24-mouvement-libre.md` § 2 and § 3.1).
//!
//! - `data/map/navgrid.png` (lot M1): one 8-bit cost per cell, 10 = plain,
//!   255 = impassable; each cell covers `scale` × `scale` map pixels. When
//!   the file is absent, a uniform fallback grid is built from
//!   `land_mask.png` (cost of a plain on land, 255 at sea, subsampled by
//!   `scale`).
//! - `data/map/province_ids.png`: the province under every map pixel
//!   (`R | G << 8` = the `index` property of `provinces.geojson`).
//!
//! Both rasters are decoded lazily, once per map directory and per process
//! (a shared cache), and shared between clones of [`GameData`].

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use crate::ids::{ProvinceId, SettlementId};
use crate::load::GameData;

/// Cost of an impassable cell.
pub const IMPASSABLE: u8 = 255;
/// Cost of a plain cell: the unit of movement points on the grid.
pub const PLAIN_COST: u8 = 10;
/// Default grid side (cells) and map pixels per cell (2048² over 4096²).
pub const DEFAULT_GRID_SIZE: u32 = 2048;
pub const DEFAULT_GRID_SCALE: u32 = 2;
/// Default kilometres per cell (719 m per map pixel × 2).
pub const DEFAULT_CELL_KM: f64 = 1.438;
/// Cells searched around a settlement stranded at sea (fallback grid) for
/// the land it must be linked to.
const STRANDED_SEARCH_CELLS: i64 = 12;

/// A grid of movement costs over the campaign map.
#[derive(Clone, PartialEq)]
pub struct NavGrid {
    pub width: u32,
    pub height: u32,
    /// Map pixels per cell side.
    pub scale: u32,
    /// Kilometres per cell side.
    pub cell_km: f64,
    /// Row-major costs, `IMPASSABLE` for water and cliffs.
    pub costs: Vec<u8>,
    /// Smallest passable cost (A* heuristic); `PLAIN_COST` if none.
    pub min_cost: u8,
    /// `true` for the uniform grid built without `navgrid.png`.
    pub fallback: bool,
    /// Connected component of every cell (0 = impassable), computed on
    /// first use (8-connectivity, like the steps of the path finding).
    components: OnceLock<Vec<u32>>,
}

impl fmt::Debug for NavGrid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NavGrid")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("scale", &self.scale)
            .field("cell_km", &self.cell_km)
            .field("min_cost", &self.min_cost)
            .field("fallback", &self.fallback)
            .finish_non_exhaustive()
    }
}

impl NavGrid {
    /// A grid from row-major `costs` (`width × height`).
    pub fn from_costs(width: u32, height: u32, scale: u32, cell_km: f64, costs: Vec<u8>) -> Self {
        assert_eq!(costs.len(), (width as usize) * (height as usize));
        let mut grid = NavGrid {
            width,
            height,
            scale: scale.max(1),
            cell_km,
            costs,
            min_cost: PLAIN_COST,
            fallback: false,
            components: OnceLock::new(),
        };
        grid.refresh_min_cost();
        grid
    }

    /// A grid where every cell costs `cost`.
    pub fn uniform(width: u32, height: u32, scale: u32, cell_km: f64, cost: u8) -> Self {
        NavGrid::from_costs(
            width,
            height,
            scale,
            cell_km,
            vec![cost; (width as usize) * (height as usize)],
        )
    }

    /// Connected component of cell `(x, y)` (0 when impassable or outside).
    pub fn component(&self, x: i64, y: i64) -> u32 {
        if !self.contains(x, y) {
            return 0;
        }
        let labels = self.components.get_or_init(|| self.label_components());
        labels[self.index(x as u32, y as u32)]
    }

    fn label_components(&self) -> Vec<u32> {
        let (w, h) = (self.width as usize, self.height as usize);
        let mut labels = vec![0u32; w * h];
        let mut next = 0u32;
        let mut stack = Vec::new();
        for seed in 0..w * h {
            if labels[seed] != 0 || self.costs[seed] == IMPASSABLE {
                continue;
            }
            next += 1;
            labels[seed] = next;
            stack.push(seed);
            while let Some(i) = stack.pop() {
                let (x, y) = (i % w, i / w);
                let mut visit = |j: usize| {
                    if labels[j] == 0 && self.costs[j] != IMPASSABLE {
                        labels[j] = next;
                        stack.push(j);
                    }
                };
                for dy in -1i64..=1 {
                    for dx in -1i64..=1 {
                        let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                        if (dx, dy) != (0, 0)
                            && nx >= 0
                            && ny >= 0
                            && (nx as usize) < w
                            && (ny as usize) < h
                        {
                            visit(ny as usize * w + nx as usize);
                        }
                    }
                }
            }
        }
        labels
    }

    /// Recomputes [`NavGrid::min_cost`] after edits (and forgets the
    /// connected components).
    pub fn refresh_min_cost(&mut self) {
        self.components = OnceLock::new();
        self.min_cost = self
            .costs
            .iter()
            .copied()
            .filter(|c| *c != IMPASSABLE && *c > 0)
            .min()
            .unwrap_or(PLAIN_COST);
    }

    #[inline]
    pub fn contains(&self, x: i64, y: i64) -> bool {
        x >= 0
            && y >= 0
            && (x as u64) < u64::from(self.width)
            && (y as u64) < u64::from(self.height)
    }

    #[inline]
    pub fn index(&self, x: u32, y: u32) -> usize {
        (y as usize) * (self.width as usize) + x as usize
    }

    /// Cost of entering cell `(x, y)`; `IMPASSABLE` outside the grid.
    #[inline]
    pub fn cost(&self, x: i64, y: i64) -> u8 {
        if self.contains(x, y) {
            self.costs[self.index(x as u32, y as u32)].max(1)
        } else {
            IMPASSABLE
        }
    }

    #[inline]
    pub fn passable(&self, x: i64, y: i64) -> bool {
        self.cost(x, y) != IMPASSABLE
    }

    /// Sets the cost of a cell (ignored outside the grid).
    pub fn set(&mut self, x: i64, y: i64, cost: u8) {
        if self.contains(x, y) {
            let index = self.index(x as u32, y as u32);
            self.costs[index] = cost;
        }
    }

    /// Cell containing map pixel `(px, py)`, clamped to the grid.
    pub fn cell_of(&self, px: f32, py: f32) -> (u32, u32) {
        let scale = self.scale as f32;
        let clamp = |v: f32, max: u32| (v / scale).floor().clamp(0.0, (max - 1) as f32) as u32;
        (clamp(px, self.width), clamp(py, self.height))
    }

    /// Map-pixel centre of cell `(x, y)`.
    pub fn cell_center(&self, x: u32, y: u32) -> [f32; 2] {
        let scale = self.scale as f32;
        [(x as f32 + 0.5) * scale, (y as f32 + 0.5) * scale]
    }

    /// Map pixels per kilometre.
    pub fn px_per_km(&self) -> f64 {
        f64::from(self.scale) / self.cell_km.max(1e-6)
    }
}

/// Province index of every map pixel (`province_ids.png`).
#[derive(Clone, PartialEq)]
pub struct ProvinceRaster {
    pub width: u32,
    pub height: u32,
    /// Row-major province indices (0 = sea or none).
    pub indices: Vec<u16>,
    /// Province id of each index.
    pub ids: Vec<Option<ProvinceId>>,
}

impl fmt::Debug for ProvinceRaster {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProvinceRaster")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish_non_exhaustive()
    }
}

impl ProvinceRaster {
    /// Province under map pixel `(px, py)`, `None` at sea or off the map.
    pub fn province_at(&self, px: f32, py: f32) -> Option<&ProvinceId> {
        if !(px >= 0.0 && py >= 0.0) {
            return None;
        }
        let (x, y) = (px as u32, py as u32);
        if x >= self.width || y >= self.height {
            return None;
        }
        let index = self.indices[(y as usize) * (self.width as usize) + x as usize];
        self.ids.get(usize::from(index)).and_then(Option::as_ref)
    }
}

/// The decoded rasters of a map directory.
#[derive(Debug, Clone, PartialEq)]
pub struct MapRasters {
    pub navgrid: NavGrid,
    pub provinces: Option<ProvinceRaster>,
}

/// Lazily loaded [`MapRasters`], shared by the clones of a [`GameData`].
#[derive(Clone, Default)]
pub struct RasterHandle {
    source: Option<RasterSource>,
    cell: Arc<OnceLock<Arc<MapRasters>>>,
}

#[derive(Clone)]
struct RasterSource {
    map_dir: PathBuf,
    navgrid_file: Option<String>,
    grid_size: u32,
    scale: u32,
    cell_km: f64,
    province_ids: Vec<Option<ProvinceId>>,
    settlements: Vec<[f32; 2]>,
}

impl fmt::Debug for RasterHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RasterHandle")
            .field("map_dir", &self.source.as_ref().map(|s| &s.map_dir))
            .field("loaded", &self.cell.get().is_some())
            .finish()
    }
}

impl RasterHandle {
    /// A handle already holding `rasters` (tests, tools).
    pub fn with(rasters: MapRasters) -> Self {
        let cell = OnceLock::new();
        let _ = cell.set(Arc::new(rasters));
        RasterHandle {
            source: None,
            cell: Arc::new(cell),
        }
    }

    fn get(&self) -> &MapRasters {
        self.cell.get_or_init(|| match &self.source {
            Some(source) => cached_load(source),
            None => Arc::new(MapRasters {
                navgrid: NavGrid::uniform(
                    DEFAULT_GRID_SIZE,
                    DEFAULT_GRID_SIZE,
                    DEFAULT_GRID_SCALE,
                    DEFAULT_CELL_KM,
                    PLAIN_COST,
                ),
                provinces: None,
            }),
        })
    }
}

/// Process-wide cache: every `GameData::load` of the same `data/` shares
/// one decoded copy of the rasters.
fn cached_load(source: &RasterSource) -> Arc<MapRasters> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Arc<MapRasters>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = cache.lock().unwrap_or_else(|poison| poison.into_inner());
    if let Some(rasters) = guard.get(&source.map_dir) {
        return Arc::clone(rasters);
    }
    let rasters = Arc::new(load_rasters(source));
    guard.insert(source.map_dir.clone(), Arc::clone(&rasters));
    rasters
}

fn load_rasters(source: &RasterSource) -> MapRasters {
    let grid = source
        .navgrid_file
        .as_ref()
        .map(|file| source.map_dir.join(file))
        .filter(|path| path.is_file())
        .and_then(|path| decode_gray8(&path).ok())
        .map(|(costs, width, height)| {
            NavGrid::from_costs(width, height, source.scale, source.cell_km, costs)
        });
    let mut grid = match grid {
        Some(grid) => grid,
        None => fallback_grid(source),
    };
    open_settlement_cells(&mut grid, &source.settlements);
    grid.refresh_min_cost();
    let provinces = decode_rgb8(&source.map_dir.join("province_ids.png"))
        .ok()
        .map(|(rgb, width, height)| ProvinceRaster {
            width,
            height,
            indices: rgb
                .as_chunks::<3>()
                .0
                .iter()
                .map(|p| u16::from(p[0]) | (u16::from(p[1]) << 8))
                .collect(),
            ids: source.province_ids.clone(),
        });
    MapRasters {
        navgrid: grid,
        provinces,
    }
}

/// Uniform grid from `land_mask.png` (plain on land, impassable at sea),
/// or an all-plain grid when the mask is missing too.
fn fallback_grid(source: &RasterSource) -> NavGrid {
    let size = source.grid_size;
    let scale = source.scale.max(1);
    let mut grid = match decode_gray8(&source.map_dir.join("land_mask.png")) {
        Ok((mask, width, height)) => {
            let (cells_w, cells_h) = (width / scale, height / scale);
            let mut costs = vec![IMPASSABLE; (cells_w as usize) * (cells_h as usize)];
            let threshold = (scale * scale).div_ceil(2);
            for cy in 0..cells_h {
                for cx in 0..cells_w {
                    let mut land = 0;
                    for dy in 0..scale {
                        let row = ((cy * scale + dy) as usize) * width as usize;
                        for dx in 0..scale {
                            if mask[row + (cx * scale + dx) as usize] > 127 {
                                land += 1;
                            }
                        }
                    }
                    if land >= threshold {
                        costs[(cy as usize) * cells_w as usize + cx as usize] = PLAIN_COST;
                    }
                }
            }
            NavGrid::from_costs(cells_w, cells_h, scale, source.cell_km, costs)
        }
        Err(_) => NavGrid::uniform(size, size, scale, source.cell_km, PLAIN_COST),
    };
    grid.fallback = true;
    grid
}

/// Every settlement cell is passable (spec § 2); a settlement stranded at
/// sea is linked to the nearest land by a straight causeway of plain cells.
pub fn open_settlement_cells(grid: &mut NavGrid, settlements: &[[f32; 2]]) {
    for point in settlements {
        let (x, y) = grid.cell_of(point[0], point[1]);
        let (x, y) = (i64::from(x), i64::from(y));
        if !grid.passable(x, y) {
            grid.set(x, y, PLAIN_COST);
        }
        let linked = (-1..=1)
            .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
            .any(|(dx, dy)| (dx, dy) != (0, 0) && grid.passable(x + dx, y + dy));
        if linked {
            continue;
        }
        let mut best: Option<(i64, i64, i64)> = None;
        for dy in -STRANDED_SEARCH_CELLS..=STRANDED_SEARCH_CELLS {
            for dx in -STRANDED_SEARCH_CELLS..=STRANDED_SEARCH_CELLS {
                if (dx, dy) == (0, 0) || !grid.passable(x + dx, y + dy) {
                    continue;
                }
                let d = dx * dx + dy * dy;
                if best.is_none_or(|b| d < b.0) {
                    best = Some((d, dx, dy));
                }
            }
        }
        if let Some((_, dx, dy)) = best {
            let steps = dx.abs().max(dy.abs());
            for step in 1..steps {
                let cx = x + (dx * step + steps / 2 * dx.signum()) / steps;
                let cy = y + (dy * step + steps / 2 * dy.signum()) / steps;
                if !grid.passable(cx, cy) {
                    grid.set(cx, cy, PLAIN_COST);
                }
            }
        }
    }
}

type Decoded = (Vec<u8>, u32, u32);

fn decode(path: &Path, color: png::ColorType) -> Result<Decoded, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mut decoder = png::Decoder::new(BufReader::new(file));
    decoder.set_transformations(png::Transformations::IDENTITY);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let info = reader.info();
    if info.color_type != color || info.bit_depth != png::BitDepth::Eight {
        return Err(format!(
            "{}: expected {color:?}/8, got {:?}/{:?}",
            path.display(),
            info.color_type,
            info.bit_depth
        ));
    }
    let (width, height) = (info.width, info.height);
    let mut bytes = vec![0u8; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut bytes).map_err(|e| e.to_string())?;
    bytes.truncate(frame.buffer_size());
    Ok((bytes, width, height))
}

fn decode_gray8(path: &Path) -> Result<Decoded, String> {
    decode(path, png::ColorType::Grayscale)
}

fn decode_rgb8(path: &Path) -> Result<Decoded, String> {
    decode(path, png::ColorType::Rgb)
}

impl GameData {
    /// Prepares the lazy loading of the rasters of `map_dir` (called by
    /// [`GameData::load`] once the map metadata and settlements are known).
    pub(crate) fn prepare_rasters(&mut self, map_dir: &Path) {
        let navgrid = self
            .map
            .as_ref()
            .and_then(|m| m.extra.get("navgrid"))
            .cloned()
            .unwrap_or_default();
        let navgrid_file = navgrid
            .get("file")
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .or_else(|| Some("navgrid.png".to_owned()));
        let scale = navgrid
            .get("scale")
            .and_then(|v| v.as_u64())
            .map_or(DEFAULT_GRID_SCALE, |v| v.max(1) as u32);
        let grid_size = navgrid
            .get("size_px")
            .and_then(|v| v.as_u64())
            .map_or(DEFAULT_GRID_SIZE, |v| v.max(1) as u32);
        let cell_km = self.map.as_ref().map_or(DEFAULT_CELL_KM, |m| {
            m.meters_per_px * f64::from(scale) / 1000.0
        });
        let mut province_ids: Vec<Option<ProvinceId>> = Vec::new();
        for (id, geometry) in &self.province_geometry {
            if let Some(index) = geometry.extra.get("index").and_then(|v| v.as_u64()) {
                let index = index as usize;
                if province_ids.len() <= index {
                    province_ids.resize(index + 1, None);
                }
                province_ids[index] = Some(id.clone());
            }
        }
        let settlements = self
            .settlements
            .keys()
            .filter_map(|id| self.settlement_point(id))
            .collect();
        self.rasters = RasterHandle {
            source: Some(RasterSource {
                map_dir: map_dir.to_path_buf(),
                navgrid_file,
                grid_size,
                scale,
                cell_km,
                province_ids,
                settlements,
            }),
            cell: Arc::new(OnceLock::new()),
        };
    }

    /// The navigation grid (decoded on first use).
    pub fn navgrid(&self) -> &NavGrid {
        &self.rasters.get().navgrid
    }

    /// The decoded `province_ids.png`, when present (lot M5a: vision
    /// counts seen land per province).
    pub fn province_raster(&self) -> Option<&ProvinceRaster> {
        self.rasters.get().provinces.as_ref()
    }

    /// Province under map pixel `(px, py)` according to `province_ids.png`
    /// (`None` at sea, off the map or without the raster).
    pub fn province_at_point(&self, px: f32, py: f32) -> Option<&ProvinceId> {
        self.rasters
            .get()
            .provinces
            .as_ref()
            .and_then(|raster| raster.province_at(px, py))
    }

    /// Replaces the rasters (tests: small synthetic grids).
    pub fn set_map_rasters(&mut self, rasters: MapRasters) {
        self.rasters = RasterHandle::with(rasters);
    }

    /// Map-pixel position of a settlement: `settlements_px.json`, else its
    /// longitude/latitude projected linearly over the map extent.
    pub fn settlement_point(&self, id: &SettlementId) -> Option<[f32; 2]> {
        if let Some(point) = self.settlement_px.get(id) {
            return Some(*point);
        }
        let settlement = self.settlements.get(id)?;
        let (extent, size) = self
            .map
            .as_ref()
            .and_then(|m| {
                let extent = m.extra.get("extent_lonlat")?.as_array()?;
                let values: Vec<f64> = extent.iter().filter_map(|v| v.as_f64()).collect();
                (values.len() == 4)
                    .then(|| ([values[0], values[1], values[2], values[3]], m.size_px))
            })
            .unwrap_or(([-11.0, 35.0, 16.0, 60.0], [4096, 4096]));
        let [lon, lat] = settlement.lonlat;
        let x = (lon - extent[0]) / (extent[2] - extent[0]) * f64::from(size[0]);
        let y = (extent[3] - lat) / (extent[3] - extent[1]) * f64::from(size[1]);
        Some([x as f32, y as f32])
    }
}

/// Reads `settlements_px.json` (`{settlement id: [x, y]}`), ignoring
/// malformed entries.
pub(crate) fn read_settlement_px(path: &Path) -> BTreeMap<SettlementId, [f32; 2]> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    let Ok(raw) = serde_json::from_str::<BTreeMap<String, [f64; 2]>>(&text) else {
        return BTreeMap::new();
    };
    raw.into_iter()
        .filter_map(|(id, [x, y])| Some((SettlementId::new(&id).ok()?, [x as f32, y as f32])))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stranded_settlement_is_linked_to_land() {
        let mut grid = NavGrid::uniform(20, 20, 2, 1.438, IMPASSABLE);
        for x in 0..20 {
            grid.set(x, 0, PLAIN_COST);
        }
        open_settlement_cells(&mut grid, &[[21.0, 21.0]]);
        assert!(grid.passable(10, 10));
        let mut y = 10;
        while y > 0 {
            assert!(
                (-1..=1).any(|dx| grid.passable(10 + dx, y - 1)),
                "gap at row {}",
                y - 1
            );
            y -= 1;
        }
    }

    #[test]
    fn cell_of_clamps() {
        let grid = NavGrid::uniform(10, 10, 2, 1.438, PLAIN_COST);
        assert_eq!(grid.cell_of(-5.0, 3.0), (0, 1));
        assert_eq!(grid.cell_of(100.0, 19.9), (9, 9));
        assert_eq!(grid.cell_center(1, 2), [3.0, 5.0]);
    }
}
