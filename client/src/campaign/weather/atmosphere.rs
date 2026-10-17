//! Atmosphere: THE single distance-fog layer of the campaign, a port of the depth fog of the Godot
//! `Environment` (`campaign_map.tscn` + `campaign_atmosphere.gd::apply_distance`,
//! `campaign_lighting.gd::aerial`, `data/fx/campaign_lighting.json` `aerial`, season `fog_color` of
//! `data/fx/atmosphere.json`). Godot's weather adds NO distance fog (only a ground veil capped at
//! `weather_mist_max` = 0.07 in the terrain shader, ADR 0142: the mist cap was the real cause of
//! the washed-out look), so rain / snow / storm / morning fog no longer touch this layer.
//! One exception (ADR 0596, amends 0142): the "fog" weather at the camera focus pulls the depth
//! fog in (`weather_fog` of `data/fx/atmosphere.json`), eased in and out; every other weather and
//! clear skies keep the layer untouched.
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
use crate::campaign::terrain::lighting::CampaignLight;
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
    /// `aerial.aerial_perspective` `[low view, top view]`: share of the sky tint in the fog.
    pub aerial: [f32; 2],
    /// Sky tint of the aerial perspective (`ambient.color` of the lighting data: the cold sky).
    pub sky: Vec3,
    /// Season `fog_sun_scatter`, spring..winter.
    pub sun_scatter: [f32; 4],
    /// Sharpness of the sun lobe in the fog (Bevy `directional_light_exponent`).
    pub scatter_exponent: f32,
    /// "Fog" weather overrides (`campaign.weather_fog`).
    pub weather_fog: WeatherFog,
}

/// How the "fog" weather reshapes the distance fog at full strength (`campaign.weather_fog`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeatherFog {
    /// Multipliers of the fog begin / end distances.
    pub begin_scale: f32,
    pub end_scale: f32,
    /// Density reached (the larger of this and the clear density applies).
    pub density: f32,
    /// Depth curve reached (< clear curve: the veil thickens sooner).
    pub curve: f32,
    /// Grey-white fog colour (sRGB).
    pub color: Vec3,
    /// Share of the sky tint kept in the fog colour (aerial perspective).
    pub aerial_keep: f32,
    /// Strength at weather intensity 0 (intensity 1 gives full strength).
    pub min_strength: f32,
    /// Seconds to ease in / out when the weather under the camera changes.
    pub ease_seconds: f32,
}

impl Default for WeatherFog {
    fn default() -> Self {
        WeatherFog {
            begin_scale: 0.5,
            end_scale: 0.3,
            density: 0.95,
            curve: 1.0,
            color: Vec3::new(0.78, 0.8, 0.82),
            aerial_keep: 0.25,
            min_strength: 0.7,
            ease_seconds: 2.5,
        }
    }
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
            aerial: [0.6, 0.12],
            sky: Vec3::new(0.54, 0.65, 0.92),
            sun_scatter: [0.3, 0.36, 0.34, 0.2],
            scatter_exponent: 8.0,
            weather_fog: WeatherFog::default(),
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
        for (i, name) in SEASONS.iter().enumerate() {
            if let Some(v) = atmosphere["campaign"]["seasons"][*name]["fog_sun_scatter"].as_f64() {
                out.sun_scatter[i] = v as f32;
            }
        }
        out.sky = vec3(&lighting["ambient"]["color"], out.sky);
        let a = &lighting["aerial"];
        out.aerial = pair(&a["aerial_perspective"], out.aerial);
        out.blue = vec3(&a["blue"], out.blue);
        let num = |v: &Value, d: f32| v.as_f64().map_or(d, |x| x as f32);
        out.blue_mix = num(&a["blue_mix"], out.blue_mix).clamp(0.0, 1.0);
        out.low_pitch = num(&a["low_pitch_deg"], out.low_pitch);
        out.top_pitch = num(&a["top_pitch_deg"], out.top_pitch);
        out.density = pair(&a["density"], out.density);
        out.begin_factor = pair(&a["begin_factor"], out.begin_factor);
        out.end_factor = pair(&a["end_factor"], out.end_factor);
        let w = &atmosphere["campaign"]["weather_fog"];
        let wf = &mut out.weather_fog;
        wf.begin_scale = num(&w["begin_scale"], wf.begin_scale).max(0.05);
        wf.end_scale = num(&w["end_scale"], wf.end_scale).max(0.05);
        wf.density = num(&w["density"], wf.density).clamp(0.0, 1.0);
        wf.curve = num(&w["curve"], wf.curve).max(0.2);
        wf.color = vec3(&w["color"], wf.color);
        wf.aerial_keep = num(&w["aerial_keep"], wf.aerial_keep).clamp(0.0, 1.0);
        wf.min_strength = num(&w["min_strength"], wf.min_strength).clamp(0.0, 1.0);
        wf.ease_seconds = num(&w["ease_seconds"], wf.ease_seconds).max(0.0);
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

