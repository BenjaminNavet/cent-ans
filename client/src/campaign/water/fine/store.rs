//! Fine data read for the close rendering (`fine_geo_store.gd`, ADR 0036):
//! - index of the CAFV tiles of the rivers (`data/map/rivers_fine.json`) and of the draped roads
//!   (`fine_anchors.json` -> `roads`), binary tiles outside git under `<relief root>/pyramid/`;
//! - refined anchors of settlements, hamlets and crossings (`fine_anchors.json`).
//!
//! Tiles load on demand on the IO task pool ([`FineGeoStore::request`] / [`FineGeoStore::poll`]),
//! LRU cache per layer. No game rule.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bevy::math::Vec2;
use bevy::tasks::{IoTaskPool, Task, TaskPool, block_on};
use serde_json::Value;

use super::cafv::{CafvTile, LAYER_RIVERS, LAYER_ROADS, TILE_UNITS};

/// Root tile side of the relief pyramid (`ReliefPyramid.ROOT_TILE_UNITS`).
const ROOT_TILE_UNITS: f32 = 256.0;

/// Tile key (`row << 12 | col`, as Godot).
pub fn key_of(col: i32, row: i32) -> i32 {
    (row << 12) | col
}

pub fn col_of(key: i32) -> i32 {
    key & 0xFFF
}

pub fn row_of(key: i32) -> i32 {
    key >> 12
}

/// World rectangle (x0, z0, x1, z1) of a tile.
pub fn tile_rect(col: i32, row: i32) -> [f32; 4] {
    let (x, z) = (col as f32 * TILE_UNITS, row as f32 * TILE_UNITS);
    [x, z, x + TILE_UNITS, z + TILE_UNITS]
}

/// Distance from `p` to a rectangle (0 inside).
pub fn rect_distance(r: [f32; 4], p: Vec2) -> f32 {
    Vec2::new(p.x.clamp(r[0], r[2]), p.y.clamp(r[1], r[3])).distance(p)
}

/// Refined crossing anchor (`fine_anchors.json` -> `crossings`, order of `crossings_px.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct FineCrossing {
    pub id: String,
    pub px: Vec2,
    pub snapped: bool,
    pub dir: Vec2,
    pub width_m: f32,
    pub z_water: f32,
    pub z_deck: f32,
}

/// Refined settlement anchor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FineSettlement {
    pub px: Vec2,
    pub z: f32,
    pub moved_m: f32,
}

#[derive(Default)]
struct Layer {
    index: HashSet<i32>,
    dir: PathBuf,
    pattern: String,
    tiles: HashMap<i32, (Arc<CafvTile>, u64)>,
    broken: HashSet<i32>,
}

/// Index, anchors and the tile cache.
#[derive(Default)]
pub struct FineGeoStore {
    layers: [Layer; 2],
    jobs: HashMap<(u16, i32), Task<Option<CafvTile>>>,
    frame: u64,
    pub max_cached_tiles: usize,
    pub max_jobs: usize,
    /// Cache origin offset in E2 tiles (`root_origin_tiles` x 4, ADR 0115).
    pub origin_tiles: (i32, i32),
    pub settlements: HashMap<String, FineSettlement>,
    /// Hamlets in `hamlets.json` order: (x, y, z, moved m).
    pub hamlets: Vec<[f32; 4]>,
    pub crossings: Vec<FineCrossing>,
    /// Road widths (m) by class (`roads.width_m`; a number applies to all).
    pub road_width_m: f32,
    pub loaded_tiles: u64,
}

fn read_json(path: &Path) -> Option<Value> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

fn vec2(v: &Value) -> Option<Vec2> {
    let a = v.as_array()?;
    Some(Vec2::new(
        a.first()?.as_f64()? as f32,
        a.get(1)?.as_f64()? as f32,
    ))
}

fn layer_slot(layer: u16) -> usize {
    if layer == LAYER_ROADS { 1 } else { 0 }
}

