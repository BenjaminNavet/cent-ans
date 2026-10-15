//! Campaign light (port of `campaign_lighting.gd` + `turn_light.gd` + the `campaign.seasons` presets
//! of `data/fx/atmosphere.json`): the sun of the season (elevation, azimuth, colour, energy) varied
//! from one turn to the next, the cool sky ambient, the colour grade of the Godot environment
//! (saturation, contrast, lift, highlights), and the golden dusk while the end of the turn is resolved
//! (feeds [`SkyDusk`]). Purely visual: it writes the terrain material, the `CampaignSun` light and the
//! grade of the campaign camera, and never reads a rule.

use bevy::prelude::*;
use serde_json::Value;

use super::CampaignSun;
use super::TerrainMaterialHandle;
use super::material::TerrainMaterial;
use super::season::SeasonWeights;
use crate::campaign::hud::flow::TurnFlow;
use crate::campaign::weather::aurora::SkyDusk;
use crate::sim::Sim;

/// Illuminance of the `DirectionalLight` per unit of Godot light energy.
const LUX_PER_ENERGY: f32 = 2_300.0;
/// Stand-in for the filmic tonemap of the Godot environment (which compresses the mid tones the
/// ground is lit with): measured on matching views, the mean ground colour is 0.75 of the raw lit one.
const TERRAIN_EXPOSURE: f32 = 0.78;
/// Dusk of `turn_light.gd`.
const DUSK_ELEVATION: f32 = 14.0;
const DUSK_AZIMUTH: f32 = 255.0;
const DUSK_COLOR: Vec3 = Vec3::new(1.0, 0.72, 0.42);
const DUSK_ENERGY_DROP: f32 = 0.18;
const DUSK_FADE_IN: f32 = 0.7;
const DUSK_FADE_OUT: f32 = 1.6;

/// Look of one season (`campaign.seasons.<season>` of `atmosphere.json`).
#[derive(Clone, Copy, Debug)]
pub struct SeasonLight {
    pub elevation: f32,
    pub azimuth: f32,
    pub color: Vec3,
    pub energy: f32,
    pub saturation: f32,
    pub shadow_saturation: f32,
    pub contrast: f32,
    pub lift: Vec3,
    pub highlights: Vec3,
}

impl Default for SeasonLight {
    fn default() -> Self {
        SeasonLight {
            elevation: 20.0,
            azimuth: 245.0,
            color: Vec3::new(1.0, 0.86, 0.66),
            energy: 1.5,
            saturation: 1.0,
            shadow_saturation: 1.0,
            contrast: 1.0,
            lift: Vec3::ZERO,
            highlights: Vec3::ONE,
        }
    }
}

/// Everything read from the two data files.
#[derive(Resource, Clone, Debug)]
pub struct LightData {
    pub seasons: [SeasonLight; 4],
    pub grade_strength: f32,
    pub ambient_color: [Vec3; 4],
    pub ambient_energy: [f32; 4],
    pub bounds: [f32; 2],
    pub variation: Option<[f32; 4]>,
}

impl Default for LightData {
    fn default() -> Self {
        LightData {
            seasons: [SeasonLight::default(); 4],
            grade_strength: 0.0,
            ambient_color: [Vec3::new(0.54, 0.65, 0.92); 4],
            ambient_energy: [0.56; 4],
            bounds: [15.0, 28.0],
            variation: None,
        }
    }
}

fn vec3(value: Option<&Value>, fallback: Vec3) -> Vec3 {
    match value.and_then(Value::as_array) {
        Some(a) if a.len() >= 3 => Vec3::new(
            a[0].as_f64().unwrap_or(0.0) as f32,
            a[1].as_f64().unwrap_or(0.0) as f32,
            a[2].as_f64().unwrap_or(0.0) as f32,
        ),
        _ => fallback,
    }
}

fn number(value: Option<&Value>, fallback: f32) -> f32 {
    value.and_then(Value::as_f64).map_or(fallback, |v| v as f32)
}

