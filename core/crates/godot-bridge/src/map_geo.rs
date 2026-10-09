//! Loader of the vector map data for GDScript (lot SC DT4, ADR 0206).
//!
//! Parses `provinces.geojson`, `rivers.geojson`, `coastline.geojson`, `roads.geojson` and
//! `rivers_render.json` with serde (`data_model::map_geo`) and hands back ready-to-use Godot values
//! (`PackedVector2Array` lines in dictionaries shaped like the former GDScript parsers). The
//! provinces file is parsed once per path and cached. Rendering support only, no game rule.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use data_model::map_geo::{self, Line, MapGeoError, ProvinceFeature};
use godot::classes::{FileAccess, RefCounted};
use godot::prelude::*;

type ProvinceCache = Mutex<HashMap<String, Arc<Vec<ProvinceFeature>>>>;

fn province_cache() -> &'static ProvinceCache {
    static CACHE: OnceLock<ProvinceCache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Reads a file through Godot (`res://` and `user://` paths work); `None` if it is missing.
fn read_bytes(path: &GString) -> Option<PackedByteArray> {
    if !FileAccess::file_exists(path) {
        return None;
    }
    let bytes = FileAccess::get_file_as_bytes(path);
    (!bytes.is_empty()).then_some(bytes)
}

/// Reads and parses `path`; logs and returns `None` when the file is absent or invalid.
fn load<T>(path: &GString, parse: impl FnOnce(&[u8]) -> Result<T, MapGeoError>) -> Option<T> {
    let bytes = read_bytes(path)?;
    match parse(bytes.as_slice()) {
        Ok(value) => Some(value),
        Err(error) => {
            godot_error!("MapGeoLoader: {path}: {error}");
            None
        }
    }
}

fn packed(line: &Line) -> PackedVector2Array {
    line.iter().map(|p| Vector2::new(p[0], p[1])).collect()
}

fn vec2(p: [f32; 2]) -> Vector2 {
    Vector2::new(p[0], p[1])
}

fn packed_lines(lines: &[Line]) -> Array<PackedVector2Array> {
    lines.iter().map(packed).collect()
}

fn provinces_of(path: &GString) -> Option<Arc<Vec<ProvinceFeature>>> {
    let key = path.to_string();
    if let Some(cached) = province_cache().lock().ok()?.get(&key) {
        return Some(Arc::clone(cached));
    }
    let parsed = Arc::new(load(path, map_geo::parse_provinces)?);
    province_cache()
        .lock()
        .ok()?
        .insert(key, Arc::clone(&parsed));
    Some(parsed)
}

/// Native parser of the map vector files.
#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct MapGeoLoader {
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for MapGeoLoader {
    fn init(base: Base<RefCounted>) -> Self {
        MapGeoLoader { base }
    }
}

#[godot_api]
impl MapGeoLoader {
    /// Provinces of `provinces.geojson` as `MapData` entries (`index`, `id`, `name`, `owner`,
    /// `terrain`, `capital_name`, `centroid`, `capital_px`, `neighbors`, `area_px`, `rings`);
    /// empty if the file is missing or invalid. Parsed once per path.
    #[func]
    fn provinces(&self, path: GString) -> Array<VarDictionary> {
        let Some(provinces) = provinces_of(&path) else {
            return Array::new();
        };
        provinces
            .iter()
            .map(|p| {
                let neighbors: Array<GString> = p
                    .neighbors
                    .iter()
                    .map(|n| GString::from(n.as_str()))
                    .collect();
                vdict! {
                    "index" => p.index,
                    "id" => p.id.as_str(),
                    "name" => p.name.as_str(),
                    "owner" => p.owner.as_str(),
                    "terrain" => p.terrain.as_str(),
                    "capital_name" => p.capital_name.as_str(),
                    "centroid" => vec2(p.centroid),
                    "capital_px" => vec2(p.capital_px),
                    "neighbors" => &neighbors,
                    "area_px" => p.area_px,
                    "rings" => &packed_lines(&p.rings),
                }
            })
            .collect()
    }

    /// Provinces for the faction picker (`id`, `owner`, `polygons` of 3+ points, and `seat` when
    /// the file gives `capital_px` or `centroid`). Shares the cache of `provinces`.
    #[func]
    fn province_polygons(&self, path: GString) -> Array<VarDictionary> {
        let Some(provinces) = provinces_of(&path) else {
            return Array::new();
        };
        provinces
            .iter()
            .map(|p| {
                let polygons: Array<PackedVector2Array> = p
                    .rings
                    .iter()
                    .filter(|ring| ring.len() >= 3)
                    .map(packed)
                    .collect();
                let mut entry = vdict! {
                    "id" => p.id.as_str(),
                    "owner" => p.owner.as_str(),
                    "polygons" => &polygons,
                };
                if p.has_seat {
                    entry.set("seat", vec2(p.capital_px));
                }
                entry
            })
            .collect()
    }

    /// Rivers of `rivers.geojson` as `{name, importance, points}`.
    #[func]
    fn rivers(&self, path: GString) -> Array<VarDictionary> {
        load(&path, map_geo::parse_rivers)
            .unwrap_or_default()
            .iter()
            .map(|r| {
                vdict! {
                    "name" => r.name.as_str(),
                    "importance" => r.importance,
                    "points" => &packed(&r.points),
                }
            })
            .collect()
    }

    /// Coast lines of `coastline.geojson`.
    #[func]
    fn coastline(&self, path: GString) -> Array<PackedVector2Array> {
        packed_lines(&load(&path, map_geo::parse_coastline).unwrap_or_default())
    }

    /// Roads of `roads.geojson` as `{type, main, points}`.
    #[func]
    fn roads(&self, path: GString) -> Array<VarDictionary> {
        load(&path, map_geo::parse_roads)
            .unwrap_or_default()
            .iter()
            .map(|r| {
                vdict! {
                    "type" => r.kind.as_str(),
                    "main" => r.kind == "main",
                    "points" => &packed(&r.points),
                }
            })
            .collect()
    }

    /// `rivers_render.json` as `{bank_px, zones: [{id, name, px, radius_px, boundary_bridges}],
    /// rivers: [{name, importance, points, widths}]}`; empty if missing or invalid.
    #[func]
    fn rendered_rivers(&self, path: GString) -> VarDictionary {
        let Some(rendered) = load(&path, map_geo::parse_rendered_rivers) else {
            return VarDictionary::new();
        };
        let zones: Array<VarDictionary> = rendered
            .zones
            .iter()
            .map(|z| {
                vdict! {
                    "id" => z.id.as_str(),
                    "name" => z.name.as_str(),
                    "px" => vec2(z.px),
                    "radius_px" => z.radius_px,
                    "boundary_bridges" => z.boundary_bridges,
                }
            })
            .collect();
        let rivers: Array<VarDictionary> = rendered
            .rivers
            .iter()
            .map(|r| {
                let widths: PackedFloat32Array = r.widths.iter().copied().collect();
                vdict! {
                    "name" => r.name.as_str(),
                    "importance" => r.importance,
                    "points" => &packed(&r.points),
                    "widths" => &widths,
                }
            })
            .collect();
        vdict! {"bank_px" => rendered.bank_px, "zones" => &zones, "rivers" => &rivers}
    }

    /// Forgets the cached provinces (tests that rewrite the file).
    #[func]
    fn clear_cache(&self) {
        if let Ok(mut cache) = province_cache().lock() {
            cache.clear();
        }
    }
}
