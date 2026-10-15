//! Look of the 1:1 towns (`data/art/town_look.json`): block tints, wall, street and ground
//! colours, ribbon lift, market furniture. Visual only.

use std::path::Path;

use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct MarketLook {
    pub area_m2_per_prop: f32,
    pub max_props: usize,
    pub kinds: Vec<String>,
    pub well: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeepLook {
    pub side_m: f32,
    pub height_m: f32,
    pub tower_radius_m: f32,
    pub tower_height_m: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TownLook {
    pub block_colors: Vec<[f32; 3]>,
    pub masonry: [f32; 3],
    pub planks: [f32; 3],
    pub street: [f32; 3],
    pub ground: [f32; 3],
    pub ribbon_lift_world: f32,
    pub ground_lift_world: f32,
    pub default_block_height_m: f32,
    pub block_height_jitter: f32,
    pub foundation_sink_m: f32,
    pub market: MarketLook,
    pub keep: KeepLook,
}

fn rgb(v: &Value, default: [f32; 3]) -> [f32; 3] {
    let f = |i: usize, d: f32| v[i].as_f64().map_or(d, |x| x as f32);
    [f(0, default[0]), f(1, default[1]), f(2, default[2])]
}

fn num(v: &Value, default: f32) -> f32 {
    v.as_f64().map_or(default, |x| x as f32)
}

impl Default for TownLook {
    fn default() -> Self {
        Self::from_json(&Value::Null)
    }
}

impl TownLook {
    pub fn load(data_dir: &Path) -> Self {
        std::fs::read_to_string(data_dir.join("art/town_look.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .map(|v| Self::from_json(&v))
            .unwrap_or_default()
    }

    /// Missing keys fall back to neutral values (the file is validated by its schema).
    pub fn from_json(doc: &Value) -> Self {
        let mut block_colors: Vec<[f32; 3]> = doc["block_colors"]
            .as_array()
            .map(|a| a.iter().map(|c| rgb(c, [0.55, 0.45, 0.35])).collect())
            .unwrap_or_default();
        if block_colors.is_empty() {
            block_colors.push([0.55, 0.45, 0.35]);
        }
        let m = &doc["market"];
        let k = &doc["castle_keep"];
        Self {
            block_colors,
            masonry: rgb(&doc["masonry_color"], [0.6, 0.6, 0.55]),
            planks: rgb(&doc["planks_color"], [0.45, 0.34, 0.24]),
            street: rgb(&doc["street_color"], [0.42, 0.35, 0.27]),
            ground: rgb(&doc["ground_color"], [0.36, 0.31, 0.22]),
            ribbon_lift_world: num(&doc["ribbon_lift_world"], 0.003),
            ground_lift_world: num(&doc["ground_lift_world"], 0.0015),
            default_block_height_m: num(&doc["default_block_height_m"], 7.0),
            block_height_jitter: num(&doc["block_height_jitter"], 0.3),
            foundation_sink_m: num(&doc["foundation_sink_m"], 0.3),
            market: MarketLook {
                area_m2_per_prop: num(&m["area_m2_per_prop"], 220.0),
                max_props: m["max_props"].as_u64().unwrap_or(12) as usize,
                kinds: m["kinds"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|s| s.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default(),
                well: m["well"].as_bool().unwrap_or(true),
            },
            keep: KeepLook {
                side_m: num(&k["side_m"], 18.0),
                height_m: num(&k["height_m"], 22.0),
                tower_radius_m: num(&k["tower_radius_m"], 4.0),
                tower_height_m: num(&k["tower_height_m"], 16.0),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_look_loads() {
        let root = crate::paths::find_root().unwrap();
        let look = TownLook::load(&crate::paths::data_dir(&root));
        assert!(look.block_colors.len() >= 3);
        assert!(look.ribbon_lift_world > look.ground_lift_world);
        assert!(!look.market.kinds.is_empty());
    }
}
