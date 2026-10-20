//! Look of the campaign forests (lot TS-T2, ADR 0710), read from `data/art/campaign_forest_look.json`:
//! darker and more varied canopy, crisp forest edges with a few trees standing in front, trees
//! grown far away so a forest stays a dark mass, frosted conifers and grey-brown bare broadleaves
//! in winter. The shader terms travel in the `look_*` vectors of the batch parameters.

use bevy::prelude::*;
use serde_json::Value;

pub const LOOK_FILE: &str = "art/campaign_forest_look.json";

/// Tuning of the forest look (defaults = the shipped data file).
#[derive(Clone, Debug, PartialEq)]
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
    pub open: OpenLook,
    pub groves: GroveLook,
    pub clearings: ClearingLook,
}

/// Trees outside the forests (hedgerows, bocage, lone trees) fade out with the camera distance
/// so the forest masses read in the medium view (lot TS-T8).
#[derive(Clone, Debug, PartialEq)]
pub struct OpenLook {
    /// Camera distance up to which every open-land tree is drawn.
    pub full_distance: f32,
    /// Camera distance from which only `far_share` of them remain.
    pub far_distance: f32,
    pub far_share: f32,
    /// Open-land trees closer than this to the focus point are always drawn (hedges near the
    /// player's gaze), fading out over `focus_fade`.
    pub focus_keep: f32,
    pub focus_fade: f32,
}

/// Groves: compact tree groups on the meadows, a few per cell.
#[derive(Clone, Debug, PartialEq)]
pub struct GroveLook {
    pub cell_px: f32,
    /// Probability of a grove per cell.
    pub per_cell: f32,
    pub trees: (u32, u32),
    pub radius_px: (f32, f32),
    /// Minimum crops share of the ground (meadows and fields, not rock or scrub).
    pub min_crops: f32,
    /// Hard cap of groves per tile.
    pub max_per_tile: u32,
    /// Share of the groves kept at the far end of the open-land fade (they stay visible).
    pub far_share: f32,
    /// Share drawn per quality preset: low, medium, high, ultra.
    pub quality_share: [f32; 4],
}

/// Inhabited clearings: holes in the big forests where a life object can stand.
#[derive(Clone, Debug, PartialEq)]
pub struct ClearingLook {
    pub cell_px: f32,
    pub per_cell: f32,
    pub radius_px: (f32, f32),
    /// Forest mask required at the centre and on the ring of the clearing.
    pub min_forest: f32,
    /// Share of the radius the outline wobbles.
    pub wobble: f32,
    /// Models (`dn_manifest.json` ids) a later lot places in the clearings.
    pub kinds: Vec<String>,
    /// TS-T7c: the life objects placed in the clearings (`clearing_life.rs`).
    pub life: ClearingLife,
}

/// One life object of a clearing: glb `models/<model>_lod<lod>.glb`.
#[derive(Clone, Debug, PartialEq)]
pub struct LifeObject {
    pub model: String,
    /// Real width (m) of the object, base of the on-screen size.
    pub size_m: f32,
    /// Share of the screen height it holds at the most.
    pub fraction: f32,
}

/// Sizing, range and budget of the life objects of the clearings.
#[derive(Clone, Debug, PartialEq)]
pub struct ClearingLife {
    /// Camera distance beyond which no object is shown, and where the fade out starts.
    pub max_distance: f32,
    pub fade_from: f32,
    /// Camera distance under which the size is real, and from which it holds its screen share.
    pub real_below: f32,
    pub full_from: f32,
    pub max_factor: f32,
    pub fov_deg: f32,
    /// Level of detail of the glb files.
    pub lod: usize,
    /// Objects shown at the most, per quality (low, medium, high, ultra).
    pub budget: [usize; 4],
    /// By id of `ClearingLook::kinds`.
    pub objects: std::collections::BTreeMap<String, LifeObject>,
}

impl Default for ClearingLife {
    fn default() -> Self {
        let object = |model: &str, size_m: f32, fraction: f32| LifeObject {
            model: model.into(),
            size_m,
            fraction,
        };
        ClearingLife {
            max_distance: 90.0,
            fade_from: 70.0,
            real_below: 20.0,
            full_from: 60.0,
            max_factor: 6.0,
            fov_deg: 45.0,
            lod: 1,
            budget: [0, 8, 20, 40],
            objects: [
                (
                    "env_charcoal_kiln".to_string(),
                    object("dn/buildings/env_charcoal_kiln", 6.0, 0.03),
                ),
                (
                    "econ_charcoal_stack_burners".to_string(),
                    object("dn/props/econ_charcoal_stack_burners", 6.0, 0.03),
                ),
                (
                    "econ_timber_yard_logs".to_string(),
                    object("dn/buildings/econ_timber_yard_logs", 8.0, 0.035),
                ),
            ]
            .into_iter()
            .collect(),
        }
    }
}

