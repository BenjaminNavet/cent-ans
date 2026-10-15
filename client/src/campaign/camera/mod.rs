//! Campaign 3D camera (zoom levels, pan, orbit) and its input.
//!
//! Ports `game/scripts/map/campaign_camera.gd` (rig: [`rig::CampaignRig`]),
//! `close_camera_profile.gd` (in `terrain::profile`), `ui/camera_feel.gd` ([`rig::CameraFeel`],
//! `data/ui/camera_feel.json`) and the `map_pan_*` / `map_rotate_*` keys of `project.godot`
//! (WASD / arrows pan, Q / E rotate, wheel zoom, middle drag pans, Alt or Shift + middle drag
//! rotates, screen-edge pan while the window is focused, ScrollLock toggles it).
//!
//! The camera renders into the `OffscreenTarget` when present. Script-friendly placement:
//! `CENT_ANS_CAMERA="lon,lat,distance[,yaw_deg]"` (real longitude/latitude through the map
//! projection, distance in map units) or `CENT_ANS_CAMERA="xz:x,z,distance[,yaw_deg]"` (map units).
//! `client/run_bg.sh` starts the app through `open`, which drops the environment: write the same
//! text in `out/camera.txt` (repository root) instead.
//! Dump section `campaign_camera`.

pub mod focus;
pub mod rig;
pub mod zoom;

use bevy::camera::{ClearColorConfig, RenderTarget};
use bevy::input::gestures::PinchGesture;
use bevy::input::mouse::{MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::render::view::Tonemapping;
use bevy::window::PrimaryWindow;
use serde_json::{Value, json};

use crate::agent::OffscreenTarget;
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::paths;
use crate::ui::kit::UiCamera;

use self::rig::{CameraFeel, CampaignRig, Ground, RigInput};
use super::terrain::profile::CloseCameraProfile;
use super::terrain::relief_state::ReliefState;
use super::terrain::surface::Surface;

/// Vertical field of view of the campaign camera (campaign_map.tscn: 55 degrees).
pub const FOV_DEG: f32 = 55.0;
/// Environment variable placing the camera at the start (see module docs).
pub const CAMERA_ENV: &str = "CENT_ANS_CAMERA";
/// File (under the repository root) read when the variable is not set.
pub const CAMERA_FILE: &str = "out/camera.txt";

/// Marker of the campaign 3D camera.
#[derive(Component)]
pub struct CampaignCamera;

/// Set of the systems that move the camera (terrain streaming runs after it).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CameraSet;

/// Request to move the campaign camera (alerts, minimap, army strip, hotkeys). Shared interface
/// fixed before wave 3; the camera lot applies it.
#[derive(Message, Debug, Clone, PartialEq)]
pub enum FocusCamera {
    Province(String),
    Army(String),
    /// Map point (world x, z), keeping the current distance.
    Point(Vec2),
    /// Keep the given army in view while it moves (`None` stops following).
    Follow(Option<String>),
}

pub struct CampaignCameraPlugin;

impl Plugin for CampaignCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<FocusCamera>()
            .add_systems(
                OnEnter(AppState::Campaign),
                (setup_rig, spawn_camera).chain(),
            )
            .add_systems(OnExit(AppState::Campaign), teardown)
            .add_systems(
                Update,
                drive_camera
                    .in_set(CameraSet)
                    .run_if(in_state(AppState::Campaign))
                    .run_if(resource_exists::<CampaignRig>),
            )
            .init_resource::<zoom::ZoomView>()
            .register_dump(dump_camera)
            .register_dump(dump_zoom);
        let tiers = paths::find_root()
            .map(|root| zoom::load_tiers(&root))
            .unwrap_or_default();
        app.insert_resource(tiers).add_systems(
            Update,
            zoom::update_zoom_view
                .after(drive_camera)
                .run_if(in_state(AppState::Campaign)),
        );
        app.add_systems(
            Update,
            apply_camera_settings
                .before(drive_camera)
                .run_if(in_state(AppState::Campaign))
                .run_if(resource_exists::<CampaignRig>),
        );
        app.add_systems(
            Update,
            apply_reduce_motion
                .before(drive_camera)
                .run_if(in_state(AppState::Campaign))
                .run_if(resource_exists::<CampaignRig>),
        );
        focus::add(app);
    }
}