impl LightData {
    pub fn from_json(atmosphere: &Value, lighting: &Value) -> LightData {
        let mut data = LightData::default();
        let campaign = &atmosphere["campaign"];
        data.grade_strength = number(campaign.get("grade_strength"), 0.0);
        for (i, name) in super::season::SEASONS.iter().enumerate() {
            let s = &campaign["seasons"][*name];
            if s.is_null() {
                continue;
            }
            let d = SeasonLight::default();
            let g = &s["grade"];
            data.seasons[i] = SeasonLight {
                elevation: number(s.get("sun_elevation"), d.elevation),
                azimuth: number(s.get("sun_azimuth"), d.azimuth),
                color: vec3(s.get("sun_color"), d.color),
                energy: number(s.get("sun_energy"), d.energy),
                saturation: number(g.get("saturation"), 1.0),
                shadow_saturation: number(g.get("shadow_saturation"), 1.0),
                contrast: number(g.get("contrast"), 1.0),
                lift: vec3(g.get("lift"), Vec3::ZERO),
                highlights: vec3(g.get("highlights"), Vec3::ONE),
            };
        }
        let ambient = &lighting["ambient"];
        let base_color = vec3(ambient.get("color"), data.ambient_color[0]);
        let base_energy = number(ambient.get("energy"), 0.56);
        for (i, name) in super::season::SEASONS.iter().enumerate() {
            let over = &ambient["seasons"][*name];
            data.ambient_color[i] = vec3(over.get("color"), base_color);
            data.ambient_energy[i] = number(over.get("energy"), base_energy);
        }
        if let Some(b) = lighting["elevation_bounds_deg"].as_array()
            && b.len() == 2
        {
            data.bounds = [
                b[0].as_f64().unwrap_or(15.0) as f32,
                b[1].as_f64().unwrap_or(28.0) as f32,
            ];
        }
        let v = &lighting["turn_variation"];
        if v["enabled"].as_bool() == Some(true) {
            data.variation = Some([
                number(v.get("elevation_deg"), 0.0),
                number(v.get("azimuth_deg"), 0.0),
                number(v.get("warmth"), 0.0),
                number(v.get("energy"), 0.0),
            ]);
        }
        data
    }

    pub fn load(root: &std::path::Path) -> LightData {
        let read = |path: &str| {
            std::fs::read_to_string(root.join(path))
                .ok()
                .and_then(|t| serde_json::from_str::<Value>(&t).ok())
                .unwrap_or(Value::Null)
        };
        LightData::from_json(
            &read("data/fx/atmosphere.json"),
            &read("data/fx/campaign_lighting.json"),
        )
    }
}

/// Phase of the turn in [-1, 1] (`CampaignLighting.turn_phase`).
pub fn turn_phase(turn: u32) -> f32 {
    let t = turn as f32;
    (0.65 * (t * 2.39996).sin() + 0.35 * (t * 0.91 + 1.3).sin()).clamp(-1.0, 1.0)
}

/// Unit vector towards the sun (Godot convention of `turn_light.gd`: azimuth measured from north,
/// x east, z south).
pub fn toward_sun(elevation_deg: f32, azimuth_deg: f32) -> Vec3 {
    let (e, a) = (elevation_deg.to_radians(), azimuth_deg.to_radians());
    Vec3::new(a.sin() * e.cos(), e.sin(), -a.cos() * e.cos()).normalize()
}

/// Light of the frame, target and smoothed.
#[derive(Resource, Clone, Copy, Debug)]
pub struct CampaignLight {
    pub elevation: f32,
    pub azimuth: f32,
    pub color: Vec3,
    pub energy: f32,
    pub ambient: Vec3,
    pub saturation: f32,
    pub shadow_saturation: f32,
    pub contrast: f32,
    pub lift: Vec3,
    pub highlights: Vec3,
    pub strength: f32,
    pub dusk: f32,
}

impl Default for CampaignLight {
    fn default() -> Self {
        CampaignLight {
            elevation: 34.0,
            azimuth: 245.0,
            color: Vec3::new(1.0, 0.95, 0.86),
            energy: 1.1,
            ambient: Vec3::new(0.495, 0.4675, 0.418),
            saturation: 1.0,
            shadow_saturation: 1.0,
            contrast: 1.0,
            lift: Vec3::ZERO,
            highlights: Vec3::ONE,
            strength: 0.0,
            dusk: 0.0,
        }
    }
}

fn blend<T>(values: &[T; 4], w: Vec4, get: impl Fn(&T) -> f32) -> f32 {
    (0..4).map(|i| get(&values[i]) * w[i]).sum()
}

fn blend3<T>(values: &[T; 4], w: Vec4, get: impl Fn(&T) -> Vec3) -> Vec3 {
    (0..4).map(|i| get(&values[i]) * w[i]).sum()
}