impl Default for OpenLook {
    fn default() -> Self {
        OpenLook {
            full_distance: 60.0,
            far_distance: 260.0,
            far_share: 0.06,
            focus_keep: 50.0,
            focus_fade: 40.0,
        }
    }
}

impl Default for GroveLook {
    fn default() -> Self {
        GroveLook {
            cell_px: 18.0,
            per_cell: 0.25,
            trees: (4, 9),
            radius_px: (1.2, 2.6),
            min_crops: 0.15,
            max_per_tile: 60,
            far_share: 0.5,
            quality_share: [0.3, 0.65, 1.0, 1.0],
        }
    }
}

impl Default for ClearingLook {
    fn default() -> Self {
        ClearingLook {
            cell_px: 64.0,
            per_cell: 0.5,
            radius_px: (3.0, 5.5),
            min_forest: 0.9,
            wobble: 0.25,
            kinds: vec![
                "env_charcoal_kiln".into(),
                "econ_charcoal_stack_burners".into(),
                "econ_timber_yard_logs".into(),
            ],
            life: ClearingLife::default(),
        }
    }
}

/// Kind of tree for the thinning of the medium view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// Inside a forest or on its edge ramp.
    Forest,
    /// Hedgerow, bocage or lone tree on open land.
    Open,
    /// Compact grove on a meadow.
    Grove,
}

impl OpenLook {
    /// Share of the open-land trees drawn at camera `distance` (before the focus exemption).
    pub fn camera_share(&self, distance: f32, far_share: f32) -> f32 {
        let t = super::mask::smoothstep(
            self.full_distance,
            self.far_distance.max(self.full_distance + 1.0),
            distance,
        );
        1.0 + (far_share - 1.0) * t
    }
}

/// Thinning of the trees outside the forests for one assembly.
#[derive(Clone, Debug, PartialEq)]
pub struct OpenThinning {
    pub open: OpenLook,
    pub camera_distance: f32,
    pub grove_far_share: f32,
    /// Quality multiplier of the groves.
    pub grove_quality: f32,
}

impl OpenThinning {
    /// Everything drawn (tests, forests only).
    pub fn none() -> OpenThinning {
        OpenThinning {
            open: OpenLook {
                far_share: 1.0,
                ..OpenLook::default()
            },
            camera_distance: 0.0,
            grove_far_share: 1.0,
            grove_quality: 1.0,
        }
    }

    pub fn new(look: &ForestLook, camera_distance: f32, quality_level: &str) -> OpenThinning {
        let rank = LEVELS.iter().position(|l| *l == quality_level).unwrap_or(2);
        OpenThinning {
            open: look.open.clone(),
            camera_distance,
            grove_far_share: look.groves.far_share,
            grove_quality: look.groves.quality_share[rank],
        }
    }

    /// Share of the trees of `role` at `focus_distance` from the focus that is drawn.
    pub fn keep(&self, role: Role, focus_distance: f32) -> f32 {
        match role {
            Role::Forest => 1.0,
            Role::Open => {
                let camera = self
                    .open
                    .camera_share(self.camera_distance, self.open.far_share);
                let near = 1.0
                    - super::mask::smoothstep(
                        self.open.focus_keep,
                        self.open.focus_keep + self.open.focus_fade.max(1.0),
                        focus_distance,
                    );
                camera + (1.0 - camera) * near
            }
            Role::Grove => {
                self.open
                    .camera_share(self.camera_distance, self.grove_far_share)
                    * self.grove_quality
            }
        }
    }
}

/// Quality levels, in the order of `GroveLook::quality_share`.
pub const LEVELS: [&str; 4] = ["low", "medium", "high", "ultra"];

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
            bare: Vec3::new(0.22, 0.18, 0.15),
            bare_openwork: 0.28,
            open: OpenLook::default(),
            groves: GroveLook::default(),
            clearings: ClearingLook::default(),
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

