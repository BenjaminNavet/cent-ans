//! Native node selection of the streamed relief quadtree (lot PB3g, ADR 0092).
//!
//! `ReliefLod` wraps `relief_lod::Selector` for `ReliefQuadtree`: the GDScript side mirrors
//! page residency (`add_page` / `remove_page`), calls `update` once per frame and only
//! creates, moves and hides the `MeshInstance3D` nodes listed in the returned changes.
//! It also keeps the bytes of the resident pages, shared (`Arc`) with native jobs such as
//! `VegetationScatter.request_reground` instead of copying them for every request.
//! Main thread only; rendering support, no game rule.

use std::collections::HashMap;
use std::sync::Arc;

use godot::classes::RefCounted;
use godot::prelude::*;
use relief_lod::{Config, Pyramid, Selector, View, PARAM_COUNT, V3};

/// Floats per added node in `update()["added"]`.
pub const ADDED_STRIDE: usize = 9;

#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct ReliefLod {
    selector: Selector,
    bytes: HashMap<i64, Arc<Vec<u8>>>,
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for ReliefLod {
    fn init(base: Base<RefCounted>) -> Self {
        ReliefLod {
            selector: Selector::default(),
            bytes: HashMap::new(),
            base,
        }
    }
}

fn v3(v: Vector3) -> V3 {
    V3::new(v.x as f32, v.y as f32, v.z as f32)
}

#[godot_api]
impl ReliefLod {
    /// Tiles of the pyramid: `tiles[level]` = PackedInt32Array of `row × (16 << level) + col`.
    #[func]
    fn set_pyramid(&mut self, max_level: i64, tiles: VarArray) {
        let lists = tiles
            .iter_shared()
            .map(|v| {
                v.try_to::<PackedInt32Array>()
                    .map(|a| a.as_slice().iter().map(|&i| i as i64).collect())
                    .unwrap_or_default()
            })
            .collect();
        self.selector
            .set_pyramid(Pyramid::new(max_level as i32, lists));
    }

    /// 16 × 16 (min, max) heights in metres per E0 chunk.
    #[func]
    fn set_bounds(&mut self, bounds: PackedVector2Array) {
        self.selector.set_bounds(
            bounds
                .as_slice()
                .iter()
                .map(|b| (b.x as f32, b.y as f32))
                .collect(),
        );
    }

    #[func]
    fn mark_broken(&mut self, key: i64) {
        self.selector.mark_broken(key);
    }

    /// A page was uploaded to `layer`; its bytes are kept (one copy) for native readers.
    #[func]
    fn add_page(
        &mut self,
        key: i64,
        layer: i64,
        t_upload: f64,
        frame: i64,
        bytes: PackedByteArray,
    ) {
        self.selector.add_page(key, layer as i32, t_upload, frame);
        self.bytes.insert(key, Arc::new(bytes.to_vec()));
    }

    #[func]
    fn remove_page(&mut self, key: i64) {
        self.selector.remove_page(key);
        self.bytes.remove(&key);
    }

    #[func]
    fn clear_pages(&mut self) {
        self.selector.clear_pages();
        self.bytes.clear();
    }

    #[func]
    fn page_count(&self) -> i64 {
        self.selector.page_count() as i64
    }

    /// Least recently used page not used since before frame `limit`, −1 if none.
    #[func]
    fn oldest_page(&self, limit: i64) -> i64 {
        self.selector.oldest_page(limit)
    }

    #[func]
    fn clear_slots(&mut self) {
        self.selector.clear_slots();
    }