/// Light aimed at for the season weights `w` on turn `turn` (before the dusk).
pub fn target(data: &LightData, w: Vec4, turn: Option<u32>) -> CampaignLight {
    let mut elevation = blend(&data.seasons, w, |s| s.elevation);
    let mut azimuth = blend(&data.seasons, w, |s| s.azimuth);
    let mut color = blend3(&data.seasons, w, |s| s.color);
    let mut energy = blend(&data.seasons, w, |s| s.energy);
    if let (Some([de, da, warm, de_energy]), Some(turn)) = (data.variation, turn) {
        let p = turn_phase(turn);
        elevation -= p * de;
        azimuth += p * da;
        let warmth = p * warm;
        color = Vec3::new(
            color.x,
            (color.y * (1.0 - 0.4 * warmth)).clamp(0.0, 1.0),
            (color.z * (1.0 - warmth)).clamp(0.0, 1.0),
        );
        energy *= 1.0 - p * de_energy;
    }
    elevation = elevation.clamp(data.bounds[0], data.bounds[1]);
    let ambient_color = blend3(&data.ambient_color, w, |c| *c);
    let ambient_energy = blend(&data.ambient_energy, w, |e| *e);
    CampaignLight {
        elevation,
        azimuth,
        color,
        energy,
        ambient: ambient_color * ambient_energy,
        saturation: blend(&data.seasons, w, |s| s.saturation),
        shadow_saturation: blend(&data.seasons, w, |s| s.shadow_saturation),
        contrast: blend(&data.seasons, w, |s| s.contrast),
        lift: blend3(&data.seasons, w, |s| s.lift),
        highlights: blend3(&data.seasons, w, |s| s.highlights),
        strength: data.grade_strength,
        dusk: 0.0,
    }
}

/// Light after the dusk of `dusk` in [0, 1] (`TurnLight._apply`).
pub fn with_dusk(light: &CampaignLight, dusk: f32) -> CampaignLight {
    let t = dusk * dusk * (3.0 - 2.0 * dusk);
    let mut out = *light;
    out.elevation = light.elevation + (DUSK_ELEVATION - light.elevation) * t;
    let mut delta = (DUSK_AZIMUTH - light.azimuth) % 360.0;
    if delta > 180.0 {
        delta -= 360.0;
    } else if delta < -180.0 {
        delta += 360.0;
    }
    out.azimuth = light.azimuth + delta * t;
    out.color = light.color.lerp(DUSK_COLOR, t);
    out.energy = light.energy * (1.0 - DUSK_ENERGY_DROP * t);
    out.dusk = dusk;
    out
}

/// Smoothed light and dusk state.
#[derive(Resource, Default)]
struct LightState {
    smooth: Option<CampaignLight>,
    dusk: f32,
}

fn load_data(mut commands: Commands) {
    let data = crate::paths::find_root()
        .map(|root| LightData::load(&root))
        .unwrap_or_default();
    commands.insert_resource(data);
    commands.insert_resource(LightState::default());
}

#[allow(clippy::too_many_arguments)]
fn update_light(
    time: Res<Time<Real>>,
    data: Option<Res<LightData>>,
    weights: Res<SeasonWeights>,
    sim: Res<Sim>,
    flow: Res<TurnFlow>,
    mut state: ResMut<LightState>,
    mut dusk_out: ResMut<SkyDusk>,
    mut light: ResMut<CampaignLight>,
) {
    let Some(data) = data else { return };
    let dt = time.delta_secs();
    let want = if flow.running() { 1.0 } else { 0.0 };
    let speed = 1.0
        / if want > 0.5 {
            DUSK_FADE_IN
        } else {
            DUSK_FADE_OUT
        };
    state.dusk = if state.dusk < want {
        (state.dusk + speed * dt).min(want)
    } else {
        (state.dusk - speed * dt).max(want)
    };
    dusk_out.0 = state.dusk;
    let aim = target(&data, weights.weights, sim.state.as_ref().map(|s| s.turn()));
    let blended = match state.smooth {
        None => aim,
        Some(current) => {
            // 5 s between two turns, 8 s between two seasons: a short exponential is enough.
            let k = (1.0 - (-dt / 2.0).exp()).clamp(0.0, 1.0);
            CampaignLight {
                elevation: current.elevation + (aim.elevation - current.elevation) * k,
                azimuth: current.azimuth + (aim.azimuth - current.azimuth) * k,
                color: current.color.lerp(aim.color, k),
                energy: current.energy + (aim.energy - current.energy) * k,
                ambient: current.ambient.lerp(aim.ambient, k),
                saturation: current.saturation + (aim.saturation - current.saturation) * k,
                shadow_saturation: current.shadow_saturation
                    + (aim.shadow_saturation - current.shadow_saturation) * k,
                contrast: current.contrast + (aim.contrast - current.contrast) * k,
                lift: current.lift.lerp(aim.lift, k),
                highlights: current.highlights.lerp(aim.highlights, k),
                strength: aim.strength,
                dusk: 0.0,
            }
        }
    };
    state.smooth = Some(blended);
    *light = with_dusk(&blended, state.dusk);
}

