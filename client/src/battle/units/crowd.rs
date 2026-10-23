//! Crowd rendering settings and per-soldier culling (lot foule1, `data/fx/battle_crowd.json`).
//!
//! The soldiers have no entity: a regiment is one instanced batch. Skinning every soldier of every
//! batch, seen or not, in the colour view and in each sun cascade is the main GPU cost of a big
//! melee. Here the CPU sorts the soldiers of a regiment into three classes before they are written
//! to the instance buffer:
//! - [`Class::Visible`]: inside the camera frustum (plus a margin): drawn in colour and in shadows;
//! - [`Class::ShadowOnly`]: outside the frustum but whose shadow, thrown along the sun direction,
//!   can enter it: drawn in the cascades only;
//! - `Class::Culled`: neither.
//!
//! The buffer holds the visible soldiers first, then the shadow-only ones, so the colour draw is a
//! prefix of the shadow draw (see [`SkinnedColourCount`](super::skinned::SkinnedColourCount)).

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use bevy::camera::Projection;
use bevy::math::{Vec3, Vec4};
use bevy::prelude::{Component, GlobalTransform, Transform};
use serde_json::Value;

/// Dev hook for A/B captures in one process (`CENT_ANS_CROWD_FLIP_EVERY=<frames>`): every that
/// many frames all the optimisations of this file switch off, then on again, so two captures a
/// period apart show the same scene with and without them.
pub static BASELINE_PHASE: AtomicBool = AtomicBool::new(false);

/// Whether the current frame is in the "optimisations off" phase of the flip hook.
pub fn baseline_phase() -> bool {
    BASELINE_PHASE.load(Ordering::Relaxed)
}

/// Whether the shadow cascades use the lighter skinning right now.
pub fn shadow_lite_now() -> bool {
    CrowdSettings::bundled().shadow_skin.enabled && !baseline_phase()
}

/// Flips [`BASELINE_PHASE`] every `CENT_ANS_CROWD_FLIP_EVERY` frames (no-op without the variable).
pub fn flip_phase(mut frame: bevy::prelude::Local<u64>) {
    static EVERY: OnceLock<u64> = OnceLock::new();
    let every = *EVERY.get_or_init(|| {
        std::env::var("CENT_ANS_CROWD_FLIP_EVERY")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0)
    });
    if every == 0 {
        return;
    }
    *frame += 1;
    BASELINE_PHASE.store((*frame / every) % 2 == 1, Ordering::Relaxed);
}

/// Marks the instanced batches of a live regiment (main or impostor draw), as opposed to corpses:
/// the Solari trial reads their records to place shadow proxies (`scene/solari_soldiers.rs`).
#[derive(Component, Clone, Copy, Debug)]
pub struct ProxyCaster {
    /// Horseman (horse + rider) rather than a foot soldier.
    pub mounted: bool,
}

/// Where a soldier has to be drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Visible,
    ShadowOnly,
    Culled,
}

/// Parameters of the per-soldier culling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CullSettings {
    pub enabled: bool,
    /// Slack added to every frustum plane (m): camera moving between the test and the draw.
    pub margin_m: f32,
    /// Bounding radius of a foot soldier around his centre (m), see `foot_centre_height_m`.
    pub foot_radius_m: f32,
    /// Same for a horseman (horse, rider and lance).
    pub mounted_radius_m: f32,
    /// Height above the pose position of the foot soldier's bounding sphere centre (m). The
    /// pose position is on the ground, but a soldier with his lance is over 4 m tall: a sphere
    /// centred on the feet let near soldiers vanish when their feet left the bottom of the screen.
    pub foot_centre_height_m: f32,
    /// Same for a horseman.
    pub mounted_centre_height_m: f32,
    /// Height of the tallest caster whose shadow is followed (m).
    pub shadow_height_m: f32,
    /// Longest shadow followed on the ground (m).
    pub shadow_max_length_m: f32,
    /// Floor of the sun elevation (sine of the angle) used for the shadow length.
    pub min_sun_elevation: f32,
}

