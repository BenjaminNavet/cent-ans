//! Post-tonemap pass of the campaign camera. It started as the Godot environment adjustment
//! (`campaign_map.tscn`: `adjustment_contrast` 1.06, `adjustment_saturation` 1.08, Godot's
//! `apply_bcs` on display values after the tonemapper) plus the season colour grade of
//! `campaign_lighting` (one grade for the whole frame instead of the ground and the foliage
//! grading themselves).
//!
//! TS-T3 (ADR 0710, `data/fx/campaign_post.json`) turns it into a depth-aware pass: it also binds
//! the view and the depth buffer of the campaign camera, rebuilds the world position of every
//! pixel and lays the valley mist (by altitude: the low ground fills, the peaks emerge), the
//! aerial perspective (the far land dissolves into a cold grey-blue) and the cold (bluer, denser
//! haze in winter, under snow, in the north, high up) over the whole world, trees and towns
//! included, then a weighted desaturation that spares the vivid, warm and bright hues. The UI has
//! its own camera and is never touched. Everything fades out on the parchment (ADR 0124).

use bevy::core_pipeline::FullscreenShader;
use bevy::core_pipeline::schedule::Core3d;
use bevy::core_pipeline::{Core3dSystems, tonemapping::tonemapping};
use bevy::prelude::*;
use bevy::render::extract_component::{
    ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
    UniformComponentPlugin,
};
use bevy::render::render_resource::binding_types::{
    sampler, texture_2d, texture_depth_2d, texture_depth_2d_multisampled, uniform_buffer,
};
use bevy::render::render_resource::{
    BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries, CachedRenderPipelineId,
    ColorTargetState, ColorWrites, FragmentState, Operations, PipelineCache,
    RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor, Sampler,
    SamplerBindingType, SamplerDescriptor, ShaderStages, ShaderType, TextureFormat,
    TextureSampleType, TextureUsages,
};
use bevy::render::renderer::{RenderContext, RenderDevice, ViewQuery};
use bevy::render::view::{
    ExtractedView, Msaa, ViewDepthStencilTexture, ViewTarget, ViewUniform, ViewUniformOffset,
    ViewUniforms,
};
use bevy::render::{Render, RenderApp, RenderStartup, RenderSystems};
use bevy::shader::Shader;
use serde_json::{Value, json};
use std::collections::HashMap;

use crate::agent::dump::DumpAppExt;
use crate::campaign::camera::CampaignCamera;

/// `adjustment_contrast` of the campaign environment (fallback of `legacy_adjust.contrast`).
pub const CONTRAST: f32 = 1.06;
/// `adjustment_saturation` of the campaign environment (fallback of `legacy_adjust.saturation`).
pub const SATURATION: f32 = 1.08;

const SHADER_PATH: &str = "embedded://cent_ans_client/campaign/terrain/post.wesl";

/// Post pass of the campaign camera (uniform of `post.wesl`).
#[derive(Component, ExtractComponent, Clone, Copy, Debug, PartialEq, ShaderType)]
#[extract_app(RenderApp)]
pub struct CampaignAdjust {
    /// x contrast, y saturation (legacy `apply_bcs`), z value contrast, w 1 = TS-T3 grade on.
    pub params: Vec4,
    /// Environment grade (`CampaignLight`): x saturation, y contrast, z shadow saturation, w strength.
    pub grade: Vec4,
    pub lift: Vec4,
    pub highlights: Vec4,
    /// x desaturation, y extra green desaturation, zw protected chroma ramp.
    pub desat: Vec4,
    /// x warm protection, yz protected luma ramp, w master strength (0 on the parchment).
    pub desat2: Vec4,
    /// Valley mist: x full altitude (m), y top altitude (m), z opacity, w share near the focus.
    pub mist_a: Vec4,
    /// xy distance ramp (world), z noise scale (1/px), w noise amount.
    pub mist_b: Vec4,
    /// xy noise drift offset (px), z share over the sea, w metres per world unit of height.
    pub mist_c: Vec4,
    /// rgb mist colour (linear), w coldness at the focus.
    pub mist_color: Vec4,
    pub mist_cold: Vec4,
    /// Aerial: x begin, y end (world distance), z strength, w desaturation.
    pub aerial_a: Vec4,
    /// rgb haze colour (linear), w share on the sky.
    pub aerial_color: Vec4,
    pub aerial_cold: Vec4,
    /// x, y altitude (m) ramp of the cold, z its share.
    pub cold_alt: Vec4,
    /// Relative hollows: xy depth (m) ramp, z tap radius (share of the screen height), w plain share.
    pub mist_d: Vec4,
}