    /// Season `fog_sun_scatter` for the season weights.
    pub fn scatter(&self, weights: Vec4) -> f32 {
        let w = weights.to_array();
        let total: f32 = w.iter().sum::<f32>().max(1e-4);
        w.iter()
            .zip(self.sun_scatter)
            .map(|(wi, s)| wi / total * s)
            .sum()
    }

    /// Aerial perspective share (`CampaignLighting.aerial`) at a camera pitch.
    pub fn aerial_at(&self, pitch_deg: f32) -> f32 {
        let low = 1.0 - smoothstep(self.low_pitch, self.top_pitch, pitch_deg);
        self.aerial[1] + (self.aerial[0] - self.aerial[1]) * low
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

/// Weight of the sky tint in the fog for a view direction of vertical component `dir_y`
/// (Godot `fog_aerial_perspective` samples the sky along the view ray: full towards the
/// horizon, nil straight down). Mirrors `wx_aerial_weight` of `ground.wesl`.
pub fn aerial_weight(aerial: f32, dir_y: f32) -> f32 {
    aerial * (1.0 - smoothstep(0.0, 0.8, -dir_y))
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
    /// `campaign_atmosphere.gd::apply_distance` + `CampaignLighting.aerial` (clear weather).
    pub fn at(look: &FogLook, distance: f32, pitch_deg: f32) -> DepthFog {
        DepthFog::at_weather(look, distance, pitch_deg, 0.0)
    }

    /// Same with the "fog" weather at strength `mist` (0 = exactly [`DepthFog::at`]).
    pub fn at_weather(look: &FogLook, distance: f32, pitch_deg: f32, mist: f32) -> DepthFog {
        let close = 1.0 - smoothstep(look.close_full, look.close_begin, distance);
        let low = 1.0 - smoothstep(look.low_pitch, look.top_pitch, pitch_deg);
        let mix = |p: [f32; 2]| p[1] + (p[0] - p[1]) * low;
        let (bf, ef) = (mix(look.begin_factor), mix(look.end_factor));
        let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
        let clear = DepthFog {
            begin: lerp(
                distance * bf,
                look.close_fog_begin.max(distance * bf),
                close,
            ),
            end: lerp(distance * ef, look.close_fog_end.max(distance * ef), close),
            density: mix(look.density),
            curve: look.curve,
        };
        let m = mist.clamp(0.0, 1.0);
        if m <= 0.0 {
            return clear;
        }
        let wf = &look.weather_fog;
        let begin = lerp(clear.begin, clear.begin * wf.begin_scale, m);
        let end = lerp(clear.end, clear.end * wf.end_scale, m);
        DepthFog {
            begin: begin.min(end * 0.9),
            end,
            density: lerp(clear.density, clear.density.max(wf.density), m),
            curve: lerp(clear.curve, wf.curve, m),
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
    /// Aerial perspective share applied to the camera fog colour (centre ray).
    pub aerial: f32,
    /// Season fog colour (sRGB, eased) before the aerial blend: the terrain shader blends it
    /// towards `sky` per pixel with `wx_aerial_fog_color`.
    pub base: [f32; 3],
    /// Unweighted aerial perspective share at the camera pitch (`aerial_at`).
    pub aerial_share: f32,
    /// Sky tint of the aerial perspective (sRGB).
    pub sky: [f32; 3],
    /// Sun scatter amount (`fog_sun_scatter`) applied.
    pub scatter: f32,
    /// Eased strength (0..1) of the "fog" weather under the camera.
    pub mist: f32,
}

/// Target strength of the "fog" weather for the weather at the camera focus.
pub fn mist_target(look: &FogLook, entry: &super::model::Entry) -> f32 {
    if entry.kind == super::model::Kind::Fog {
        let min = look.weather_fog.min_strength;
        min + (1.0 - min) * entry.intensity.clamp(0.0, 1.0)
    } else {
        0.0
    }
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
    light: Option<Res<CampaignLight>>,
    mut state: ResMut<Atmosphere>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    cameras: Query<(Entity, Option<&DistanceFog>), With<CampaignCamera>>,
) {
    let distance = rig.as_ref().map_or(view.distance, |r| r.distance);
    if distance <= 0.0 {
        return;
    }
    let pitch = rig.as_ref().map_or(60.0, |r| r.pitch_deg());
    let season_weights = weights.map_or(Vec4::new(1.0, 0.0, 0.0, 0.0), |w| w.weights);
    // Aerial perspective: the fog takes the sky tint along the camera's centre ray (the terrain
    // shader re-evaluates it per pixel with `wx_aerial_fog_color`).
    let aerial = look.aerial_at(pitch);
    let weight = aerial_weight(aerial, -pitch.to_radians().sin());
    let wf = look.weather_fog;
    let mist_goal = mist_target(&look, &view.focus_entry);
    let mist_rate = if state.fog.is_none() || wf.ease_seconds <= 0.0 {
        1.0
    } else {
        (time.delta_secs() / wf.ease_seconds).clamp(0.0, 1.0)
    };
    state.mist += (mist_goal - state.mist) * mist_rate;
    if (state.mist - mist_goal).abs() < 1e-3 {
        state.mist = mist_goal;
    }
    let mist = state.mist;
    let clear_color = look.color(season_weights).lerp(look.sky, weight);
    let veil = wf.color.lerp(look.sky, wf.aerial_keep * weight);
    let target = clear_color.lerp(veil, mist);
    let scatter = look.scatter(season_weights) * (1.0 - mist);
    let sun_color = light.map_or(Vec3::ONE, |l| l.color);
    let t = if state.fog.is_none() {
        1.0
    } else {
        (EASE_PER_SECOND * time.delta_secs()).clamp(0.0, 1.0)
    };
    for (i, c) in target.to_array().iter().enumerate() {
        state.color[i] += (c - state.color[i]) * t;
    }
    let base = look.color(season_weights).lerp(wf.color, mist);
    for (i, c) in base.to_array().iter().enumerate() {
        state.base[i] += (c - state.base[i]) * t;
    }
    state.aerial_share = aerial;
    state.sky = look
        .sky
        .lerp(wf.color.lerp(look.sky, wf.aerial_keep), mist)
        .to_array();
    state.aerial = weight;
    state.scatter = scatter;
    let depth = DepthFog::at_weather(&look, distance, pitch, mist);
    let (start, end) = depth.fit_linear();
    state.fog = Some(DepthFogSnapshot { depth, start, end });
    let fog = DistanceFog {
        color: Color::srgba(
            state.color[0],
            state.color[1],
            state.color[2],
            depth.density,
        ),
        directional_light_color: Color::linear_rgba(
            sun_color.x * scatter,
            sun_color.y * scatter,
            sun_color.z * scatter,
            1.0,
        ),
        directional_light_exponent: look.scatter_exponent,
        falloff: FogFalloff::Linear { start, end },
    };
    state.applied = false;
    for (entity, current) in &cameras {
        let same = current.is_some_and(|c| {
            c.color == fog.color
                && c.directional_light_color == fog.directional_light_color
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
        "aerial_weight": a.aerial,
        "sun_scatter": a.scatter,
        "weather_mist": a.mist,
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
    fn fog_weather_pulls_the_fog_in_and_clear_is_unchanged() {
        let look = FogLook::default();
        for (d, pitch) in [(150.0, 45.0), (40.0, 35.0), (600.0, 40.0)] {
            let clear = DepthFog::at(&look, d, pitch);
            assert_eq!(clear, DepthFog::at_weather(&look, d, pitch, 0.0));
            let fog = DepthFog::at_weather(&look, d, pitch, 1.0);
            assert!(fog.end < clear.end && fog.begin < clear.begin);
            let probe = d * 3.0;
            assert!(fog.linear_amount(probe) > clear.linear_amount(probe) + 0.1);
        }
        use super::super::model::{Entry, Kind};
        let entry = |kind| Entry {
            kind,
            intensity: 0.5,
        };
        assert_eq!(mist_target(&look, &entry(Kind::Clear)), 0.0);
        assert_eq!(mist_target(&look, &entry(Kind::Rain)), 0.0);
        assert!(mist_target(&look, &entry(Kind::Fog)) > 0.7);
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
    fn aerial_perspective_follows_pitch_and_view_direction() {
        let look = FogLook::default();
        assert!((look.aerial_at(30.0) - 0.6).abs() < 1e-5);
        assert!((look.aerial_at(70.0) - 0.12).abs() < 1e-5);
        // Looking along the horizon the sky tint is full, straight down it is nil.
        assert!((aerial_weight(0.6, 0.0) - 0.6).abs() < 1e-6);
        assert_eq!(aerial_weight(0.6, -1.0), 0.0);
        assert!(aerial_weight(0.6, -0.5) < aerial_weight(0.6, -0.2));
    }

    #[test]
    fn sun_scatter_is_the_season_value() {
        let look = FogLook::default();
        assert!((look.scatter(Vec4::new(0.0, 1.0, 0.0, 0.0)) - 0.36).abs() < 1e-6);
        assert!((look.scatter(Vec4::new(0.5, 0.0, 0.0, 0.5)) - 0.25).abs() < 1e-6);
        let text = include_str!("../../../../data/fx/atmosphere.json");
        let atmosphere: Value = serde_json::from_str(text).unwrap();
        let look = FogLook::from_json(&atmosphere, &Value::Null);
        assert!((look.sun_scatter[3] - 0.2).abs() < 1e-6);
    }

    #[test]
    fn colour_blends_the_season_with_the_aerial_blue() {
        let look = FogLook::default();
        let c = look.color(Vec4::new(1.0, 0.0, 0.0, 0.0));
        assert!((c.x - 0.65).abs() < 1e-4 && (c.z - 0.88).abs() < 1e-4);
    }
}
