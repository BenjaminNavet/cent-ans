//! Render quality: the player's `quality`, `msaa`, `shadows` and `view_distance` settings turned into
//! Bevy render settings.
//!
//! Ports `game/scripts/visual/render_quality.gd`: presets from `data/fx/render_quality.json`
//! (low / medium / high / ultra), « Automatique » = level of the detected GPU. The resolved values
//! live in the public [`RenderQuality`] resource (other lots read `veg_max_distance`,
//! `model_shadow_distance`, `level`…). This lot applies, to every 3D camera and sun:
//! - **MSAA** (`Msaa` component; `msaa` of the preset in battle, `map_msaa` on the campaign map);
//! - **shadows**: `DirectionalLightShadowMap` size (`shadow_atlas`), `shadow_maps_enabled` of each
//!   sun (never turned on where its lot left it off, see [`SunBase`]), cascade bounds scaled by
//!   `shadow_distance` (`map_shadow_range` on the map);
//! - **view distance**: far plane of each perspective camera scaled by `view_distance` (see
//!   [`FarBase`]; the camera rigs may keep rewriting their own far plane).
//!
//! Not ported (no Bevy equivalent yet): MetalFX upscale, SSAO/SSIL quality, texture packs.

use bevy::light::{CascadeShadowConfig, DirectionalLightShadowMap};
use bevy::prelude::*;
use serde_json::Value;

use super::model::Settings;
use crate::app::AppState;

const DATA_PATH: &str = "data/fx/render_quality.json";
pub const LEVELS: [&str; 4] = ["low", "medium", "high", "ultra"];

/// One preset of `render_quality.json` (only the keys this lot applies or exposes).
#[derive(Debug, Clone, PartialEq)]
pub struct Preset {
    pub msaa: Msaa,
    pub map_msaa: Msaa,
    pub shadow_atlas: usize,
    pub shadow_splits: usize,
    pub shadow_distance: f32,
    pub map_shadow_range: f32,
    pub model_shadow_distance: f32,
    pub veg_max_distance: f32,
    /// Ambient occlusion in battle (`ssao`; it excludes MSAA in Bevy, SMAA takes over).
    pub ssao: bool,
    /// SSAO sample count level (`low`..`ultra`).
    pub ssao_quality: String,
    /// Battle: multiplier of the distance at which regiments become impostors (`battle_lod`).
    pub battle_lod: f32,
}

fn msaa_from(name: &str) -> Option<Msaa> {
    match name {
        "off" => Some(Msaa::Off),
        "2x" => Some(Msaa::Sample2),
        "4x" => Some(Msaa::Sample4),
        _ => None,
    }
}

impl Preset {
    /// The `high` preset of the data file, used when the file cannot be read.
    pub fn fallback() -> Self {
        Self {
            msaa: Msaa::Sample2,
            map_msaa: Msaa::Off,
            shadow_atlas: 8192,
            shadow_splits: 4,
            shadow_distance: 1.0,
            map_shadow_range: 1.0,
            model_shadow_distance: 250.0,
            veg_max_distance: 900.0,
            ssao: true,
            ssao_quality: "high".to_owned(),
            battle_lod: 1.0,
        }
    }

    fn from_json(value: &Value) -> Option<Self> {
        let text = |key: &str| value.get(key).and_then(Value::as_str);
        let number = |key: &str| value.get(key).and_then(Value::as_f64);
        Some(Self {
            msaa: msaa_from(text("msaa")?)?,
            map_msaa: msaa_from(text("map_msaa")?)?,
            shadow_atlas: number("shadow_atlas")? as usize,
            shadow_splits: number("shadow_splits")? as usize,
            shadow_distance: number("shadow_distance")? as f32,
            map_shadow_range: number("map_shadow_range")? as f32,
            model_shadow_distance: number("model_shadow_distance")? as f32,
            veg_max_distance: number("veg_max_distance")? as f32,
            ssao: value.get("ssao").and_then(Value::as_bool).unwrap_or(false),
            ssao_quality: text("ssao_quality").unwrap_or("medium").to_owned(),
            battle_lod: number("battle_lod").unwrap_or(1.0) as f32,
        })
    }
}

/// Presets by level, read once at start-up.
#[derive(Resource, Debug, Clone)]
pub struct QualityPresets {
    pub levels: Vec<(String, Preset)>,
    pub from_file: bool,
}