impl Default for CampaignAdjust {
    fn default() -> Self {
        CampaignAdjust {
            params: Vec4::new(CONTRAST, SATURATION, 0.0, 0.0),
            grade: Vec4::new(1.0, 1.0, 1.0, 0.0),
            lift: Vec4::ZERO,
            highlights: Vec4::ONE,
            desat: Vec4::new(0.0, 0.0, 1.0, 2.0),
            desat2: Vec4::new(0.0, 2.0, 3.0, 0.0),
            mist_a: Vec4::new(0.0, 1.0, 0.0, 1.0),
            mist_b: Vec4::new(0.0, 1.0, 0.0, 0.0),
            mist_c: Vec4::new(0.0, 0.0, 1.0, 1.0 / super::profile::HEIGHT_SCALE),
            mist_color: Vec4::ONE,
            mist_cold: Vec4::ONE,
            aerial_a: Vec4::new(0.0, 1.0, 0.0, 0.0),
            aerial_color: Vec4::ONE,
            aerial_cold: Vec4::ONE,
            cold_alt: Vec4::new(0.0, 1.0, 0.0, 0.0),
            mist_d: Vec4::new(0.0, 1.0, 0.03, 0.0),
        }
    }
}

/// Valley mist settings (`valley_mist`).
#[derive(Debug, Clone, PartialEq)]
pub struct MistLook {
    pub enabled: bool,
    pub full_m: f32,
    pub top_m: f32,
    pub follow_focus: f32,
    pub clear_opacity: f32,
    pub morning_opacity: f32,
    pub fog_opacity: f32,
    pub max_opacity: f32,
    /// Opacity cap in clear weather (the "fog" weather raises it up to `max_opacity`).
    pub clear_max: f32,
    /// Depth (m) below the neighbourhood from which a hollow starts / is fully misted.
    pub hollow_m: [f32; 2],
    /// Radius of the neighbourhood, share of the screen height.
    pub tap_radius: f32,
    /// Share of the absolute low-ground (plain) mist in clear weather, at morning, in fog.
    pub plain_clear: f32,
    pub plain_morning: f32,
    pub plain_fog: f32,
    pub near_share: f32,
    pub far_ratio: [f32; 2],
    pub noise_scale: f32,
    pub noise_drift: [f32; 2],
    pub noise_amount: f32,
    pub sea_share: f32,
    pub color: Vec3,
    pub cold_color: Vec3,
}

/// Aerial perspective settings (`aerial`).
#[derive(Debug, Clone, PartialEq)]
pub struct AerialLook {
    pub enabled: bool,
    pub begin_ratio: f32,
    pub end_ratio: f32,
    pub strength: f32,
    pub desaturate: f32,
    pub sky_share: f32,
    pub color: Vec3,
    pub cold_color: Vec3,
}

/// Cold settings (`cold`).
#[derive(Debug, Clone, PartialEq)]
pub struct ColdLook {
    pub winter: f32,
    pub snow: f32,
    pub latitude_deg: [f32; 2],
    pub latitude_share: f32,
    pub altitude_m: [f32; 2],
    pub altitude_share: f32,
    pub density_gain: f32,
}

/// Grade settings (`grade`).
#[derive(Debug, Clone, PartialEq)]
pub struct GradeLook {
    pub enabled: bool,
    pub desaturation: f32,
    pub green_desaturation: f32,
    pub protect_chroma: [f32; 2],
    pub warm_protect: f32,
    pub protect_luma: [f32; 2],
    pub value_contrast: f32,
}

/// `data/fx/campaign_post.json`.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct PostLook {
    pub enabled: bool,
    pub contrast: f32,
    pub saturation: f32,
    pub mist: MistLook,
    pub aerial: AerialLook,
    pub cold: ColdLook,
    pub grade: GradeLook,
}

