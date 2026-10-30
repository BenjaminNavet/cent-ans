//! VIS-R1 campaign sky: a gradient dome (zenith -> horizon) drawn behind everything, replacing the
//! flat grey clear colour. Fallback of the Bevy `Atmosphere` prototype (ADR vis-r1: the
//! atmosphere needs an HDR camera, which the campaign post pass does not support, and costs more).
//!
//! The dome is a unit sphere whose vertex stage ignores the model matrix: the shader puts every
//! vertex at the far plane (reverse-Z depth 0) around the camera, so it only shows where nothing
//! else was drawn and never needs a far-plane-sized mesh. Colours:
//! - horizon = the live camera fog colour (`Atmosphere::color`: season, aerial perspective, "fog"
//!   weather), so the far ground dissolves into the very same tone as the sky;
//! - zenith = horizon x (`palette.json` `sky.zenith` / `sky.horizon`), pulled to the horizon
//!   colour under heavy weather (overcast share per kind, `campaign_sky.json`).
//!
//! Settings: `data/fx/campaign_sky.json` (schema `fx_campaign_sky.schema.json`).

use bevy::asset::uuid_handle;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
};
use bevy::shader::{Shader, ShaderRef};
use bevy::state::state_scoped::DespawnOnExit;
use serde_json::{Value, json};

use super::WeatherView;
use super::atmosphere::Atmosphere;
use super::model::Kind;
use crate::app::AppState;
use crate::campaign::terrain::lighting::{CampaignLight, toward_sun};

const DOME_SHADER: Handle<Shader> = uuid_handle!("6f0b1a52-3c1e-4d0a-9d31-0a5f7c1e0f51");

/// Sky settings (`data/fx/campaign_sky.json`) and the palette sky (`palette.json` `sky`).
#[derive(Resource, Debug, Clone)]
pub struct SkyLook {
    pub enabled: bool,
    /// `"dome"` (gradient dome) or `"none"` (flat clear colour, A/B).
    pub kind: String,
    /// sRGB zenith / horizon of the palette (the ratio gives the gradient).
    pub zenith: Vec3,
    pub horizon: Vec3,
    /// Gradient exponent: bigger = the horizon band hugs the horizon.
    pub exponent: f32,
    /// Share of the zenith colour kept under full overcast.
    pub overcast_zenith_keep: f32,
    /// Overcast share by weather kind (clear, fog, rain, snow, storm), times intensity.
    pub overcast: [f32; 5],
    /// Sun glow: strength, exponent.
    pub sun_glow: [f32; 2],
    /// Multiplier on the final sky colour (calibrates the sky against the tonemapped ground).
    pub gain: f32,
    /// L3: share of the sky tint of the aerial perspective in the horizon colour (and in the
    /// far haze of the post pass): the one veil colour of land, sea and sky.
    pub haze_sky_share: f32,
    /// L3: elevation share over which the horizon colour turns to the saturated sky blue.
    pub band_saturation: f32,
    pub clouds: CloudLook,
}

/// Campaign sky clouds (L3): a procedural layer on the dome, seen from the strategic camera
/// above the horizon. Same family as the battle clouds (soft banks, lit side, flat base).
#[derive(Debug, Clone, Copy)]
pub struct CloudLook {
    pub enabled: bool,
    /// Sky coverage in clear weather (0..1), and the extra share added by the overcast.
    pub cover: f32,
    pub overcast_cover: f32,
    /// Edge softness of the banks.
    pub soft: f32,
    /// Bank frequency on the projected plane.
    pub scale: f32,
    /// Elevation (0..1 of the quarter circle) over which the clouds fade in from the horizon.
    pub horizon_fade: f32,
    /// Drift speed (plane units per second), x and z.
    pub drift: [f32; 2],
    /// Lit / shaded colour (sRGB) and the opacity ceiling.
    pub lit: Vec3,
    pub shade: Vec3,
    pub opacity: f32,
}

impl Default for CloudLook {
    fn default() -> Self {
        CloudLook {
            enabled: true,
            cover: 0.42,
            overcast_cover: 0.55,
            soft: 0.22,
            scale: 2.4,
            horizon_fade: 0.12,
            drift: [0.004, 0.0012],
            lit: Vec3::new(0.97, 0.96, 0.93),
            shade: Vec3::new(0.62, 0.68, 0.78),
            opacity: 0.92,
        }
    }
}

impl Default for SkyLook {
    fn default() -> Self {
        SkyLook {
            enabled: true,
            kind: "dome".into(),
            zenith: Vec3::new(0.3, 0.5, 0.8),
            horizon: Vec3::new(0.72, 0.8, 0.88),
            exponent: 0.55,
            overcast_zenith_keep: 0.15,
            overcast: [0.0, 0.9, 0.75, 0.7, 0.95],
            sun_glow: [0.25, 24.0],
            gain: 1.0,
            haze_sky_share: 0.6,
            band_saturation: 0.18,
            clouds: CloudLook::default(),
        }
    }
}