fn apply_light(
    light: Res<CampaignLight>,
    handle: Option<Res<TerrainMaterialHandle>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
    mut suns: Query<(&mut Transform, &mut DirectionalLight), With<CampaignSun>>,
) {
    if !light.is_changed() {
        return;
    }
    let dir = toward_sun(light.elevation, light.azimuth);
    if let Some(handle) = handle
        && let Some(mut material) = materials.get_mut(&handle.0)
    {
        let p = &mut material.params;
        p.sun_dir = dir.extend(0.0);
        p.sun_color = (light.color * light.energy).extend(0.0);
        p.ambient = light.ambient.extend(TERRAIN_EXPOSURE);
        p.grade = Vec4::new(
            light.saturation,
            light.contrast,
            light.shadow_saturation,
            light.strength,
        );
        p.lift = light.lift.extend(0.0);
        p.highlights = light.highlights.extend(0.0);
    }
    for (mut transform, mut sun) in &mut suns {
        *transform = Transform::IDENTITY.looking_to(-dir, Vec3::Y);
        sun.color = Color::linear_rgb(light.color.x, light.color.y, light.color.z);
        sun.illuminance = light.energy * LUX_PER_ENERGY;
    }
}

pub struct CampaignLightPlugin;

impl Plugin for CampaignLightPlugin {
    fn build(&self, app: &mut App) {
        use crate::app::AppState;
        app.init_resource::<CampaignLight>()
            .add_systems(OnEnter(AppState::Campaign), load_data)
            .add_systems(
                PreUpdate,
                (update_light, apply_light)
                    .chain()
                    .run_if(in_state(AppState::Campaign))
                    .run_if(resource_exists::<LightState>),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real() -> LightData {
        let atmosphere: Value =
            serde_json::from_str(include_str!("../../../../data/fx/atmosphere.json")).unwrap();
        let lighting: Value =
            serde_json::from_str(include_str!("../../../../data/fx/campaign_lighting.json"))
                .unwrap();
        LightData::from_json(&atmosphere, &lighting)
    }

    #[test]
    fn real_data_gives_a_low_warm_sun_and_a_cool_sky() {
        let data = real();
        let light = target(&data, Vec4::new(0.0, 1.0, 0.0, 0.0), None);
        assert!((light.elevation - 21.0).abs() < 1e-4);
        assert!(light.color.x > light.color.z, "warm sun");
        assert!(light.ambient.z > light.ambient.x, "cool ambient");
        assert!((light.saturation - 0.77).abs() < 1e-4);
        assert!(data.grade_strength > 0.8);
    }

    #[test]
    fn turn_variation_stays_inside_the_bounds() {
        let data = real();
        for turn in 0..200 {
            let l = target(&data, Vec4::new(1.0, 0.0, 0.0, 0.0), Some(turn));
            assert!(l.elevation >= data.bounds[0] && l.elevation <= data.bounds[1]);
        }
    }

    #[test]
    fn dusk_lowers_and_warms_the_sun() {
        let data = real();
        let day = target(&data, Vec4::new(1.0, 0.0, 0.0, 0.0), None);
        let dusk = with_dusk(&day, 1.0);
        assert!(dusk.elevation < day.elevation);
        assert!(dusk.color.z < day.color.z);
        assert!(dusk.energy < day.energy);
        assert_eq!(with_dusk(&day, 0.0).elevation, day.elevation);
    }

    #[test]
    fn sun_comes_from_the_west_south_west() {
        let dir = toward_sun(20.0, 245.0);
        assert!(dir.x < -0.5 && dir.z > 0.0 && dir.y > 0.0);
    }
}