impl Default for CullSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            margin_m: 8.0,
            foot_radius_m: 3.0,
            mounted_radius_m: 5.0,
            foot_centre_height_m: 2.0,
            mounted_centre_height_m: 2.5,
            shadow_height_m: 3.2,
            shadow_max_length_m: 70.0,
            min_sun_elevation: 0.12,
        }
    }
}

/// Parts of the lighter skinning of the shadow cascades (each one a shader def).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShadowSkin {
    pub enabled: bool,
    /// Nearest animation frame instead of the interpolation of two (`SHADOW_SINGLE_FRAME`).
    pub single_frame: bool,
    /// Drop the bones of low weight (`SHADOW_BONE_CUT`).
    pub bone_cut: bool,
    /// No fade from the previous clip or exchange (`SHADOW_NO_FADE`).
    pub no_fade: bool,
    /// No procedural lance (length, angle, pennon) (`SHADOW_NO_PROP`).
    pub no_prop: bool,
}

impl ShadowSkin {
    /// Shader defs to add to the shadow vertex stage.
    pub fn shader_defs(&self) -> Vec<&'static str> {
        let mut defs = Vec::new();
        if self.enabled {
            for (on, name) in [
                (self.single_frame, "SHADOW_SINGLE_FRAME"),
                (self.bone_cut, "SHADOW_BONE_CUT"),
                (self.no_fade, "SHADOW_NO_FADE"),
                (self.no_prop, "SHADOW_NO_PROP"),
            ] {
                if on {
                    defs.push(name);
                }
            }
        }
        defs
    }
}

impl Default for ShadowSkin {
    fn default() -> Self {
        Self {
            enabled: true,
            single_frame: true,
            bone_cut: true,
            no_fade: true,
            no_prop: true,
        }
    }
}

/// All the switches of `battle_crowd.json`, after the environment overrides.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CrowdSettings {
    pub cull: CullSettings,
    /// Lighter skinning in the shadow cascades.
    pub shadow_skin: ShadowSkin,
    /// Lighter skinning for the farthest level of detail in colour.
    pub far_skin: bool,
}

fn switch(env: &str, current: bool) -> bool {
    match std::env::var(env).ok().as_deref() {
        Some("0" | "off" | "false") => false,
        Some("1" | "on" | "true") => true,
        _ => current,
    }
}

impl CrowdSettings {
    /// Reads the JSON text (missing keys keep their defaults); no environment override.
    pub fn from_json(text: &str) -> Self {
        let mut s = Self::default();
        let Ok(json) = serde_json::from_str::<Value>(text) else {
            return s;
        };
        let num = |v: &Value, key: &str, into: &mut f32| {
            if let Some(x) = v.get(key).and_then(Value::as_f64) {
                *into = x as f32;
            }
        };
        let flag = |v: &Value, into: &mut bool| {
            if let Some(b) = v.get("enabled").and_then(Value::as_bool) {
                *into = b;
            }
        };
        if let Some(cull) = json.get("cull") {
            flag(cull, &mut s.cull.enabled);
            num(cull, "margin_m", &mut s.cull.margin_m);
            num(cull, "foot_radius_m", &mut s.cull.foot_radius_m);
            num(cull, "mounted_radius_m", &mut s.cull.mounted_radius_m);
            num(
                cull,
                "foot_centre_height_m",
                &mut s.cull.foot_centre_height_m,
            );
            num(
                cull,
                "mounted_centre_height_m",
                &mut s.cull.mounted_centre_height_m,
            );
            num(cull, "shadow_height_m", &mut s.cull.shadow_height_m);
            num(cull, "shadow_max_length_m", &mut s.cull.shadow_max_length_m);
            num(cull, "min_sun_elevation", &mut s.cull.min_sun_elevation);
        }
        if let Some(v) = json.get("shadow_skin") {
            let k = &mut s.shadow_skin;
            flag(v, &mut k.enabled);
            for (key, into) in [
                ("single_frame", &mut k.single_frame),
                ("bone_cut", &mut k.bone_cut),
                ("no_fade", &mut k.no_fade),
                ("no_prop", &mut k.no_prop),
            ] {
                if let Some(b) = v.get(key).and_then(Value::as_bool) {
                    *into = b;
                }
            }
        }
        if let Some(v) = json.get("far_skin") {
            flag(v, &mut s.far_skin);
        }
        s
    }