/// Camera placement requested by `CENT_ANS_CAMERA`: (x, z, distance, yaw in radians).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraPlacement {
    pub x: f32,
    pub z: f32,
    pub distance: f32,
    pub yaw: f32,
}

/// Parses the value of [`CAMERA_ENV`]. `to_px` converts (lon, lat) to map units.
pub fn parse_camera_env(
    text: &str,
    to_px: impl Fn(f64, f64) -> Option<[f64; 2]>,
) -> Result<CameraPlacement, String> {
    let (units, rest) = match text.trim().strip_prefix("xz:") {
        Some(rest) => (true, rest),
        None => (false, text.trim()),
    };
    let numbers: Vec<f64> = rest
        .split(',')
        .map(|part| {
            part.trim()
                .parse::<f64>()
                .map_err(|e| format!("{part:?}: {e}"))
        })
        .collect::<Result<_, _>>()?;
    if numbers.len() < 3 || numbers.len() > 4 {
        return Err(format!("expected lon,lat,distance[,yaw_deg]: {text:?}"));
    }
    let [x, z] = if units {
        [numbers[0], numbers[1]]
    } else {
        to_px(numbers[0], numbers[1])
            .ok_or_else(|| format!("({}, {}) is outside the map", numbers[0], numbers[1]))?
    };
    Ok(CameraPlacement {
        x: x as f32,
        z: z as f32,
        distance: numbers[2] as f32,
        yaw: numbers.get(3).copied().unwrap_or(0.0).to_radians() as f32,
    })
}

#[derive(Resource)]
struct SavedUiClear(ClearColorConfig);

