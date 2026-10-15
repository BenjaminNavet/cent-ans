//! Optional cinematic shot at the first contact.
//!
//! Ports game/scripts/battle/battle_cinematic.gd: when two lines clash for the first time
//! (regiments in melee, at least `min_soldiers_in_contact` men), the camera leaves the player for
//! a few seconds (`duration_s`): it turns slowly around the point of impact at human height, black
//! bars on top and bottom, slow motion (`slowmo_scale`, setting « Ralenti du plan »). Escape or a
//! click skips it; the camera then gives control back exactly where the player left it (the
//! player's camera state is never touched, only the rendered transform is overridden after the
//! transform of the frame). Nothing is imposed: setting « Plan cinématique au premier choc »,
//! never when the AI plays both sides.
//!
//! Parameters: `data/fx/battle_staging.json` (`cinematic`). The victory shot (`victory_shot`) is
//! not ported (needs the winning general's standard of the units lot).

use bevy::prelude::*;

use super::deploy::Deployment;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCamera;
use crate::settings::Settings;
use crate::ui::kit::style::UiTier;

/// Parameters of the shot (defaults are the values of the data file).
#[derive(Debug, Clone, PartialEq)]
pub struct CinematicConfig {
    pub duration_s: f32,
    pub slowmo_scale: f32,
    pub distance_m: f32,
    pub height_m: f32,
    pub orbit_deg: f32,
    pub fov_deg: f32,
    pub min_soldiers_in_contact: u32,
    pub blend_in_s: f32,
}

impl Default for CinematicConfig {
    fn default() -> Self {
        Self {
            duration_s: 6.0,
            slowmo_scale: 0.35,
            distance_m: 30.0,
            height_m: 5.5,
            orbit_deg: 50.0,
            fov_deg: 52.0,
            min_soldiers_in_contact: 60,
            blend_in_s: 0.6,
        }
    }
}

impl CinematicConfig {
    /// Reads the `cinematic` object of the staging file text; missing keys keep their default.
    pub fn from_json(text: &str) -> Self {
        let mut config = Self::default();
        let Ok(root) = serde_json::from_str::<serde_json::Value>(text) else {
            return config;
        };
        let cine = &root["cinematic"];
        let number = |key: &str, into: &mut f32| {
            if let Some(v) = cine[key].as_f64() {
                *into = v as f32;
            }
        };
        number("duration_s", &mut config.duration_s);
        number("slowmo_scale", &mut config.slowmo_scale);
        number("distance_m", &mut config.distance_m);
        number("height_m", &mut config.height_m);
        number("orbit_deg", &mut config.orbit_deg);
        number("fov_deg", &mut config.fov_deg);
        number("blend_in_s", &mut config.blend_in_s);
        if let Some(v) = cine["min_soldiers_in_contact"].as_u64() {
            config.min_soldiers_in_contact = v as u32;
        }
        config
    }

    fn load() -> Self {
        crate::paths::find_root()
            .ok()
            .and_then(|root| {
                std::fs::read_to_string(
                    crate::paths::data_dir(&root).join("fx/battle_staging.json"),
                )
                .ok()
            })
            .map(|text| Self::from_json(&text))
            .unwrap_or_default()
    }
}

#[derive(Resource, Debug, Default)]
pub struct Cinematic {
    config: Option<CinematicConfig>,
    pub active: bool,
    /// Played (or skipped) once for this battle.
    pub done: bool,
    t: f32,
    focus: Vec3,
    yaw: f32,
    saved_speed: f64,
    saved_fov: f32,
}

#[derive(Component)]
pub struct CinematicBar;

/// Pose of the shot at time `t`: (eye, aim, blend, field of view in degrees) for the orbit
/// around `focus` seen side-on (`yaw`), `ground` the height of the relief under the eye.
pub fn pose(
    config: &CinematicConfig,
    focus: Vec3,
    yaw: f32,
    t: f32,
    ground: impl Fn(f32, f32) -> f32,
) -> (Vec3, Vec3, f32) {
    let k = (t / config.duration_s.max(0.01)).clamp(0.0, 1.0);
    let orbit = config.orbit_deg.to_radians();
    let angle = yaw - orbit * 0.5 + orbit * k;
    let dist = config.distance_m * (1.15 + (0.9 - 1.15) * k);
    let mut eye = focus + Vec3::new(angle.sin(), 0.0, angle.cos()) * dist;
    eye.y = ground(eye.x, eye.z).max(focus.y) + config.height_m;
    let blend = (t / config.blend_in_s.max(0.01)).clamp(0.0, 1.0);
    (
        eye,
        focus + Vec3::Y * 1.8,
        blend * blend * (3.0 - 2.0 * blend),
    )
}