    /// The bundled file, read once, with `CENT_ANS_CROWD=off` and the per-switch overrides.
    pub fn bundled() -> Self {
        static SETTINGS: OnceLock<CrowdSettings> = OnceLock::new();
        *SETTINGS.get_or_init(|| {
            let mut s = crate::paths::find_root()
                .ok()
                .and_then(|root| {
                    std::fs::read_to_string(
                        crate::paths::data_dir(&root).join("fx/battle_crowd.json"),
                    )
                    .ok()
                })
                .map(|text| Self::from_json(&text))
                .unwrap_or_default();
            if std::env::var("CENT_ANS_CROWD").is_ok_and(|v| v == "off") {
                s.cull.enabled = false;
                s.shadow_skin.enabled = false;
                s.far_skin = false;
            }
            s.cull.enabled = switch("CENT_ANS_CROWD_CULL", s.cull.enabled);
            s.shadow_skin.enabled = switch("CENT_ANS_CROWD_SHADOW_SKIN", s.shadow_skin.enabled);
            // `CENT_ANS_CROWD_SHADOW_PARTS=single_frame,bone_cut`: only these parts (bisecting).
            if let Ok(parts) = std::env::var("CENT_ANS_CROWD_SHADOW_PARTS") {
                let has = |name: &str| parts.split(',').any(|p| p.trim() == name);
                let k = &mut s.shadow_skin;
                (k.single_frame, k.bone_cut, k.no_fade, k.no_prop) = (
                    has("single_frame"),
                    has("bone_cut"),
                    has("no_fade"),
                    has("no_prop"),
                );
            }
            s.far_skin = switch("CENT_ANS_CROWD_FAR_SKIN", s.far_skin);
            s
        })
    }
}

/// What the classification needs from the frame: the six frustum planes (inward unit normal in
/// `xyz`, offset in `w`: a point `p` is inside when `n . p + w >= 0`) and the sun.
#[derive(Debug, Clone, Copy)]
pub struct CullView {
    pub planes: [Vec4; 6],
    /// Direction the sun light travels (unit). `None`: no sun, shadows are not followed.
    pub sun: Option<Vec3>,
}

impl CullView {
    /// View of this frame from the camera's local transform (set earlier in the frame, so the
    /// planes are the ones the renderer will use) and its projection; `sun` is the direction
    /// the light travels.
    pub fn from_camera(transform: &Transform, projection: &Projection, sun: Option<Vec3>) -> Self {
        let frustum = projection.compute_frustum(&GlobalTransform::from(*transform));
        Self {
            planes: frustum.half_spaces.map(|half_space| half_space.normal_d()),
            sun: sun
                .map(Vec3::normalize_or_zero)
                .filter(|d| *d != Vec3::ZERO),
        }
    }
}

/// Signed distance of `p` to the plane (positive inside).
fn distance(plane: Vec4, p: Vec3) -> f32 {
    plane.truncate().dot(p) + plane.w
}

/// Whether a sphere of `radius` at `p` is at least partly inside every plane (slack `margin`).
pub fn sphere_in_frustum(planes: &[Vec4; 6], p: Vec3, radius: f32, margin: f32) -> bool {
    planes
        .iter()
        .all(|plane| distance(*plane, p) + radius + margin >= 0.0)
}

