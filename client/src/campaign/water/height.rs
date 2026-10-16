//! Ground height hook. Terrain is built by another lot; until it plugs a real sampler, ribbons
//! (rivers, roads, lakes) sit slightly above a flat ground and the sea is at `y = 0`.
//!
//! World convention (same as the Godot map): world `x` = map pixel x, world `z` = map pixel y,
//! `y` = altitude in metres × [`HEIGHT_SCALE`] (`MapData.HEIGHT_SCALE`).

use std::sync::Arc;

use bevy::prelude::*;

/// World units per metre of altitude (`MapData.HEIGHT_SCALE` in Godot).
pub const HEIGHT_SCALE: f32 = 0.006;

/// Source of ground heights at map pixel positions.
pub trait HeightSampler: Send + Sync + 'static {
    /// Altitude in metres at map pixel `(x, y)`.
    fn height_m(&self, x: f32, y: f32) -> f32;

    /// Altitude (m) at which a draped ribbon is seated so that it never ends up under the
    /// displayed ground (default: [`Self::height_m`]).
    fn seat_height_m(&self, x: f32, y: f32) -> f32 {
        self.height_m(x, y)
    }

    /// World `y` of the ground surface at map pixel `(x, y)` (never below the sea).
    fn surface_world(&self, x: f32, y: f32) -> f32 {
        self.height_m(x, y).max(0.0) * HEIGHT_SCALE
    }
}

/// Flat ground at a constant altitude (default until the terrain lot provides a sampler).
#[derive(Debug, Clone, Copy, Default)]
pub struct FlatGround {
    pub altitude_m: f32,
}

impl HeightSampler for FlatGround {
    fn height_m(&self, _x: f32, _y: f32) -> f32 {
        self.altitude_m
    }
}

/// Resource holding the sampler used when water meshes are built. The terrain plugin may replace
/// it before `OnEnter(Campaign)` (or emit [`crate::campaign::water::RebuildWater`]).
#[derive(Resource, Clone)]
pub struct GroundHeight(pub Arc<dyn HeightSampler>);

impl Default for GroundHeight {
    fn default() -> Self {
        Self(Arc::new(FlatGround::default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_ground_scales_and_clamps() {
        assert_eq!(
            FlatGround { altitude_m: 100.0 }.surface_world(3.0, 4.0),
            0.6
        );
        assert_eq!(
            FlatGround { altitude_m: -50.0 }.surface_world(0.0, 0.0),
            0.0
        );
    }
}