fn parse_life(v: &Value) -> ClearingLife {
    let d = ClearingLife::default();
    let budget = |i: usize, key: &str| {
        v["budget"][key]
            .as_u64()
            .map_or(d.budget[i], |n| n as usize)
    };
    ClearingLife {
        max_distance: num(&v["max_distance"], d.max_distance),
        fade_from: num(&v["fade_from"], d.fade_from),
        real_below: num(&v["real_below"], d.real_below),
        full_from: num(&v["full_from"], d.full_from),
        max_factor: num(&v["max_factor"], d.max_factor),
        fov_deg: num(&v["fov_deg"], d.fov_deg),
        lod: v["lod"].as_u64().map_or(d.lod, |n| n.min(2) as usize),
        budget: [
            budget(0, "low"),
            budget(1, "medium"),
            budget(2, "high"),
            budget(3, "ultra"),
        ],
        objects: v["objects"]
            .as_object()
            .map(|o| {
                o.iter()
                    .filter_map(|(id, e)| {
                        Some((
                            id.clone(),
                            LifeObject {
                                model: e["model"].as_str()?.to_string(),
                                size_m: num(&e["size_m"], 5.0).max(0.1),
                                fraction: num(&e["fraction"], 0.03),
                            },
                        ))
                    })
                    .collect()
            })
            .unwrap_or(d.objects),
    }
}

impl ForestLook {
    pub fn parse(v: &Value) -> ForestLook {
        let d = ForestLook::default();
        let (c, e, f, w) = (&v["canopy"], &v["edge"], &v["far"], &v["winter"]);
        let t = &v["treeline"];
        let (o, g, cl) = (&v["open"], &v["groves"], &v["clearings"]);
        let (od, gd, cd) = (
            OpenLook::default(),
            GroveLook::default(),
            ClearingLook::default(),
        );
        let pair = |v: &Value, d: (f32, f32)| (num(&v[0], d.0), num(&v[1], d.1));
        let quality = |i: usize| num(&g["quality_share"][LEVELS[i]], gd.quality_share[i]);
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
            open: OpenLook {
                full_distance: num(&o["full_distance"], od.full_distance),
                far_distance: num(&o["far_distance"], od.far_distance),
                far_share: num(&o["far_share"], od.far_share),
                focus_keep: num(&o["focus_keep_units"], od.focus_keep),
                focus_fade: num(&o["focus_fade_units"], od.focus_fade),
            },
            groves: GroveLook {
                cell_px: num(&g["cell_px"], gd.cell_px),
                per_cell: num(&g["per_cell"], gd.per_cell),
                trees: {
                    let p = pair(&g["trees"], (gd.trees.0 as f32, gd.trees.1 as f32));
                    (p.0 as u32, p.1 as u32)
                },
                radius_px: pair(&g["radius_px"], gd.radius_px),
                min_crops: num(&g["min_crops"], gd.min_crops),
                max_per_tile: num(&g["max_per_tile"], gd.max_per_tile as f32) as u32,
                far_share: num(&g["far_share"], gd.far_share),
                quality_share: [quality(0), quality(1), quality(2), quality(3)],
            },
            clearings: ClearingLook {
                cell_px: num(&cl["cell_px"], cd.cell_px),
                per_cell: num(&cl["per_cell"], cd.per_cell),
                radius_px: pair(&cl["radius_px"], cd.radius_px),
                min_forest: num(&cl["min_forest"], cd.min_forest),
                wobble: num(&cl["wobble"], cd.wobble),
                kinds: cl["kinds"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|k| k.as_str().map(str::to_string))
                            .collect()
                    })
                    .filter(|k: &Vec<String>| !k.is_empty())
                    .unwrap_or(cd.kinds),
                life: parse_life(&cl["life"]),
            },
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

    #[test]
    fn open_land_fades_with_the_camera_but_stays_near_the_focus() {
        let thin = OpenThinning::new(&ForestLook::default(), 400.0, "high");
        assert_eq!(thin.keep(Role::Forest, 500.0), 1.0);
        // Far view: hedges nearly gone, except around the focus.
        assert!(thin.keep(Role::Open, 300.0) < 0.1);
        assert!((thin.keep(Role::Open, 10.0) - 1.0).abs() < 1e-5);
        // Groves stay much more than the scattered trees.
        assert!(thin.keep(Role::Grove, 300.0) > 0.4);
        // Close view: everything drawn; medium view: in between and decreasing.
        let close = OpenThinning::new(&ForestLook::default(), 40.0, "high");
        assert_eq!(close.keep(Role::Open, 300.0), 1.0);
        let shares: Vec<f32> = [100.0, 150.0, 220.0, 300.0]
            .iter()
            .map(|d| OpenThinning::new(&ForestLook::default(), *d, "high").keep(Role::Open, 300.0))
            .collect();
        assert!(shares.windows(2).all(|w| w[1] < w[0]), "{shares:?}");
        // Low quality draws fewer groves.
        let low = OpenThinning::new(&ForestLook::default(), 400.0, "low");
        assert!(low.keep(Role::Grove, 0.0) < thin.keep(Role::Grove, 0.0));
    }
}
