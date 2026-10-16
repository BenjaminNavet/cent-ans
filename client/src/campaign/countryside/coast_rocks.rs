//! Coast rocks of the outcrop catalogue (`coast` entries of `data/art/rock_outcrops.yaml`: granite
//! chaos of Brittany, chalk cliffs of the Channel…), port of `RockOutcrops._coast_term` and of
//! the parts of `coast_look.gd` they use (`region_at`, `kind_at`, `shore_probe`).
//!
//! Data: `data/map/coast_dist.png` (signed distance to the shore, L8, `(v - 128) / 2` px, > 0 on
//! land, covering the whole map) and `data/map/coast_types.json` (geology regions as polygons in
//! map px, `default`, `cliff` rule: altitude read `probe_px` inland from the nearest shore pixel).

use std::path::Path;

use bevy::prelude::*;
use serde_json::Value;

use crate::campaign::vegetation::data::{Plane, read_png};

/// Radius (map px) of the search for the shore around a point (`SHORE_SEARCH_PX`).
const SHORE_SEARCH_PX: i32 = 6;

/// `coast` block of a catalogue entry.
#[derive(Debug, Clone, PartialEq)]
pub struct CoastRule {
    pub max_px: f32,
    pub weight: f32,
    /// Allowed geologies (`rock` of the region); empty: any.
    pub rock: Vec<String>,
    pub cliff_only: bool,
}

impl CoastRule {
    pub fn parse(v: &Value) -> Option<CoastRule> {
        let o = v.as_object()?;
        Some(CoastRule {
            max_px: o.get("max_px").and_then(Value::as_f64).unwrap_or(6.0) as f32,
            weight: o.get("weight").and_then(Value::as_f64).unwrap_or(1.0) as f32,
            rock: o
                .get("rock")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|r| r.as_str().map(str::to_owned))
                .collect(),
            cliff_only: o
                .get("cliff_only")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        })
    }
}

#[derive(Debug, Clone)]
struct Region {
    rock: String,
    polygon: Vec<Vec2>,
}

/// Shore distance raster and coast geology.
pub struct CoastData {
    dist: Plane,
    regions: Vec<Region>,
    default_rock: String,
    probe_px: f32,
    cliff_min_m: f32,
    cliff_max_m: f32,
}

impl std::fmt::Debug for CoastData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CoastData")
            .field("raster", &(self.dist.w, self.dist.h))
            .field("regions", &self.regions.len())
            .finish()
    }
}