impl QualityPresets {
    pub fn load() -> Self {
        let parsed = crate::paths::find_root()
            .ok()
            .and_then(|root| std::fs::read_to_string(root.join(DATA_PATH)).ok())
            .and_then(|text| serde_json::from_str::<Value>(&text).ok());
        let levels: Vec<(String, Preset)> = parsed
            .as_ref()
            .and_then(|tree| tree.get("presets"))
            .map(|presets| {
                LEVELS
                    .iter()
                    .filter_map(|level| {
                        let preset = Preset::from_json(presets.get(*level)?)?;
                        Some(((*level).to_owned(), preset))
                    })
                    .collect()
            })
            .unwrap_or_default();
        if levels.len() == LEVELS.len() {
            Self {
                levels,
                from_file: true,
            }
        } else {
            warn!("render quality: {DATA_PATH} unreadable, built-in `high` preset");
            Self {
                levels: LEVELS
                    .iter()
                    .map(|l| ((*l).to_owned(), Preset::fallback()))
                    .collect(),
                from_file: false,
            }
        }
    }

    pub fn get(&self, level: &str) -> &Preset {
        self.levels
            .iter()
            .find(|(name, _)| name == level)
            .or_else(|| self.levels.iter().find(|(name, _)| name == "high"))
            .map(|(_, preset)| preset)
            .expect("presets are never empty")
    }
}

/// Level suggested for a GPU: `high` on a recent Apple Silicon (M2 and later, or M1 Pro / Max /
/// Ultra), `medium` otherwise (`level_for_adapter` of the Godot script).
pub fn level_for_adapter(adapter: &str) -> &'static str {
    let name = adapter.to_lowercase();
    let name = name.trim();
    let Some(rest) = name.strip_prefix("apple m") else {
        return "medium";
    };
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    match digits.parse::<u32>() {
        Ok(generation) if generation >= 2 => "high",
        Ok(_) if ["pro", "max", "ultra"].iter().any(|t| name.contains(t)) => "high",
        _ => "medium",
    }
}

/// Resolved render quality, read by the lots that scale their detail.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct RenderQuality {
    /// `low` .. `ultra` (« Automatique » resolved).
    pub level: String,
    /// Off when `ssao_battle` (SSAO needs it off; SMAA replaces it).
    pub msaa_battle: Msaa,
    /// SSAO + SMAA in battle: the preset asks for SSAO and the player forced no MSAA level.
    pub ssao_battle: bool,
    /// SSAO quality level of the preset (`ssao_quality`).
    pub ssao_quality: String,
    /// Multiplier of the regiments' impostor distance (`battle_lod` of the preset).
    pub battle_lod: f32,
    pub msaa_campaign: Msaa,
    pub shadows: bool,
    pub shadow_map_size: usize,
    pub shadow_cascades: usize,
    /// Multiplier of the cascade distances (battle).
    pub shadow_distance: f32,
    /// Same on the campaign map.
    pub map_shadow_range: f32,
    pub model_shadow_distance: f32,
    pub veg_max_distance: f32,
    /// Multiplier of the camera far planes (`Settings::view_distance`).
    pub view_distance: f32,
}

/// Side of one cascade's shadow map. Godot's `shadow_atlas` is ONE atlas shared by the cascades
/// (4 splits = 4 quadrants of `atlas / 2`); Bevy allocates a full `size` x `size` map PER cascade,
/// so passing the atlas size made 4 x 8192^2 depth maps (1 GB, ~16 ms of campaign shadow fill at
/// d=30). Two splits keep the whole atlas side (Godot packs them as two half-height strips).
pub fn cascade_map_size(atlas: usize, splits: usize) -> usize {
    let atlas = atlas.next_power_of_two();
    if splits >= 3 {
        (atlas / 2).max(1024)
    } else {
        atlas
    }
}

impl RenderQuality {
    pub fn resolve(settings: &Settings, presets: &QualityPresets, auto_level: &str) -> Self {
        let level = if LEVELS.contains(&settings.quality.as_str()) {
            settings.quality.as_str()
        } else {
            auto_level
        };
        let preset = presets.get(level);
        let forced = msaa_from(&settings.msaa);
        let ssao_battle = preset.ssao && forced.is_none();
        Self {
            level: level.to_owned(),
            msaa_battle: if ssao_battle {
                Msaa::Off
            } else {
                forced.unwrap_or(preset.msaa)
            },
            ssao_battle,
            ssao_quality: preset.ssao_quality.clone(),
            battle_lod: preset.battle_lod,
            msaa_campaign: forced.unwrap_or(preset.map_msaa),
            shadows: settings.shadows,
            shadow_map_size: cascade_map_size(preset.shadow_atlas, preset.shadow_splits),
            shadow_cascades: preset.shadow_splits,
            shadow_distance: preset.shadow_distance,
            map_shadow_range: preset.map_shadow_range,
            model_shadow_distance: preset.model_shadow_distance,
            veg_max_distance: preset.veg_max_distance,
            view_distance: settings.view_distance.clamp(0.25, 2.0),
        }
    }
}

