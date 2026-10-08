//! Node selection of the streamed relief quadtree (lot PB3g, ADR 0092).
//!
//! Native port of `ReliefQuadtree._select` / `_apply_items` (lot ZG2, ADR 0036): CDLOD
//! selection by screen-space vertex spacing, page residency (layers, LRU), per-node page
//! parameters and the difference with the previous frame. The GDScript side only creates,
//! moves and hides the `MeshInstance3D` nodes this crate reports. Rendering support only,
//! no game rule; pure Rust (no Godot types) so that it is unit-tested with `cargo test`.
//!
//! Numerics mirror GDScript on purpose: scalars in `f64`, vectors (`Vector3`, `Plane`,
//! `PackedFloat32Array`) in `f32`, so that both implementations select the same nodes.

use std::collections::{HashMap, HashSet};

/// Quads per side of a full patch.
pub const PATCH_QUADS: f64 = 64.0;
/// Depth of a node the size of an E0 tile (pyramid level = depth − `DEPTH_E0`).
pub const DEPTH_E0: i32 = 4;
/// Side of a depth-0 node, in map units: the scale of depths only, not the size of the world
/// (ADR 0115). The world (W × H map units, multiples of `ROOT_TILE_UNITS`) is covered by a grid of
/// nodes at depth [`root_depth`].
pub const DEPTH0_UNITS: f64 = ROOT_TILE_UNITS * (1i64 << DEPTH_E0) as f64;
/// Largest E0 grid side the 12-bit tile keys support at `MAX_LEVEL` (4096 >> 7).
pub const MAX_ROOT_TILES: i64 = 0x1000 >> MAX_LEVEL;
/// Finest pyramid level the key layout supports (`ReliefPyramid.MAX_LEVEL`).
pub const MAX_LEVEL: i32 = 7;
/// Side of an E0 tile in map units.
pub const ROOT_TILE_UNITS: f64 = 256.0;
/// Instance parameters per node, in `ReliefQuadtree.PARAM_NAMES` order.
pub const PARAM_COUNT: usize = 7;

/// Pixels per side of a page.
pub const PAGE_PX: usize = 512;

/// `ReliefQuadtree._bilinear`: altitude in metres at pixel coordinates (fx, fy) of a page of
/// little-endian 16-bit samples (clamped to the edge), same arithmetic as GDScript.
pub fn page_bilinear(bytes: &[u8], fx: f64, fy: f64, h_min: f64, h_range: f64) -> f64 {
    let last = (PAGE_PX - 1) as f64;
    let fx = fx.clamp(0.0, last);
    let fy = fy.clamp(0.0, last);
    let i = (fx as usize).min(PAGE_PX - 2);
    let j = (fy as usize).min(PAGE_PX - 2);
    let tx = fx - i as f64;
    let ty = fy - j as f64;
    let o = (j * PAGE_PX + i) * 2;
    let at = |k: usize| -> i64 {
        match bytes.get(k..k + 2) {
            Some(b) => u16::from_le_bytes([b[0], b[1]]) as i64,
            None => 0,
        }
    };
    let (a, b, c, d) = (
        at(o),
        at(o + 2),
        at(o + PAGE_PX * 2),
        at(o + PAGE_PX * 2 + 2),
    );
    let top = a as f64 + (b - a) as f64 * tx;
    let v = (top + (c as f64 + (d - c) as f64 * tx - top) * ty) / 65535.0;
    h_min + v * h_range
}

const SIDES: [(i64, i64); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
const DIAGONALS: [(i64, i64); 4] = [(-1, -1), (1, -1), (-1, 1), (1, 1)];

/// Tile key (`ReliefPyramid.key_of`): level on 3 bits, row and column on 12 bits each.
pub fn key_of(level: i64, col: i64, row: i64) -> i64 {
    (level << 24) | (row << 12) | col
}

pub fn level_of_key(key: i64) -> i64 {
    key >> 24
}

pub fn col_of_key(key: i64) -> i64 {
    key & 0xfff
}

pub fn row_of_key(key: i64) -> i64 {
    (key >> 12) & 0xfff
}

/// Tiles along an axis at a pyramid level, for `root_tiles` E0 tiles along that axis.
pub fn tiles_across(root_tiles: i64, level: i64) -> i64 {
    root_tiles << level
}

/// Shallowest depth whose nodes tile a world of `cols` × `rows` E0 tiles exactly (0 for the
/// 16 × 16 map: one root, as before ADR 0115; 2 for 28 × 24: 7 × 6 roots of 1024 units).
pub fn root_depth(cols: i64, rows: i64) -> i32 {
    (0..DEPTH_E0)
        .find(|&n| {
            let span = 1i64 << (DEPTH_E0 - n);
            cols % span == 0 && rows % span == 0
        })
        .unwrap_or(DEPTH_E0)
}

/// World tile of a tile of the cache (`relief_pyramid.json` `root_origin_tiles`, ADR 0115): cache
/// tile (level, col, row) is world tile (col + dx·2^level, row + dy·2^level).
pub fn cache_to_world(level: i64, col: i64, row: i64, origin: (i64, i64)) -> (i64, i64) {
    (col + (origin.0 << level), row + (origin.1 << level))
}

/// Inverse of [`cache_to_world`] (file names of the cache).
pub fn world_to_cache(level: i64, col: i64, row: i64, origin: (i64, i64)) -> (i64, i64) {
    (col - (origin.0 << level), row - (origin.1 << level))
}

pub fn tile_units(level: i64) -> f64 {
    ROOT_TILE_UNITS / (1i64 << level) as f64
}

pub fn node_size(n: i32) -> f64 {
    DEPTH0_UNITS / (1i64 << n) as f64
}

/// Node key of an item (`ReliefQuadtree._add_item`): quadrant 4 = whole node.
pub fn item_key(n: i32, c: i64, r: i64, quadrant: i32) -> i64 {
    ((n as i64) << 40) | (r << 20) | (c << 3) | quadrant as i64
}

/// `Vector3` with Godot's single-precision arithmetic.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl V3 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        V3 { x, y, z }
    }

    fn clamp(self, lo: V3, hi: V3) -> V3 {
        fn c(a: f32, lo: f32, hi: f32) -> f32 {
            if a < lo {
                lo
            } else if a > hi {
                hi
            } else {
                a
            }
        }
        V3::new(
            c(self.x, lo.x, hi.x),
            c(self.y, lo.y, hi.y),
            c(self.z, lo.z, hi.z),
        )
    }

    fn length_squared_to(self, other: V3) -> f32 {
        let dx = other.x - self.x;
        let dy = other.y - self.y;
        let dz = other.z - self.z;
        dx * dx + dy * dy + dz * dz
    }
}

