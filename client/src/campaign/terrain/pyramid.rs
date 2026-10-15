//! Streamed relief pyramid index: port of `relief_pyramid.gd`.
//!
//! `data/map/pyramid/relief_pyramid.json` lists the 512 px 16-bit tiles of E1+ (run-length per
//! row); E0 is `data/map/height/` (`map.json` `height_tiles`). Level k: a tile covers
//! `256 / 2^k` map units; the tile (k, col, row) of the cache is the world tile
//! `(col + dx*2^k, row + dy*2^k)` (`root_origin_tiles`). Everything here is in world tile
//! coordinates, except [`ReliefPyramid::tile_path`] which goes back to cache file names.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde_json::Value;

pub const TILE_PX: usize = 512;
pub const ROOT_TILE_UNITS: f64 = 256.0;
pub const MAX_LEVEL: i32 = 7;

#[derive(Debug, Clone, Default)]
pub struct ReliefPyramid {
    pub tiles_dir: PathBuf,
    pattern: String,
    e0_dir: PathBuf,
    e0_pattern: String,
    /// E0 tiles along x and y (`map.json` `size_px` / 256).
    pub root_cols: i64,
    pub root_rows: i64,
    origin: (i64, i64),
    pub h_min_m: f32,
    pub h_range_m: f32,
    pub max_level: i32,
    tiles: Vec<HashSet<i64>>,
    /// Tiles listed by the manifest but absent on disk.
    pub missing_tiles: usize,
    /// World size in map units (`map.json` `size_px`).
    pub world_px: (u32, u32),
}

pub fn tile_units(level: i32) -> f64 {
    ROOT_TILE_UNITS / (1i64 << level) as f64
}

pub fn key_of(level: i64, col: i64, row: i64) -> i64 {
    (level << 24) | (row << 12) | col
}

pub fn level_of_key(key: i64) -> i32 {
    (key >> 24) as i32
}

pub fn col_of_key(key: i64) -> i64 {
    key & 0xfff
}

pub fn row_of_key(key: i64) -> i64 {
    (key >> 12) & 0xfff
}

