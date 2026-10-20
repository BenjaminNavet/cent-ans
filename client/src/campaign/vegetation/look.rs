//! Look of the campaign forests (lot TS-T2, ADR 0710), read from `data/art/campaign_forest_look.json`:
//! darker and more varied canopy, crisp forest edges with a few trees standing in front, trees
//! grown far away so a forest stays a dark mass, frosted conifers and grey-brown bare broadleaves
//! in winter. The shader terms travel in the `look_*` vectors of the batch parameters.

use bevy::prelude::*;
use serde_json::Value;

pub const LOOK_FILE: &str = "art/campaign_forest_look.json";

/// Tuning of the forest look (defaults = the shipped data file).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ForestLook {
    pub value: f32,
    pub saturation: f32,
    pub balance: Vec3,
    pub tree_value_spread: f32,
    pub stand_value_spread: f32,
    pub stand_period: f32,
    pub height_spread: f32,
    pub height_period: f32,
    pub crown_shade: f32,
    pub edge_cut: f32,
    pub outlier_share: f32,
    pub outlier_group_px: f32,
    pub treeline_fade_m: f32,
    pub treeline_jitter_m: f32,
    pub rock_slope: Vec2,
    pub grow_start: f32,
    pub grow_end: f32,
    pub far_size_gain: f32,
    pub frost: Vec3,
    pub frost_strength: f32,
    pub green_share: f32,
    pub frost_altitude: Vec2,
    pub bare: Vec3,
    pub bare_openwork: f32,
}

impl Default for ForestLook {
    fn default() -> Self {
        ForestLook {
            value: 0.72,
            saturation: 1.3,
            balance: Vec3::new(0.86, 1.0, 0.78),
            tree_value_spread: 0.22,
            stand_value_spread: 0.3,
            stand_period: 26.0,
            height_spread: 0.3,
            height_period: 40.0,
            crown_shade: 0.35,
            edge_cut: 0.42,
            outlier_share: 0.1,
            outlier_group_px: 1.2,
            treeline_fade_m: 90.0,
            treeline_jitter_m: 120.0,
            rock_slope: Vec2::new(0.5, 0.68),
            grow_start: 140.0,
            grow_end: 600.0,
            far_size_gain: 0.7,
            frost: Vec3::new(0.8, 0.84, 0.88),
            frost_strength: 0.85,
            green_share: 0.3,
            frost_altitude: Vec2::new(1.5, 4.0),
            bare: Vec3::new(0.3, 0.25, 0.2),
            bare_openwork: 0.2,
        }
    }
}

fn num(v: &Value, default: f32) -> f32 {
    v.as_f64().map_or(default, |x| x as f32)
}

fn rgb(v: &Value, default: Vec3) -> Vec3 {
    Vec3::new(
        num(&v[0], default.x),
        num(&v[1], default.y),
        num(&v[2], default.z),
    )
}

impl ForestLook {
    pub fn parse(v: &Value) -> ForestLook {
        let d = ForestLook::default();
        let (c, e, f, w) = (&v["canopy"], &v["edge"], &v["far"], &v["winter"]);
        let t = &v["treeline"];
        ForestLook {
            value: num(&c["value"], d.value),
            saturation: num(&c["saturation"], d.saturation),
            balance: rgb(&c["balance"], d.balance),
            tree_value_spread: num(&c["tree_value_spread"], d.tree_value_spread),
            stand_value_spread: num(&c["stand_value_spread"], d.stand_value_spread),
            stand_period: num(&c["stand_period_units"], d.stand_period),
            height_spread: num(&c["height_spread"], d.height_spread),
            height_period: num(&c["height_period_units"], d.height_period),
            crown_shade: num(&c["crown_shade"], d.crown_shade),
            edge_cut: num(&e["cut"], d.edge_cut),
            outlier_share: num(&e["outlier_share"], d.outlier_share),
            outlier_group_px: num(&e["outlier_group_px"], d.outlier_group_px),
            treeline_fade_m: num(&t["fade_m"], d.treeline_fade_m),
            treeline_jitter_m: num(&t["jitter_m"], d.treeline_jitter_m),
            rock_slope: Vec2::new(
                num(&t["slope"][0], d.rock_slope.x),
                num(&t["slope"][1], d.rock_slope.y),
            ),
            grow_start: num(&f["grow_start_units"], d.grow_start),
            grow_end: num(&f["grow_end_units"], d.grow_end),
            far_size_gain: num(&f["size_gain"], d.far_size_gain),
            frost: rgb(&w["frost"], d.frost),
            frost_strength: num(&w["frost_strength"], d.frost_strength),
            green_share: num(&w["green_share"], d.green_share),
            frost_altitude: Vec2::new(
                num(&w["frost_altitude_units"][0], d.frost_altitude.x),
                num(&w["frost_altitude_units"][1], d.frost_altitude.y),
            ),
            bare: rgb(&w["bare"], d.bare),
            bare_openwork: num(&w["bare_openwork"], d.bare_openwork),
        }
    }

    pub fn load() -> ForestLook {
        crate::paths::find_root()
            .ok()
            .and_then(|root| std::fs::read(crate::paths::data_dir(&root).join(LOOK_FILE)).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .map(|v| ForestLook::parse(&v))
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_file_parses_to_the_defaults() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let bytes = std::fs::read(root.join("data").join(LOOK_FILE)).expect("look file");
        let value: Value = serde_json::from_slice(&bytes).expect("json");
        assert_eq!(ForestLook::parse(&value), ForestLook::default());
    }
}
