//! Tactical camera: pivot on the ground (`target`), yaw and distance, tilt chosen from the
//! distance (grazing close, plunging far). Port of game/scripts/battle/battle_camera.gd: W A S D
//! and arrows pan, screen edges pan, wheel zooms (damped), Q / E turn, middle button turns and
//! tilts, Shift + middle button pans.

use bevy::camera::RenderTarget;
use bevy::input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use sim_battle::Battlefield;

use super::BattleSession;
use crate::agent::OffscreenTarget;
use crate::app::AppState;

pub const MIN_DISTANCE: f32 = 12.0;
pub const MAX_DISTANCE: f32 = 900.0;
const PAN_SPEED: f32 = 1.1;
const ROTATE_SPEED: f32 = 1.6;
const EDGE_MARGIN: f32 = 8.0;
/// `data/ui/camera_feel.json` `battle.zoom_damping`.
const ZOOM_DAMPING: f32 = 9.0;
const PITCH_NEAR_DEG: f32 = 6.0;
const PITCH_FAR_DEG: f32 = 62.0;
const PITCH_CURVE: f32 = 0.55;
const AIM_HEIGHT_NEAR: f32 = 1.7;
const GROUND_CLEARANCE_NEAR: f32 = 1.6;
const GROUND_CLEARANCE_FAR: f32 = 4.0;
const CLOSE_RANGE: f32 = 80.0;
const ORBIT_MOUSE_SPEED: f32 = 0.006;
const MANUAL_PITCH_MIN_DEG: f32 = 5.0;
const MANUAL_PITCH_MAX_DEG: f32 = 85.0;
const MANUAL_PITCH_DRAG_SPEED: f32 = 0.15;
/// Opening shot (`camera_feel.json` `battle.opening_distance_m`, a little wider to see both lines).
const OPENING_DISTANCE: f32 = 180.0;

#[derive(Component)]
pub struct BattleCamera;

/// Camera pose in the battle: ground pivot, yaw (rad), distance to the pivot (m).
#[derive(Resource, Debug, Clone)]
pub struct BattleCameraState {
    pub target: Vec3,
    pub yaw: f32,
    pub distance: f32,
    pub target_distance: f32,
    /// Manual tilt (degrees) set by the middle button; `None`: tilt from the distance.
    pub manual_pitch_deg: Option<f32>,
    /// `(min_x, min_z, max_x, max_z)` the pivot stays within.
    pub bounds: [f32; 4],
    pub last_pitch_deg: f32,
}

impl BattleCameraState {
    pub fn new(target: Vec3, yaw: f32, distance: f32, field: &Battlefield) -> Self {
        Self {
            target,
            yaw,
            distance,
            target_distance: distance,
            manual_pitch_deg: None,
            bounds: [
                -200.0,
                -200.0,
                field.width as f32 + 200.0,
                field.depth as f32 + 200.0,
            ],
            last_pitch_deg: PITCH_NEAR_DEG,
        }
    }

    fn clamp_target(&mut self) {
        self.target.x = self.target.x.clamp(self.bounds[0], self.bounds[2]);
        self.target.z = self.target.z.clamp(self.bounds[1], self.bounds[3]);
    }

    /// Pans by `screen_move` (right, forward as screen axes) rotated by the yaw.
    pub fn pan(&mut self, screen_move: Vec2) {
        let forward = Vec3::new(-self.yaw.sin(), 0.0, -self.yaw.cos());
        let right = Vec3::new(self.yaw.cos(), 0.0, -self.yaw.sin());
        self.target += right * screen_move.x - forward * screen_move.y;
        self.clamp_target();
    }

