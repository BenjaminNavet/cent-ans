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
//! Dump section `campaign_camera`.

pub mod rig;

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

/// Marker of the campaign 3D camera.
#[derive(Component)]
pub struct CampaignCamera;

/// Set of the systems that move the camera (terrain streaming runs after it).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CameraSet;

pub struct CampaignCameraPlugin;

impl Plugin for CampaignCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
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
        .register_dump(dump_camera);
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
    if let Ok(text) = std::env::var(CAMERA_ENV) {
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
    let axis = |plus: &[KeyCode], minus: &[KeyCode]| {
        let down = |codes: &[KeyCode]| codes.iter().any(|k| keys.pressed(*k)) as i32 as f32;
        down(plus) - down(minus)
    };
    input.pan.x = axis(
        &[KeyCode::KeyD, KeyCode::ArrowRight],
        &[KeyCode::KeyA, KeyCode::ArrowLeft],
    );
    input.pan.y = axis(
        &[KeyCode::KeyS, KeyCode::ArrowDown],
        &[KeyCode::KeyW, KeyCode::ArrowUp],
    );
    input.rotate = axis(&[KeyCode::KeyE], &[KeyCode::KeyQ]);
    if keys.just_pressed(KeyCode::ScrollLock) {
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
        "gliding": rig.is_gliding(),
    });
    value["position"] = json!([rig.eye.x, rig.eye.y, rig.eye.z]);
    ("campaign_camera".into(), value)
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