/// Tiles present in the pyramid (`ReliefPyramid`: manifest, E0 files, broken tiles), in world
/// tile coordinates (the `root_origin_tiles` offset of the cache already applied).
#[derive(Default)]
pub struct Pyramid {
    max_level: i32,
    /// E0 tiles along x and y (W / 256, H / 256).
    cols: i64,
    rows: i64,
    tiles: Vec<HashSet<i64>>,
    broken: HashSet<i64>,
    max_under: HashMap<i64, i32>,
}

impl Pyramid {
    /// `tiles[level]`: indices `row × tiles_across(cols, level) + col` of the tiles of each level,
    /// over a world of `cols` × `rows` E0 tiles.
    pub fn new(max_level: i32, tiles: Vec<Vec<i64>>, cols: i64, rows: i64) -> Self {
        debug_assert!(cols <= MAX_ROOT_TILES && rows <= MAX_ROOT_TILES);
        let tiles: Vec<HashSet<i64>> = tiles
            .into_iter()
            .map(|list| list.into_iter().collect())
            .collect();
        let mut max_under: HashMap<i64, i32> = HashMap::new();
        for level in 1..=max_level {
            let Some(set) = tiles.get(level as usize) else {
                break;
            };
            let across = tiles_across(cols, level as i64);
            for &index in set {
                let (col, row) = (index % across, index / across);
                for up in (0..level).rev() {
                    let shift = level - up;
                    let akey = key_of(up as i64, col >> shift, row >> shift);
                    let entry = max_under.entry(akey).or_insert(-1);
                    if *entry >= level {
                        break;
                    }
                    *entry = level;
                }
            }
        }
        Pyramid {
            max_level,
            cols,
            rows,
            tiles,
            broken: HashSet::new(),
            max_under,
        }
    }

    pub fn max_level(&self) -> i32 {
        self.max_level
    }

    /// E0 tiles along x and y.
    pub fn root_tiles(&self) -> (i64, i64) {
        (self.cols, self.rows)
    }

    pub fn mark_broken(&mut self, key: i64) {
        self.broken.insert(key);
    }

    pub fn has_tile(&self, level: i64, col: i64, row: i64) -> bool {
        if level < 0 || level > MAX_LEVEL as i64 || level as usize >= self.tiles.len() {
            return false;
        }
        let across = tiles_across(self.cols, level);
        if col < 0 || row < 0 || col >= across || row >= tiles_across(self.rows, level) {
            return false;
        }
        if self.broken.contains(&key_of(level, col, row)) {
            return false;
        }
        self.tiles[level as usize].contains(&(row * across + col))
    }

    /// Finest level present in the subtree of the tile (itself included), −1 if none.
    pub fn max_level_under(&self, level: i64, col: i64, row: i64) -> i64 {
        let best = if self.has_tile(level, col, row) {
            level
        } else {
            -1
        };
        let under = self
            .max_under
            .get(&key_of(level, col, row))
            .copied()
            .unwrap_or(-1) as i64;
        best.max(under)
    }

    /// Finest level ≤ `level` present over the tile (−1 if none).
    pub fn finest_ancestor(&self, level: i64, col: i64, row: i64) -> i64 {
        for up in (0..=level.min(self.max_level as i64)).rev() {
            let shift = level - up;
            if self.has_tile(up, col >> shift, row >> shift) {
                return up;
            }
        }
        -1
    }
}

/// Tunables (`ReliefQuadtree` exports), passed every frame since presets may change them.
#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub max_vertex_px: f64,
    pub max_items: i64,
    pub max_depth: i32,
    pub extra_depth: i32,
    pub morph_ratio: f64,
    pub skirt_factor: f64,
    pub skirt_max: f64,
    pub fade_seconds: f64,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            max_vertex_px: 4.0,
            max_items: 700,
            max_depth: 14,
            extra_depth: 3,
            morph_ratio: 0.7,
            skirt_factor: 1.5,
            skirt_max: 4.0,
            fade_seconds: 0.35,
        }
    }
}