impl FineGeoStore {
    /// Loads the indexes (from `map_dir`) and the anchors; tile files resolve under `tiles_root`
    /// (the relief root, usually `map_dir`). True when at least one layer has tiles on disk.
    pub fn load_from(map_dir: &Path, tiles_root: &Path) -> (FineGeoStore, bool) {
        let mut store = FineGeoStore {
            max_cached_tiles: 64,
            max_jobs: 3,
            road_width_m: 3.0,
            ..Default::default()
        };
        if let Some(pyramid) = read_json(&map_dir.join("relief_pyramid.json"))
            .or_else(|| read_json(&tiles_root.join("pyramid/relief_pyramid.json")))
            && let Some(o) = pyramid.get("root_origin_tiles").and_then(vec2)
        {
            let per_root = (ROOT_TILE_UNITS / TILE_UNITS) as i32;
            store.origin_tiles = (o.x as i32 * per_root, o.y as i32 * per_root);
        }
        if let Some(rivers) = read_json(&map_dir.join("rivers_fine.json")) {
            store.load_index(LAYER_RIVERS, &rivers, tiles_root);
        }
        if let Some(anchors) = read_json(&map_dir.join("fine_anchors.json")) {
            store.load_anchors(&anchors);
            if let Some(roads) = anchors.get("roads") {
                store.load_index(LAYER_ROADS, roads, tiles_root);
                if let Some(w) = roads.get("width_m").and_then(Value::as_f64) {
                    store.road_width_m = w as f32;
                }
            }
        }
        let ok = store.available(LAYER_RIVERS) || store.available(LAYER_ROADS);
        (store, ok)
    }

    fn load_index(&mut self, layer: u16, manifest: &Value, root: &Path) {
        let origin = self.origin_tiles;
        let l = &mut self.layers[layer_slot(layer)];
        l.dir = root.join(manifest.get("dir").and_then(Value::as_str).unwrap_or(""));
        l.pattern = manifest
            .get("pattern")
            .and_then(Value::as_str)
            .unwrap_or("E2/{col}_{row}.bin")
            .to_owned();
        l.index = manifest
            .get("tiles")
            .and_then(Value::as_array)
            .map(|tiles| {
                tiles
                    .iter()
                    .map(|t| {
                        let c = t.get("col").and_then(Value::as_i64).unwrap_or(0) as i32;
                        let r = t.get("row").and_then(Value::as_i64).unwrap_or(0) as i32;
                        key_of(c + origin.0, r + origin.1)
                    })
                    .collect()
            })
            .unwrap_or_default();
    }

    fn load_anchors(&mut self, anchors: &Value) {
        if let Some(map) = anchors.get("settlements").and_then(Value::as_object) {
            for (id, e) in map {
                let Some(px) = e.get("px").and_then(vec2) else {
                    continue;
                };
                let num = |k: &str| e.get(k).and_then(Value::as_f64).unwrap_or(0.0) as f32;
                self.settlements.insert(
                    id.clone(),
                    FineSettlement {
                        px,
                        z: num("z"),
                        moved_m: num("moved_m"),
                    },
                );
            }
        }
        if let Some(items) = anchors.pointer("/hamlets/items").and_then(Value::as_array) {
            for item in items {
                let Some(a) = item.as_array() else { continue };
                let n = |i: usize| a.get(i).and_then(Value::as_f64).unwrap_or(0.0) as f32;
                self.hamlets.push([n(0), n(1), n(2), n(3)]);
            }
        }
        if let Some(list) = anchors.get("crossings").and_then(Value::as_array) {
            for raw in list {
                let num = |k: &str| raw.get(k).and_then(Value::as_f64).map(|v| v as f32);
                let ground = num("z_ground").unwrap_or(0.0);
                self.crossings.push(FineCrossing {
                    id: raw
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_owned(),
                    px: raw.get("px").and_then(vec2).unwrap_or(Vec2::ZERO),
                    snapped: raw.get("snapped").and_then(Value::as_bool).unwrap_or(false),
                    dir: raw
                        .get("dir")
                        .and_then(vec2)
                        .unwrap_or(Vec2::X)
                        .normalize_or(Vec2::X),
                    width_m: num("width_m").unwrap_or(0.0),
                    z_water: num("z_water").unwrap_or(ground),
                    z_deck: num("z_deck").unwrap_or(ground),
                });
            }
        }
    }

