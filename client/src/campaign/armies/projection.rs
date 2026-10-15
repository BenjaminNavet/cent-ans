//! Map pixel <-> world conventions of the Godot map: world `(x, ground, z) = (px.x, h, px.y)`,
//! one world unit per map pixel (`ArmyMarkers`: `Vector3(centroid.x, _ground(c), centroid.y)`).

use std::sync::Arc;

use bevy::prelude::*;

use crate::campaign::terrain::surface::GroundHeight;

/// Displayed ground height at a map point (`MapData.surface_world_at` in Godot). The game uses
/// [`GroundHeight`] (terrain lot, includes the dynamic relief); tests use [`HeightSampler`].
pub trait Ground {
    fn height(&self, x: f32, z: f32) -> f32;
}

impl Ground for GroundHeight<'_> {
    fn height(&self, x: f32, z: f32) -> f32 {
        self.at(x, z).unwrap_or(0.0)
    }
}

/// Closure-backed ground (tests, flat fallback).
#[derive(Clone)]
pub struct HeightSampler(Arc<dyn Fn(f32, f32) -> f32 + Send + Sync>);

impl HeightSampler {
    pub fn new(f: impl Fn(f32, f32) -> f32 + Send + Sync + 'static) -> Self {
        Self(Arc::new(f))
    }

    pub fn flat() -> Self {
        Self::new(|_, _| 0.0)
    }

    /// Ground height at map pixel `(x, y)`.
    pub fn at(&self, x: f32, y: f32) -> f32 {
        (self.0)(x, y)
    }
}

impl Ground for HeightSampler {
    fn height(&self, x: f32, z: f32) -> f32 {
        self.at(x, z)
    }
}

impl Default for HeightSampler {
    fn default() -> Self {
        Self::flat()
    }
}

/// World position on the ground of map pixel `p`.
pub fn px_to_world(p: Vec2, ground: &impl Ground) -> Vec3 {
    Vec3::new(p.x, ground.height(p.x, p.y), p.y)
}

/// Map pixel of a world position.
pub fn world_to_px(w: Vec3) -> Vec2 {
    Vec2::new(w.x, w.z)
}

/// Intersection of a ray with the ground (fixed-point on the height sampler).
pub fn ray_ground(ray: Ray3d, ground: &impl Ground) -> Option<Vec2> {
    let mut height = 0.0;
    let mut hit = None;
    for _ in 0..4 {
        let plane = InfinitePlane3d::new(Vec3::Y);
        let distance = ray.intersect_plane(Vec3::new(0.0, height, 0.0), plane)?;
        let point = ray.get_point(distance);
        hit = Some(world_to_px(point));
        height = ground.height(point.x, point.z);
    }
    hit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let g = HeightSampler::flat();
        let w = px_to_world(Vec2::new(12.0, 34.0), &g);
        assert_eq!(w, Vec3::new(12.0, 0.0, 34.0));
        assert_eq!(world_to_px(w), Vec2::new(12.0, 34.0));
    }

    #[test]
    fn ray_hits_flat_ground() {
        let ray = Ray3d::new(
            Vec3::new(5.0, 10.0, 5.0),
            Dir3::new(Vec3::new(0.0, -1.0, 1.0)).unwrap(),
        );
        let hit = ray_ground(ray, &HeightSampler::flat()).unwrap();
        assert!((hit - Vec2::new(5.0, 15.0)).length() < 1e-4);
    }
}