/// Camera and frame state.
#[derive(Clone, Debug, Default)]
pub struct View {
    pub cam: V3,
    /// Frustum planes (normal, d) as `Camera3D.get_frustum`.
    pub planes: Vec<(V3, f32)>,
    /// Projection factor: viewport height / (2 tan(fov / 2)).
    pub k: f64,
    /// Vertical scale, (1 + gain) and (1 − squash max) of the displayed relief.
    pub vertical_scale: f64,
    pub up: f64,
    pub down: f64,
    /// Seconds (`Time.get_ticks_msec() / 1000`).
    pub now: f64,
    pub shadow_cast_distance: f64,
    pub frame: i64,
}

/// One selected node (`ReliefQuadtree._items`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Item {
    pub key: i64,
    pub n: i32,
    pub quadrant: i32,
    pub origin: (f32, f32),
    pub fine: i64,
    pub coarse: i64,
    pub ymin: f32,
    pub ymax: f32,
    pub dist: f64,
}

/// A node to show: geometry of its `MeshInstance3D`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Added {
    pub key: i64,
    pub n: i32,
    pub quadrant: i32,
    pub origin: (f32, f32),
    /// Vertex spacing (transform scale).
    pub spacing: f32,
    /// `custom_aabb`: y origin and height (x and z span `quads`).
    pub aabb_y: f32,
    pub aabb_h: f32,
    pub quads: f32,
    pub cast_shadow: bool,
}

/// Instance parameters to send: `mask` bit i set = parameter i changed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParamChange {
    pub key: i64,
    pub mask: u8,
    pub values: [[f32; 4]; PARAM_COUNT],
}

/// What changed since the previous frame.
#[derive(Clone, Debug, Default)]
pub struct Update {
    pub removed: Vec<i64>,
    pub added: Vec<Added>,
    /// Existing nodes whose shadow casting changed.
    pub casts: Vec<(i64, bool)>,
    pub params: Vec<ParamChange>,
    /// Wanted pages not resident, by priority (coarsest level, then distance).
    pub wanted: Vec<i64>,
    pub missing: i64,
    pub items: usize,
    pub px_scale: f64,
}

#[derive(Clone, Copy, Debug)]
struct Page {
    layer: i32,
    last_used: i64,
    t_upload: f64,
    seq: u64,
}

#[derive(Clone, Copy, Debug)]
struct Slot {
    sig: (i64, i64, u64, u64),
    fading: bool,
    cast: bool,
    params: [[f32; 4]; PARAM_COUNT],
}

type PageParams = [[f32; 4]; 3];

/// Selection state of one quadtree.
pub struct Selector {
    pyramid: Pyramid,
    /// Depth of the root nodes ([`root_depth`] of the pyramid grid).
    root_depth: i32,
    /// cols × rows (min, max) heights in metres per E0 chunk (row-major).
    bounds: Vec<(f32, f32)>,
    pages: HashMap<i64, Page>,
    seq: u64,
    residency_version: u64,
    range_version: u64,
    k_proj: f64,
    px_scale: f64,
    ranges: Vec<f32>,
    items: Vec<Item>,
    wanted: HashMap<i64, f64>,
    page_cache: HashMap<i64, i64>,
    missing: i64,
    param_cache: HashMap<i64, PageParams>,
    param_cache_version: u64,
    slots: HashMap<i64, Slot>,
    // Per-frame view state.
    cam: V3,
    planes: Vec<(V3, f32)>,
    vertical_scale: f64,
    up: f64,
    down: f64,
    config: Config,
}

impl Default for Selector {
    fn default() -> Self {
        Selector::new(Pyramid::default(), Vec::new())
    }
}

impl Selector {
    pub fn new(pyramid: Pyramid, bounds: Vec<(f32, f32)>) -> Self {
        Selector {
            root_depth: Self::depth_of(&pyramid),
            pyramid,
            bounds,
            pages: HashMap::new(),
            seq: 0,
            residency_version: 0,
            range_version: 0,
            k_proj: 1.0,
            px_scale: 1.0,
            ranges: Vec::new(),
            items: Vec::new(),
            wanted: HashMap::new(),
            page_cache: HashMap::new(),
            missing: 0,
            param_cache: HashMap::new(),
            param_cache_version: u64::MAX,
            slots: HashMap::new(),
            cam: V3::default(),
            planes: Vec::new(),
            vertical_scale: 1.0,
            up: 1.0,
            down: 1.0,
            config: Config::default(),
        }
    }

    fn depth_of(pyramid: &Pyramid) -> i32 {
        let (cols, rows) = pyramid.root_tiles();
        root_depth(cols.max(1), rows.max(1))
    }

    pub fn set_pyramid(&mut self, pyramid: Pyramid) {
        self.root_depth = Self::depth_of(&pyramid);
        self.pyramid = pyramid;
    }

    /// Depth of the root nodes.
    pub fn root_depth(&self) -> i32 {
        self.root_depth
    }

    /// E0 tiles along x and y of the world.
    pub fn root_tiles(&self) -> (i64, i64) {
        self.pyramid.root_tiles()
    }