fn setup_rig(mut commands: Commands) {
    let root = match paths::find_root() {
        Ok(root) => root,
        Err(message) => {
            error!("campaign camera: {message}");
            return;
        }
    };
    let profile = CloseCameraProfile::load(&root);
    let feel = std::fs::read_to_string(root.join("data/ui/camera_feel.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .map(|json| CameraFeel::from_json(&json))
        .unwrap_or_default();
    let mut rig = CampaignRig::new(profile, feel);
    let map_dir = root.join("data/map");
    let meta = data_model::MapMeta::load(&map_dir);
    let size = match &meta {
        Ok(meta) => Vec2::new(meta.size_px[0] as f32, meta.size_px[1] as f32),
        Err(message) => {
            warn!("campaign camera: map.json: {message}; using 7080 x 6104");
            Vec2::new(7080.0, 6104.0)
        }
    };
    // GC (ADR 0158): maquette towns have no close view; the floor distance comes from the data.
    if let Ok(text) = std::fs::read_to_string(root.join("data/ui/campaign_map.json"))
        && let Ok(json) = serde_json::from_str::<Value>(&text)
    {
        let map = &json["map"];
        if map["town_style"].as_str() == Some("maquette") {
            rig.floor_distance = map["camera_floor_distance"].as_f64().unwrap_or(0.0) as f32;
        }
    }
    rig.setup(size, size.x.max(size.y) * 0.55, &Ground::flat());
    // `open -g` (client/run_bg.sh) does not forward the environment: `out/camera.txt` carries the
    // same value for scripted runs.
    let requested = std::env::var(CAMERA_ENV)
        .ok()
        .or_else(|| std::fs::read_to_string(root.join(CAMERA_FILE)).ok());
    if let Some(text) = requested {
        let to_px = |lon: f64, lat: f64| meta.as_ref().ok().and_then(|m| m.lonlat_to_px(lon, lat));
        match parse_camera_env(&text, to_px) {
            Ok(placement) => {
                rig.target_focus = Vec3::new(placement.x, 0.0, placement.z);
                rig.target_yaw = placement.yaw;
                rig.target_distance = placement
                    .distance
                    .clamp(rig.floor_distance.max(0.1), rig.max_distance);
                rig.snap(&Ground::flat());
            }
            Err(message) => warn!("{CAMERA_ENV}: {message}"),
        }
    }
    commands.insert_resource(rig);
}

fn spawn_camera(
    mut commands: Commands,
    rig: Option<Res<CampaignRig>>,
    target: Option<Res<OffscreenTarget>>,
    mut ui_cameras: Query<&mut Camera, With<UiCamera>>,
) {
    if rig.is_none() {
        return;
    }
    // The UI camera clears the whole target; the world has to be drawn first, then the UI over it.
    if let Ok(mut camera) = ui_cameras.single_mut() {
        commands.insert_resource(SavedUiClear(camera.clear_color));
        camera.clear_color = ClearColorConfig::None;
    }
    let mut camera = commands.spawn((
        Name::new("CampaignCamera"),
        CampaignCamera,
        Camera3d::default(),
        Camera {
            order: -1,
            clear_color: ClearColorConfig::Custom(Color::srgb(0.74, 0.76, 0.76)),
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            fov: FOV_DEG.to_radians(),
            near: 1.0,
            far: 6000.0,
            ..default()
        }),
        Tonemapping::AgX,
        Transform::from_xyz(0.0, 100.0, 0.0),
        DespawnOnExit(AppState::Campaign),
    ));
    if let Some(target) = target {
        camera.insert(RenderTarget::Image(target.0.clone().into()));
    }
}

/// Re-reads the feel when « Réduire les animations » changes.
/// Base speed of the keyboard pan (`FlowController.BASE_PAN_SPEED`), times `camera/speed`.
const BASE_PAN_SPEED: f32 = 1.2;

/// `camera/edge_pan` and `camera/speed` of the settings, on a new rig and whenever they change
/// (the F2-like toggle of the key map only flips the rig until the next change of the setting).
fn apply_camera_settings(
    settings: Option<Res<crate::settings::Settings>>,
    mut rig: ResMut<CampaignRig>,
) {
    let Some(settings) = settings else { return };
    if !(settings.is_changed() || rig.is_added()) {
        return;
    }
    rig.edge_pan_enabled = settings.edge_pan;
    rig.pan_speed = BASE_PAN_SPEED * settings.camera_speed.clamp(0.4, 2.5);
}

fn apply_reduce_motion(
    reduce: Option<Res<crate::ui::kit::modal::ReduceMotion>>,
    mut rig: ResMut<CampaignRig>,
    mut applied: Local<Option<bool>>,
) {
    let wanted = reduce.is_some_and(|r| r.0);
    if *applied == Some(wanted) {
        return;
    }
    let first = applied.is_none();
    *applied = Some(wanted);
    if first && !wanted {
        return;
    }
    let Ok(root) = paths::find_root() else {
        return;
    };
    if let Some(json) = std::fs::read_to_string(root.join("data/ui/camera_feel.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
    {
        rig.feel = CameraFeel::from_json_motion(&json, wanted);
    }
}

fn teardown(
    mut commands: Commands,
    saved: Option<Res<SavedUiClear>>,
    mut ui_cameras: Query<&mut Camera, With<UiCamera>>,
) {
    if let Some(saved) = saved
        && let Ok(mut camera) = ui_cameras.single_mut()
    {
        camera.clear_color = saved.0;
    }
    commands.remove_resource::<SavedUiClear>();
    commands.remove_resource::<CampaignRig>();
}

#[allow(clippy::too_many_arguments)]
fn drive_camera(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    keymap: Res<crate::settings::KeyMap>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<MouseWheel>,
    mut motion: MessageReader<MouseMotion>,
    mut pinch: MessageReader<PinchGesture>,
    window: Query<&Window, With<PrimaryWindow>>,
    surface: Option<Res<Surface>>,
    relief: Option<Res<ReliefState>>,
    mut rig: ResMut<CampaignRig>,
    mut camera: Query<(&mut Transform, &mut Projection, &Camera), With<CampaignCamera>>,
) {
    let window = window.single().ok();
    let mut input = RigInput {
        viewport_height: window.map_or(800.0, |w| w.height()),
        ..default()
    };
    let axis = |plus: &str, minus: &str| {
        f32::from(keymap.pressed(plus, &keys) as u8) - f32::from(keymap.pressed(minus, &keys) as u8)
    };
    input.pan.x = axis("map_pan_right", "map_pan_left");
    input.pan.y = axis("map_pan_down", "map_pan_up");
    input.rotate = axis("map_rotate_right", "map_rotate_left");
    if keymap.just_pressed("map_toggle_edge_pan", &keys) {
        rig.edge_pan_enabled = !rig.edge_pan_enabled;
    }
    if rig.edge_pan_enabled
        && let Some(window) = window
        && window.focused
        && let Some(cursor) = window.cursor_position()
    {
        let (w, h, m) = (window.width(), window.height(), rig.edge_margin_px);
        if cursor.x >= 0.0 && cursor.y >= 0.0 && cursor.x <= w && cursor.y <= h {
            if cursor.x < m {
                input.pan.x -= 1.0;
            } else if cursor.x > w - m {
                input.pan.x += 1.0;
            }
            if cursor.y < m {
                input.pan.y -= 1.0;
            } else if cursor.y > h - m {
                input.pan.y += 1.0;
            }
        }
    }
    let step = rig.zoom_step;
    for event in wheel.read() {
        let factor = match event.unit {
            // One notch up zooms in by `zoom_step`.
            MouseScrollUnit::Line => (1.0 - step).powf(event.y),
            // Trackpad: about 20 pixels per notch.
            MouseScrollUnit::Pixel => (1.0 - step).powf(event.y / 20.0),
        };
        input.zoom_factors.push(factor);
    }
    for event in pinch.read() {
        input.zoom_factors.push(1.0 / (1.0 + event.0).max(0.01));
    }
    input.middle_pressed = mouse.just_pressed(MouseButton::Middle);
    input.middle_released = mouse.just_released(MouseButton::Middle);
    input.rotate_modifier = keys.pressed(KeyCode::AltLeft)
        || keys.pressed(KeyCode::AltRight)
        || keys.pressed(KeyCode::ShiftLeft)
        || keys.pressed(KeyCode::ShiftRight);
    input.motion = motion.read().map(|m| m.delta).sum();

    let height = |x: f32, z: f32| match (&surface, &relief) {
        (Some(surface), Some(relief)) => surface.display_height_at(relief, x, z),
        _ => 0.0,
    };
    let ground = Ground {
        height: &height,
        pyramid: surface.as_ref().map(|s| s.pyramid.as_ref()),
    };
    let pose = rig.step(&input, time.delta_secs().min(0.1), &ground);
    if let Ok((mut transform, mut projection, _)) = camera.single_mut() {
        *transform = Transform::from_translation(pose.eye).looking_at(pose.look_at, Vec3::Y);
        if let Projection::Perspective(perspective) = &mut *projection {
            perspective.near = pose.near;
            perspective.far = pose.far;
        }
    }
}

fn dump_camera(world: &World) -> (String, Value) {
    let Some(rig) = world.get_resource::<CampaignRig>() else {
        return ("campaign_camera".into(), json!({ "active": false }));
    };
    let mut value = json!({
        "active": true,
        "focus": [rig.focus.x, rig.focus.y, rig.focus.z],
        "distance": rig.distance,
        "target_distance": rig.target_distance,
        "yaw_deg": rig.yaw.to_degrees(),
        "pitch_deg": rig.pitch_deg(),
        "fov_deg": FOV_DEG,
        "min_distance": rig.min_distance.max(rig.floor_distance),
        "edge_pan": rig.edge_pan_enabled,
        "pan_speed": rig.pan_speed,
        "gliding": rig.is_gliding(),
    });
    if let Some(followed) = world.get_resource::<focus::FollowedArmy>() {
        value["followed_army"] = json!(followed.id.as_ref().map(|id| id.to_string()));
        value["rig_follow"] = json!(rig.follow.map(|p| [p.x, p.y]));
    }
    if let Some(last) = world.get_resource::<focus::LastFocus>() {
        value["last_focus"] = json!(last.0);
    }
    value["position"] = json!([rig.eye.x, rig.eye.y, rig.eye.z]);
    ("campaign_camera".into(), value)
}

fn dump_zoom(world: &World) -> (String, Value) {
    let view = world
        .get_resource::<zoom::ZoomView>()
        .copied()
        .unwrap_or_default();
    ("campaign_zoom".into(), zoom::dump(&view))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_formats() {
        let to_px = |lon: f64, lat: f64| Some([lon * 100.0, lat * 10.0]);
        let ll = parse_camera_env("2.35, 48.85, 300", to_px).unwrap();
        assert_eq!(
            (ll.x, ll.z, ll.distance, ll.yaw),
            (235.0, 488.5, 300.0, 0.0)
        );
        let xz = parse_camera_env("xz:3500,3000,120,90", to_px).unwrap();
        assert_eq!((xz.x, xz.z, xz.distance), (3500.0, 3000.0, 120.0));
        assert!((xz.yaw - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        assert!(parse_camera_env("1,2", to_px).is_err());
        assert!(parse_camera_env("a,b,c", to_px).is_err());
        assert!(parse_camera_env("1,2,3", |_, _| None).is_err());
    }
}