    /// One frame. `view`: [k, vertical_scale, up, down, now, shadow_cast_distance, frame];
    /// `config`: [max_vertex_px, max_items, max_depth, extra_depth, morph_ratio,
    /// skirt_factor, skirt_max, fade_seconds]. Returns the changes: `removed`, `added_keys` +
    /// `added` (9 floats each: origin x, z, spacing, n, quadrant, quads, aabb y, aabb height,
    /// cast shadow), `cast_keys` + `cast_on`, `param_keys` + `param_masks` + `param_values`
    /// (7 × 4 floats each), `wanted` (by priority), `missing`, `items`, `px_scale`.
    #[func]
    fn update(
        &mut self,
        camera: Vector3,
        planes: VarArray,
        view: PackedFloat64Array,
        config: PackedFloat64Array,
    ) -> VarDictionary {
        let view = view_of(camera, &planes, &view);
        let update = self.selector.update(&view, config_of(&config));
        let mut out = VarDictionary::new();
        out.set(
            "removed",
            &PackedInt64Array::from(update.removed).to_variant(),
        );
        let mut added_keys = Vec::with_capacity(update.added.len());
        let mut added = Vec::with_capacity(update.added.len() * ADDED_STRIDE);
        for a in &update.added {
            added_keys.push(a.key);
            added.extend_from_slice(&[
                a.origin.0,
                a.origin.1,
                a.spacing,
                a.n as f32,
                a.quadrant as f32,
                a.quads,
                a.aabb_y,
                a.aabb_h,
                if a.cast_shadow { 1.0 } else { 0.0 },
            ]);
        }
        out.set(
            "added_keys",
            &PackedInt64Array::from(added_keys).to_variant(),
        );
        out.set("added", &PackedFloat32Array::from(added).to_variant());
        let (cast_keys, cast_on): (Vec<i64>, Vec<u8>) =
            update.casts.iter().map(|&(k, on)| (k, on as u8)).unzip();
        out.set("cast_keys", &PackedInt64Array::from(cast_keys).to_variant());
        out.set("cast_on", &PackedByteArray::from(cast_on).to_variant());
        let mut keys = Vec::with_capacity(update.params.len());
        let mut masks = Vec::with_capacity(update.params.len());
        let mut values = Vec::with_capacity(update.params.len() * PARAM_COUNT * 4);
        for p in &update.params {
            keys.push(p.key);
            masks.push(p.mask);
            for v in &p.values {
                values.extend_from_slice(v);
            }
        }
        out.set("param_keys", &PackedInt64Array::from(keys).to_variant());
        out.set("param_masks", &PackedByteArray::from(masks).to_variant());
        out.set(
            "param_values",
            &PackedFloat32Array::from(values).to_variant(),
        );
        out.set(
            "wanted",
            &PackedInt64Array::from(update.wanted).to_variant(),
        );
        out.set("missing", &update.missing.to_variant());
        out.set("items", &(update.items as i64).to_variant());
        out.set("px_scale", &update.px_scale.to_variant());
        out
    }

    /// Nodes of the last selection (tests): `keys`, `fine`, `coarse` (PackedInt64Array),
    /// `dist`, `ymin`, `ymax` (PackedFloat64Array).
    #[func]
    fn items(&self) -> VarDictionary {
        let items = self.selector.items();
        let mut out = VarDictionary::new();
        let ints = |f: &dyn Fn(&relief_lod::Item) -> i64| {
            PackedInt64Array::from(items.iter().map(f).collect::<Vec<_>>()).to_variant()
        };
        let floats = |f: &dyn Fn(&relief_lod::Item) -> f64| {
            PackedFloat64Array::from(items.iter().map(f).collect::<Vec<_>>()).to_variant()
        };
        out.set("keys", &ints(&|i| i.key));
        out.set("fine", &ints(&|i| i.fine));
        out.set("coarse", &ints(&|i| i.coarse));
        out.set("dist", &floats(&|i| i.dist));
        out.set("ymin", &floats(&|i| i.ymin as f64));
        out.set("ymax", &floats(&|i| i.ymax as f64));
        out
    }

    #[func]
    fn px_scale(&self) -> f64 {
        self.selector.px_scale()
    }
}

impl ReliefLod {
    /// Shared bytes of a resident page (little-endian 16-bit samples).
    pub fn page_bytes(&self, key: i64) -> Option<Arc<Vec<u8>>> {
        self.bytes.get(&key).cloned()
    }
}

fn view_of(camera: Vector3, planes: &VarArray, view: &PackedFloat64Array) -> View {
    let v = |i: usize, default: f64| view.get(i).unwrap_or(default);
    View {
        cam: v3(camera),
        planes: planes
            .iter_shared()
            .filter_map(|p| p.try_to::<Plane>().ok())
            .map(|p| (v3(p.normal), p.d as f32))
            .collect(),
        k: v(0, 1.0),
        vertical_scale: v(1, 1.0),
        up: v(2, 1.0),
        down: v(3, 1.0),
        now: v(4, 0.0),
        shadow_cast_distance: v(5, f64::INFINITY),
        frame: v(6, 0.0) as i64,
    }
}

fn config_of(config: &PackedFloat64Array) -> Config {
    let d = Config::default();
    let c = |i: usize, default: f64| config.get(i).unwrap_or(default);
    Config {
        max_vertex_px: c(0, d.max_vertex_px),
        max_items: c(1, d.max_items as f64) as i64,
        max_depth: c(2, d.max_depth as f64) as i32,
        extra_depth: c(3, d.extra_depth as f64) as i32,
        morph_ratio: c(4, d.morph_ratio),
        skirt_factor: c(5, d.skirt_factor),
        skirt_max: c(6, d.skirt_max),
        fade_seconds: c(7, d.fade_seconds),
    }
}
