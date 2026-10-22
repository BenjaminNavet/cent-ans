//! Optional Bevy post-effects (pbr lot, `docs/wip/bevy-pbr.md`): depth of field, motion blur, auto
//! exposure, colour grading, chromatic aberration, contrast adaptive sharpening and PCSS soft shadows.
//!
//! All OFF by default. Each effect is on when the player sets it `on`, or leaves it on `auto` and the
//! quality preset lists it in its `postfx` key (`data/fx/render_quality.json`); the presets list none
//! until an effect is measured. Parameters come from `data/fx/postfx.json`. The effects are added to
//! and removed from the battle and campaign cameras (and the suns for soft shadows); nothing is
//! inserted while an effect is off, so the disabled cost is zero.
//!
//! Colour grading only touches the campaign camera: the battle camera's `ColorGrading` belongs to
//! the battle grade (`battle/scene/lighting.rs`).

use bevy::anti_alias::contrast_adaptive_sharpening::ContrastAdaptiveSharpening;
use bevy::camera::Hdr;
use bevy::core_pipeline::prepass::MotionVectorPrepass;
use bevy::light::DirectionalLight;
use bevy::post_process::auto_exposure::AutoExposure;
use bevy::post_process::dof::{DepthOfField, DepthOfFieldMode};
use bevy::post_process::effect_stack::ChromaticAberration;
use bevy::post_process::motion_blur::MotionBlur;
use bevy::prelude::*;
use bevy::render::view::{ColorGrading, ColorGradingGlobal, ColorGradingSection, Tonemapping};
use serde::Deserialize;
use serde_json::{Value, json};

use super::model::Settings;
use super::quality::RenderQuality;
use crate::battle::scene::camera::BattleCamera;
use crate::campaign::camera::CampaignCamera;

const DATA_PATH: &str = "data/fx/postfx.json";

/// Effect ids: the keys of `postfx` in the presets, and the suffix of the `fx_*` settings.
pub const EFFECTS: [&str; 7] = [
    "depth_of_field",
    "motion_blur",
    "auto_exposure",
    "color_grading",
    "chromatic_aberration",
    "sharpening",
    "soft_shadows",
];

/// Which effects are on (settings over preset).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PostFxFlags {
    pub depth_of_field: bool,
    pub motion_blur: bool,
    pub auto_exposure: bool,
    pub color_grading: bool,
    pub chromatic_aberration: bool,
    pub sharpening: bool,
    pub soft_shadows: bool,
}

impl PostFxFlags {
    /// `on` / `off` of a setting win; anything else (`auto`) follows the preset's list.
    pub fn resolve(settings: &Settings, preset: &[String]) -> Self {
        let pick = |id: &str, setting: &str| match setting {
            "on" => true,
            "off" => false,
            _ => preset.iter().any(|enabled| enabled == id),
        };
        Self {
            depth_of_field: pick("depth_of_field", &settings.fx_depth_of_field),
            motion_blur: pick("motion_blur", &settings.fx_motion_blur),
            auto_exposure: pick("auto_exposure", &settings.fx_auto_exposure),
            color_grading: pick("color_grading", &settings.fx_color_grading),
            chromatic_aberration: pick("chromatic_aberration", &settings.fx_chromatic_aberration),
            sharpening: pick("sharpening", &settings.fx_sharpening),
            soft_shadows: pick("soft_shadows", &settings.fx_soft_shadows),
        }
    }

