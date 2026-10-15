//! Map projection `lon/lat <-> map pixel`: thin wrapper over the engine projection
//! (`data_model::MapMeta`, the same one `godot-bridge/src/map_projection.rs` exposes), so the
//! client never depends on `godot-bridge`.
//!
//! # World convention (identical to the Godot map, `game/scripts/map/map_data.gd`)
//! - map pixel `(x, y)`: origin at the north-west corner, `x` east, `y` south;
//! - Bevy world: `X = x`, `Z = y`, `Y` = up (altitude). One world unit = one map pixel
//!   = `meters_per_px` metres (719 m on the composed map);
//! - overlays of this module are flat at `Y = overlay height + small offset`.

use std::path::Path;

use bevy::prelude::*;
use data_model::MapMeta;

/// Engine projection of one map directory.
#[derive(Resource, Clone)]
pub struct MapProjection {
    meta: MapMeta,
}

impl MapProjection {
    /// Reads `<map_dir>/map.json` (and the warp grids of the composed CRS).
    pub fn load(map_dir: &Path) -> Result<Self, String> {
        Ok(Self {
            meta: MapMeta::load(map_dir)?,
        })
    }

    /// `[width, height]` of the rasters, in pixels.
    pub fn size_px(&self) -> [u32; 2] {
        self.meta.size_px
    }

    pub fn meters_per_px(&self) -> f64 {
        self.meta.meters_per_px
    }

    /// Map pixel of `(lon, lat)` in degrees; `None` outside the map.
    pub fn lonlat_to_px(&self, lon: f64, lat: f64) -> Option<Vec2> {
        self.meta
            .lonlat_to_px(lon, lat)
            .map(|p| Vec2::new(p[0] as f32, p[1] as f32))
    }

    /// `(lon, lat)` of a map pixel; `None` outside the map.
    pub fn px_to_lonlat(&self, px: Vec2) -> Option<(f64, f64)> {
        self.meta.px_to_lonlat(f64::from(px.x), f64::from(px.y))
    }

    /// World position (flat, at `height`) of a map pixel.
    pub fn px_to_world(px: Vec2, height: f32) -> Vec3 {
        Vec3::new(px.x, height, px.y)
    }

    /// Map pixel under a world position (inverse of [`Self::px_to_world`]).
    pub fn world_to_px(world: Vec3) -> Vec2 {
        Vec2::new(world.x, world.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn projection() -> MapProjection {
        let root = crate::paths::find_root_from(Path::new(env!("CARGO_MANIFEST_DIR")))
            .expect("repository root");
        MapProjection::load(&root.join("data/map")).expect("map.json")
    }

    #[test]
    fn size_and_scale_come_from_map_json() {
        let p = projection();
        assert_eq!(p.size_px(), [7080, 6104]);
        assert!((p.meters_per_px() - 719.0).abs() < 1.0);
    }

    #[test]
    fn paris_round_trips_through_the_warp() {
        let p = projection();
        let px = p.lonlat_to_px(2.35, 48.85).expect("Paris is on the map");
        assert!(px.x > 0.0 && px.x < 7080.0 && px.y > 0.0 && px.y < 6104.0);
        let (lon, lat) = p.px_to_lonlat(px).expect("inverse");
        assert!((lon - 2.35).abs() < 0.02 && (lat - 48.85).abs() < 0.02, "{lon} {lat}");
    }

    #[test]
    fn world_axes_follow_godot() {
        let world = MapProjection::px_to_world(Vec2::new(10.0, 20.0), 0.5);
        assert_eq!(world, Vec3::new(10.0, 0.5, 20.0));
        assert_eq!(MapProjection::world_to_px(world), Vec2::new(10.0, 20.0));
    }
}