    pub fn set_bounds(&mut self, bounds: Vec<(f32, f32)>) {
        self.bounds = bounds;
    }

    pub fn mark_broken(&mut self, key: i64) {
        self.pyramid.mark_broken(key);
    }

    // --- Residency ---

    /// A page was uploaded to `layer` (`ReliefQuadtree._upload`).
    pub fn add_page(&mut self, key: i64, layer: i32, t_upload: f64, frame: i64) {
        self.pages.insert(
            key,
            Page {
                layer,
                last_used: frame,
                t_upload,
                seq: self.seq,
            },
        );
        self.seq += 1;
        self.residency_version += 1;
    }

    pub fn remove_page(&mut self, key: i64) {
        if self.pages.remove(&key).is_some() {
            self.residency_version += 1;
        }
    }

    pub fn has_page(&self, key: i64) -> bool {
        self.pages.contains_key(&key)
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Least recently used page among those not used since before `limit` (`_alloc_layer`:
    /// first in insertion order on ties), −1 if none.
    pub fn oldest_page(&self, limit: i64) -> i64 {
        let mut best: Option<(i64, u64, i64)> = None;
        for (&key, page) in &self.pages {
            if page.last_used >= limit {
                continue;
            }
            let rank = (page.last_used, page.seq);
            if best.is_none_or(|(u, s, _)| rank < (u, s)) {
                best = Some((page.last_used, page.seq, key));
            }
        }
        best.map_or(-1, |(_, _, key)| key)
    }

    pub fn items(&self) -> &[Item] {
        &self.items
    }

    pub fn px_scale(&self) -> f64 {
        self.px_scale
    }

    // --- Frame ---

    /// Selects the nodes for `view`, updates page use and returns the changes to apply.
    pub fn update(&mut self, view: &View, config: Config) -> Update {
        self.select_nodes(view, config);
        let mut update = Update {
            wanted: self.wanted_order(),
            missing: self.missing,
            items: self.items.len(),
            px_scale: self.px_scale,
            ..Default::default()
        };
        self.apply(view, &mut update);
        update
    }

    /// Selection only (`update_view` up to `_apply_items`); `items()` then holds the nodes.
    pub fn select_nodes(&mut self, view: &View, config: Config) {
        self.config = config;
        self.cam = view.cam;
        self.planes.clone_from(&view.planes);
        self.vertical_scale = view.vertical_scale;
        self.up = view.up;
        self.down = view.down;
        self.prepare(view.k);
        self.items.clear();
        self.wanted.clear();
        self.page_cache.clear();
        self.missing = 0;
        let (cols, rows) = self.pyramid.root_tiles();
        let span = 1i64 << (DEPTH_E0 - self.root_depth);
        for r in 0..rows / span {
            for c in 0..cols / span {
                self.select(self.root_depth, c, r);
            }
        }
        let count = self.items.len() as f64;
        let max_items = self.config.max_items as f64;
        if count > max_items {
            self.px_scale = (self.px_scale * 1.2).min(8.0);
        } else if count < max_items * 0.6 && self.px_scale > 1.0 {
            self.px_scale = (self.px_scale / 1.1).max(1.0);
        }
    }

    fn prepare(&mut self, k: f64) {
        let threshold = self.config.max_vertex_px * self.px_scale;
        let depth = self.config.max_depth.clamp(0, 30);
        let wanted_len = depth as usize + 2;
        if (k / threshold - self.k_proj).abs() > 0.001 * self.k_proj
            || self.ranges.len() != wanted_len
        {
            self.k_proj = k / threshold;
            self.ranges.clear();
            for n in 0..wanted_len as i32 {
                let size = node_size(n);
                self.ranges
                    .push((2.0 * size / PATCH_QUADS * self.k_proj).max(3.0 * size) as f32);
            }
            self.range_version += 1;
        }
    }

    fn range(&self, n: i32) -> f32 {
        self.ranges
            .get(n as usize)
            .copied()
            .unwrap_or(f32::INFINITY)
    }

    fn select(&mut self, n: i32, c: i64, r: i64) -> bool {
        let size = node_size(n);
        let ox = c as f64 * size;
        let oz = r as f64 * size;
        let yb = self.y_bounds(n, c, r);
        let bmin = V3::new(ox as f32, yb.0, oz as f32);
        let bmax = V3::new((ox + size) as f32, yb.1, (oz + size) as f32);
        if n > self.root_depth && !self.box_in_sphere(bmin, bmax, self.range(n)) {
            return false;
        }
        if !self.box_in_frustum(bmin, bmax) {
            return true;
        }
        if n >= self.depth_cap(n, c, r) || !self.box_in_sphere(bmin, bmax, self.range(n + 1)) {
            self.add_item(n, c, r, 4, bmin, bmax);
            return true;
        }
        for q in 0..4i64 {
            if !self.select(n + 1, c * 2 + (q & 1), r * 2 + (q >> 1)) {
                let half = size * 0.5;
                let qmin = V3::new(
                    (ox + (q & 1) as f64 * half) as f32,
                    yb.0,
                    (oz + (q >> 1) as f64 * half) as f32,
                );
                let qmax = V3::new(
                    qmin.x + half as f32,
                    qmin.y + (yb.1 - yb.0),
                    qmin.z + half as f32,
                );
                self.add_item(n, c, r, q as i32, qmin, qmax);
            }
        }
        true
    }

    fn depth_cap(&self, n: i32, c: i64, r: i64) -> i32 {
        if n < DEPTH_E0 {
            return self.config.max_depth;
        }
        let level = (n - DEPTH_E0) as i64;
        let mut data = self.pyramid.max_level_under(level, c, r);
        if data < 0 {
            data = self.pyramid.finest_ancestor(level, c, r);
        }
        ((DEPTH_E0 as i64 + data + self.config.extra_depth as i64) as i32)
            .min(self.config.max_depth)
    }

    fn y_bounds(&self, n: i32, c: i64, r: i64) -> (f32, f32) {
        let (cols, rows) = self.pyramid.root_tiles();
        if cols <= 0 || rows <= 0 || (self.bounds.len() as i64) < cols * rows {
            return self.to_world((-400.0, 5000.0));
        }
        if n >= DEPTH_E0 {
            let shift = n - DEPTH_E0;
            let index = (r >> shift).clamp(0, rows - 1) * cols + (c >> shift).clamp(0, cols - 1);
            return self.to_world(self.bounds[index as usize]);
        }
        let span = 1i64 << (DEPTH_E0 - n);
        let mut result = (f32::INFINITY, f32::NEG_INFINITY);
        for j in 0..span {
            for i in 0..span {
                let b = self.bounds[((r * span + j) * cols + c * span + i) as usize];
                result = (result.0.min(b.0), result.1.max(b.1));
            }
        }
        self.to_world(result)
    }

    /// Metres → displayed heights (`ReliefQuadtree._to_world_bounds`).
    fn to_world(&self, b: (f32, f32)) -> (f32, f32) {
        let vs = self.vertical_scale;
        let lo = b.0 as f64 * vs * if b.0 > 0.0 { self.down } else { 1.0 };
        let hi = b.1 as f64 * vs * if b.1 > 0.0 { self.up } else { 1.0 };
        (lo as f32, hi as f32)
    }

    fn box_in_sphere(&self, bmin: V3, bmax: V3, radius: f32) -> bool {
        let closest = self.cam.clamp(bmin, bmax);
        let radius = radius as f64;
        (closest.length_squared_to(self.cam) as f64) <= radius * radius
    }

    fn box_in_frustum(&self, bmin: V3, bmax: V3) -> bool {
        for (normal, d) in &self.planes {
            let p = V3::new(
                if normal.x > 0.0 { bmin.x } else { bmax.x },
                if normal.y > 0.0 { bmin.y } else { bmax.y },
                if normal.z > 0.0 { bmin.z } else { bmax.z },
            );
            let dot = normal.x * p.x + normal.y * p.y + normal.z * p.z;
            if dot - d > 0.0 {
                return false;
            }
        }
        true
    }

    fn add_item(&mut self, n: i32, c: i64, r: i64, quadrant: i32, bmin: V3, bmax: V3) {
        let dist = self
            .cam
            .length_squared_to(self.cam.clamp(bmin, bmax))
            .sqrt() as f64;
        let fine = self.page_of(n, c, r, dist);
        let coarse = if n == self.root_depth {
            fine
        } else {
            self.page_of(n - 1, c >> 1, r >> 1, dist)
        };
        self.items.push(Item {
            key: item_key(n, c, r, quadrant),
            n,
            quadrant,
            origin: (bmin.x, bmin.z),
            fine,
            coarse,
            ymin: bmin.y,
            ymax: bmax.y,
            dist,
        });
    }

    /// Page used by node (n, c, r): the tile of its level (or its finest existing ancestor),
    /// wanted if absent; meanwhile, the finest resident ancestor (−1: heightmap).
    fn page_of(&mut self, n: i32, c: i64, r: i64, dist: f64) -> i64 {
        if n < DEPTH_E0 {
            return -1;
        }
        let level = (n - DEPTH_E0) as i64;
        let cache_key = ((n as i64) << 40) | (r << 20) | c;
        if let Some(&cached) = self.page_cache.get(&cache_key) {
            return cached;
        }
        let mut result = -1;
        let target = self.pyramid.finest_ancestor(level, c, r);
        if target >= 0 {
            let shift = level - target;
            let want = key_of(target, c >> shift, r >> shift);
            if self.pages.contains_key(&want) {
                result = want;
            } else {
                self.missing += 1;
                let priority = target as f64 * 1.0e6 + dist;
                let entry = self.wanted.entry(want).or_insert(f64::INFINITY);
                if priority < *entry {
                    *entry = priority;
                }
                for up in (0..target).rev() {
                    let s = level - up;
                    let key = key_of(up, c >> s, r >> s);
                    if self.pages.contains_key(&key) {
                        result = key;
                        break;
                    }
                }
            }
        }
        self.page_cache.insert(cache_key, result);
        result
    }

    fn wanted_order(&self) -> Vec<i64> {
        let mut order: Vec<(f64, i64)> = self.wanted.iter().map(|(&k, &p)| (p, k)).collect();
        order.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        order.into_iter().map(|(_, key)| key).collect()
    }

    // --- Instances ---

    fn apply(&mut self, view: &View, update: &mut Update) {
        let frame = view.frame;
        let fade_span = self.config.fade_seconds.max(0.001);
        let mut alive: HashSet<i64> = HashSet::with_capacity(self.items.len());
        let items = std::mem::take(&mut self.items);
        for item in &items {
            alive.insert(item.key);
            let cast = item.dist < view.shadow_cast_distance;
            let fresh = !self.slots.contains_key(&item.key);
            if fresh {
                let s = node_size(item.n) / PATCH_QUADS;
                let quads = if item.quadrant == 4 {
                    PATCH_QUADS
                } else {
                    PATCH_QUADS / 2.0
                };
                let skirt = (s * self.config.skirt_factor).min(self.config.skirt_max) + 0.02;
                update.added.push(Added {
                    key: item.key,
                    n: item.n,
                    quadrant: item.quadrant,
                    origin: item.origin,
                    spacing: s as f32,
                    aabb_y: (item.ymin as f64 - skirt) as f32,
                    aabb_h: (item.ymax as f64 - item.ymin as f64 + skirt) as f32,
                    quads: quads as f32,
                    cast_shadow: cast,
                });
            }
            let mut fade = 1.0;
            if item.fine >= 0 {
                if let Some(page) = self.pages.get_mut(&item.fine) {
                    page.last_used = frame;
                    fade = ((view.now - page.t_upload) / fade_span).clamp(0.0, 1.0);
                }
            }
            if item.coarse >= 0 {
                if let Some(page) = self.pages.get_mut(&item.coarse) {
                    page.last_used = frame;
                }
            }
            let sig = (
                item.fine,
                item.coarse,
                self.residency_version,
                self.range_version,
            );
            let previous = self.slots.get(&item.key).copied();
            if let Some(slot) = previous {
                if slot.cast != cast {
                    update.casts.push((item.key, cast));
                }
            }
            let refresh = match previous {
                None => true,
                Some(slot) => slot.sig != sig || fade < 1.0 || slot.fading,
            };
            let mut params = previous.map(|slot| slot.params).unwrap_or_default();
            if refresh {
                let values = self.node_params(item, fade, frame);
                let mut mask = 0u8;
                for (i, value) in values.iter().enumerate() {
                    if previous.is_none() || params[i] != *value {
                        mask |= 1 << i;
                    }
                }
                if mask != 0 {
                    update.params.push(ParamChange {
                        key: item.key,
                        mask,
                        values,
                    });
                }
                params = values;
            }
            let fading = if refresh {
                fade < 1.0
            } else {
                previous.is_some_and(|slot| slot.fading)
            };
            let sig = if refresh {
                sig
            } else {
                previous.map_or(sig, |slot| slot.sig)
            };
            self.slots.insert(
                item.key,
                Slot {
                    sig,
                    fading,
                    cast,
                    params,
                },
            );
        }
        self.items = items;
        self.slots.retain(|key, _| {
            if alive.contains(key) {
                true
            } else {
                update.removed.push(*key);
                false
            }
        });
    }

    /// `ReliefQuadtree._set_page_params` values.
    fn node_params(&mut self, item: &Item, fade: f64, frame: i64) -> [[f32; 4]; PARAM_COUNT] {
        let s = node_size(item.n) / PATCH_QUADS;
        let mut morph = [
            1.0e9f32,
            1.0,
            fade as f32,
            ((s * self.config.skirt_factor).min(self.config.skirt_max) + 0.02) as f32,
        ];
        if item.n > self.root_depth {
            let reach = self.range(item.n) as f64;
            morph[0] = (self.config.morph_ratio * reach) as f32;
            morph[1] = (1.0 / ((1.0 - self.config.morph_ratio) * reach).max(1e-4)) as f32;
        }
        let fine = self.page_params(item.fine, frame);
        let coarse = self.page_params(item.coarse, frame);
        [
            fine[0], coarse[0], fine[1], fine[2], coarse[1], coarse[2], morph,
        ]
    }

    /// [extent, side neighbours, diagonal neighbours] of a page, cached until the next
    /// residency change (neighbour pages are marked used when computed, as in GDScript).
    fn page_params(&mut self, key: i64, frame: i64) -> PageParams {
        if self.param_cache_version != self.residency_version {
            self.param_cache.clear();
            self.param_cache_version = self.residency_version;
        }
        if let Some(cached) = self.param_cache.get(&key) {
            return *cached;
        }
        let params = [
            self.page_vec(key),
            self.neighbors(key, false, frame),
            self.neighbors(key, true, frame),
        ];
        self.param_cache.insert(key, params);
        params
    }

    fn page_vec(&self, key: i64) -> [f32; 4] {
        let Some(page) = (key >= 0).then(|| self.pages.get(&key)).flatten() else {
            return [-1.0, 0.0, 0.0, 1.0];
        };
        let level = level_of_key(key);
        let t = tile_units(level);
        [
            page.layer as f32,
            (col_of_key(key) as f64 * t) as f32,
            (row_of_key(key) as f64 * t) as f32,
            t as f32,
        ]
    }

    fn neighbors(&mut self, key: i64, diagonal: bool, frame: i64) -> [f32; 4] {
        if key < 0 {
            return [-1.0; 4];
        }
        let level = level_of_key(key);
        let (c, r) = (col_of_key(key), row_of_key(key));
        let offsets = if diagonal { &DIAGONALS } else { &SIDES };
        let (cols, rows) = self.pyramid.root_tiles();
        let (across, down) = (tiles_across(cols, level), tiles_across(rows, level));
        let mut result = [-1.0f32; 4];
        for (i, (dx, dy)) in offsets.iter().enumerate() {
            let (nc, nr) = (c + dx, r + dy);
            if nc >= 0 && nr >= 0 && nc < across && nr < down {
                if let Some(page) = self.pages.get_mut(&key_of(level, nc, nr)) {
                    result[i] = page.layer as f32;
                    page.last_used = frame;
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pyramid() -> Pyramid {
        // Every E0 tile, E1 over E0 (8, 7), E2 over its north-west quarter.
        let e0: Vec<i64> = (0..256).collect();
        let e1 = vec![14 * 32 + 16, 14 * 32 + 17, 15 * 32 + 16, 15 * 32 + 17];
        let e2 = vec![28 * 64 + 32, 28 * 64 + 33, 29 * 64 + 32, 29 * 64 + 33];
        Pyramid::new(2, vec![e0, e1, e2], 16, 16)
    }

    /// Frustum of a camera at `eye` looking straight down with a 90° field of view.
    fn view_down(eye: V3, frame: i64) -> View {
        let s = std::f32::consts::FRAC_1_SQRT_2;
        let planes = vec![
            (V3::new(0.0, 1.0, 0.0), eye.y - 0.05), // near (normal outwards: up)
            (V3::new(0.0, -1.0, 0.0), -(eye.y - 4000.0)), // far
            (V3::new(-s, s, 0.0), -s * eye.x + s * eye.y), // left
            (V3::new(s, s, 0.0), s * eye.x + s * eye.y), // right
            (V3::new(0.0, s, -s), -s * eye.z + s * eye.y), // top
            (V3::new(0.0, s, s), s * eye.z + s * eye.y), // bottom
        ];
        View {
            cam: eye,
            planes,
            k: 360.0,
            vertical_scale: 0.01,
            up: 1.0,
            down: 1.0,
            now: 10.0,
            shadow_cast_distance: f64::INFINITY,
            frame,
        }
    }

    #[test]
    fn keys_round_trip() {
        let key = key_of(5, 1234, 987);
        assert_eq!(level_of_key(key), 5);
        assert_eq!(col_of_key(key), 1234);
        assert_eq!(row_of_key(key), 987);
        assert_eq!(item_key(3, 2, 1, 4), (3 << 40) | (1 << 20) | (2 << 3) | 4);
    }

    #[test]
    fn pyramid_queries_match_gdscript() {
        let p = pyramid();
        assert!(p.has_tile(1, 17, 15) && !p.has_tile(1, 18, 15));
        assert_eq!(p.max_level_under(0, 8, 7), 2);
        assert_eq!(p.max_level_under(1, 17, 15), 1);
        assert_eq!(p.finest_ancestor(5, 32 * 8, 28 * 8), 2);
        assert_eq!(p.finest_ancestor(3, 70, 60), 1);
        assert_eq!(p.max_level_under(0, 0, 0), 0);
        let mut p = p;
        p.mark_broken(key_of(2, 32, 28));
        assert_eq!(p.finest_ancestor(2, 32, 28), 1);
    }

    #[test]
    fn selection_refines_under_the_camera_and_wants_pages() {
        let mut sel = Selector::new(pyramid(), vec![(0.0, 100.0); 256]);
        let eye = V3::new(2120.0, 3.0, 1800.0);
        let update = sel.update(&view_down(eye, 1), Config::default());
        assert!(update.items > 0);
        assert_eq!(update.added.len(), update.items);
        assert!(update.removed.is_empty());
        let deepest = sel.items().iter().map(|i| i.n).max().unwrap();
        // E2 data + 3 extra levels = depth 9 under the camera.
        assert_eq!(deepest, DEPTH_E0 + 2 + 3);
        // Nothing resident: pages wanted, coarsest first.
        assert!(update.missing > 0 && !update.wanted.is_empty());
        let levels: Vec<i64> = update.wanted.iter().map(|&k| level_of_key(k)).collect();
        assert!(levels.windows(2).all(|w| w[0] <= w[1]));
        // Every point of the map is covered at most once: areas add up to the visible part.
        for item in sel.items() {
            assert!(item.fine == -1 && item.coarse == -1);
        }
    }

    #[test]
    fn second_frame_only_reports_changes() {
        let mut sel = Selector::new(pyramid(), vec![(0.0, 100.0); 256]);
        let eye = V3::new(2120.0, 3.0, 1800.0);
        let first = sel.update(&view_down(eye, 1), Config::default());
        let second = sel.update(&view_down(eye, 2), Config::default());
        assert!(second.added.is_empty() && second.removed.is_empty());
        assert!(second.params.is_empty() && second.casts.is_empty());
        assert_eq!(first.items, second.items);
        // A page arrives: nodes using it get new parameters, with the fade.
        let key = first.wanted[0];
        sel.add_page(key, 3, 10.0, 2);
        let third = sel.update(&view_down(eye, 3), Config::default());
        assert!(!third.params.is_empty());
        assert!(third
            .params
            .iter()
            .any(|p| p.values[0][0] == 3.0 && p.values[6][2] == 0.0));
        // Moving away drops nodes.
        let far = sel.update(
            &view_down(V3::new(1000.0, 800.0, 1000.0), 4),
            Config::default(),
        );
        assert!(!far.removed.is_empty());
    }

    /// World of 28 × 24 E0 tiles (7168 × 6144 units, ADR 0115): E0 everywhere, E1 over (20, 3).
    fn wide_pyramid() -> Pyramid {
        let e0: Vec<i64> = (0..28 * 24).collect();
        let e1 = vec![6 * 56 + 40, 6 * 56 + 41, 7 * 56 + 40, 7 * 56 + 41];
        Pyramid::new(1, vec![e0, e1], 28, 24)
    }

    #[test]
    fn root_depth_tiles_the_world() {
        assert_eq!(root_depth(16, 16), 0);
        assert_eq!(root_depth(28, 24), 2);
        assert_eq!(root_depth(20, 12), 2);
        assert_eq!(root_depth(24, 24), 1);
        assert_eq!(root_depth(18, 18), 3);
        assert_eq!(root_depth(7, 5), DEPTH_E0);
        assert_eq!(node_size(0), 4096.0);
        assert_eq!(node_size(DEPTH_E0), ROOT_TILE_UNITS);
    }

    #[test]
    fn cache_offset_round_trips() {
        assert_eq!(cache_to_world(0, 3, 4, (0, 5)), (3, 9));
        assert_eq!(cache_to_world(3, 10, 20, (0, 5)), (10, 60));
        assert_eq!(world_to_cache(3, 10, 60, (0, 5)), (10, 20));
        assert_eq!(cache_to_world(7, 1, 1, (0, 0)), (1, 1));
    }

    #[test]
    fn wide_pyramid_bounds_are_rectangular() {
        let p = wide_pyramid();
        assert_eq!(p.root_tiles(), (28, 24));
        assert!(p.has_tile(0, 27, 23));
        assert!(!p.has_tile(0, 28, 0) && !p.has_tile(0, 0, 24));
        assert!(p.has_tile(1, 41, 7) && !p.has_tile(1, 42, 7));
        assert_eq!(p.max_level_under(0, 20, 3), 1);
        assert_eq!(p.finest_ancestor(4, 20 * 16 + 3, 3 * 16 + 15), 1);
        assert_eq!(p.finest_ancestor(4, 0, 0), 0);
    }

    #[test]
    fn wide_world_far_view_selects_the_root_grid() {
        let mut sel = Selector::new(wide_pyramid(), vec![(0.0, 100.0); 28 * 24]);
        assert_eq!(sel.root_depth(), 2);
        // Far above the middle of the world: every root visible, none refined.
        let eye = V3::new(3584.0, 1.0e6, 3072.0);
        let mut view = view_down(eye, 1);
        view.planes[1].1 = -(eye.y - 2.0e6);
        view.k = 1.0;
        let update = sel.update(&view, Config::default());
        assert_eq!(update.items, 42);
        let mut covered = 0.0;
        for item in sel.items() {
            assert_eq!(item.n, 2);
            assert!(item.origin.0 < 7168.0 && item.origin.1 < 6144.0);
            covered += node_size(item.n) * node_size(item.n);
        }
        assert_eq!(covered, 7168.0 * 6144.0);
    }

    #[test]
    fn wide_world_refines_in_the_south_east() {
        let mut sel = Selector::new(wide_pyramid(), vec![(0.0, 100.0); 28 * 24]);
        // Near the ground over E1 tile (41, 7), beyond x = 4096.
        let eye = V3::new(5300.0, 3.0, 950.0);
        let update = sel.update(&view_down(eye, 1), Config::default());
        assert!(update.items > 0);
        let deepest = sel.items().iter().map(|i| i.n).max().unwrap();
        assert_eq!(deepest, DEPTH_E0 + 1 + 3);
        assert!(update.wanted.contains(&key_of(1, 41, 7)));
        // No node outside the world.
        for item in sel.items() {
            let size = node_size(item.n) as f32;
            assert!(item.origin.0 + size <= 7168.0 + 1e-3 && item.origin.1 + size <= 6144.0 + 1e-3);
        }
    }

    #[test]
    fn oldest_page_follows_use_then_insertion_order() {
        let mut sel = Selector::default();
        sel.add_page(10, 0, 0.0, 1);
        sel.add_page(11, 1, 0.0, 1);
        sel.add_page(12, 2, 0.0, 0);
        assert_eq!(sel.oldest_page(5), 12);
        sel.remove_page(12);
        assert_eq!(sel.oldest_page(5), 10);
        assert_eq!(sel.oldest_page(1), -1);
        sel.remove_page(10);
        sel.add_page(10, 0, 0.0, 1);
        assert_eq!(sel.oldest_page(5), 11);
    }

    #[test]
    fn budget_widens_the_threshold() {
        let mut sel = Selector::new(pyramid(), vec![(0.0, 100.0); 256]);
        let eye = V3::new(2120.0, 3.0, 1800.0);
        let config = Config {
            max_items: 2,
            ..Config::default()
        };
        let update = sel.update(&view_down(eye, 1), config);
        assert!(update.items > 2);
        assert!((update.px_scale - 1.2).abs() < 1e-9);
    }
}
