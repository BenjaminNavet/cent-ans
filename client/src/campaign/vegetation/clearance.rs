//! Clearings of the generalised trees (port of `tree_clearance.gd`).
//!
//! A grown tree (crown radius `style::crown_radius()`) must not overhang a lake (polygon of
//! `lakes.json` widened by the crown), the sea (`coast_dist.png`) nor sit on a main road. The
//! scatter of the `vegetation` crate only keeps the *foot* of the trees off the water, so the
//! tile worker filters its output through [`Clearance::filter_tile`]. Purely visual.

use std::path::Path;

use bevy::math::Vec2;
use serde_json::Value;

use super::data::Plane;
use super::tiles::TreeInst;

/// Half width (map px) cleared on each side of a main road (`generalised_road_clearance`).
pub const ROAD_CLEARANCE: f32 = 0.35;
/// Levels per map px of `coast_dist.png` (`coast_dist_scale` of the terrain shader).
const COAST_DIST_SCALE: f32 = 2.0;

/// A lake contour (map px) and its box widened by the crown radius.
#[derive(Clone, Debug)]
pub struct LakePolygon {
    pub points: Vec<Vec2>,
    pub low: Vec2,
    pub high: Vec2,
}

/// Everything the tile filter reads (shared between workers).
#[derive(Default)]
pub struct Clearance {
    pub lakes: Vec<LakePolygon>,
    /// Segments of the main roads: `(a, b)`.
    pub road_segments: Vec<(Vec2, Vec2)>,
    pub coast: Plane,
    pub map_size: (f64, f64),
    pub crown_radius: f32,
}

/// Distance from `p` to the segment `a`-`b`, squared.
fn segment_distance_sq(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_squared();
    let t = if len_sq > 1e-12 {
        ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a + ab * t - p).length_squared()
}