    pub fn zoom_by(&mut self, factor: f32) {
        self.target_distance = (self.target_distance * factor).clamp(MIN_DISTANCE, MAX_DISTANCE);
    }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Tilt (degrees) for `distance` metres from the pivot (`_apply` of battle_camera.gd).
pub fn auto_pitch_deg(distance: f32) -> f32 {
    let t = ((distance - MIN_DISTANCE) / (MAX_DISTANCE - MIN_DISTANCE)).clamp(0.0, 1.0);
    PITCH_NEAR_DEG + (PITCH_FAR_DEG - PITCH_NEAR_DEG) * t.powf(PITCH_CURVE)
}

/// Eye and aim points, the eye kept above the ground; also returns the tilt used (degrees).
pub fn eye_and_aim(
    state: &BattleCameraState,
    height_at: impl Fn(f32, f32) -> f32,
) -> (Vec3, Vec3, f32) {
    let pitch_deg = match state.manual_pitch_deg {
        Some(deg) => deg.clamp(MANUAL_PITCH_MIN_DEG, MANUAL_PITCH_MAX_DEG),
        None => auto_pitch_deg(state.distance),
    };
    let pitch = pitch_deg.to_radians();
    let close = smoothstep(MIN_DISTANCE, CLOSE_RANGE, state.distance);
    let ground = height_at(state.target.x, state.target.z);
    let aim = Vec3::new(state.target.x, ground, state.target.z)
        + Vec3::Y * (AIM_HEIGHT_NEAR + (0.0 - AIM_HEIGHT_NEAR) * close);
    let offset = Vec3::new(
        state.yaw.sin() * pitch.cos(),
        pitch.sin(),
        state.yaw.cos() * pitch.cos(),
    ) * state.distance;
    let mut eye = aim + offset;
    let clearance = GROUND_CLEARANCE_NEAR + (GROUND_CLEARANCE_FAR - GROUND_CLEARANCE_NEAR) * close;
    eye.y = eye.y.max(height_at(eye.x, eye.z) + clearance);
    (eye, aim, pitch_deg)
}

pub struct BattleCameraPlugin;

impl Plugin for BattleCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (control_camera, apply_camera)
                .chain()
                .run_if(in_state(AppState::Battle))
                .run_if(resource_exists::<BattleSession>),
        )
        .add_systems(OnExit(AppState::Battle), |mut commands: Commands| {
            commands.remove_resource::<BattleCameraState>();
        });
    }
}

/// Spawns the camera (rendering to the offscreen target when asked) and its state, framed on the
/// player's army looking at the enemy.
pub fn spawn_battle_camera(
    commands: &mut Commands,
    field: &Battlefield,
    target: Option<&OffscreenTarget>,
    sky: Color,
    centre: Vec3,
    yaw: f32,
) {
    let state = BattleCameraState::new(centre, yaw, OPENING_DISTANCE, field);
    let mut camera = commands.spawn((
        Name::new("BattleCamera"),
        BattleCamera,
        Camera3d::default(),
        Camera {
            // Under the UI camera (order 0), which draws the HUD on top.
            order: -1,
            clear_color: ClearColorConfig::Custom(sky),
            ..default()
        },
        Projection::Perspective(PerspectiveProjection {
            fov: 55.0_f32.to_radians(),
            near: 0.3,
            far: 12_000.0,
            ..default()
        }),
        Transform::from_translation(centre + Vec3::new(0.0, 60.0, 120.0))
            .looking_at(centre, Vec3::Y),
        DespawnOnExit(AppState::Battle),
    ));
    if let Some(target) = target {
        camera.insert(RenderTarget::Image(target.0.clone().into()));
    }
    commands.insert_resource(state);
}