impl ReliefPyramid {
    /// Loads `map.json` (world size, E0 tiles) and `pyramid/relief_pyramid.json` under `map_dir`.
    pub fn load(map_dir: &Path) -> Result<ReliefPyramid, String> {
        let read = |path: &Path| -> Result<Value, String> {
            let text =
                std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
        };
        let meta = read(&map_dir.join("map.json"))?;
        let mut pyramid = ReliefPyramid {
            e0_pattern: "h_{col}_{row}.png".into(),
            pattern: "E{level}/{col}_{row}.png".into(),
            h_min_m: -200.0,
            h_range_m: 5000.0,
            tiles: (0..=MAX_LEVEL).map(|_| HashSet::new()).collect(),
            ..Default::default()
        };
        let size = meta["size_px"]
            .as_array()
            .ok_or("map.json: size_px missing")?;
        let (w, h) = (
            size[0].as_f64().unwrap_or(0.0),
            size[1].as_f64().unwrap_or(0.0),
        );
        pyramid.world_px = (w as u32, h as u32);
        pyramid.root_cols = (w / ROOT_TILE_UNITS).ceil() as i64;
        pyramid.root_rows = (h / ROOT_TILE_UNITS).ceil() as i64;
        if let Some(tiles) = meta.get("height_tiles") {
            let dir = map_dir.join(tiles["dir"].as_str().unwrap_or("height"));
            pyramid.e0_dir = dir.clone();
            if let Some(p) = tiles["pattern"].as_str() {
                pyramid.e0_pattern = p.to_owned();
            }
            let present = list_files(&dir);
            for row in 0..pyramid.root_rows {
                for col in 0..pyramid.root_cols {
                    if present.contains(&pyramid.e0_file_name(col, row)) {
                        pyramid.tiles[0].insert(row * pyramid.root_cols + col);
                    }
                }
            }
        }
        let manifest_path = map_dir.join("pyramid/relief_pyramid.json");
        let manifest = read(&manifest_path)?;
        if manifest["tile_px"].as_u64().unwrap_or(TILE_PX as u64) != TILE_PX as u64
            || manifest["root_tile_units"]
                .as_f64()
                .unwrap_or(ROOT_TILE_UNITS)
                != ROOT_TILE_UNITS
        {
            return Err("unsupported tile geometry in relief_pyramid.json".into());
        }
        if let Some(o) = manifest["root_origin_tiles"].as_array()
            && o.len() == 2
        {
            pyramid.origin = (o[0].as_i64().unwrap_or(0), o[1].as_i64().unwrap_or(0));
        }
        pyramid.h_min_m = manifest["height_min_m"].as_f64().unwrap_or(-200.0) as f32;
        pyramid.h_range_m = manifest["height_range_m"].as_f64().unwrap_or(5000.0) as f32;
        if let Some(p) = manifest["pattern"].as_str() {
            pyramid.pattern = p.to_owned();
        }
        let base = manifest_path.parent().unwrap_or(map_dir);
        let sub = manifest["dir"].as_str().unwrap_or("pyramid");
        pyramid.tiles_dir = if sub.is_empty() || sub == "." {
            base.to_path_buf()
        } else {
            base.join(sub)
        };
        let empty = Vec::new();
        for entry in manifest["levels"].as_array().unwrap_or(&empty) {
            let level = entry["level"].as_i64().unwrap_or(0) as i32;
            if !(1..=MAX_LEVEL).contains(&level) {
                continue;
            }
            let across = pyramid.root_cols << level;
            let rows = pyramid.root_rows << level;
            let (dx, dy) = (pyramid.origin.0 << level, pyramid.origin.1 << level);
            let mut set = HashSet::new();
            for row_entry in entry["tiles_rle"].as_array().unwrap_or(&empty) {
                let row = row_entry["row"].as_i64().unwrap_or(-1) + dy;
                if row < 0 || row >= rows {
                    continue;
                }
                for run in row_entry["runs"].as_array().unwrap_or(&empty) {
                    let start = run[0].as_i64().unwrap_or(0) + dx;
                    let length = run[1].as_i64().unwrap_or(0);
                    for col in start.max(0)..(start + length).min(across) {
                        set.insert(row * across + col);
                    }
                }
            }
            if set.is_empty() {
                continue;
            }
            // A manifest ahead of the cache keeps only the tiles present on disk.
            let level_dir = {
                let relative = pyramid.pattern.replace("{level}", &level.to_string());
                pyramid
                    .tiles_dir
                    .join(Path::new(&relative).parent().unwrap_or(Path::new("")))
            };
            let present = list_files(&level_dir);
            let before = set.len();
            set.retain(|&index| {
                let (col, row) = (index % across, index / across);
                present.contains(&pyramid.cache_file_name(level, col, row))
            });
            pyramid.missing_tiles += before - set.len();
            if !set.is_empty() {
                pyramid.max_level = pyramid.max_level.max(level);
                pyramid.tiles[level as usize] = set;
            }
        }
        if pyramid.max_level == 0 {
            return Err("no tile above E0 in the relief pyramid".into());
        }
        Ok(pyramid)
    }

    pub fn cols(&self, level: i32) -> i64 {
        self.root_cols << level
    }

    pub fn rows(&self, level: i32) -> i64 {
        self.root_rows << level
    }

    pub fn tile_count(&self) -> usize {
        self.tiles.iter().map(HashSet::len).sum()
    }

    pub fn level_tile_count(&self, level: i32) -> usize {
        self.tiles.get(level as usize).map_or(0, HashSet::len)
    }

    pub fn has_tile(&self, level: i32, col: i64, row: i64) -> bool {
        if !(0..=MAX_LEVEL).contains(&level) {
            return false;
        }
        let across = self.cols(level);
        if col < 0 || row < 0 || col >= across || row >= self.rows(level) {
            return false;
        }
        self.tiles
            .get(level as usize)
            .is_some_and(|set| set.contains(&(row * across + col)))
    }

    /// Tile of level `level` containing the map point (x, z) (may be out of bounds).
    pub fn tile_at(level: i32, x: f64, z: f64) -> (i64, i64) {
        let t = tile_units(level);
        ((x / t).floor() as i64, (z / t).floor() as i64)
    }

    /// North-west corner (x, z) of a tile.
    pub fn tile_origin(level: i32, col: i64, row: i64) -> (f64, f64) {
        let t = tile_units(level);
        (col as f64 * t, row as f64 * t)
    }

    /// Finest level present at the map point (0 = E0, -1 outside the tiles).
    pub fn finest_level_at(&self, x: f64, z: f64) -> i32 {
        for level in (0..=self.max_level).rev() {
            let (col, row) = Self::tile_at(level, x, z);
            if self.has_tile(level, col, row) {
                return level;
            }
        }
        -1
    }