fn inside_polygon(p: Vec2, polygon: &[Vec2]) -> bool {
    let mut inside = false;
    let mut j = polygon.len() - 1;
    for i in 0..polygon.len() {
        let (a, b) = (polygon[i], polygon[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

impl LakePolygon {
    pub fn new(points: Vec<Vec2>, crown: f32) -> Option<LakePolygon> {
        if points.len() < 3 {
            return None;
        }
        let low = points.iter().fold(Vec2::splat(f32::MAX), |m, p| m.min(*p)) - Vec2::splat(crown);
        let high = points.iter().fold(Vec2::splat(f32::MIN), |m, p| m.max(*p)) + Vec2::splat(crown);
        Some(LakePolygon { points, low, high })
    }

    fn touches(&self, low: Vec2, high: Vec2) -> bool {
        self.low.x <= high.x && self.high.x >= low.x && self.low.y <= high.y && self.high.y >= low.y
    }

    /// True when `p` is inside the contour or closer than `crown` to its edge (contour widened).
    pub fn blocks(&self, p: Vec2, crown: f32) -> bool {
        if p.x < self.low.x || p.x > self.high.x || p.y < self.low.y || p.y > self.high.y {
            return false;
        }
        if inside_polygon(p, &self.points) {
            return true;
        }
        let crown_sq = crown * crown;
        let n = self.points.len();
        (0..n).any(|i| segment_distance_sq(p, self.points[i], self.points[(i + 1) % n]) < crown_sq)
    }
}

impl Clearance {
    /// Reads `lakes.json` (`polygon_px`) and the main roads of `roads.geojson`.
    pub fn load(
        map_dir: &Path,
        map_size: (f64, f64),
        crown_radius: f32,
        coast: Plane,
    ) -> Clearance {
        let mut out = Clearance {
            coast,
            map_size,
            crown_radius,
            ..Default::default()
        };
        if let Ok(text) = std::fs::read_to_string(map_dir.join("lakes.json"))
            && let Ok(value) = serde_json::from_str::<Value>(&text)
            && let Some(lakes) = value["lakes"].as_array()
        {
            for lake in lakes {
                let points: Vec<Vec2> = lake["polygon_px"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|p| {
                                Some(Vec2::new(p[0].as_f64()? as f32, p[1].as_f64()? as f32))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                out.lakes.extend(LakePolygon::new(points, crown_radius));
            }
        }
        if let Ok(bytes) = std::fs::read(map_dir.join("roads.geojson"))
            && let Ok(roads) = data_model::map_geo::parse_roads(&bytes)
        {
            for road in roads.iter().filter(|r| r.kind == "main") {
                for pair in road.points.windows(2) {
                    out.road_segments.push((
                        Vec2::new(pair[0][0], pair[0][1]),
                        Vec2::new(pair[1][0], pair[1][1]),
                    ));
                }
            }
        }
        out
    }

    /// Signed distance to the coast (map px, negative at sea), huge without the raster.
    pub fn coast_distance(&self, x: f32, y: f32) -> f32 {
        if self.coast.is_empty() {
            return 1e6;
        }
        let level = self.coast.texel(self.map_size, x as f64, y as f64, 0) * 255.0;
        (level - 128.0) / COAST_DIST_SCALE
    }

    /// Removes the trees of the tile `[origin, origin + size]` whose crown touches a lake or the
    /// sea or whose foot is on a main road. Returns the number removed.
    pub fn filter_tile(&self, trees: &mut Vec<TreeInst>, origin: (f64, f64), size: f64) -> usize {
        let margin = self.crown_radius.max(ROAD_CLEARANCE) + 1.0;
        let low = Vec2::new(origin.0 as f32, origin.1 as f32) - Vec2::splat(margin);
        let high = low + Vec2::splat(size as f32 + 2.0 * margin);
        let lakes: Vec<&LakePolygon> = self.lakes.iter().filter(|l| l.touches(low, high)).collect();
        let roads: Vec<&(Vec2, Vec2)> = self
            .road_segments
            .iter()
            .filter(|(a, b)| {
                a.min(*b).x <= high.x
                    && a.max(*b).x >= low.x
                    && a.min(*b).y <= high.y
                    && a.max(*b).y >= low.y
            })
            .collect();
        let before = trees.len();
        let road_sq = ROAD_CLEARANCE * ROAD_CLEARANCE;
        trees.retain(|tree| {
            let p = Vec2::new(tree.x, tree.z);
            // Crown radius of this tree, nominal for the nominal height.
            let radius = self.crown_radius * (tree.height / super::data::style::TREE_HEIGHT as f32);
            if self.coast_distance(tree.x, tree.z) < radius {
                return false;
            }
            if lakes.iter().any(|lake| lake.blocks(p, radius)) {
                return false;
            }
            !roads
                .iter()
                .any(|(a, b)| segment_distance_sq(p, *a, *b) < road_sq)
        });
        before - trees.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(x: f32, z: f32) -> TreeInst {
        TreeInst {
            x,
            z,
            y: 0.0,
            yaw_deg: 0.0,
            height: 1.5,
            row: 0,
            seed: 0.5,
        }
    }

    fn square_lake() -> LakePolygon {
        let points = vec![
            Vec2::new(10.0, 10.0),
            Vec2::new(20.0, 10.0),
            Vec2::new(20.0, 20.0),
            Vec2::new(10.0, 20.0),
        ];
        LakePolygon::new(points, 0.9).unwrap()
    }

    #[test]
    fn lake_blocks_inside_and_within_the_crown_of_the_shore() {
        let lake = square_lake();
        assert!(lake.blocks(Vec2::new(15.0, 15.0), 0.9));
        assert!(lake.blocks(Vec2::new(9.5, 15.0), 0.9));
        assert!(!lake.blocks(Vec2::new(8.5, 15.0), 0.9));
        assert!(!lake.blocks(Vec2::new(40.0, 15.0), 0.9));
    }

    #[test]
    fn filter_removes_lake_and_road_trees_only() {
        let clearance = Clearance {
            lakes: vec![square_lake()],
            road_segments: vec![(Vec2::new(30.0, 0.0), Vec2::new(30.0, 40.0))],
            crown_radius: 0.9,
            ..Default::default()
        };
        let mut trees = vec![
            tree(15.0, 15.0),
            tree(30.2, 5.0),
            tree(31.0, 5.0),
            tree(25.0, 25.0),
        ];
        let removed = clearance.filter_tile(&mut trees, (0.0, 0.0), 256.0);
        assert_eq!(removed, 2);
        assert_eq!(trees.len(), 2);
    }

    #[test]
    fn coast_distance_is_signed_and_far_without_raster() {
        let mut clearance = Clearance::default();
        assert!(clearance.coast_distance(1.0, 1.0) > 1e5);
        clearance.map_size = (2.0, 1.0);
        clearance.coast = Plane {
            w: 2,
            h: 1,
            ch: 1,
            data: vec![120, 160],
        };
        assert!(clearance.coast_distance(0.5, 0.5) < 0.0);
        assert!(clearance.coast_distance(1.5, 0.5) > 10.0);
    }

    #[test]
    fn real_data_loads() {
        let root = crate::paths::find_root().expect("repository root");
        let map_dir = root.join("data/map");
        let clearance = Clearance::load(&map_dir, (7080.0, 6104.0), 0.9, Plane::empty());
        assert!(clearance.lakes.len() > 50);
        // `roads.geojson` currently has no `main` road (all `computed`): as in Godot, no road clearing.
    }
}