impl Default for PostLook {
    fn default() -> Self {
        PostLook {
            enabled: true,
            contrast: CONTRAST,
            saturation: SATURATION,
            mist: MistLook {
                enabled: true,
                full_m: 120.0,
                top_m: 750.0,
                follow_focus: 0.55,
                clear_opacity: 0.24,
                morning_opacity: 0.3,
                fog_opacity: 0.45,
                max_opacity: 0.82,
                clear_max: 0.3,
                hollow_m: [30.0, 300.0],
                tap_radius: 0.035,
                plain_clear: 0.0,
                plain_morning: 0.25,
                plain_fog: 0.8,
                near_share: 0.55,
                far_ratio: [0.7, 1.8],
                noise_scale: 0.012,
                noise_drift: [0.9, 0.25],
                noise_amount: 0.55,
                sea_share: 0.5,
                color: Vec3::new(0.84, 0.86, 0.88),
                cold_color: Vec3::new(0.72, 0.79, 0.9),
            },
            aerial: AerialLook {
                enabled: true,
                begin_ratio: 1.15,
                end_ratio: 3.6,
                strength: 0.5,
                desaturate: 0.6,
                sky_share: 0.35,
                color: Vec3::new(0.6, 0.66, 0.74),
                cold_color: Vec3::new(0.56, 0.65, 0.79),
            },
            cold: ColdLook {
                winter: 0.9,
                snow: 0.6,
                latitude_deg: [50.0, 62.0],
                latitude_share: 0.6,
                altitude_m: [1400.0, 3200.0],
                altitude_share: 0.5,
                density_gain: 0.45,
            },
            grade: GradeLook {
                enabled: true,
                desaturation: 0.16,
                green_desaturation: 0.22,
                protect_chroma: [0.32, 0.6],
                warm_protect: 0.85,
                protect_luma: [0.88, 1.0],
                value_contrast: 0.22,
            },
        }
    }
}

fn num(v: &Value, fallback: f32) -> f32 {
    v.as_f64().map_or(fallback, |x| x as f32)
}

fn pair(v: &Value, fallback: [f32; 2]) -> [f32; 2] {
    match v.as_array() {
        Some(a) if a.len() == 2 => [num(&a[0], fallback[0]), num(&a[1], fallback[1])],
        _ => fallback,
    }
}

fn rgb(v: &Value, fallback: Vec3) -> Vec3 {
    match v.as_array() {
        Some(a) if a.len() >= 3 => Vec3::new(
            num(&a[0], fallback.x),
            num(&a[1], fallback.y),
            num(&a[2], fallback.z),
        ),
        _ => fallback,
    }
}