/// The adapter name, when the renderer shares it with the main world.
fn adapter_name(world_info: Option<&bevy::render::renderer::RenderAdapterInfo>) -> Option<String> {
    world_info.map(|info| info.name.clone())
}

/// Keeps [`RenderQuality`] in step with the settings.
pub fn resolve_quality(
    settings: Res<Settings>,
    presets: Res<QualityPresets>,
    adapter: Option<Res<bevy::render::renderer::RenderAdapterInfo>>,
    mut quality: ResMut<RenderQuality>,
) {
    if !settings.is_changed() && !adapter.as_ref().is_some_and(|a| a.is_added()) {
        return;
    }
    let auto = match adapter_name(adapter.as_deref()) {
        Some(name) => level_for_adapter(&name),
        None if cfg!(target_os = "macos") => "high",
        None => "medium",
    };
    let wanted = RenderQuality::resolve(&settings, &presets, auto);
    if *quality != wanted {
        *quality = wanted;
    }
}

/// MSAA of every 3D camera: campaign map or battle preset depending on the state.
///
/// The UI camera draws over the world on the same target (window or image) with no clear. Bevy keys
/// the main texture of a view by (target, format, samples): a UI camera whose MSAA differs from the
/// world camera's gets its own fresh (black) texture and the final blit replaces the world with
/// it. Its MSAA was only set once when the scene was entered, so a quality that resolved later
/// (adapter detected after the first frame, settings changed, state switch) left it mismatched.
/// While a world scene runs, the UI camera therefore follows `wanted` every frame.
pub fn apply_msaa(
    quality: Res<RenderQuality>,
    state: Option<Res<State<AppState>>>,
    mut cameras: Query<(&mut Msaa, &Camera), With<Camera3d>>,
    mut ui_cameras: Query<&mut Msaa, (With<crate::ui::kit::UiCamera>, Without<Camera3d>)>,
) {
    let current = state.as_deref().map(|s| *s.get());
    let wanted = match current {
        Some(AppState::Battle) => quality.msaa_battle,
        _ => quality.msaa_campaign,
    };
    let mut world_active = false;
    for (mut msaa, camera) in &mut cameras {
        if camera.is_active {
            world_active = true;
            if *msaa != wanted {
                *msaa = wanted;
            }
        }
    }
    if world_active && matches!(current, Some(AppState::Campaign | AppState::Battle)) {
        for mut msaa in &mut ui_cameras {
            if *msaa != wanted {
                *msaa = wanted;
            }
        }
    }
}

/// What a sun was before this lot touched it.
#[derive(Component, Debug, Clone)]
pub struct SunBase {
    pub shadows: bool,
    pub bounds: Option<Vec<f32>>,
}

/// Far plane of a camera before the view-distance multiplier, and what was last written.
#[derive(Component, Debug, Clone, Copy)]
pub struct FarBase {
    pub base: f32,
    pub applied: f32,
}

pub fn capture_suns(
    mut commands: Commands,
    suns: Query<(Entity, &DirectionalLight, Option<&CascadeShadowConfig>), Without<SunBase>>,
) {
    for (entity, light, cascades) in &suns {
        commands.entity(entity).insert(SunBase {
            shadows: light.shadow_maps_enabled,
            bounds: cascades.map(|c| c.bounds.clone()),
        });
    }
}

