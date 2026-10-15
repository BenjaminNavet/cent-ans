//! Atmosphere: THE single distance-fog layer of the campaign, a port of the depth fog of the Godot
//! `Environment` (`campaign_map.tscn` + `campaign_atmosphere.gd::apply_distance`,
//! `campaign_lighting.gd::aerial`, `data/fx/campaign_lighting.json` `aerial`, season `fog_color` of
//! `data/fx/atmosphere.json`). Godot's weather adds NO distance fog (only a ground veil capped at
//! `weather_mist_max` = 0.07 in the terrain shader, ADR 0142: the mist cap was the real cause of
//! the washed-out look), so rain / snow / storm / morning fog no longer touch this layer.
//!
//! Godot amount: `pow(smoothstep(begin, end, d), curve) * density`, `begin`/`end` = camera
//! distance x factors (pitch-dependent pairs), relaxed towards `close_fog_begin/end` in the close
//! view. Bevy `DistanceFog` only knows linear ramps, so a line is fitted through the curve
//! ([`DepthFog::fit_linear`]); the terrain shader honours the same `DistanceFog`
//! (`camera_distance_fog`) and its own depth fog is switched off here (`params.fog.w = 0`): one
//! layer for ground and objects.

use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::prelude::*;
use serde_json::{Value, json};

use super::WeatherView;
use crate::app::AppState;
use crate::campaign::camera::CampaignCamera;
use crate::campaign::camera::rig::CampaignRig;
use crate::campaign::terrain::material::TerrainMaterial;
use crate::campaign::terrain::season::{SEASONS, SeasonWeights};

/// Seconds-based easing rate of the colour.
const EASE_PER_SECOND: f32 = 4.0;

/// Parameters read from the data files (defaults = the values of the Godot scene / data).
#[derive(Resource, Debug, Clone)]
pub struct FogLook {
    /// Season fog colour (`fog_color`), spring..winter.
    pub season_color: [Vec3; 4],
    pub blue: Vec3,
    pub blue_mix: f32,
    pub low_pitch: f32,
    pub top_pitch: f32,
    /// `[low view, top view]` pairs.
    pub density: [f32; 2],
    pub begin_factor: [f32; 2],
    pub end_factor: [f32; 2],
    /// Depth fog curve of the scene (`fog_depth_curve`).
    pub curve: f32,
    pub close_full: f32,
    pub close_begin: f32,
    pub close_fog_begin: f32,
    pub close_fog_end: f32,
}

impl Default for FogLook {
    fn default() -> Self {
        FogLook {
            season_color: [Vec3::new(0.74, 0.79, 0.86); 4],
            blue: Vec3::new(0.56, 0.68, 0.9),
            blue_mix: 0.5,
            low_pitch: 32.0,
            top_pitch: 60.0,
            density: [0.85, 0.5],
            begin_factor: [1.25, 1.9],
            end_factor: [5.5, 8.0],
            curve: 1.6,
            close_full: 6.0,
            close_begin: 30.0,
            close_fog_begin: 18.0,
            close_fog_end: 240.0,
        }
    }
}

fn vec3(value: &Value, fallback: Vec3) -> Vec3 {
    match value.as_array() {
        Some(a) if a.len() >= 3 => Vec3::new(
            a[0].as_f64().unwrap_or(0.0) as f32,
            a[1].as_f64().unwrap_or(0.0) as f32,
            a[2].as_f64().unwrap_or(0.0) as f32,
        ),
        _ => fallback,
    }
}

fn pair(value: &Value, fallback: [f32; 2]) -> [f32; 2] {
    match value.as_array() {
        Some(a) if a.len() == 2 => [
            a[0].as_f64().unwrap_or(fallback[0] as f64) as f32,
            a[1].as_f64().unwrap_or(fallback[1] as f64) as f32,
        ],
        _ => fallback,
    }
}

impl FogLook {
    pub fn from_json(atmosphere: &Value, lighting: &Value) -> FogLook {
        let mut out = FogLook::default();
        for (i, name) in SEASONS.iter().enumerate() {
            out.season_color[i] = vec3(
                &atmosphere["campaign"]["seasons"][*name]["fog_color"],
                out.season_color[i],
            );
        }
        let a = &lighting["aerial"];
        out.blue = vec3(&a["blue"], out.blue);
        let num = |v: &Value, d: f32| v.as_f64().map_or(d, |x| x as f32);
        out.blue_mix = num(&a["blue_mix"], out.blue_mix).clamp(0.0, 1.0);
        out.low_pitch = num(&a["low_pitch_deg"], out.low_pitch);
        out.top_pitch = num(&a["top_pitch_deg"], out.top_pitch);
        out.density = pair(&a["density"], out.density);
        out.begin_factor = pair(&a["begin_factor"], out.begin_factor);
        out.end_factor = pair(&a["end_factor"], out.end_factor);
        out
    }

    pub fn load(root: &std::path::Path) -> FogLook {
        let read = |path: &str| {
            std::fs::read_to_string(root.join(path))
                .ok()
                .and_then(|t| serde_json::from_str::<Value>(&t).ok())
                .unwrap_or(Value::Null)
        };
        FogLook::from_json(
            &read("data/fx/atmosphere.json"),
            &read("data/fx/campaign_lighting.json"),
        )
    }