impl PostLook {
    pub fn from_json(data: &Value) -> PostLook {
        let mut out = PostLook::default();
        if data.is_null() {
            return out;
        }
        let flag = |v: &Value, d: bool| v.as_bool().unwrap_or(d);
        out.enabled = flag(&data["enabled"], out.enabled);
        out.contrast = num(&data["legacy_adjust"]["contrast"], out.contrast);
        out.saturation = num(&data["legacy_adjust"]["saturation"], out.saturation);
        let m = &data["valley_mist"];
        let mist = &mut out.mist;
        mist.enabled = flag(&m["enabled"], mist.enabled);
        mist.full_m = num(&m["full_m"], mist.full_m);
        mist.top_m = num(&m["top_m"], mist.top_m).max(mist.full_m + 1.0);
        mist.follow_focus = num(&m["follow_focus"], mist.follow_focus);
        mist.clear_opacity = num(&m["clear_opacity"], mist.clear_opacity);
        mist.morning_opacity = num(&m["morning_opacity"], mist.morning_opacity);
        mist.fog_opacity = num(&m["fog_opacity"], mist.fog_opacity);
        mist.max_opacity = num(&m["max_opacity"], mist.max_opacity).clamp(0.0, 1.0);
        mist.clear_max = num(&m["clear_max"], mist.clear_max).clamp(0.0, mist.max_opacity);
        mist.hollow_m = pair(&m["hollow_m"], mist.hollow_m);
        mist.tap_radius = num(&m["tap_radius"], mist.tap_radius).clamp(0.002, 0.2);
        mist.plain_clear = num(&m["plain_clear"], mist.plain_clear);
        mist.plain_morning = num(&m["plain_morning"], mist.plain_morning);
        mist.plain_fog = num(&m["plain_fog"], mist.plain_fog);
        mist.near_share = num(&m["near_share"], mist.near_share);
        mist.far_ratio = pair(&m["far_ratio"], mist.far_ratio);
        mist.noise_scale = num(&m["noise_scale"], mist.noise_scale);
        mist.noise_drift = pair(&m["noise_drift"], mist.noise_drift);
        mist.noise_amount = num(&m["noise_amount"], mist.noise_amount);
        mist.sea_share = num(&m["sea_share"], mist.sea_share);
        mist.color = rgb(&m["color"], mist.color);
        mist.cold_color = rgb(&m["cold_color"], mist.cold_color);
        let a = &data["aerial"];
        let aerial = &mut out.aerial;
        aerial.enabled = flag(&a["enabled"], aerial.enabled);
        aerial.begin_ratio = num(&a["begin_ratio"], aerial.begin_ratio);
        aerial.end_ratio = num(&a["end_ratio"], aerial.end_ratio).max(aerial.begin_ratio + 0.01);
        aerial.strength = num(&a["strength"], aerial.strength);
        aerial.desaturate = num(&a["desaturate"], aerial.desaturate);
        aerial.sky_share = num(&a["sky_share"], aerial.sky_share);
        aerial.color = rgb(&a["color"], aerial.color);
        aerial.cold_color = rgb(&a["cold_color"], aerial.cold_color);
        let c = &data["cold"];
        let cold = &mut out.cold;
        cold.winter = num(&c["winter"], cold.winter);
        cold.snow = num(&c["snow"], cold.snow);
        cold.latitude_deg = pair(&c["latitude_deg"], cold.latitude_deg);
        cold.latitude_share = num(&c["latitude_share"], cold.latitude_share);
        cold.altitude_m = pair(&c["altitude_m"], cold.altitude_m);
        cold.altitude_share = num(&c["altitude_share"], cold.altitude_share);
        cold.density_gain = num(&c["density_gain"], cold.density_gain);
        let g = &data["grade"];
        let grade = &mut out.grade;
        grade.enabled = flag(&g["enabled"], grade.enabled);
        grade.desaturation = num(&g["desaturation"], grade.desaturation);
        grade.green_desaturation = num(&g["green_desaturation"], grade.green_desaturation);
        grade.protect_chroma = pair(&g["protect_chroma"], grade.protect_chroma);
        grade.warm_protect = num(&g["warm_protect"], grade.warm_protect);
        grade.protect_luma = pair(&g["protect_luma"], grade.protect_luma);
        grade.value_contrast = num(&g["value_contrast"], grade.value_contrast);
        out
    }