fn vec3_of(value: &Value, fallback: Vec3) -> Vec3 {
    match value.as_array() {
        Some(a) if a.len() >= 3 => Vec3::new(
            a[0].as_f64().unwrap_or(0.0) as f32,
            a[1].as_f64().unwrap_or(0.0) as f32,
            a[2].as_f64().unwrap_or(0.0) as f32,
        ),
        _ => fallback,
    }
}

impl SkyLook {
    /// Colour (sRGB) of the one haze: the live fog colour pulled towards the sky tint of the
    /// aerial perspective. Land, sea (post pass) and the dome horizon all use it.
    pub fn haze_color(&self, fog_color: Vec3, sky_tint: Vec3) -> Vec3 {
        fog_color.lerp(sky_tint, self.haze_sky_share)
    }

    pub fn from_json(sky: &Value, palette: &Value) -> SkyLook {
        let d = SkyLook::default();
        let num = |v: &Value, fb: f32| v.as_f64().map_or(fb, |x| x as f32);
        let mut overcast = d.overcast;
        for (i, kind) in Kind::ALL.iter().enumerate() {
            overcast[i] = num(&sky["overcast_by_kind"][kind.key()], overcast[i]);
        }
        SkyLook {
            enabled: sky["enabled"].as_bool().unwrap_or(d.enabled),
            kind: sky["kind"].as_str().unwrap_or(&d.kind).to_owned(),
            zenith: vec3_of(&palette["sky"]["zenith"], d.zenith),
            horizon: vec3_of(&palette["sky"]["horizon"], d.horizon),
            exponent: num(&sky["gradient_exponent"], d.exponent),
            overcast_zenith_keep: num(&sky["overcast_zenith_keep"], d.overcast_zenith_keep),
            overcast,
            sun_glow: [
                num(&sky["sun_glow"]["strength"], d.sun_glow[0]),
                num(&sky["sun_glow"]["exponent"], d.sun_glow[1]),
            ],
            gain: num(&sky["gain"], d.gain),
            haze_sky_share: num(&sky["haze_sky_share"], d.haze_sky_share).clamp(0.0, 1.0),
            band_saturation: num(&sky["band_saturation"], d.band_saturation).clamp(0.0, 1.0),
            clouds: {
                let c = &sky["clouds"];
                let k = d.clouds;
                CloudLook {
                    enabled: c["enabled"].as_bool().unwrap_or(k.enabled),
                    cover: num(&c["cover"], k.cover).clamp(0.0, 1.0),
                    overcast_cover: num(&c["overcast_cover"], k.overcast_cover).clamp(0.0, 1.0),
                    soft: num(&c["soft"], k.soft).clamp(0.01, 1.0),
                    scale: num(&c["scale"], k.scale).max(0.1),
                    horizon_fade: num(&c["horizon_fade"], k.horizon_fade).clamp(0.0, 1.0),
                    drift: [
                        num(&c["drift"][0], k.drift[0]),
                        num(&c["drift"][1], k.drift[1]),
                    ],
                    lit: vec3_of(&c["lit"], k.lit),
                    shade: vec3_of(&c["shade"], k.shade),
                    opacity: num(&c["opacity"], k.opacity).clamp(0.0, 1.0),
                }
            },
        }
    }

    pub fn load(root: &std::path::Path) -> SkyLook {
        let read = |path: &str| {
            std::fs::read_to_string(root.join(path))
                .ok()
                .and_then(|t| serde_json::from_str::<Value>(&t).ok())
                .unwrap_or(Value::Null)
        };
        SkyLook::from_json(
            &read("data/fx/campaign_sky.json"),
            &read("data/fx/palette.json"),
        )
    }

    /// Overcast share (0..1) of a weather entry.
    pub fn overcast_of(&self, kind: Kind, intensity: f32) -> f32 {
        let i = Kind::ALL.iter().position(|k| *k == kind).unwrap_or(0);
        let base = self.overcast[i];
        if kind == Kind::Clear {
            0.0
        } else {
            (base * (0.6 + 0.4 * intensity.clamp(0.0, 1.0))).clamp(0.0, 1.0)
        }
    }

    /// sRGB zenith for a horizon colour: palette ratio, pulled to the horizon when overcast.
    pub fn zenith_for(&self, horizon: Vec3, overcast: f32) -> Vec3 {
        let ratio = self.zenith / self.horizon.max(Vec3::splat(1e-3));
        let clear = (horizon * ratio).min(Vec3::ONE);
        clear.lerp(horizon, overcast * (1.0 - self.overcast_zenith_keep))
    }
}