#[allow(clippy::too_many_arguments)]
fn control_camera(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    scroll: Res<AccumulatedMouseScroll>,
    motion: Res<AccumulatedMouseMotion>,
    windows: Query<&Window, With<PrimaryWindow>>,
    state: Option<ResMut<BattleCameraState>>,
) {
    let Some(mut state) = state else { return };
    let delta = time.delta_secs();
    let ctrl = keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);
    let held =
        |letter: KeyCode, arrow: KeyCode| (!ctrl && keys.pressed(letter)) || keys.pressed(arrow);
    let mut movement = Vec2::ZERO;
    if held(KeyCode::KeyW, KeyCode::ArrowUp) {
        movement.y -= 1.0;
    }
    if held(KeyCode::KeyS, KeyCode::ArrowDown) {
        movement.y += 1.0;
    }
    if held(KeyCode::KeyA, KeyCode::ArrowLeft) {
        movement.x -= 1.0;
    }
    if held(KeyCode::KeyD, KeyCode::ArrowRight) {
        movement.x += 1.0;
    }
    if let Ok(window) = windows.single()
        && window.focused
        && let Some(cursor) = window.cursor_position()
    {
        let size = window.size();
        if cursor.x < EDGE_MARGIN {
            movement.x -= 1.0;
        } else if cursor.x > size.x - EDGE_MARGIN {
            movement.x += 1.0;
        }
        if cursor.y < EDGE_MARGIN {
            movement.y -= 1.0;
        } else if cursor.y > size.y - EDGE_MARGIN {
            movement.y += 1.0;
        }
    }
    if !ctrl && keys.pressed(KeyCode::KeyQ) {
        state.yaw += ROTATE_SPEED * delta;
    }
    if !ctrl && keys.pressed(KeyCode::KeyE) {
        state.yaw -= ROTATE_SPEED * delta;
    }
    if movement != Vec2::ZERO {
        let step = movement.normalize() * state.distance * PAN_SPEED * delta;
        state.pan(step);
    }
    if scroll.delta.y != 0.0 {
        let lines = match scroll.unit {
            MouseScrollUnit::Line => scroll.delta.y,
            MouseScrollUnit::Pixel => scroll.delta.y / 40.0,
        };
        state.zoom_by(if lines > 0.0 {
            0.88f32.powf(lines)
        } else {
            1.14f32.powf(-lines)
        });
    }
    if buttons.just_pressed(MouseButton::Middle) && !keys.pressed(KeyCode::ShiftLeft) {
        state.manual_pitch_deg = Some(state.last_pitch_deg);
    }
    if buttons.pressed(MouseButton::Middle) {
        if keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
            let step = -motion.delta * state.distance * 0.0022;
            state.pan(step);
        } else {
            state.yaw += -motion.delta.x * ORBIT_MOUSE_SPEED;
            let current = state.manual_pitch_deg.unwrap_or(state.last_pitch_deg);
            state.manual_pitch_deg = Some(
                (current - motion.delta.y * MANUAL_PITCH_DRAG_SPEED)
                    .clamp(MANUAL_PITCH_MIN_DEG, MANUAL_PITCH_MAX_DEG),
            );
        }
    }
    let blend = 1.0 - (-ZOOM_DAMPING * delta).exp();
    state.distance += (state.target_distance - state.distance) * blend;
}

fn apply_camera(
    session: Res<BattleSession>,
    state: Option<ResMut<BattleCameraState>>,
    mut cameras: Query<&mut Transform, With<BattleCamera>>,
) {
    let Some(mut state) = state else { return };
    let sim = session.sim();
    let field = sim.field();
    let height_at = |x: f32, z: f32| field.height(f64::from(x), f64::from(z)) as f32;
    let (tx, tz) = (state.target.x, state.target.z);
    state.target.y = height_at(tx, tz);
    let (eye, aim, pitch) = eye_and_aim(&state, height_at);
    state.last_pitch_deg = pitch;
    for mut transform in &mut cameras {
        *transform = Transform::from_translation(eye).looking_at(aim, Vec3::Y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_state(distance: f32, yaw: f32) -> BattleCameraState {
        BattleCameraState {
            target: Vec3::new(600.0, 0.0, 250.0),
            yaw,
            distance,
            target_distance: distance,
            manual_pitch_deg: None,
            bounds: [0.0, 0.0, 1200.0, 800.0],
            last_pitch_deg: 6.0,
        }
    }

    #[test]
    fn tilt_grows_with_distance() {
        assert!((auto_pitch_deg(MIN_DISTANCE) - PITCH_NEAR_DEG).abs() < 1e-4);
        assert!((auto_pitch_deg(MAX_DISTANCE) - PITCH_FAR_DEG).abs() < 1e-3);
        assert!(auto_pitch_deg(100.0) > auto_pitch_deg(30.0));
    }

    #[test]
    fn eye_stays_above_ground_and_behind_the_pivot() {
        // Yaw pi: eye on the -z side of the pivot, looking towards +z.
        let state = flat_state(200.0, std::f32::consts::PI);
        let (eye, aim, _) = eye_and_aim(&state, |_, _| 5.0);
        assert!(eye.z < aim.z);
        assert!(eye.y > 5.0 + GROUND_CLEARANCE_FAR - 0.01);
        let low = flat_state(MIN_DISTANCE, 0.0);
        let (eye, _, _) = eye_and_aim(&low, |_, _| 5.0);
        assert!(eye.y >= 5.0 + GROUND_CLEARANCE_NEAR - 0.01);
    }

    #[test]
    fn pan_follows_the_yaw_and_stays_in_bounds() {
        let mut state = flat_state(100.0, 0.0);
        state.pan(Vec2::new(0.0, -10.0)); // forward at yaw 0 is -z
        assert!((state.target.z - 240.0).abs() < 1e-3);
        state.pan(Vec2::new(5000.0, 0.0));
        assert_eq!(state.target.x, 1200.0);
        state.zoom_by(100.0);
        assert_eq!(state.target_distance, MAX_DISTANCE);
    }
}