/// Whether the segment `a`-`b`, thickened by `radius + margin`, can touch the frustum. It is
/// rejected only when both ends are outside the same plane, so it never wrongly culls.
pub fn segment_in_frustum(planes: &[Vec4; 6], a: Vec3, b: Vec3, radius: f32, margin: f32) -> bool {
    let slack = radius + margin;
    planes
        .iter()
        .all(|plane| distance(*plane, a) + slack >= 0.0 || distance(*plane, b) + slack >= 0.0)
}

/// End of the shadow thrown on the ground by the top of a caster standing at `foot`.
pub fn shadow_tip(foot: Vec3, sun: Vec3, s: &CullSettings) -> Vec3 {
    let elevation = (-sun.y).max(s.min_sun_elevation);
    let flat = Vec3::new(sun.x, 0.0, sun.z);
    let length = (s.shadow_height_m / elevation).min(s.shadow_max_length_m);
    foot + flat.normalize_or_zero() * length
}

/// Class of a soldier standing at `foot` (pose position on the ground).
pub fn classify(view: &CullView, foot: Vec3, mounted: bool, s: &CullSettings) -> Class {
    let (radius, centre_height) = if mounted {
        (s.mounted_radius_m, s.mounted_centre_height_m)
    } else {
        (s.foot_radius_m, s.foot_centre_height_m)
    };
    let centre = foot + Vec3::Y * centre_height;
    if sphere_in_frustum(&view.planes, centre, radius, s.margin_m) {
        return Class::Visible;
    }
    let Some(sun) = view.sun else {
        return Class::Culled;
    };
    let tip = shadow_tip(foot, sun, s);
    if segment_in_frustum(&view.planes, foot, tip, radius, s.margin_m) {
        Class::ShadowOnly
    } else {
        Class::Culled
    }
}

/// Soldiers of one regiment sorted for the instance buffer.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Sorted {
    /// Indices of the visible soldiers, then of the shadow-only ones.
    pub order: Vec<usize>,
    /// Number of visible soldiers (prefix of `order`).
    pub visible: usize,
}

