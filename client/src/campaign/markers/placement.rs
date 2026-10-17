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

/// Share of the footprint (a disc of `radius` map units around `centre`) that lies on water.
pub fn water_share(centre: Vec2, radius: f32, is_water: &dyn Fn(Vec2) -> bool) -> f32 {
    let mut total = 0.0_f32;
    let mut wet = 0.0_f32;
    let mut sample = |p: Vec2, weight: f32| {
        total += weight;
        if is_water(p) {
            wet += weight;
        }
    };
    sample(centre, 1.0);
    for (ring, count) in [(0.4_f32, 6_u32), (0.8, 12)] {
        for k in 0..count {
            let angle = std::f32::consts::TAU * (k as f32 + 0.5) / count as f32;
            sample(centre + Vec2::from_angle(angle) * radius * ring, 1.0);
        }
    }
    wet / total
}

/// Moves a coastal place's maquette inland so its footprint does not hang over the sea
/// (mapgeo: the stylised block of Constantinople stuck out over the Bosphorus). The smallest
/// shift (at most `MAX_SHORE_SHIFT` x `radius`) that lowers the water share is chosen; a
/// footprint already dry (<= `DRY_SHARE`) stays where the simulation puts the place.
pub fn shore_fit(centre: Vec2, radius: f32, is_water: &dyn Fn(Vec2) -> bool) -> Vec2 {
    const DRY_SHARE: f32 = 0.04;
    const MAX_SHORE_SHIFT: f32 = 1.25;
    let here = water_share(centre, radius, is_water);
    if here <= DRY_SHARE || radius <= 0.0 {
        return centre;
    }
    let mut best = (here, 0.0_f32, centre);
    for step in 1..=5 {
        let distance = radius * MAX_SHORE_SHIFT * step as f32 / 5.0;
        for k in 0..24 {
            let p = centre + Vec2::from_angle(std::f32::consts::TAU * k as f32 / 24.0) * distance;
            let share = water_share(p, radius, is_water);
            // A clearly drier spot wins; among equally dry ones the nearest.
            if share < best.0 - 0.02 {
                best = (share, distance, p);
            }
        }
    }
    best.2
}

/// Stable 32-bit hash of a place id: Godot 4's `String.hash()` (djb2 over the Unicode code
/// points, `h = h * 33 + c` from 5381, wrapping), so variants and yaws match Godot.
pub fn stable_hash(text: &str) -> u32 {
    text.chars().fold(5381_u32, |h, c| {
        h.wrapping_shl(5).wrapping_add(h).wrapping_add(c as u32)
    })
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
    fn variant_and_yaw_use_godot_hash() {
        // `"set_paris".hash()` = 1125892655 in Godot 4: odd -> variant `b`; 1125892655 % 2 = 1.
        let v = vec!["a".to_string(), "b".to_string()];
        assert_eq!(variant_of("set_paris", &v), Some("b"));
        // `"set_paris|yaw".hash() % 360` in degrees.
        let deg = (stable_hash("set_paris|yaw") % 360) as f32;
        assert!((yaw_of("set_paris") - deg.to_radians()).abs() < 1e-5);
    }

    #[test]
    fn world_convention() {
        assert_eq!(
            px_to_world(Vec2::new(10.0, 20.0), 3.0),
            Vec3::new(10.0, 3.0, 20.0)
        );
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

    #[test]
    fn shore_fit_pulls_a_coastal_footprint_inland() {
        // Sea for x > 0: a place on the shore moves west, a dry one stays, a drowned one too.
        let sea = |p: Vec2| p.x > 0.0;
        let on_shore = shore_fit(Vec2::new(0.5, 3.0), 4.0, &sea);
        assert!(on_shore.x < -1.5, "{on_shore}");
        assert!(water_share(on_shore, 4.0, &sea) < 0.05);
        let inland = Vec2::new(-20.0, 0.0);
        assert_eq!(shore_fit(inland, 4.0, &sea), inland);
        let open_sea = Vec2::new(500.0, 0.0);
        assert_eq!(shore_fit(open_sea, 4.0, &sea), open_sea);
    }
}