    pub fn load(root: &std::path::Path) -> PostLook {
        let data = std::fs::read_to_string(root.join("data/fx/campaign_post.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .unwrap_or(Value::Null);
        PostLook::from_json(&data)
    }

    /// Coldness (0..1) from the winter weight, snow at the focus and the latitude of the focus.
    pub fn coldness(&self, winter: f32, snow: f32, latitude: Option<f32>) -> f32 {
        let c = &self.cold;
        let lat = latitude.map_or(0.0, |l| {
            smoothstep(c.latitude_deg[0], c.latitude_deg[1], l) * c.latitude_share
        });
        (winter * c.winter + snow * c.snow + lat).clamp(0.0, 1.0)
    }

    /// Valley-mist opacity for the morning share, the "fog" weather strength and the coldness.
    pub fn mist_opacity(&self, morning: f32, fog: f32, coldness: f32) -> f32 {
        let m = &self.mist;
        if !self.enabled || !m.enabled {
            return 0.0;
        }
        let base = m.clear_opacity
            + m.morning_opacity * morning.clamp(0.0, 1.0)
            + m.fog_opacity * fog.clamp(0.0, 1.0);
        let cap = m.clear_max + (m.max_opacity - m.clear_max) * fog.clamp(0.0, 1.0);
        (base * (1.0 + self.cold.density_gain * coldness)).clamp(0.0, cap)
    }

    /// Share of the plain (absolute low-ground) mist: none in clear weather by default, some at
    /// morning, most in the "fog" weather. The relative hollows are misted whatever the weather.
    pub fn plain_share(&self, morning: f32, fog: f32) -> f32 {
        let m = &self.mist;
        (m.plain_clear + m.plain_morning * morning.clamp(0.0, 1.0) + m.plain_fog * fog.clamp(0.0, 1.0))
            .clamp(0.0, 1.0)
    }
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn linear(c: Vec3) -> Vec3 {
    let l = Color::srgb(c.x, c.y, c.z).to_linear();
    Vec3::new(l.red, l.green, l.blue)
}

/// What the pass applied last (dump).
#[derive(Resource, Debug, Clone, Default)]
pub struct PostStats {
    pub master: f32,
    pub coldness: f32,
    pub mist_opacity: f32,
    pub mist_band_m: [f32; 2],
    pub aerial_band: [f32; 2],
    pub plain_share: f32,
    pub depth_bound: bool,
}

pub fn add_plugin(app: &mut App) {
    bevy::asset::embedded_asset!(app, "post.wesl");
    let look = crate::paths::find_root()
        .map(|root| PostLook::load(&root))
        .unwrap_or_default();
    app.insert_resource(look)
        .init_resource::<PostStats>()
        .add_plugins((
            ExtractComponentPlugin::<CampaignAdjust>::default(),
            UniformComponentPlugin::<CampaignAdjust>::default(),
        ))
        .add_systems(Update, (attach, sync_grade, sync_atmosphere).chain())
        .register_dump(dump);
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        return;
    };
    render_app
        .add_systems(RenderStartup, init_pipeline)
        .add_systems(Render, prepare_pipelines.in_set(RenderSystems::Prepare))
        .add_systems(
            Core3d,
            campaign_post_pass
                .in_set(Core3dSystems::PostProcess)
                .after(tonemapping),
        );
}

/// Campaign cameras that do not carry the pass yet.
type BareCameras = (With<CampaignCamera>, Without<CampaignAdjust>);

/// Puts the pass on every campaign camera that lacks it; the depth buffer must be readable.
fn attach(mut commands: Commands, mut cameras: Query<(Entity, &mut Camera3d), BareCameras>) {
    if crate::campaign::perf_switch::off("post") {
        return;
    }
    for (camera, mut camera_3d) in &mut cameras {
        let usages = TextureUsages::from(camera_3d.depth_texture_usages);
        if !usages.contains(TextureUsages::TEXTURE_BINDING) {
            camera_3d.depth_texture_usages = (usages | TextureUsages::TEXTURE_BINDING).into();
        }
        commands
            .entity(camera)
            .try_insert(CampaignAdjust::default());
    }
}

/// Feeds the season / turn grade of the environment to the pass (the ground and the foliage no
/// longer grade themselves: `TerrainParams::grade.w` stays 0, see `lighting::apply_light`).
fn sync_grade(
    light: Res<super::lighting::CampaignLight>,
    mut passes: Query<&mut CampaignAdjust, With<CampaignCamera>>,
) {
    let grade = Vec4::new(
        light.saturation,
        light.contrast,
        light.shadow_saturation,
        light.strength,
    );
    let lift = light.lift.extend(0.0);
    let highlights = light.highlights.extend(0.0);
    for mut pass in &mut passes {
        if pass.grade != grade || pass.lift != lift || pass.highlights != highlights {
            pass.grade = grade;
            pass.lift = lift;
            pass.highlights = highlights;
        }
    }
}

/// TS-T3: valley mist, aerial perspective, cold and grade from the data, the camera, the season
/// and the weather at the focus.
#[allow(clippy::too_many_arguments)]
fn sync_atmosphere(
    time: Res<Time>,
    look: Res<PostLook>,
    view: Option<Res<crate::campaign::weather::WeatherView>>,
    atmosphere: Option<Res<crate::campaign::weather::atmosphere::Atmosphere>>,
    rig: Option<Res<crate::campaign::camera::rig::CampaignRig>>,
    zoom: Option<Res<crate::campaign::camera::zoom::ZoomView>>,
    relief: Option<Res<super::relief_state::ReliefState>>,
    weights: Option<Res<super::season::SeasonWeights>>,
    map: Option<Res<crate::campaign::provinces::data::ProvinceMap>>,
    mut drift: Local<Vec2>,
    mut stats: ResMut<PostStats>,
    mut passes: Query<&mut CampaignAdjust, With<CampaignCamera>>,
) {
    if passes.is_empty() {
        return;
    }
    let strategic = zoom.as_ref().map_or(0.0, |z| z.strategic.clamp(0.0, 1.0));
    let master = if look.enabled { 1.0 - strategic } else { 0.0 };
    let distance = rig.as_ref().map_or(300.0, |r| r.distance).max(1.0);
    let vscale = relief
        .as_ref()
        .map_or(super::profile::HEIGHT_SCALE, |r| r.vertical_scale)
        .max(1e-5);
    let focus = rig.as_ref().map_or(Vec3::ZERO, |r| r.focus);
    let focus_alt = (focus.y / vscale).max(0.0);
    let winter = weights.as_ref().map_or(0.0, |w| {
        let total = w.weights.x + w.weights.y + w.weights.z + w.weights.w;
        w.weights.w / total.max(1e-4)
    });
    let snow = view.as_ref().map_or(0.0, |v| {
        if v.focus_entry.kind == crate::campaign::weather::Kind::Snow {
            0.5 + 0.5 * v.focus_entry.intensity.clamp(0.0, 1.0)
        } else {
            0.0
        }
    });
    let latitude = map.as_ref().and_then(|m| {
        m.projection
            .px_to_lonlat(Vec2::new(focus.x, focus.z))
            .map(|(_, lat)| lat as f32)
    });
    let coldness = look.coldness(winter, snow, latitude);
    let morning = view.as_ref().map_or(0.0, |v| v.morning_mist);
    let fog = atmosphere.as_ref().map_or(0.0, |a| a.mist);
    let opacity = look.mist_opacity(morning, fog, coldness);
    let m = &look.mist;
    *drift += Vec2::from(m.noise_drift) * time.delta_secs();
    let follow = m.follow_focus * focus_alt;
    let a = &look.aerial;
    let aerial_strength = if look.enabled && a.enabled {
        a.strength
    } else {
        0.0
    };
    let g = &look.grade;
    let grade_on = look.enabled && g.enabled;
    let master_grade = if grade_on { 1.0 - strategic } else { 0.0 };
    let params = Vec4::new(
        look.contrast,
        look.saturation,
        g.value_contrast * master_grade,
        if grade_on { 1.0 } else { 0.0 },
    );
    let desat = Vec4::new(
        g.desaturation * master_grade,
        g.green_desaturation * master_grade,
        g.protect_chroma[0],
        g.protect_chroma[1].max(g.protect_chroma[0] + 1e-3),
    );
    let desat2 = Vec4::new(
        g.warm_protect,
        g.protect_luma[0],
        g.protect_luma[1].max(g.protect_luma[0] + 1e-3),
        master,
    );
    let mist_a = Vec4::new(m.full_m + follow, m.top_m + follow, opacity, m.near_share);
    let mist_b = Vec4::new(
        m.far_ratio[0] * distance,
        m.far_ratio[1].max(m.far_ratio[0] + 0.01) * distance,
        m.noise_scale,
        m.noise_amount,
    );
    let mist_c = Vec4::new(drift.x, drift.y, m.sea_share, 1.0 / vscale);
    let mist_color = linear(m.color).extend(coldness);
    let mist_cold = linear(m.cold_color).extend(0.0);
    let aerial_a = Vec4::new(
        a.begin_ratio * distance,
        a.end_ratio * distance,
        aerial_strength,
        a.desaturate,
    );
    let aerial_color = linear(a.color).extend(a.sky_share);
    let aerial_cold = linear(a.cold_color).extend(0.0);
    let c = &look.cold;
    let mist_d = Vec4::new(
        m.hollow_m[0],
        m.hollow_m[1].max(m.hollow_m[0] + 1.0),
        m.tap_radius,
        look.plain_share(morning, fog),
    );
    let cold_alt = Vec4::new(
        c.altitude_m[0],
        c.altitude_m[1].max(c.altitude_m[0] + 1.0),
        c.altitude_share,
        0.0,
    );
    for mut pass in &mut passes {
        pass.params = params;
        pass.desat = desat;
        pass.desat2 = desat2;
        pass.mist_a = mist_a;
        pass.mist_b = mist_b;
        pass.mist_c = mist_c;
        pass.mist_color = mist_color;
        pass.mist_cold = mist_cold;
        pass.aerial_a = aerial_a;
        pass.aerial_color = aerial_color;
        pass.aerial_cold = aerial_cold;
        pass.cold_alt = cold_alt;
        pass.mist_d = mist_d;
    }
    *stats = PostStats {
        master,
        coldness,
        mist_opacity: opacity,
        mist_band_m: [mist_a.x, mist_a.y],
        aerial_band: [aerial_a.x, aerial_a.y],
        plain_share: mist_d.w,
        depth_bound: true,
    };
}

fn dump(world: &World) -> (String, Value) {
    let look = world.resource::<PostLook>();
    let stats = world.resource::<PostStats>();
    (
        "campaign_post".to_owned(),
        json!({
            "enabled": look.enabled,
            "mist_enabled": look.mist.enabled,
            "aerial_enabled": look.aerial.enabled,
            "grade_enabled": look.grade.enabled,
            "master": stats.master,
            "coldness": stats.coldness,
            "mist_opacity": stats.mist_opacity,
            "mist_band_m": stats.mist_band_m,
            "aerial_band": stats.aerial_band,
            "plain_share": stats.plain_share,
            "attached": stats.depth_bound,
        }),
    )
}

// ---------------------------------------------------------------------------------------------
// Render world: a fullscreen pass that also binds the view uniform and the depth buffer
// (`FullscreenMaterial` binds the colour only).

#[derive(Resource)]
struct PostPipeline {
    single: BindGroupLayoutDescriptor,
    multi: BindGroupLayoutDescriptor,
    sampler: Sampler,
    shader: Handle<Shader>,
    fullscreen: FullscreenShader,
    cache: HashMap<(TextureFormat, bool), CachedRenderPipelineId>,
}

fn layout(multisampled: bool) -> BindGroupLayoutDescriptor {
    let entries = if multisampled {
        BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                uniform_buffer::<CampaignAdjust>(true),
                uniform_buffer::<ViewUniform>(true),
                texture_depth_2d_multisampled(),
            ),
        )
    } else {
        BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                uniform_buffer::<CampaignAdjust>(true),
                uniform_buffer::<ViewUniform>(true),
                texture_depth_2d(),
            ),
        )
    };
    BindGroupLayoutDescriptor::new("campaign_post_layout", &entries)
}