    /// Whether the layer has an index and its tile folder exists (cache outside git).
    pub fn available(&self, layer: u16) -> bool {
        let l = &self.layers[layer_slot(layer)];
        !l.index.is_empty() && l.dir.is_dir()
    }

    pub fn tile_count(&self, layer: u16) -> usize {
        self.layers[layer_slot(layer)].index.len()
    }

    pub fn has_tile(&self, layer: u16, col: i32, row: i32) -> bool {
        self.layers[layer_slot(layer)]
            .index
            .contains(&key_of(col, row))
    }

    /// File of the world tile (col, row): cache frame (origin offset removed).
    pub fn tile_path(&self, layer: u16, col: i32, row: i32) -> PathBuf {
        let l = &self.layers[layer_slot(layer)];
        let c = col - self.origin_tiles.0;
        let r = row - self.origin_tiles.1;
        l.dir.join(
            l.pattern
                .replace("{col}", &c.to_string())
                .replace("{row}", &r.to_string()),
        )
    }

    /// Loaded tile (marks its use for the LRU).
    pub fn get(&mut self, layer: u16, col: i32, row: i32) -> Option<Arc<CafvTile>> {
        let frame = self.frame;
        let entry = self.layers[layer_slot(layer)]
            .tiles
            .get_mut(&key_of(col, row))?;
        entry.1 = frame;
        Some(entry.0.clone())
    }

    /// Loaded, or known unreadable (no new request).
    pub fn is_loaded(&self, layer: u16, col: i32, row: i32) -> bool {
        let l = &self.layers[layer_slot(layer)];
        let key = key_of(col, row);
        l.tiles.contains_key(&key) || l.broken.contains(&key)
    }

    /// Ready to mesh: absent from the index, or loaded.
    pub fn is_ready(&self, layer: u16, col: i32, row: i32) -> bool {
        !self.has_tile(layer, col, row) || self.is_loaded(layer, col, row)
    }

    /// Asks for a tile (read on the IO pool); no effect if loaded, requested or over budget.
    pub fn request(&mut self, layer: u16, col: i32, row: i32) {
        if !self.has_tile(layer, col, row) || self.is_loaded(layer, col, row) {
            return;
        }
        let jkey = (layer, key_of(col, row));
        if self.jobs.contains_key(&jkey) || self.jobs.len() >= self.max_jobs {
            return;
        }
        let path = self.tile_path(layer, col, row);
        let offset = self.origin_tiles;
        let pool = IoTaskPool::get_or_init(TaskPool::default);
        let task = pool.spawn(async move {
            std::fs::read(&path)
                .ok()
                .and_then(|b| CafvTile::parse(&b, offset))
        });
        self.jobs.insert(jkey, task);
    }

    pub fn pending(&self) -> usize {
        self.jobs.len()
    }

    /// Collects the finished loads (all of them with `block`); returns the number arrived.
    pub fn poll(&mut self, block: bool) -> usize {
        self.frame += 1;
        let done: Vec<(u16, i32)> = self
            .jobs
            .iter()
            .filter(|(_, task)| block || task.is_finished())
            .map(|(k, _)| *k)
            .collect();
        for jkey in &done {
            let task = self.jobs.remove(jkey).expect("job");
            // Finished (or `block`): `block_on` returns at once (or waits for the read).
            let tile = block_on(task);
            self.store(jkey.0, jkey.1, tile);
        }
        done.len()
    }

    /// Loads a tile now on the calling thread (tests, flush).
    pub fn load_sync(&mut self, layer: u16, col: i32, row: i32) -> Option<Arc<CafvTile>> {
        if !self.has_tile(layer, col, row) {
            return None;
        }
        if let Some(t) = self.get(layer, col, row) {
            return Some(t);
        }
        let key = key_of(col, row);
        let tile = match self.jobs.remove(&(layer, key)) {
            Some(task) => block_on(task),
            None => std::fs::read(self.tile_path(layer, col, row))
                .ok()
                .and_then(|b| CafvTile::parse(&b, self.origin_tiles)),
        };
        self.store(layer, key, tile);
        self.get(layer, col, row)
    }

