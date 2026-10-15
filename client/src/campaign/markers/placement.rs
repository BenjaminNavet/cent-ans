//! Placement maths of the stylised maquettes (ADR 0158): deterministic variant and yaw, collision
//! reduction of neighbours, world position convention. Port of `TownMaquetteData` statics.

use std::collections::HashMap;

use bevy::prelude::*;

/// World convention shared with the Godot map: 1 world unit = 1 map pixel (719 m on the composed
/// map), `x = px.x`, `z = px.y`, `y` up. The projection lon/lat -> px is already applied in
/// `data/map/settlements_px.json`, so no projection is needed for places.
pub fn px_to_world(px: Vec2, height: f32) -> Vec3 {
    Vec3::new(px.x, height, px.y)
}

/// Ground height under a map position (world units). Flat `0` until the terrain lot provides a
/// sampler; replace through the `HeightSampler` resource.
#[derive(Resource)]
pub struct HeightSampler(pub Box<dyn Fn(Vec2) -> f32 + Send + Sync>);

impl Default for HeightSampler {
    fn default() -> Self {
        Self(Box::new(|_| 0.0))
    }
}

impl HeightSampler {
    pub fn at(&self, px: Vec2) -> f32 {
        (self.0)(px)
    }
}

/// Stable 32-bit FNV-1a hash (Godot's `String.hash` is not reproducible outside the engine).
pub fn stable_hash(text: &str) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for b in text.bytes() {
        h ^= u32::from(b);
        h = h.wrapping_mul(0x0100_0193);
    }
    h
}

/// Variant letter of a place (deterministic by id).
pub fn variant_of<'a>(id: &str, variants: &'a [String]) -> Option<&'a str> {
    if variants.is_empty() {
        return None;
    }
    Some(&variants[stable_hash(id) as usize % variants.len()])
}

/// Yaw (radians) of a place, drawn from its id.
pub fn yaw_of(id: &str) -> f32 {
    (stable_hash(&format!("{id}|yaw")) % 360) as f32 * std::f32::consts::PI / 180.0
}

/// Model name `<kind>_<family>_<variant>` (no file test, see `model_candidates`).
pub fn model_name_raw(kind: &str, family: &str, variant: &str, default_family: &str) -> String {
    let family = if family.is_empty() {
        default_family
    } else {
        family
    };
    format!("{kind}_{family}_{variant}")
}

/// Candidate model names in fallback order: the family's, the default family's, the old western
/// kit `<kind>_<variant>`.
pub fn model_candidates(
    kind: &str,
    family: &str,
    variant: &str,
    default_family: &str,
) -> [String; 3] {
    [
        model_name_raw(kind, family, variant, default_family),
        model_name_raw(kind, default_family, variant, default_family),
        format!("{kind}_{variant}"),
    ]
}

/// Collision reduction of places too close. `centers` and `radii` are given by decreasing
/// importance; `fixed[i]` = never reduced (landmark city). A place whose disc touches (within
/// `margin`) the disc of a more important one is shrunk until it no longer does, down to
/// `min_scale`. Returns the factor per place (1 = base size). Deterministic.
pub fn solve_collisions(
    centers: &[Vec2],
    radii: &[f32],
    fixed: &[bool],
    min_scale: f32,
    margin: f32,
) -> Vec<f32> {
    let count = centers.len();
    let mut scales = vec![1.0_f32; count];
    let largest = radii.iter().copied().fold(0.0_f32, f32::max);
    let cell = (largest * 2.0 + margin).max(1.0);
    let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    for i in 0..count {
        let c = centers[i];
        let (cx, cy) = ((c.x / cell).floor() as i32, (c.y / cell).floor() as i32);
        if radii[i] > 0.0 && !fixed.get(i).copied().unwrap_or(false) {
            let mut allowed = radii[i];
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if let Some(list) = grid.get(&(cx + dx, cy + dy)) {
                        for &j in list {
                            allowed =
                                allowed.min(c.distance(centers[j]) - radii[j] * scales[j] - margin);
                        }
                    }
                }
            }
            scales[i] = (allowed / radii[i]).clamp(min_scale, 1.0);
        }
        grid.entry((cx, cy)).or_default().push(i);
    }
    scales
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_stable_and_yaw_in_range() {
        assert_eq!(stable_hash("set_paris"), stable_hash("set_paris"));
        assert_ne!(stable_hash("set_paris"), stable_hash("set_rouen"));
        let y = yaw_of("set_paris");
        assert!((0.0..std::f32::consts::TAU).contains(&y));
        let v = vec!["a".to_string(), "b".to_string()];
        assert!(matches!(variant_of("x", &v), Some("a" | "b")));
        assert_eq!(variant_of("x", &[]), None);
    }

    #[test]
    fn world_convention() {
        assert_eq!(
            px_to_world(Vec2::new(10.0, 20.0), 3.0),
            Vec3::new(10.0, 3.0, 20.0)
        );
        assert_eq!(HeightSampler::default().at(Vec2::ONE), 0.0);
    }

    #[test]
    fn model_fallback_order() {
        let c = model_candidates("town", "byz", "a", "west");
        assert_eq!(c, ["town_byz_a", "town_west_a", "town_a"]);
        assert_eq!(model_name_raw("town", "", "b", "west"), "town_west_b");
    }

    #[test]
    fn collisions_shrink_the_less_important_one() {
        let centers = [
            Vec2::new(0.0, 0.0),
            Vec2::new(5.0, 0.0),
            Vec2::new(100.0, 0.0),
        ];
        let radii = [4.0, 4.0, 4.0];
        let s = solve_collisions(&centers, &radii, &[false; 3], 0.5, 0.3);
        assert_eq!(s[0], 1.0);
        // allowed = 5 - 4 - 0.3 = 0.7 -> 0.175 -> clamped to min 0.5
        assert_eq!(s[1], 0.5);
        assert_eq!(s[2], 1.0);
        // Fixed places are never reduced.
        let s = solve_collisions(&centers, &radii, &[false, true, false], 0.5, 0.3);
        assert_eq!(s[1], 1.0);
        // A far neighbour with room: partial reduction.
        let centers = [Vec2::new(0.0, 0.0), Vec2::new(7.0, 0.0)];
        let s = solve_collisions(&centers, &[4.0, 4.0], &[false; 2], 0.1, 0.0);
        assert!((s[1] - 0.75).abs() < 1e-6);
    }
}