pub fn apply_shadows(
    quality: Res<RenderQuality>,
    state: Option<Res<State<AppState>>>,
    mut map: ResMut<DirectionalLightShadowMap>,
    mut suns: Query<(
        &SunBase,
        &mut DirectionalLight,
        Option<&mut CascadeShadowConfig>,
    )>,
) {
    let wanted_size = crate::campaign::perf_switch::value("SHADOW_SIZE")
        .map_or(quality.shadow_map_size, |v| v as usize);
    if map.size != wanted_size {
        map.size = wanted_size;
    }
    let scale = match state.as_deref().map(|s| *s.get()) {
        Some(AppState::Battle) => quality.shadow_distance,
        _ => quality.map_shadow_range,
    };
    for (base, mut light, cascades) in &mut suns {
        let enabled = base.shadows && quality.shadows;
        if light.shadow_maps_enabled != enabled {
            light.shadow_maps_enabled = enabled;
        }
        if let (Some(mut config), Some(bounds)) = (cascades, &base.bounds) {
            let wanted: Vec<f32> = bounds.iter().map(|b| b * scale).collect();
            if config.bounds != wanted {
                config.bounds = wanted;
            }
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn capture_far(
    mut commands: Commands,
    cameras: Query<(Entity, &Projection), (With<Camera3d>, Without<FarBase>)>,
) {
    for (entity, projection) in &cameras {
        if let Projection::Perspective(p) = projection {
            commands.entity(entity).insert(FarBase {
                base: p.far,
                applied: p.far,
            });
        }
    }
}

/// `far = base x view_distance`. A far plane that is not the one written last is a new base
/// (a camera rig that sets its own far plane each frame stays in control of the base).
pub fn apply_view_distance(
    quality: Res<RenderQuality>,
    mut cameras: Query<(&mut Projection, &mut FarBase)>,
) {
    for (mut projection, mut far) in &mut cameras {
        let Projection::Perspective(p) = &mut *projection else {
            continue;
        };
        if (p.far - far.applied).abs() > 1e-3 {
            far.base = p.far;
        }
        let wanted = far.base * quality.view_distance;
        if (p.far - wanted).abs() > 1e-3 {
            p.far = wanted;
        }
        far.applied = wanted;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_camera_follows_the_world_camera_msaa() {
        use crate::ui::kit::UiCamera;
        let mut app = App::new();
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.init_state::<AppState>();
        let mut quality =
            RenderQuality::resolve(&Settings::default(), &QualityPresets::load(), "medium");
        quality.msaa_campaign = Msaa::Sample2;
        quality.msaa_battle = Msaa::Off;
        app.insert_resource(quality);
        app.add_systems(Update, apply_msaa);
        let world_camera = app
            .world_mut()
            .spawn((Camera3d::default(), Camera::default(), Msaa::Sample4))
            .id();
        let ui_camera = app
            .world_mut()
            .spawn((Camera2d, UiCamera, Msaa::Sample4))
            .id();
        // Not in a world scene yet: the UI camera is left alone.
        app.update();
        assert_eq!(app.world().get::<Msaa>(ui_camera), Some(&Msaa::Sample4));
        app.world_mut()
            .resource_mut::<NextState<AppState>>()
            .set(AppState::Campaign);
        app.update();
        app.update();
        assert_eq!(app.world().get::<Msaa>(world_camera), Some(&Msaa::Sample2));
        assert_eq!(app.world().get::<Msaa>(ui_camera), Some(&Msaa::Sample2));
        // The quality resolves differently later (auto level, settings): both follow.
        app.world_mut()
            .resource_mut::<RenderQuality>()
            .msaa_campaign = Msaa::Off;
        app.update();
        assert_eq!(app.world().get::<Msaa>(ui_camera), Some(&Msaa::Off));
    }

    #[test]
    fn adapter_levels() {
        assert_eq!(level_for_adapter("Apple M3 Max"), "high");
        assert_eq!(level_for_adapter("Apple M1"), "medium");
        assert_eq!(level_for_adapter("Apple M1 Pro"), "high");
        assert_eq!(level_for_adapter("Intel UHD"), "medium");
    }

    #[test]
    fn presets_from_the_data_file_and_overrides() {
        let presets = QualityPresets::load();
        assert!(presets.from_file, "data/fx/render_quality.json");
        let low = presets.get("low");
        assert_eq!(low.msaa, Msaa::Off);
        assert_eq!(low.shadow_atlas, 2048);
        let mut settings = Settings::default();
        let auto = RenderQuality::resolve(&settings, &presets, "medium");
        assert_eq!(auto.level, "medium");
        // Medium asks for SSAO: MSAA off in battle (SMAA instead), the map keeps its MSAA.
        assert!(auto.ssao_battle);
        assert_eq!(auto.msaa_battle, Msaa::Off);
        assert!(!RenderQuality::resolve(&settings, &presets, "low").ssao_battle);
        settings.quality = "ultra".into();
        settings.msaa = "4x".into();
        settings.view_distance = 0.5;
        let ultra = RenderQuality::resolve(&settings, &presets, "medium");
        assert_eq!(ultra.level, "ultra");
        assert_eq!(ultra.msaa_campaign, Msaa::Sample4);
        // A forced MSAA level wins over SSAO.
        assert!(!ultra.ssao_battle);
        assert_eq!(ultra.msaa_battle, Msaa::Sample4);
        assert_eq!(ultra.shadow_map_size, 4096);
        assert_eq!(ultra.view_distance, 0.5);
    }
}
