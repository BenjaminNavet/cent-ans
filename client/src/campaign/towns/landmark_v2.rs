//! Emblematic 1:1 cities, geo-referenced v2 format (`data/landmarks_v2/<id>.json`, schema
//! `landmark_v2.schema.json`, ADR 0078). Port of `landmark_v2_library.gd`.
//!
//! Offsets are `[dE, dN]` metres from the city origin (EPSG:3035); the local plan frame is
//! `x` east, `y` south (= `[dE, -dN]`). The city is anchored on its settlement in
//! `data/map/settlements_px.json` (the live composed-map position; `origin_3035` has no
//! projection on the composed map).

use std::collections::HashMap;
use std::path::Path;

use bevy::math::Vec2;
use serde_json::Value;

/// Cities by settlement id (`set_paris`...).
#[derive(Debug, Default, Clone)]
pub struct LandmarkLibrary {
    pub cities: HashMap<String, Value>,
}

impl LandmarkLibrary {
    pub fn load(data_dir: &Path) -> Self {
        let mut cities = HashMap::new();
        if let Ok(dir) = std::fs::read_dir(data_dir.join("landmarks_v2")) {
            let mut entries: Vec<_> = dir.filter_map(Result::ok).collect();
            entries.sort_by_key(|e| e.file_name());
            for entry in entries {
                let Some(v) = std::fs::read_to_string(entry.path())
                    .ok()
                    .and_then(|t| serde_json::from_str::<Value>(&t).ok())
                else {
                    continue;
                };
                if v["format"].as_i64() == Some(2)
                    && let Some(id) = v["settlement"].as_str()
                {
                    cities.insert(id.to_string(), v.clone());
                }
            }
        }
        Self { cities }
    }

    pub fn get(&self, settlement: &str) -> Option<&Value> {
        self.cities.get(settlement)
    }
}

pub fn num(v: &Value, default: f32) -> f32 {
    v.as_f64().map_or(default, |x| x as f32)
}

/// v2 offset `[dE, dN]` to the local plan frame.
pub fn local(p: &Value) -> Vec2 {
    Vec2::new(num(&p[0], 0.0), -num(&p[1], 0.0))
}

pub fn local_line(points: &Value) -> Vec<Vec2> {
    points
        .as_array()
        .map(|a| a.iter().map(local).collect())
        .unwrap_or_default()
}

/// v2 angle (degrees, counter-clockwise from east, y north) to the local yaw (radians, from +x
/// towards +y = south): the sign flips.
pub fn yaw_of(angle_deg: f32) -> f32 {
    -angle_deg.to_radians()
}

/// Item present in year `year` (`from_year` / `until_year` inclusive).
pub fn present(item: &Value, year: i32) -> bool {
    if let Some(f) = item["from_year"].as_i64()
        && i64::from(year) < f
    {
        return false;
    }
    if let Some(u) = item["until_year"].as_i64()
        && i64::from(year) > u
    {
        return false;
    }
    true
}

pub fn year_of(city: &Value) -> i32 {
    city["year"].as_i64().unwrap_or(1340) as i32
}

/// `[a, b]` pair with a default.
pub fn pair(v: &Value, default: [f32; 2]) -> [f32; 2] {
    [num(&v[0], default[0]), num(&v[1], default[1])]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn frame_conversions() {
        assert_eq!(local(&json!([10, 4])), Vec2::new(10.0, -4.0));
        assert!((yaw_of(90.0) + std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    }

    #[test]
    fn dated_items() {
        let a = json!({"until_year": 1344});
        assert!(present(&a, 1340));
        assert!(!present(&a, 1429));
        assert!(present(&json!({"from_year": 1400}), 1429));
        assert!(present(&json!({}), 1000));
    }
}