fn point_in_polygon(p: Vec2, poly: &[Vec2]) -> bool {
    let mut inside = false;
    let mut j = poly.len().wrapping_sub(1);
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

impl CoastData {
    pub fn load(data_dir: &Path) -> Option<CoastData> {
        let dist = read_png(&data_dir.join("map/coast_dist.png")).ok()?;
        let text = std::fs::read_to_string(data_dir.join("map/coast_types.json")).ok()?;
        let doc: Value = serde_json::from_str(&text).ok()?;
        Some(CoastData::from_parts(dist, &doc))
    }

    pub fn from_parts(dist: Plane, doc: &Value) -> CoastData {
        let regions = doc["regions"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|r| Region {
                rock: r["rock"].as_str().unwrap_or("rock").to_owned(),
                polygon: r["polygon_px"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|p| Some(Vec2::new(p[0].as_f64()? as f32, p[1].as_f64()? as f32)))
                    .collect(),
            })
            .collect();
        let cliff = &doc["cliff"];
        let num = |v: &Value, d: f32| v.as_f64().map_or(d, |x| x as f32);
        CoastData {
            dist,
            regions,
            default_rock: doc["default"]["rock"].as_str().unwrap_or("rock").to_owned(),
            probe_px: num(&cliff["probe_px"], 1.6),
            cliff_min_m: num(&cliff["min_m"], 36.0),
            cliff_max_m: num(&cliff["max_m"], 52.0),
        }
    }

    /// Signed distance to the shore (map px, > 0 on land) of the raster pixel `cell`.
    fn coast_px(&self, cell: IVec2) -> f32 {
        let x = cell.x.clamp(0, self.dist.w as i32 - 1) as usize;
        let y = cell.y.clamp(0, self.dist.h as i32 - 1) as usize;
        let v = self.dist.data[(y * self.dist.w + x) * self.dist.ch] as f32;
        (v - 128.0) / 2.0
    }

    /// `_coast_px_at`: the raster covers the map of `map_size` px.
    pub fn distance_at(&self, map_size: Vec2, at: Vec2) -> f32 {
        if self.dist.is_empty() {
            return 1e6;
        }
        let px = (at.x * self.dist.w as f32 / map_size.x.max(1.0)) as i32;
        let py = (at.y * self.dist.h as f32 / map_size.y.max(1.0)) as i32;
        self.coast_px(IVec2::new(px, py))
    }

    /// `region_at(px)["rock"]`: the last region containing the point, else the default.
    pub fn rock_at(&self, at: Vec2) -> &str {
        self.regions
            .iter()
            .rev()
            .find(|r| r.polygon.len() >= 3 && point_in_polygon(at, &r.polygon))
            .map_or(self.default_rock.as_str(), |r| r.rock.as_str())
    }

    /// `kind_at(px).ends_with("_cliff")`: nearest shore pixel within 6 px, altitude read
    /// `probe_px` inland along the distance gradient. The raster pixels are map pixels here (the
    /// coast raster has the size of the composed map).
    pub fn is_cliff(&self, at: Vec2, height_m: impl Fn(Vec2) -> f32) -> bool {
        let origin = at.floor().as_ivec2();
        let mut best: Option<IVec2> = None;
        let mut best_d = i32::MAX;
        for dy in -SHORE_SEARCH_PX..=SHORE_SEARCH_PX {
            for dx in -SHORE_SEARCH_PX..=SHORE_SEARCH_PX {
                let cell = origin + IVec2::new(dx, dy);
                let coast = self.coast_px(cell);
                let d = dx * dx + dy * dy;
                if coast > 0.0 && coast <= 1.01 && d < best_d {
                    best = Some(cell);
                    best_d = d;
                }
            }
        }
        let Some(best) = best else {
            return false;
        };
        let gradient = Vec2::new(
            self.coast_px(best + IVec2::X) - self.coast_px(best - IVec2::X),
            self.coast_px(best + IVec2::Y) - self.coast_px(best - IVec2::Y),
        );
        let shore = best.as_vec2() + Vec2::splat(0.5);
        let inland = if gradient.length() < 1e-5 {
            shore
        } else {
            shore + gradient.normalize() * (self.probe_px - self.coast_px(best))
        };
        let h = height_m(inland);
        let t = ((h - self.cliff_min_m) / (self.cliff_max_m - self.cliff_min_m).max(1e-3))
            .clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t) >= 0.5
    }

    /// `_coast_term`: 0 far from the shore, at sea or on a geology the rule does not allow.
    pub fn term(
        &self,
        rule: &CoastRule,
        map_size: Vec2,
        at: Vec2,
        height_m: impl Fn(Vec2) -> f32,
    ) -> f32 {
        let distance = self.distance_at(map_size, at);
        if distance <= 0.0 || distance > rule.max_px {
            return 0.0;
        }
        let t = ((distance - rule.max_px * 0.5) / (rule.max_px * 0.5).max(1e-3)).clamp(0.0, 1.0);
        let term = rule.weight * (1.0 - t * t * (3.0 - 2.0 * t));
        if !rule.rock.is_empty() && !rule.rock.iter().any(|r| r == self.rock_at(at)) {
            return 0.0;
        }
        if rule.cliff_only && !self.is_cliff(at, height_m) {
            return 0.0;
        }
        term
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// 32 x 32 raster: sea for x < 16, land beyond, distance = x - 15.5.
    fn synthetic() -> CoastData {
        let mut data = Vec::new();
        for _y in 0..32 {
            for x in 0..32 {
                let d = x as f32 - 15.5;
                data.push((128.0 + d * 2.0).round().clamp(0.0, 255.0) as u8);
            }
        }
        let dist = Plane {
            w: 32,
            h: 32,
            ch: 1,
            data,
        };
        let doc = json!({
            "default": {"rock": "rock"},
            "cliff": {"probe_px": 1.6, "min_m": 36.0, "max_m": 52.0},
            "regions": [{"rock": "granite", "polygon_px": [[0, 0], [32, 0], [32, 16], [0, 16]]}],
        });
        CoastData::from_parts(dist, &doc)
    }

    #[test]
    fn coast_term_follows_the_shore_and_the_geology() {
        let coast = synthetic();
        let size = Vec2::splat(32.0);
        let granite = CoastRule {
            max_px: 6.0,
            weight: 0.9,
            rock: vec!["granite".into()],
            cliff_only: false,
        };
        let flat = |_: Vec2| 10.0;
        assert!(coast.term(&granite, size, Vec2::new(17.0, 5.0), flat) > 0.8);
        assert_eq!(coast.term(&granite, size, Vec2::new(10.0, 5.0), flat), 0.0);
        assert_eq!(coast.term(&granite, size, Vec2::new(28.0, 5.0), flat), 0.0);
        assert_eq!(coast.term(&granite, size, Vec2::new(17.0, 25.0), flat), 0.0);
        let cliff = CoastRule {
            cliff_only: true,
            rock: Vec::new(),
            ..granite
        };
        assert_eq!(coast.term(&cliff, size, Vec2::new(17.0, 25.0), flat), 0.0);
        assert!(coast.term(&cliff, size, Vec2::new(17.0, 25.0), |_| 80.0) > 0.0);
    }

    #[test]
    fn real_coast_data_loads() {
        let root = crate::paths::find_root().expect("repository root");
        let coast = CoastData::load(&crate::paths::data_dir(&root)).expect("coast data");
        assert!(!coast.regions.is_empty());
        assert!(coast.dist.w > 1000);
    }
}
