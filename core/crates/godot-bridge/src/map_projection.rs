//! Map projection for GDScript: the engine one (`data_model::MapMeta`), exposed so that the game
//! layer has no projection of its own (ADR 0355, lot C5). Reads `<map_dir>/map.json` and, for the
//! composed CRS, its warp grids; one parsed projection per directory. Rendering support only.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use data_model::MapMeta;
use godot::prelude::*;

type Cache = Mutex<HashMap<String, Arc<MapMeta>>>;

fn cache() -> &'static Cache {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Value returned for a point outside the map (no projection): both components NaN.
const OUTSIDE: Vector2 = Vector2::new(f32::NAN, f32::NAN);

/// Native projection `lon/lat <-> map pixel` of one map directory.
#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct MapProjectionNative {
    meta: Option<Arc<MapMeta>>,
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for MapProjectionNative {
    fn init(base: Base<RefCounted>) -> Self {
        MapProjectionNative { meta: None, base }
    }
}

#[godot_api]
impl MapProjectionNative {
    /// Loads (once per directory) `map_dir/map.json` and its warp; `false` with an error log if
    /// unreadable.
    #[func]
    fn open(&mut self, map_dir: GString) -> bool {
        let key = map_dir.to_string();
        if let Some(found) = cache().lock().ok().and_then(|c| c.get(&key).cloned()) {
            self.meta = Some(found);
            return true;
        }
        match MapMeta::load(Path::new(&key)) {
            Ok(meta) => {
                let meta = Arc::new(meta);
                if let Ok(mut c) = cache().lock() {
                    c.insert(key, Arc::clone(&meta));
                }
                self.meta = Some(meta);
                true
            }
            Err(error) => {
                godot_error!("MapProjection: {key}: {error}");
                false
            }
        }
    }

    /// CRS of the opened map (`"EPSG:3035"` or `"composed"`), empty before `open`.
    #[func]
    fn crs(&self) -> GString {
        self.meta
            .as_ref()
            .map_or_else(GString::new, |m| GString::from(m.crs.as_str()))
    }

    /// Whether the opened map is the composed one.
    #[func]
    fn is_composed(&self) -> bool {
        self.meta.as_ref().is_some_and(|m| m.is_composed())
    }

    /// Map pixel of `Vector2(lon, lat)` (degrees); NaN components outside the map.
    #[func]
    fn lonlat_to_px(&self, lonlat: Vector2) -> Vector2 {
        self.meta
            .as_ref()
            .and_then(|m| m.lonlat_to_px(f64::from(lonlat.x), f64::from(lonlat.y)))
            .map_or(OUTSIDE, |p| Vector2::new(p[0] as f32, p[1] as f32))
    }

    /// Real `Vector2(lon, lat)` of a map pixel; NaN components outside the map.
    #[func]
    fn px_to_lonlat(&self, px: Vector2) -> Vector2 {
        self.meta
            .as_ref()
            .and_then(|m| m.px_to_lonlat(f64::from(px.x), f64::from(px.y)))
            .map_or(OUTSIDE, |(lon, lat)| Vector2::new(lon as f32, lat as f32))
    }

    /// Map pixel of EPSG:3035 metres (`Vector2(E, N)` is too coarse in f32: two doubles).
    #[func]
    fn projected_to_px(&self, easting: f64, northing: f64) -> Vector2 {
        self.meta
            .as_ref()
            .and_then(|m| m.projected_to_px(easting, northing))
            .map_or(OUTSIDE, |p| Vector2::new(p[0] as f32, p[1] as f32))
    }
}