    /// Indices `row * cols(level) + col` of the tiles of a level (for `relief_lod::Pyramid`).
    pub fn tile_indices(&self, level: i32) -> Vec<i64> {
        self.tiles
            .get(level as usize)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }

    /// The pyramid in the form of the node selector.
    pub fn to_lod(&self) -> relief_lod::Pyramid {
        let tiles = (0..=self.max_level).map(|l| self.tile_indices(l)).collect();
        relief_lod::Pyramid::new(self.max_level, tiles, self.root_cols, self.root_rows)
    }

    fn e0_file_name(&self, col: i64, row: i64) -> String {
        self.e0_pattern
            .replace("{col}", &col.to_string())
            .replace("{row}", &row.to_string())
    }

    fn cache_file_name(&self, level: i32, col: i64, row: i64) -> String {
        let relative = self.pattern.replace("{level}", &level.to_string());
        let file = Path::new(&relative)
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        file.replace("{col}", &col.to_string())
            .replace("{row}", &row.to_string())
    }

    /// File of a world tile: E0 in `height/`, E1+ in the cache (origin offset removed).
    pub fn tile_path(&self, level: i32, col: i64, row: i64) -> PathBuf {
        if level == 0 {
            return self.e0_dir.join(self.e0_file_name(col, row));
        }
        let (cx, cy) = (
            col - (self.origin.0 << level),
            row - (self.origin.1 << level),
        );
        self.tiles_dir.join(
            self.pattern
                .replace("{level}", &level.to_string())
                .replace("{col}", &cx.to_string())
                .replace("{row}", &cy.to_string()),
        )
    }

    /// Tests: an index with no tiles over a `cols` x `rows` world.
    #[cfg(test)]
    pub fn test_new(cols: i64, rows: i64, max_level: i32) -> ReliefPyramid {
        ReliefPyramid {
            root_cols: cols,
            root_rows: rows,
            max_level,
            h_min_m: -200.0,
            h_range_m: 5000.0,
            tiles: (0..=MAX_LEVEL).map(|_| HashSet::new()).collect(),
            ..Default::default()
        }
    }

    /// Tests: sets the tile indices of some levels.
    #[cfg(test)]
    pub fn test_set_tiles(&mut self, levels: Vec<(i32, Vec<i64>)>) {
        self.tiles = (0..=MAX_LEVEL).map(|_| HashSet::new()).collect();
        for (level, indices) in levels {
            self.tiles[level as usize] = indices.into_iter().collect();
        }
    }

    pub fn tile_path_of_key(&self, key: i64) -> PathBuf {
        self.tile_path(level_of_key(key), col_of_key(key), row_of_key(key))
    }
}

fn list_files(dir: &Path) -> HashSet<String> {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_roundtrip_and_tile_geometry() {
        let key = key_of(3, 17, 9);
        assert_eq!(
            (level_of_key(key), col_of_key(key), row_of_key(key)),
            (3, 17, 9)
        );
        assert_eq!(tile_units(0), 256.0);
        assert_eq!(tile_units(3), 32.0);
        assert_eq!(ReliefPyramid::tile_at(1, 130.0, 255.0), (1, 1));
        assert_eq!(ReliefPyramid::tile_origin(2, 3, 1), (192.0, 64.0));
    }

    #[test]
    fn real_map_pyramid_loads_when_present() {
        let Ok(root) = crate::paths::find_root() else {
            return;
        };
        let map_dir = root.join("data/map");
        if !map_dir.join("pyramid/relief_pyramid.json").exists() {
            return;
        }
        let pyramid = ReliefPyramid::load(&map_dir).expect("pyramid");
        assert_eq!((pyramid.root_cols, pyramid.root_rows), (28, 24));
        assert_eq!(pyramid.max_level, 3);
        assert!(pyramid.level_tile_count(0) > 600);
        assert!(pyramid.level_tile_count(1) > 0);
        assert!(pyramid.finest_level_at(3500.0, 3000.0) >= 0);
        let lod = pyramid.to_lod();
        assert_eq!(lod.root_tiles(), (28, 24));
        // The files named by the index exist.
        let (c, r) = (14, 12);
        if pyramid.has_tile(0, c, r) {
            assert!(pyramid.tile_path(0, c, r).exists());
        }
    }
}