/// Looks for the first real clash and starts the shot; runs it; ends it.
#[allow(clippy::too_many_arguments)]
pub fn run_cinematic(
    mut commands: Commands,
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut edges: ResMut<crate::menu::KeyEdges>,
    settings: Option<Res<Settings>>,
    deployment: Res<Deployment>,
    mut state: ResMut<Cinematic>,
    mut session: ResMut<BattleSession>,
    cameras: Query<(&Projection, &Transform), With<BattleCamera>>,
    bars: Query<Entity, With<CinematicBar>>,
) {
    if state.config.is_none() {
        state.config = Some(CinematicConfig::load());
    }
    let config = state.config.clone().unwrap_or_default();
    if state.active {
        state.t += time.delta_secs();
        let skip = edges.escape
            || edges.enter
            || keys.just_pressed(KeyCode::Enter)
            || buttons.get_just_pressed().next().is_some();
        if edges.escape {
            edges.escape = false; // the pause menu must not open for the skip
        }
        if skip || state.t >= config.duration_s || session.snapshot.finished {
            state.active = false;
            let speed = state.saved_speed;
            session.speed = speed;
            for bar in &bars {
                commands.entity(bar).despawn();
            }
        }
        return;
    }
    if state.done || deployment.active || !settings.as_ref().is_none_or(|s| s.cinematic) {
        return;
    }
    let Some(player) = session.player else { return };
    if session.paused || session.snapshot.finished {
        return;
    }
    let sim = session.sim_mut();
    if sim.ticks() == 0 {
        return;
    }
    let melee: Vec<_> = sim
        .units()
        .iter()
        .filter(|u| u.present() && !u.synthetic && u.state.key() == "melee")
        .collect();
    let contact: u32 = melee.iter().map(|u| u.soldiers()).sum();
    if contact < config.min_soldiers_in_contact {
        return;
    }
    // Closest pair of opposing regiments in contact: the point of impact, seen side-on.
    let mut best: Option<(f32, Vec3, f32)> = None;
    for own in melee.iter().filter(|u| u.side == player) {
        for foe in melee.iter().filter(|u| u.side == player.other()) {
            let a = Vec2::new(own.x as f32, own.z as f32);
            let b = Vec2::new(foe.x as f32, foe.z as f32);
            let gap = a.distance(b);
            if best.is_none_or(|(g, ..)| gap < g) {
                let mid = (a + b) * 0.5;
                let axis = b - a;
                let yaw = axis.x.atan2(axis.y) + std::f32::consts::FRAC_PI_2;
                let y = sim.field().walk_height(f64::from(mid.x), f64::from(mid.y)) as f32;
                best = Some((gap, Vec3::new(mid.x, y, mid.y), yaw));
            }
        }
    }
    let Some((_, focus, yaw)) = best else { return };
    state.done = true;
    state.active = true;
    state.t = 0.0;
    state.focus = focus;
    state.yaw = yaw;
    state.saved_speed = session.speed;
    if settings.as_ref().is_none_or(|s| s.cinematic_slowmo) {
        session.speed = f64::from(config.slowmo_scale);
    }
    if let Ok((Projection::Perspective(p), _)) = cameras.single() {
        state.saved_fov = p.fov;
    } else {
        state.saved_fov = 55.0_f32.to_radians();
    }
    for top in [true, false] {
        commands.spawn((
            Name::new("CinematicBar"),
            CinematicBar,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(10.0),
                top: if top {
                    Val::Px(0.0)
                } else {
                    Val::Percent(90.0)
                },
                ..default()
            },
            BackgroundColor(Color::BLACK),
            GlobalZIndex(UiTier::Banner.z()),
            Pickable::IGNORE,
            DespawnOnExit(crate::app::AppState::Battle),
        ));
    }
}

/// Overrides the rendered camera while the shot plays (after the frame's transform of the
/// player's camera; the player's state is left alone), and gives the field of view back at the end.
pub fn drive_camera(
    state: Res<Cinematic>,
    mut session: ResMut<BattleSession>,
    mut cameras: Query<(&mut Transform, &mut Projection), With<BattleCamera>>,
) {
    let Some(config) = state.config.clone() else {
        return;
    };
    let Ok((mut transform, mut projection)) = cameras.single_mut() else {
        return;
    };
    let Projection::Perspective(lens) = &mut *projection else {
        return;
    };
    if !state.active {
        if state.done && (lens.fov - state.saved_fov).abs() > 1e-4 && state.saved_fov > 0.0 {
            lens.fov = state.saved_fov;
        }
        return;
    }
    let sim = session.sim_mut();
    let field = sim.field();
    let (eye, aim, blend) = pose(&config, state.focus, state.yaw, state.t, |x, z| {
        field.walk_height(f64::from(x), f64::from(z)) as f32
    });
    let target = Transform::from_translation(eye).looking_at(aim, Vec3::Y);
    let saved = *transform;
    transform.translation = saved.translation.lerp(target.translation, blend);
    transform.rotation = saved.rotation.slerp(target.rotation, blend);
    lens.fov = state.saved_fov + (config.fov_deg.to_radians() - state.saved_fov) * blend;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_reads_the_data_file_and_keeps_defaults() {
        let text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/fx/battle_staging.json"),
        )
        .expect("data file");
        let config = CinematicConfig::from_json(&text);
        assert_eq!(config.min_soldiers_in_contact, 60);
        assert!((config.distance_m - 38.0).abs() < 1e-3);
        assert!((config.slowmo_scale - 0.35).abs() < 1e-3);
        assert_eq!(CinematicConfig::from_json("{}"), CinematicConfig::default());
    }

    #[test]
    fn orbit_turns_and_the_eye_stays_above_ground() {
        let config = CinematicConfig::default();
        let focus = Vec3::new(100.0, 10.0, 100.0);
        let (eye0, aim, blend0) = pose(&config, focus, 0.0, 0.0, |_, _| 0.0);
        let (eye1, _, blend1) = pose(&config, focus, 0.0, config.duration_s, |_, _| 0.0);
        assert_eq!(blend0, 0.0);
        assert_eq!(blend1, 1.0);
        assert!(eye0.distance(eye1) > 5.0, "the eye moves round the focus");
        assert!(eye0.y >= focus.y + config.height_m - 1e-3);
        assert!((aim.y - (focus.y + 1.8)).abs() < 1e-4);
        // On a hill higher than the focus the eye rises with the relief.
        let (high, ..) = pose(&config, focus, 0.0, 1.0, |_, _| 50.0);
        assert!((high.y - (50.0 + config.height_m)).abs() < 1e-3);
    }
}