/// Sky colour at the vertical component `dir_y` of a unit direction (CPU twin of the shader).
pub fn gradient(horizon: Vec3, zenith: Vec3, dir_y: f32, exponent: f32) -> Vec3 {
    let e = dir_y.clamp(0.0, 1.0).asin() / std::f32::consts::FRAC_PI_2;
    zenith.lerp(horizon, (1.0 - e).powf(1.0 / exponent.max(0.05)))
}

#[derive(ShaderType, Debug, Clone, Copy, Default)]
pub struct DomeParams {
    /// Linear horizon colour (rgb), gradient exponent (a).
    pub horizon: Vec4,
    /// Linear zenith colour (rgb), gain (a).
    pub zenith: Vec4,
    /// Direction towards the sun (xyz), glow strength (w).
    pub sun: Vec4,
    /// Linear sun colour (rgb), glow exponent (a).
    pub sun_color: Vec4,
    /// Clouds: x cover, y softness, z scale, w horizon fade (0 cover = off).
    pub cloud_a: Vec4,
    /// Clouds: xy drift offset (plane units), z opacity, w band saturation.
    pub cloud_b: Vec4,
    /// Clouds: linear lit colour (rgb).
    pub cloud_lit: Vec4,
    /// Clouds: linear shaded colour (rgb).
    pub cloud_shade: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct DomeMaterial {
    #[uniform(0)]
    pub params: DomeParams,
}

impl Material for DomeMaterial {
    fn vertex_shader() -> ShaderRef {
        DOME_SHADER.into()
    }
    fn fragment_shader() -> ShaderRef {
        DOME_SHADER.into()
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        if let Some(depth) = descriptor.depth_stencil.as_mut() {
            // Drawn at depth 0 (the cleared value): never occludes, only fills what is empty.
            depth.depth_write_enabled = Some(false);
        }
        Ok(())
    }
}

#[derive(Component)]
struct SkyDome;

/// What the dome shows now (dump).
#[derive(Resource, Default, Debug, Clone)]
pub struct DomeState {
    pub spawned: bool,
    pub overcast: f32,
    pub horizon: [f32; 3],
    pub zenith: [f32; 3],
}

fn srgb_to_linear_vec(c: Vec3) -> Vec4 {
    let l = Color::srgb(c.x, c.y, c.z).to_linear();
    Vec4::new(l.red, l.green, l.blue, 1.0)
}

fn load_look(mut commands: Commands) {
    let look = crate::paths::find_root()
        .map(|root| SkyLook::load(&root))
        .unwrap_or_default();
    commands.insert_resource(look);
}

fn spawn_dome(
    mut commands: Commands,
    look: Res<SkyLook>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<DomeMaterial>>,
    mut state: ResMut<DomeState>,
    existing: Query<(), With<SkyDome>>,
) {
    if !look.enabled || look.kind != "dome" || !existing.is_empty() {
        return;
    }
    let mesh = meshes.add(Sphere::new(1.0).mesh().uv(48, 24));
    let material = materials.add(DomeMaterial {
        params: DomeParams::default(),
    });
    commands.spawn((
        Name::new("CampaignSkyDome"),
        SkyDome,
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::default(),
        NoFrustumCulling,
        bevy::light::NotShadowCaster,
        bevy::light::NotShadowReceiver,
        DespawnOnExit(AppState::Campaign),
    ));
    state.spawned = true;
}

fn update_dome(
    look: Res<SkyLook>,
    atmosphere: Res<Atmosphere>,
    view: Res<WeatherView>,
    time: Res<Time<Real>>,
    light: Option<Res<CampaignLight>>,
    domes: Query<&MeshMaterial3d<DomeMaterial>, With<SkyDome>>,
    mut materials: ResMut<Assets<DomeMaterial>>,
    mut state: ResMut<DomeState>,
) {
    let Ok(handle) = domes.single() else { return };
    let Some(mut material) = materials.get_mut(&handle.0) else {
        return;
    };
    let horizon = look.haze_color(
        Vec3::from_array(atmosphere.color),
        Vec3::from_array(atmosphere.sky),
    );
    let overcast = look.overcast_of(view.focus_entry.kind, view.focus_entry.intensity);
    let zenith = look.zenith_for(horizon, overcast);
    let (sun_dir, sun_color) = match light {
        Some(l) => (toward_sun(l.elevation, l.azimuth), l.color),
        None => (Vec3::Y, Vec3::ONE),
    };
    let c = look.clouds;
    let seconds = time.elapsed_secs();
    let glow = look.sun_glow[0] * (1.0 - overcast);
    let mut h = srgb_to_linear_vec(horizon);
    h.w = look.exponent;
    let mut z = srgb_to_linear_vec(zenith);
    z.w = look.gain;
    material.params = DomeParams {
        horizon: h,
        zenith: z,
        sun: sun_dir.extend(glow),
        sun_color: sun_color.extend(look.sun_glow[1]),
        cloud_a: Vec4::new(
            if c.enabled {
                (c.cover + c.overcast_cover * overcast).clamp(0.0, 1.0)
            } else {
                0.0
            },
            c.soft,
            c.scale,
            c.horizon_fade,
        ),
        cloud_b: Vec4::new(
            c.drift[0] * seconds,
            c.drift[1] * seconds,
            c.opacity,
            look.band_saturation,
        ),
        cloud_lit: srgb_to_linear_vec(c.lit),
        cloud_shade: srgb_to_linear_vec(c.shade.lerp(horizon * 0.8, overcast * 0.6)),
    };
    state.overcast = overcast;
    state.horizon = horizon.to_array();
    state.zenith = zenith.to_array();
}

fn reset(mut state: ResMut<DomeState>) {
    *state = DomeState::default();
}

/// Sky entries of the weather dump (`sky.kind`, `sky.visible`).
pub fn dump(world: &World) -> Value {
    let look = world.get_resource::<SkyLook>();
    let state = world.get_resource::<DomeState>();
    json!({
        "kind": look.map_or("none", |l| if l.enabled { l.kind.as_str() } else { "none" }),
        "visible": state.is_some_and(|s| s.spawned),
        "overcast": state.map_or(0.0, |s| s.overcast),
        "horizon": state.map(|s| s.horizon),
        "zenith": state.map(|s| s.zenith),
    })
}

pub fn build(app: &mut App) {
    app.init_resource::<DomeState>();
    if let Some(mut shaders) = app.world_mut().get_resource_mut::<Assets<Shader>>()
        && let Err(error) = shaders.insert(
            DOME_SHADER.id(),
            Shader::from_wesl(include_str!("dome.wesl"), "weather/dome.wesl"),
        )
    {
        error!("sky dome shader: {error:?}");
    }
    if !app.world().contains_resource::<Assets<Mesh>>() {
        return;
    }
    app.add_plugins(MaterialPlugin::<DomeMaterial>::default())
        .add_systems(Startup, load_look)
        .add_systems(OnEnter(AppState::Campaign), reset)
        .add_systems(
            Update,
            (spawn_dome, update_dome)
                .chain()
                .run_if(in_state(AppState::Campaign)),
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_gradient_is_bluer_at_the_zenith() {
        let look = SkyLook::default();
        let horizon = Vec3::new(0.74, 0.79, 0.86);
        let zenith = look.zenith_for(horizon, 0.0);
        assert!(zenith.z - zenith.x > 0.2, "blue zenith {zenith}");
        let near = gradient(horizon, zenith, 0.02, look.exponent);
        assert!(near.distance(horizon) < 0.12);
        let top = gradient(horizon, zenith, 1.0, look.exponent);
        assert!(top.distance(zenith) < 1e-4);
    }

    #[test]
    fn horizon_band_is_blue_enough_for_the_audit() {
        // Criterion 2: B > R + 0.08 in the sky near the horizon (0 to 8 degrees up).
        let look = SkyLook::default();
        let horizon = Vec3::new(0.74, 0.79, 0.86);
        let zenith = look.zenith_for(horizon, 0.0);
        for deg in [1.0_f32, 3.0, 6.0] {
            let c = gradient(horizon, zenith, deg.to_radians().sin(), look.exponent);
            assert!(c.z > c.x + 0.08, "{deg} deg: {c}");
        }
    }

    #[test]
    fn overcast_pulls_the_zenith_to_the_horizon() {
        let look = SkyLook::default();
        let horizon = Vec3::new(0.78, 0.8, 0.82);
        let clear = look.zenith_for(horizon, 0.0);
        let grey = look.zenith_for(horizon, 1.0);
        assert!(grey.distance(horizon) < clear.distance(horizon));
        assert_eq!(look.overcast_of(Kind::Clear, 1.0), 0.0);
        assert!(look.overcast_of(Kind::Storm, 1.0) > look.overcast_of(Kind::Rain, 0.2));
    }

    #[test]
    fn real_data_parses() {
        let sky: Value =
            serde_json::from_str(include_str!("../../../../data/fx/campaign_sky.json")).unwrap();
        let palette: Value =
            serde_json::from_str(include_str!("../../../../data/fx/palette.json")).unwrap();
        let look = SkyLook::from_json(&sky, &palette);
        assert!(look.enabled);
        assert_eq!(look.kind, "dome");
        assert!(look.zenith.z > look.horizon.z - 0.2);
    }
}