    /// Fog colour for season weights: season `fog_color` blended towards the aerial blue.
    pub fn color(&self, weights: Vec4) -> Vec3 {
        let w = weights.to_array();
        let total: f32 = w.iter().sum::<f32>().max(1e-4);
        let mut season = Vec3::ZERO;
        for (i, wi) in w.iter().enumerate() {
            season += self.season_color[i] * (wi / total);
        }
        season.lerp(self.blue, self.blue_mix)
    }
}

/// Effective Godot depth-fog parameters for a camera at `distance` with `pitch_deg`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DepthFog {
    pub begin: f32,
    pub end: f32,
    pub density: f32,
    pub curve: f32,
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl DepthFog {
    /// `campaign_atmosphere.gd::apply_distance` + `CampaignLighting.aerial`.
    pub fn at(look: &FogLook, distance: f32, pitch_deg: f32) -> DepthFog {
        let close = 1.0 - smoothstep(look.close_full, look.close_begin, distance);
        let low = 1.0 - smoothstep(look.low_pitch, look.top_pitch, pitch_deg);
        let mix = |p: [f32; 2]| p[1] + (p[0] - p[1]) * low;
        let (bf, ef) = (mix(look.begin_factor), mix(look.end_factor));
        let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
        DepthFog {
            begin: lerp(
                distance * bf,
                look.close_fog_begin.max(distance * bf),
                close,
            ),
            end: lerp(distance * ef, look.close_fog_end.max(distance * ef), close),
            density: mix(look.density),
            curve: look.curve,
        }
    }

    /// Godot fog amount (0..density) at view depth `d`.
    pub fn amount(&self, d: f32) -> f32 {
        smoothstep(self.begin, self.end, d).powf(self.curve) * self.density
    }

    /// Linear ramp `(start, end)` of the Bevy fog (alpha = `density`) fitted to the curve.
    pub fn fit_linear(&self) -> (f32, f32) {
        // Normalised position where the unit curve reaches `g`.
        let solve = |g: f32| {
            let (mut lo, mut hi) = (0.0_f32, 1.0_f32);
            for _ in 0..40 {
                let mid = 0.5 * (lo + hi);
                if smoothstep(0.0, 1.0, mid).powf(self.curve) < g {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            0.5 * (lo + hi)
        };
        let (g1, g2) = (0.2_f32, 0.8_f32);
        let (u1, u2) = (solve(g1), solve(g2));
        let slope = (g2 - g1) / (u2 - u1);
        let u_start = (u1 - g1 / slope).max(0.0);
        let u_end = u1 + (1.0 - g1) / slope;
        let span = self.end - self.begin;
        (self.begin + u_start * span, self.begin + u_end * span)
    }

    /// Fog amount of the fitted Bevy linear ramp at depth `d`.
    pub fn linear_amount(&self, d: f32) -> f32 {
        let (s, e) = self.fit_linear();
        ((d - s) / (e - s).max(1e-3)).clamp(0.0, 1.0) * self.density
    }
}

/// Plain copy of the applied parameters (dump).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DepthFogSnapshot {
    pub depth: DepthFog,
    pub start: f32,
    pub end: f32,
}

/// Smoothed state applied to the camera.
#[derive(Resource, Debug, Clone, Default)]
pub struct Atmosphere {
    pub color: [f32; 3],
    pub fog: Option<DepthFogSnapshot>,
    pub applied: bool,
}

fn load_look(mut commands: Commands) {
    let look = crate::paths::find_root()
        .map(|root| FogLook::load(&root))
        .unwrap_or_default();
    commands.insert_resource(look);
}

#[allow(clippy::too_many_arguments)]
fn update_atmosphere(
    mut commands: Commands,
    time: Res<Time<Real>>,
    look: Res<FogLook>,
    view: Res<WeatherView>,
    rig: Option<Res<CampaignRig>>,
    weights: Option<Res<SeasonWeights>>,
    mut state: ResMut<Atmosphere>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    cameras: Query<(Entity, Option<&DistanceFog>), With<CampaignCamera>>,
) {
    let distance = rig.as_ref().map_or(view.distance, |r| r.distance);
    if distance <= 0.0 {
        return;
    }
    let pitch = rig.as_ref().map_or(60.0, |r| r.pitch_deg());
    let target = look.color(weights.map_or(Vec4::new(1.0, 0.0, 0.0, 0.0), |w| w.weights));
    let t = if state.fog.is_none() {
        1.0
    } else {
        (EASE_PER_SECOND * time.delta_secs()).clamp(0.0, 1.0)
    };
    for (i, c) in target.to_array().iter().enumerate() {
        state.color[i] += (c - state.color[i]) * t;
    }
    let depth = DepthFog::at(&look, distance, pitch);
    let (start, end) = depth.fit_linear();
    state.fog = Some(DepthFogSnapshot { depth, start, end });
    let fog = DistanceFog {
        color: Color::srgba(
            state.color[0],
            state.color[1],
            state.color[2],
            depth.density,
        ),
        directional_light_color: Color::NONE,
        directional_light_exponent: 30.0,
        falloff: FogFalloff::Linear { start, end },
    };
    state.applied = false;
    for (entity, current) in &cameras {
        let same = current.is_some_and(|c| {
            c.color == fog.color
                && matches!(c.falloff, FogFalloff::Linear { start: s, end: e }
                    if (s - start).abs() < 1e-3 && (e - end).abs() < 1e-3)
        });
        if !same {
            commands.entity(entity).try_insert(fog.clone());
        }
        state.applied = true;
    }
    // One layer only: the terrain shader's own depth fog is off, it applies the camera fog.
    let stale = materials
        .iter()
        .filter(|(_, m)| m.params.fog.w != 0.0)
        .map(|(id, _)| id)
        .collect::<Vec<_>>();
    for id in stale {
        if let Some(mut material) = materials.get_mut(id) {
            material.params.fog.w = 0.0;
        }
    }
}

fn reset(mut state: ResMut<Atmosphere>) {
    *state = Atmosphere::default();
}

/// Effective parameters at the reference camera distances (comparison table of
/// `docs/wip/bm-weather.md`), pitch following the rig's own curve.
fn table(world: &World) -> Value {
    let (Some(look), Some(rig)) = (
        world.get_resource::<FogLook>(),
        world.get_resource::<CampaignRig>(),
    ) else {
        return Value::Null;
    };
    let rows: Vec<Value> = [2600.0_f32, 1200.0, 600.0, 200.0, 60.0]
        .iter()
        .map(|d| {
            let mut r = rig.clone();
            r.distance = *d;
            let pitch = r.pitch_deg();
            let fog = DepthFog::at(look, *d, pitch);
            let (start, end) = fog.fit_linear();
            let at = |depth: f32| {
                json!({
                    "depth": depth,
                    "godot": fog.amount(depth),
                    "bevy": fog.linear_amount(depth),
                })
            };
            json!({
                "distance": d, "pitch": pitch, "begin": fog.begin, "end": fog.end,
                "density": fog.density, "bevy_start": start, "bevy_end": end,
                "at_focus": at(*d), "at_u50": at(fog.begin + 0.5 * (fog.end - fog.begin)),
                "at_u75": at(fog.begin + 0.75 * (fog.end - fog.begin)), "at_end": at(fog.end),
            })
        })
        .collect();
    Value::Array(rows)
}

pub fn dump(world: &World) -> Value {
    let a = world.resource::<Atmosphere>();
    json!({
        "fog_color": a.color,
        "fog": a.fog.map(|f| json!({
            "begin": f.depth.begin, "end": f.depth.end, "density": f.depth.density,
            "curve": f.depth.curve, "bevy_start": f.start, "bevy_end": f.end,
        })),
        "applied_to_camera": a.applied,
        "table": table(world),
    })
}

pub(super) fn build(app: &mut App) {
    app.init_resource::<Atmosphere>()
        .init_resource::<FogLook>()
        .add_systems(OnEnter(AppState::Campaign), (reset, load_look))
        .add_systems(
            Update,
            update_atmosphere
                .after(super::update_view)
                .run_if(in_state(AppState::Campaign)),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_godot_scene_numbers() {
        let look = FogLook::default();
        // Far view from above (pitch >= 60): top-view pair of the data.
        let far = DepthFog::at(&look, 2600.0, 70.0);
        assert_eq!((far.begin, far.end, far.density), (4940.0, 20800.0, 0.5));
        // Low view (pitch <= 32): the low pair.
        let low = DepthFog::at(&look, 200.0, 30.0);
        assert_eq!((low.begin, low.end, low.density), (250.0, 1100.0, 0.85));
    }

    #[test]
    fn close_view_keeps_the_near_clip_values() {
        let look = FogLook::default();
        let near = DepthFog::at(&look, 5.0, 30.0);
        assert_eq!((near.begin, near.end), (18.0, 240.0));
    }

    #[test]
    fn fog_is_clear_at_the_focus_and_the_fit_tracks_the_curve() {
        let look = FogLook::default();
        for (d, pitch) in [(2600.0, 70.0), (600.0, 40.0), (60.0, 30.0)] {
            let f = DepthFog::at(&look, d, pitch);
            assert!(f.amount(d) < 0.05, "focus must stay clear at {d}");
            let mut worst = 0.0_f32;
            for i in 0..=20 {
                let depth = f.begin + (f.end - f.begin) * i as f32 / 20.0;
                worst = worst.max((f.amount(depth) - f.linear_amount(depth)).abs());
            }
            assert!(worst < 0.2 * f.density + 1e-3, "fit error {worst}");
        }
    }

    #[test]
    fn colour_blends_the_season_with_the_aerial_blue() {
        let look = FogLook::default();
        let c = look.color(Vec4::new(1.0, 0.0, 0.0, 0.0));
        assert!((c.x - 0.65).abs() < 1e-4 && (c.z - 0.88).abs() < 1e-4);
    }
}