    pub fn any(&self) -> bool {
        *self != Self::default()
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DepthOfFieldParams {
    pub mode: String,
    pub sensor_height: f32,
    pub aperture_f_stops: f32,
    pub max_circle_of_confusion_diameter: f32,
    pub max_depth: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MotionBlurParams {
    pub shutter_angle: f32,
    pub samples: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AutoExposureParams {
    pub range_min_ev: f32,
    pub range_max_ev: f32,
    pub speed_brighten: f32,
    pub speed_darken: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ColorGradingParams {
    pub exposure: f32,
    pub temperature: f32,
    pub tint: f32,
    pub hue: f32,
    pub post_saturation: f32,
    pub saturation: f32,
    pub contrast: f32,
    pub gamma: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChromaticAberrationParams {
    pub intensity: f32,
    pub max_samples: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SharpeningParams {
    pub strength: f32,
    pub denoise: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SoftShadowParams {
    pub sun_size: f32,
}

/// `data/fx/postfx.json`.
#[derive(Resource, Debug, Clone, Deserialize)]
pub struct PostFxParams {
    pub depth_of_field: DepthOfFieldParams,
    pub motion_blur: MotionBlurParams,
    pub auto_exposure: AutoExposureParams,
    pub color_grading: ColorGradingParams,
    pub chromatic_aberration: ChromaticAberrationParams,
    pub sharpening: SharpeningParams,
    pub soft_shadows: SoftShadowParams,
}

impl PostFxParams {
    /// Reads the data file; `None` when it is missing or invalid (the effects then stay off).
    pub fn load() -> Option<Self> {
        let root = crate::paths::find_root().ok()?;
        let text = std::fs::read_to_string(root.join(DATA_PATH)).ok()?;
        match serde_json::from_str(&text) {
            Ok(params) => Some(params),
            Err(error) => {
                warn!("postfx: {DATA_PATH}: {error}; effects stay off");
                None
            }
        }
    }
}

/// Marks a camera whose `Hdr` was added by the auto-exposure effect (so only that is removed).
#[derive(Component)]
pub struct PostFxHdr;

/// Marks the UI camera made HDR to follow an HDR world camera.
#[derive(Component)]
pub struct PostFxUiHdr;

/// What the effects did, for the dump section `postfx`.
#[derive(Resource, Default)]
pub struct PostFxReport(pub Value);

type WorldCameraFilter = (
    With<Camera3d>,
    Or<(With<BattleCamera>, With<CampaignCamera>)>,
);

/// Distance from the camera to the ground plane along its view axis (focus distance).
fn focus_distance(transform: &Transform, fallback: f32) -> f32 {
    let forward = transform.forward();
    if forward.y < -0.01 {
        (-transform.translation.y / forward.y).clamp(1.0, fallback)
    } else {
        fallback
    }
}

fn depth_of_field(params: &DepthOfFieldParams, focal_distance: f32) -> DepthOfField {
    DepthOfField {
        mode: if params.mode == "bokeh" {
            DepthOfFieldMode::Bokeh
        } else {
            DepthOfFieldMode::Gaussian
        },
        focal_distance,
        sensor_height: params.sensor_height,
        aperture_f_stops: params.aperture_f_stops,
        max_circle_of_confusion_diameter: params.max_circle_of_confusion_diameter,
        max_depth: params.max_depth,
    }
}

fn grading(params: &ColorGradingParams) -> ColorGrading {
    ColorGrading::with_identical_sections(
        ColorGradingGlobal {
            exposure: params.exposure,
            temperature: params.temperature,
            tint: params.tint,
            hue: params.hue,
            post_saturation: params.post_saturation,
            ..default()
        },
        ColorGradingSection {
            saturation: params.saturation,
            contrast: params.contrast,
            gamma: params.gamma,
            ..default()
        },
    )
}

/// Inserts `component` when `wanted` and absent, removes `T` when not wanted and present.
fn toggle<T: Component, B: Bundle>(
    commands: &mut Commands,
    entity: Entity,
    present: bool,
    wanted: bool,
    make: impl FnOnce() -> B,
) {
    if wanted && !present {
        commands.entity(entity).try_insert(make());
    } else if !wanted && present {
        commands.entity(entity).try_remove::<T>();
    }
}

/// Adds or removes the camera effects to match [`RenderQuality::postfx`].
#[allow(clippy::type_complexity)]
pub fn apply_camera_effects(
    mut commands: Commands,
    quality: Res<RenderQuality>,
    params: Option<Res<PostFxParams>>,
    mut cameras: Query<
        (
            Entity,
            &Transform,
            Has<CampaignCamera>,
            Has<Hdr>,
            Has<PostFxHdr>,
            Has<bevy::post_process::bloom::Bloom>,
            Option<&mut DepthOfField>,
            Has<MotionBlur>,
            Has<AutoExposure>,
            Has<ChromaticAberration>,
            Has<ContrastAdaptiveSharpening>,
            Option<&mut ColorGrading>,
        ),
        WorldCameraFilter,
    >,
) {
    // Missing data: every effect stays off.
    let flags = if params.is_some() {
        quality.postfx
    } else {
        PostFxFlags::default()
    };
    for (
        entity,
        transform,
        campaign,
        hdr,
        owns_hdr,
        bloom,
        dof,
        has_motion_blur,
        has_exposure,
        has_aberration,
        has_cas,
        grade,
    ) in &mut cameras
    {
        let Some(params) = params.as_deref() else {
            // Without data, take back anything a previous frame added.
            if dof.is_some() {
                commands.entity(entity).try_remove::<DepthOfField>();
            }
            continue;
        };
        // Depth of field: the focus follows the view axis hitting the ground.
        let focal = focus_distance(transform, params.depth_of_field.max_depth);
        match dof {
            Some(mut dof) if flags.depth_of_field => {
                if (dof.focal_distance - focal).abs() > 0.01 {
                    dof.focal_distance = focal;
                }
            }
            Some(_) => {
                commands.entity(entity).try_remove::<DepthOfField>();
            }
            None if flags.depth_of_field => {
                commands
                    .entity(entity)
                    .try_insert(depth_of_field(&params.depth_of_field, focal));
            }
            None => {}
        }
        toggle::<MotionBlur, _>(
            &mut commands,
            entity,
            has_motion_blur,
            flags.motion_blur,
            || MotionBlur {
                shutter_angle: params.motion_blur.shutter_angle,
                samples: params.motion_blur.samples,
            },
        );
        if !flags.motion_blur && has_motion_blur {
            commands.entity(entity).try_remove::<MotionVectorPrepass>();
        }
        // Auto exposure needs an HDR view (`Hdr` is a required component of it).
        if flags.auto_exposure && !has_exposure {
            let mut camera = commands.entity(entity);
            if !hdr {
                camera.try_insert((PostFxHdr, Hdr));
            }
            camera.try_insert(AutoExposure {
                range: params.auto_exposure.range_min_ev..=params.auto_exposure.range_max_ev,
                speed_brighten: params.auto_exposure.speed_brighten,
                speed_darken: params.auto_exposure.speed_darken,
                ..default()
            });
        } else if !flags.auto_exposure && has_exposure {
            let mut camera = commands.entity(entity);
            camera.try_remove::<AutoExposure>();
            if owns_hdr && !bloom {
                camera.try_remove::<(Hdr, PostFxHdr)>();
            }
        }
        toggle::<ChromaticAberration, _>(
            &mut commands,
            entity,
            has_aberration,
            flags.chromatic_aberration,
            || ChromaticAberration {
                intensity: params.chromatic_aberration.intensity,
                max_samples: params.chromatic_aberration.max_samples,
                ..default()
            },
        );
        toggle::<ContrastAdaptiveSharpening, _>(
            &mut commands,
            entity,
            has_cas,
            flags.sharpening,
            || ContrastAdaptiveSharpening {
                enabled: true,
                sharpening_strength: params.sharpening.strength,
                denoise: params.sharpening.denoise,
            },
        );
        if campaign && let Some(mut grade) = grade {
            let wanted = if flags.color_grading {
                grading(&params.color_grading)
            } else {
                ColorGrading::default()
            };
            if *grade != wanted {
                *grade = wanted;
            }
        }
    }
}

/// While a world scene draws, the UI camera (same target, no clear) must be HDR exactly when a
/// world camera is, or the composition comes out black (see `battle/scene/camera.rs`).
#[allow(clippy::type_complexity)]
pub fn sync_ui_camera_hdr(
    mut commands: Commands,
    world: Query<(&Camera, Has<Hdr>), WorldCameraFilter>,
    ui: Query<
        (Entity, Has<Hdr>, Has<PostFxUiHdr>),
        (With<crate::ui::kit::UiCamera>, Without<Camera3d>),
    >,
) {
    let want = world.iter().any(|(camera, hdr)| camera.is_active && hdr);
    for (entity, hdr, ours) in &ui {
        if want && !hdr {
            commands
                .entity(entity)
                .try_insert((Hdr, Tonemapping::None, PostFxUiHdr));
        } else if !want && hdr && ours {
            commands
                .entity(entity)
                .try_remove::<(Hdr, PostFxUiHdr)>()
                .try_insert(Tonemapping::Linear);
        }
    }
}

/// PCSS: `soft_shadow_size` on the suns that cast shadows.
pub fn apply_soft_shadows(
    quality: Res<RenderQuality>,
    params: Option<Res<PostFxParams>>,
    mut suns: Query<&mut DirectionalLight>,
) {
    let size = params
        .filter(|_| quality.postfx.soft_shadows)
        .map(|params| params.soft_shadows.sun_size);
    for mut sun in &mut suns {
        let wanted = size.filter(|_| sun.shadow_maps_enabled);
        if sun.soft_shadow_size != wanted {
            sun.soft_shadow_size = wanted;
        }
    }
}

/// Fills the dump section.
#[allow(clippy::type_complexity)]
pub fn report(
    mut report: ResMut<PostFxReport>,
    quality: Res<RenderQuality>,
    params: Option<Res<PostFxParams>>,
    cameras: Query<
        (
            Has<CampaignCamera>,
            Has<Hdr>,
            Has<DepthOfField>,
            Has<MotionBlur>,
            Has<AutoExposure>,
            Has<ChromaticAberration>,
            Has<ContrastAdaptiveSharpening>,
            Option<&ColorGrading>,
        ),
        WorldCameraFilter,
    >,
    ui: Query<Has<Hdr>, (With<crate::ui::kit::UiCamera>, Without<Camera3d>)>,
    suns: Query<&DirectionalLight>,
) {
    let flags = quality.postfx;
    let cams: Vec<Value> = cameras
        .iter()
        .map(
            |(campaign, hdr, dof, blur, exposure, aberration, cas, grade)| {
                json!({
                    "kind": if campaign { "campaign" } else { "battle" },
                    "hdr": hdr,
                    "depth_of_field": dof,
                    "motion_blur": blur,
                    "auto_exposure": exposure,
                    "chromatic_aberration": aberration,
                    "sharpening": cas,
                    "color_grading": grade.is_some_and(|g| *g != ColorGrading::default()),
                })
            },
        )
        .collect();
    report.0 = json!({
        "data_loaded": params.is_some(),
        "level": quality.level,
        "flags": {
            "depth_of_field": flags.depth_of_field,
            "motion_blur": flags.motion_blur,
            "auto_exposure": flags.auto_exposure,
            "color_grading": flags.color_grading,
            "chromatic_aberration": flags.chromatic_aberration,
            "sharpening": flags.sharpening,
            "soft_shadows": flags.soft_shadows,
        },
        "cameras": cams,
        "ui_camera_hdr": ui.iter().next(),
        "sun_soft_shadow_size": suns.iter().map(|s| s.soft_shadow_size).collect::<Vec<_>>(),
    });
}

pub fn dump(world: &World) -> (String, Value) {
    (
        "postfx".to_owned(),
        world
            .get_resource::<PostFxReport>()
            .map_or(Value::Null, |report| report.0.clone()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::QualityPresets;

    fn on(id: &str) -> Settings {
        let mut settings = Settings::default();
        match id {
            "depth_of_field" => settings.fx_depth_of_field = "on".into(),
            "motion_blur" => settings.fx_motion_blur = "on".into(),
            "auto_exposure" => settings.fx_auto_exposure = "on".into(),
            "color_grading" => settings.fx_color_grading = "on".into(),
            "chromatic_aberration" => settings.fx_chromatic_aberration = "on".into(),
            "sharpening" => settings.fx_sharpening = "on".into(),
            "soft_shadows" => settings.fx_soft_shadows = "on".into(),
            _ => panic!("unknown effect {id}"),
        }
        settings
    }

    #[test]
    fn data_file_reads_and_presets_enable_nothing() {
        assert!(PostFxParams::load().is_some(), "{DATA_PATH}");
        let presets = QualityPresets::load();
        assert!(presets.from_file);
        for (level, preset) in &presets.levels {
            assert!(
                preset.postfx.is_empty(),
                "{level} enables {:?}",
                preset.postfx
            );
            let quality = RenderQuality::resolve(&Settings::default(), &presets, level);
            assert!(!quality.postfx.any(), "{level}");
        }
        for effect in EFFECTS {
            assert!(
                RenderQuality::resolve(&on(effect), &presets, "high")
                    .postfx
                    .any()
            );
        }
    }

    #[test]
    fn setting_wins_over_preset() {
        let preset = vec!["motion_blur".to_owned(), "sharpening".to_owned()];
        let mut settings = Settings::default();
        let flags = PostFxFlags::resolve(&settings, &preset);
        assert!(flags.motion_blur && flags.sharpening && !flags.depth_of_field);
        settings.fx_motion_blur = "off".into();
        settings.fx_depth_of_field = "on".into();
        let flags = PostFxFlags::resolve(&settings, &preset);
        assert!(!flags.motion_blur && flags.sharpening && flags.depth_of_field);
    }

    #[test]
    fn effects_follow_the_flags_on_the_cameras() {
        let mut app = App::new();
        let presets = QualityPresets::load();
        let mut quality = RenderQuality::resolve(&Settings::default(), &presets, "high");
        app.insert_resource(quality.clone());
        app.insert_resource(PostFxParams::load().expect("data"));
        app.add_systems(Update, (apply_camera_effects, apply_soft_shadows));
        let campaign = app
            .world_mut()
            .spawn((
                Camera3d::default(),
                CampaignCamera,
                ColorGrading::default(),
                Transform::from_xyz(0.0, 100.0, 100.0).looking_at(Vec3::ZERO, Vec3::Y),
            ))
            .id();
        let battle = app
            .world_mut()
            .spawn((
                Camera3d::default(),
                BattleCamera,
                Transform::from_xyz(0.0, 50.0, 0.0).looking_at(Vec3::new(1.0, 50.0, 0.0), Vec3::Y),
            ))
            .id();
        let sun = app
            .world_mut()
            .spawn(DirectionalLight {
                shadow_maps_enabled: true,
                ..default()
            })
            .id();
        app.update();
        app.update();
        let world = app.world();
        for camera in [campaign, battle] {
            let entity = world.entity(camera);
            assert!(!entity.contains::<DepthOfField>());
            assert!(!entity.contains::<MotionBlur>());
            assert!(!entity.contains::<AutoExposure>());
            assert!(!entity.contains::<Hdr>());
        }
        assert_eq!(
            world.get::<DirectionalLight>(sun).unwrap().soft_shadow_size,
            None
        );

        quality.postfx = PostFxFlags {
            depth_of_field: true,
            motion_blur: true,
            auto_exposure: true,
            color_grading: true,
            chromatic_aberration: true,
            sharpening: true,
            soft_shadows: true,
        };
        *app.world_mut().resource_mut::<RenderQuality>() = quality.clone();
        app.update();
        app.update();
        let world = app.world();
        for camera in [campaign, battle] {
            let entity = world.entity(camera);
            assert!(entity.contains::<DepthOfField>());
            assert!(entity.contains::<MotionBlur>());
            assert!(entity.contains::<MotionVectorPrepass>());
            assert!(entity.contains::<AutoExposure>());
            assert!(entity.contains::<Hdr>() && entity.contains::<PostFxHdr>());
            assert!(entity.contains::<ChromaticAberration>());
            assert!(entity.contains::<ContrastAdaptiveSharpening>());
        }
        // 100 m high, looking down at 45 degrees: the ground is 141 m away.
        let focal = world.get::<DepthOfField>(campaign).unwrap().focal_distance;
        assert!((focal - 141.42).abs() < 0.1, "{focal}");
        // The battle camera looks horizontally: the focus falls back to the far limit.
        let focal = world.get::<DepthOfField>(battle).unwrap().focal_distance;
        assert_eq!(focal, 3000.0);
        assert_ne!(
            *world.get::<ColorGrading>(campaign).unwrap(),
            ColorGrading::default()
        );
        assert!(world.get::<ColorGrading>(battle).is_none());
        assert!(
            world
                .get::<DirectionalLight>(sun)
                .unwrap()
                .soft_shadow_size
                .is_some()
        );

        quality.postfx = PostFxFlags::default();
        *app.world_mut().resource_mut::<RenderQuality>() = quality;
        app.update();
        app.update();
        let world = app.world();
        for camera in [campaign, battle] {
            let entity = world.entity(camera);
            assert!(!entity.contains::<DepthOfField>());
            assert!(!entity.contains::<MotionBlur>());
            assert!(!entity.contains::<MotionVectorPrepass>());
            assert!(!entity.contains::<AutoExposure>());
            assert!(!entity.contains::<Hdr>());
            assert!(!entity.contains::<ChromaticAberration>());
            assert!(!entity.contains::<ContrastAdaptiveSharpening>());
        }
        assert_eq!(
            *world.get::<ColorGrading>(campaign).unwrap(),
            ColorGrading::default()
        );
        assert_eq!(
            world.get::<DirectionalLight>(sun).unwrap().soft_shadow_size,
            None
        );
    }

    #[test]
    fn ui_camera_follows_world_hdr() {
        let mut app = App::new();
        app.add_systems(Update, sync_ui_camera_hdr);
        let world_camera = app
            .world_mut()
            .spawn((Camera3d::default(), CampaignCamera, Hdr))
            .id();
        let ui = app
            .world_mut()
            .spawn((Camera2d, crate::ui::kit::UiCamera))
            .id();
        app.update();
        app.update();
        assert!(app.world().entity(ui).contains::<Hdr>());
        app.world_mut().entity_mut(world_camera).remove::<Hdr>();
        app.update();
        app.update();
        assert!(!app.world().entity(ui).contains::<Hdr>());
    }
}