/// Sorts the poses (`[x, y, z, facing]`) into visible / shadow-only; culled ones are dropped.
/// With culling disabled every soldier is visible, in the original order.
pub fn sort_poses(view: &CullView, poses: &[[f32; 4]], mounted: bool, s: &CullSettings) -> Sorted {
    if !s.enabled {
        return Sorted {
            order: (0..poses.len()).collect(),
            visible: poses.len(),
        };
    }
    let mut order = Vec::with_capacity(poses.len());
    let mut shadow_only = Vec::new();
    for (i, pose) in poses.iter().enumerate() {
        match classify(view, Vec3::new(pose[0], pose[1], pose[2]), mounted, s) {
            Class::Visible => order.push(i),
            Class::ShadowOnly => shadow_only.push(i),
            Class::Culled => {}
        }
    }
    let visible = order.len();
    order.extend(shadow_only);
    Sorted { order, visible }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Camera at the origin looking down -Z, 90 degrees field, near 1 m, far 1000 m.
    fn view(sun: Option<Vec3>) -> CullView {
        let s = std::f32::consts::FRAC_1_SQRT_2;
        CullView {
            planes: [
                Vec4::new(s, 0.0, -s, 0.0),       // left
                Vec4::new(-s, 0.0, -s, 0.0),      // right
                Vec4::new(0.0, s, -s, 0.0),       // bottom
                Vec4::new(0.0, -s, -s, 0.0),      // top
                Vec4::new(0.0, 0.0, -1.0, -1.0),  // near: z <= -1
                Vec4::new(0.0, 0.0, 1.0, 1000.0), // far: z >= -1000
            ],
            sun,
        }
    }

    fn no_margin() -> CullSettings {
        CullSettings {
            margin_m: 0.0,
            foot_radius_m: 1.0,
            // The geometry tests below reason about a sphere on the ground.
            foot_centre_height_m: 0.0,
            mounted_centre_height_m: 0.0,
            ..CullSettings::default()
        }
    }

    #[test]
    fn near_soldier_with_feet_below_the_screen_stays_visible() {
        // A soldier 3 m in front of the camera, 8 m below its axis: his feet are outside the view,
        // but his body and lance (2 to 5 m above the feet) are inside. With the sphere on the feet
        // he vanished (seen in melee, bottom-left of the screen).
        let s = CullSettings {
            margin_m: 0.0,
            ..CullSettings::default()
        };
        let v = view(None);
        let feet = Vec3::new(0.0, -8.0, -3.0);
        let mut on_feet = s;
        on_feet.foot_centre_height_m = 0.0;
        on_feet.foot_radius_m = 2.4;
        assert_eq!(classify(&v, feet, false, &on_feet), Class::Culled);
        assert_eq!(classify(&v, feet, false, &s), Class::Visible);
    }

    #[test]
    fn centre_of_view_is_visible() {
        let s = no_margin();
        assert_eq!(
            classify(&view(None), Vec3::new(0.0, 0.0, -50.0), false, &s),
            Class::Visible
        );
    }

    #[test]
    fn behind_the_camera_is_culled() {
        let s = no_margin();
        assert_eq!(
            classify(&view(None), Vec3::new(0.0, 0.0, 50.0), false, &s),
            Class::Culled
        );
    }

    #[test]
    fn side_edge_uses_radius_and_margin() {
        // At z = -50 the left edge is x = -50. A foot soldier (radius 1) at x = -50.9 still
        // touches it, at -52 it does not; a margin of 3 m brings it back.
        let mut s = no_margin();
        let v = view(None);
        let near_edge = Vec3::new(-50.9 * 1.0, 0.0, -50.0);
        assert_eq!(classify(&v, near_edge, false, &s), Class::Visible);
        // Plane distance is x * s - z * s ... = (50 - 52) * 0.707 = -1.41: outside radius 1.
        let beyond = Vec3::new(-52.0, 0.0, -50.0);
        assert_eq!(classify(&v, beyond, false, &s), Class::Culled);
        s.margin_m = 3.0;
        assert_eq!(classify(&v, beyond, false, &s), Class::Visible);
    }

    #[test]
    fn mounted_radius_is_larger() {
        let s = CullSettings {
            margin_m: 0.0,
            foot_radius_m: 1.0,
            mounted_radius_m: 4.0,
            ..CullSettings::default()
        };
        let v = view(None);
        let p = Vec3::new(-53.0, 0.0, -50.0); // 2.12 m outside the left plane
        assert_eq!(classify(&v, p, false, &s), Class::Culled);
        assert_eq!(classify(&v, p, true, &s), Class::Visible);
    }

    #[test]
    fn near_plane_edge() {
        let s = no_margin();
        let v = view(None);
        // z = -0.5: 0.5 m before the near plane, radius 1 reaches it.
        assert_eq!(
            classify(&v, Vec3::new(0.0, 0.0, -0.5), false, &s),
            Class::Visible
        );
        assert_eq!(
            classify(&v, Vec3::new(0.0, 0.0, 0.5), false, &s),
            Class::Culled
        );
        assert_eq!(
            classify(&v, Vec3::new(0.0, 0.0, 3.0), false, &s),
            Class::Culled
        );
    }

    #[test]
    fn shadow_of_an_offscreen_soldier_is_kept() {
        let s = no_margin();
        // Sun travelling towards +x and slightly down: a soldier left of the view throws its
        // shadow into it.
        let sun = Vec3::new(1.0, -0.5, 0.0).normalize();
        let v = view(Some(sun));
        let foot = Vec3::new(-55.0, 0.0, -50.0);
        assert_eq!(classify(&v, foot, false, &s), Class::ShadowOnly);
        // Same soldier with the sun travelling away: culled.
        let away = view(Some(Vec3::new(-1.0, -0.5, 0.0).normalize()));
        assert_eq!(classify(&away, foot, false, &s), Class::Culled);
        // No sun at all: culled.
        assert_eq!(classify(&view(None), foot, false, &s), Class::Culled);
    }

    #[test]
    fn shadow_length_is_capped_and_floored() {
        let s = CullSettings::default();
        let high = Vec3::new(1.0, -1.0, 0.0).normalize();
        let tip = shadow_tip(Vec3::ZERO, high, &s);
        assert!((tip.x - s.shadow_height_m / high.y.abs()).abs() < 1e-3);
        // Sun on the horizon: elevation floored, then length capped.
        let low = Vec3::new(1.0, -0.001, 0.0).normalize();
        let tip = shadow_tip(Vec3::ZERO, low, &s);
        assert!(
            (tip.x - (s.shadow_height_m / s.min_sun_elevation).min(s.shadow_max_length_m)).abs()
                < 1e-3
        );
        // Vertical sun: no horizontal displacement, no NaN.
        let tip = shadow_tip(Vec3::ZERO, Vec3::NEG_Y, &s);
        assert!(tip.is_finite() && tip.x == 0.0 && tip.z == 0.0);
    }

    #[test]
    fn segment_straddling_a_plane_is_kept() {
        let v = view(None);
        // Both ends outside the left plane on the same side: rejected.
        assert!(!segment_in_frustum(
            &v.planes,
            Vec3::new(-80.0, 0.0, -50.0),
            Vec3::new(-70.0, 0.0, -50.0),
            0.0,
            0.0
        ));
        // One end inside: kept.
        assert!(segment_in_frustum(
            &v.planes,
            Vec3::new(-80.0, 0.0, -50.0),
            Vec3::new(0.0, 0.0, -50.0),
            0.0,
            0.0
        ));
        // Ends on both sides of the view, passing through it: kept (no plane has both outside).
        assert!(segment_in_frustum(
            &v.planes,
            Vec3::new(-80.0, 0.0, -50.0),
            Vec3::new(80.0, 0.0, -50.0),
            0.0,
            0.0
        ));
    }

    #[test]
    fn sorted_order_counts_and_disabled_passthrough() {
        let s = no_margin();
        let sun = Vec3::new(1.0, -0.5, 0.0).normalize();
        let v = view(Some(sun));
        let poses = [
            [0.0, 0.0, -50.0, 0.0],   // visible
            [-55.0, 0.0, -50.0, 0.0], // shadow only
            [0.0, 0.0, 50.0, 0.0],    // culled (behind)
            [10.0, 0.0, -20.0, 0.0],  // visible
            [-54.0, 0.0, -50.0, 0.0], // shadow only
        ];
        let sorted = sort_poses(&v, &poses, false, &s);
        assert_eq!(sorted.visible, 2);
        assert_eq!(sorted.order, vec![0, 3, 1, 4]);
        // Written instances = visible soldiers (colour) and visible + shadow-only (shadows).
        let off = CullSettings {
            enabled: false,
            ..s
        };
        let all = sort_poses(&v, &poses, false, &off);
        assert_eq!(all.visible, 5);
        assert_eq!(all.order, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn json_overrides_defaults() {
        let s = CrowdSettings::from_json(
            r#"{"cull":{"enabled":false,"margin_m":3.5},"shadow_skin":{"enabled":false},"far_skin":{"enabled":true}}"#,
        );
        assert!(!s.cull.enabled && !s.shadow_skin.enabled && s.far_skin);
        assert!(s.shadow_skin.single_frame);
        assert_eq!(s.cull.margin_m, 3.5);
        assert_eq!(s.cull.foot_radius_m, CullSettings::default().foot_radius_m);
        assert_eq!(
            CrowdSettings::from_json("not json"),
            CrowdSettings::default()
        );
    }

    #[test]
    fn bundled_file_parses() {
        let text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/fx/battle_crowd.json"),
        )
        .expect("file");
        let s = CrowdSettings::from_json(&text);
        assert!(s.cull.enabled && s.cull.margin_m > 0.0);
    }
}