    /// Inserts a decoded tile (tests: synthetic tiles without files).
    pub fn insert(&mut self, layer: u16, tile: CafvTile) {
        let key = key_of(tile.col, tile.row);
        self.layers[layer_slot(layer)].index.insert(key);
        self.store(layer, key, Some(tile));
    }

    fn store(&mut self, layer: u16, key: i32, tile: Option<CafvTile>) {
        let frame = self.frame;
        let max = self.max_cached_tiles;
        let l = &mut self.layers[layer_slot(layer)];
        match tile {
            None => {
                l.broken.insert(key);
            }
            Some(tile) => {
                l.tiles.insert(key, (Arc::new(tile), frame));
                self.loaded_tiles += 1;
                // LRU: drop the oldest unless everything is used this frame (overflow instead).
                while l.tiles.len() > max {
                    let Some((&oldest, &(_, used))) = l.tiles.iter().min_by_key(|(_, (_, f))| *f)
                    else {
                        break;
                    };
                    if used >= frame {
                        break;
                    }
                    l.tiles.remove(&oldest);
                }
            }
        }
    }

    pub fn cached(&self, layer: u16) -> usize {
        self.layers[layer_slot(layer)].tiles.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip() {
        let k = key_of(57, 33);
        assert_eq!((col_of(k), row_of(k)), (57, 33));
        assert_eq!(rect_distance(tile_rect(1, 1), Vec2::new(70.0, 70.0)), 0.0);
        assert!((rect_distance(tile_rect(1, 1), Vec2::new(0.0, 70.0)) - 64.0).abs() < 1e-4);
    }

    #[test]
    fn loads_tiles_from_a_folder_through_the_io_pool() {
        let dir = std::env::temp_dir().join(format!("cent_ans_fine_store_{}", std::process::id()));
        let tiles = dir.join("pyramid/hydro_fine/E2");
        std::fs::create_dir_all(&tiles).unwrap();
        let tile = CafvTile::single_line(
            LAYER_RIVERS,
            2,
            3,
            0,
            &[[130.0, 200.0, 5.0, 20.0], [150.0, 210.0, 4.0, 22.0]],
        );
        std::fs::write(tiles.join("2_3.bin"), tile.encode()).unwrap();
        std::fs::write(
            dir.join("rivers_fine.json"),
            r#"{"dir":"pyramid/hydro_fine","pattern":"E2/{col}_{row}.bin","tiles":[{"col":2,"row":3}]}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("fine_anchors.json"),
            r#"{"settlements":{"set_x":{"px":[1,2],"z":3.5}},"hamlets":{"items":[[1,2,3]]},
               "crossings":[{"id":"b","px":[4,5],"snapped":true,"dir":[0,2],"width_m":40,"z_water":10,"z_deck":18}]}"#,
        )
        .unwrap();
        let (mut store, ok) = FineGeoStore::load_from(&dir, &dir);
        assert!(ok && store.available(LAYER_RIVERS) && !store.available(LAYER_ROADS));
        assert_eq!(store.settlements["set_x"].z, 3.5);
        assert_eq!(store.hamlets, vec![[1.0, 2.0, 3.0, 0.0]]);
        assert_eq!(store.crossings[0].dir, Vec2::Y);
        store.request(LAYER_RIVERS, 2, 3);
        assert_eq!(store.pending(), 1);
        store.poll(true);
        assert!(store.is_loaded(LAYER_RIVERS, 2, 3));
        assert_eq!(store.get(LAYER_RIVERS, 2, 3).unwrap().points(), 2);
        // Absent tile: ready (nothing to wait for), never requested.
        assert!(store.is_ready(LAYER_ROADS, 2, 3));
        store.request(LAYER_RIVERS, 9, 9);
        assert_eq!(store.pending(), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn lru_keeps_the_tiles_used_this_frame() {
        let mut store = FineGeoStore {
            max_cached_tiles: 2,
            ..Default::default()
        };
        for c in 0..3 {
            store.poll(false);
            store.insert(
                LAYER_RIVERS,
                CafvTile::single_line(LAYER_RIVERS, c, 0, 0, &[[0.0; 4]; 2]),
            );
        }
        assert_eq!(store.cached(LAYER_RIVERS), 2);
        assert!(!store.is_loaded(LAYER_RIVERS, 0, 0));
    }
}
