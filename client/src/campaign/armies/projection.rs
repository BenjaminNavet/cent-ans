//! Map pixel <-> world conventions of the Godot map: world `(x, ground, z) = (px.x, h, px.y)`,
//! one world unit per map pixel (`ArmyMarkers`: `Vector3(centroid.x, _ground(c), centroid.y)`).

use std::sync::Arc;

use bevy::prelude::*;

/// Ground height hook (`MapData.surface_world_at` in Godot). Flat (y = 0) until the terrain
/// lot provides one: replace the resource's content with `HeightSampler::new(...)`.
#[derive(Resource, Clone)]
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

impl Default for HeightSampler {
    fn default() -> Self {
        Self::flat()
    }
}

/// World position on the ground of map pixel `p`.
pub fn px_to_world(p: Vec2, ground: &HeightSampler) -> Vec3 {
    Vec3::new(p.x, ground.at(p.x, p.y), p.y)
}

/// Map pixel of a world position.
pub fn world_to_px(w: Vec3) -> Vec2 {
    Vec2::new(w.x, w.z)
}

/// Intersection of a ray with the ground (fixed-point on the height sampler).
pub fn ray_ground(ray: Ray3d, ground: &HeightSampler) -> Option<Vec2> {
    let mut height = 0.0;
    let mut hit = None;
    for _ in 0..4 {
        let plane = InfinitePlane3d::new(Vec3::Y);
        let distance = ray.intersect_plane(Vec3::new(0.0, height, 0.0), plane)?;
        let point = ray.get_point(distance);
        hit = Some(world_to_px(point));
        height = ground.at(point.x, point.z);
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