fn init_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
    fullscreen: Res<FullscreenShader>,
) {
    commands.insert_resource(PostPipeline {
        single: layout(false),
        multi: layout(true),
        sampler: render_device.create_sampler(&SamplerDescriptor::default()),
        shader: asset_server.load(SHADER_PATH),
        fullscreen: fullscreen.clone(),
        cache: HashMap::new(),
    });
}

#[derive(Component)]
struct PostPipelineId(CachedRenderPipelineId, bool);

fn prepare_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    pipeline: Option<ResMut<PostPipeline>>,
    views: Query<(Entity, &ExtractedView, &Msaa), With<CampaignAdjust>>,
) {
    let Some(mut pipeline) = pipeline else {
        return;
    };
    for (entity, view, msaa) in &views {
        let multisampled = *msaa != Msaa::Off;
        let key = (view.target_format, multisampled);
        let id = match pipeline.cache.get(&key) {
            Some(id) => *id,
            None => {
                let mut shader_defs = Vec::new();
                if multisampled {
                    shader_defs.push("MULTISAMPLED".into());
                }
                let desc = RenderPipelineDescriptor {
                    label: Some("campaign_post_pipeline".into()),
                    layout: vec![if multisampled {
                        pipeline.multi.clone()
                    } else {
                        pipeline.single.clone()
                    }],
                    vertex: pipeline.fullscreen.to_vertex_state(),
                    fragment: Some(FragmentState {
                        shader: pipeline.shader.clone(),
                        shader_defs,
                        targets: vec![Some(ColorTargetState {
                            format: view.target_format,
                            blend: None,
                            write_mask: ColorWrites::ALL,
                        })],
                        ..default()
                    }),
                    ..default()
                };
                let id = pipeline_cache.queue_render_pipeline(desc);
                pipeline.cache.insert(key, id);
                id
            }
        };
        commands
            .entity(entity)
            .insert(PostPipelineId(id, multisampled));
    }
}

#[allow(clippy::too_many_arguments)]
fn campaign_post_pass(
    view: ViewQuery<(
        &ViewTarget,
        &ViewUniformOffset,
        &ViewDepthStencilTexture,
        &DynamicUniformIndex<CampaignAdjust>,
        &PostPipelineId,
    )>,
    pipeline_cache: Res<PipelineCache>,
    pipeline: Option<Res<PostPipeline>>,
    view_uniforms: Res<ViewUniforms>,
    adjust_uniforms: Res<ComponentUniforms<CampaignAdjust>>,
    mut ctx: RenderContext,
) {
    let (view_target, view_offset, depth, adjust_index, pipeline_id) = view.into_inner();
    let Some(pipeline) = pipeline else {
        return;
    };
    let Some(render_pipeline) = pipeline_cache.get_render_pipeline(pipeline_id.0) else {
        return;
    };
    let (Some(view_binding), Some(adjust_binding)) = (
        view_uniforms.uniforms.binding(),
        adjust_uniforms.uniforms().binding(),
    ) else {
        return;
    };
    let texture = depth.texture();
    if !texture.usage().contains(TextureUsages::TEXTURE_BINDING)
        || (texture.sample_count() > 1) != pipeline_id.1
    {
        return;
    }
    let Some(depth_view) = depth.attachment.depth_stencil_views().depth_only_view() else {
        return;
    };
    let post = view_target.post_process_write();
    let layout = if pipeline_id.1 {
        &pipeline.multi
    } else {
        &pipeline.single
    };
    let bind_group = ctx.render_device().create_bind_group(
        "campaign_post_bind_group",
        &pipeline_cache.get_bind_group_layout(layout),
        &BindGroupEntries::sequential((
            post.source,
            &pipeline.sampler,
            adjust_binding,
            view_binding,
            depth_view,
        )),
    );
    let descriptor = RenderPassDescriptor {
        label: Some("campaign_post_pass"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: post.destination,
            depth_slice: None,
            resolve_target: None,
            ops: Operations::default(),
        })],
        ..default()
    };
    let mut pass = ctx.command_encoder().begin_render_pass(&descriptor);
    pass.set_pipeline(render_pipeline);
    pass.set_bind_group(0, &bind_group, &[adjust_index.index(), view_offset.offset]);
    pass.draw(0..3, 0..1);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data() -> Value {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        serde_json::from_str(
            &std::fs::read_to_string(root.join("data/fx/campaign_post.json")).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn data_file_reads() {
        let look = PostLook::from_json(&data());
        assert!(look.enabled && look.mist.enabled && look.aerial.enabled && look.grade.enabled);
        assert!(look.mist.top_m > look.mist.full_m);
        assert!(look.aerial.end_ratio > look.aerial.begin_ratio);
        assert_eq!(PostLook::from_json(&Value::Null), PostLook::default());
    }

    #[test]
    fn mist_thickens_in_the_morning_fog_and_cold() {
        let look = PostLook::default();
        let clear = look.mist_opacity(0.0, 0.0, 0.0);
        assert!(clear > 0.0);
        assert!(look.mist_opacity(1.0, 0.0, 0.0) > clear);
        assert!(look.mist_opacity(0.0, 1.0, 0.0) > clear);
        assert!(look.mist_opacity(0.0, 0.0, 1.0) > clear);
        assert!(look.mist_opacity(1.0, 1.0, 1.0) <= look.mist.max_opacity);
        // Clear weather stays under its own cap, whatever the morning and the cold.
        assert!(look.mist_opacity(1.0, 0.0, 1.0) <= look.mist.clear_max + 1e-6);
        // A flat plain is not veiled in clear weather, it is at morning and in fog.
        assert_eq!(look.plain_share(0.0, 0.0), 0.0);
        assert!(look.plain_share(0.0, 1.0) > look.plain_share(1.0, 0.0));
        let off = PostLook {
            enabled: false,
            ..PostLook::default()
        };
        assert_eq!(off.mist_opacity(1.0, 1.0, 1.0), 0.0);
    }

    #[test]
    fn coldness_follows_winter_snow_and_north() {
        let look = PostLook::default();
        assert_eq!(look.coldness(0.0, 0.0, Some(45.0)), 0.0);
        assert!(look.coldness(1.0, 0.0, Some(45.0)) > 0.5);
        assert!(look.coldness(0.0, 1.0, None) > 0.0);
        assert!(look.coldness(0.0, 0.0, Some(65.0)) > look.coldness(0.0, 0.0, Some(52.0)));
        assert!(look.coldness(1.0, 1.0, Some(70.0)) <= 1.0);
    }
}
