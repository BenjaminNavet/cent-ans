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
//! Parameters: `data/fx/battle_staging.json` (`cinematic`).
//!
//! Victory shot (`start_victory` / `victory_focus` of Godot, `victory_shot` of the data): when
//! the battle is over and the player won, after a hold of 3 s the same orbit turns round the
//! victorious general (or the centre of the surviving regiments), HUD hidden (it follows
//! [`Cinematic::active`]), no slow motion; Escape, Space, Enter or a click skip it. The result
//! screen waits for it ([`Cinematic::holds_result`], read by `hud/result.rs`). Not played for a
//! lost battle, a replay, or when the setting « Plan cinématique au premier choc » is off.

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
        Self::default().merged_json(text, "cinematic")
    }

    /// The config with the numbers of the `section` object of the staging file laid over it
    /// (`victory_shot` over `cinematic`, as `cfg.merged(victory_cfg, true)` in Godot).
    pub fn merged_json(&self, text: &str, section: &str) -> Self {
        let mut config = self.clone();
        let Ok(root) = serde_json::from_str::<serde_json::Value>(text) else {
            return config;
        };
        let cine = &root[section];
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

    /// Staging file text (`data/fx/battle_staging.json`).
    fn staging_text() -> Option<String> {
        let root = crate::paths::find_root().ok()?;
        std::fs::read_to_string(crate::paths::data_dir(&root).join("fx/battle_staging.json")).ok()
    }

    fn load() -> (Self, Self, bool) {
        let Some(text) = Self::staging_text() else {
            return (Self::default(), Self::default(), true);
        };
        let base = Self::from_json(&text);
        let victory = base.merged_json(&text, "victory_shot");
        let enabled = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|v| v["victory_shot"]["enabled"].as_bool())
            .unwrap_or(true);
        (base, victory, enabled)
    }
}

/// Hold (s) between the end of a won battle and the victory shot (`VICTORY_HOLD` of Godot).
pub const VICTORY_HOLD_S: f32 = 3.0;

#[derive(Resource, Debug, Default)]
pub struct Cinematic {
    config: Option<CinematicConfig>,
    /// `cinematic` overlaid with `victory_shot`.
    victory_config: CinematicConfig,
    victory_enabled: bool,
    /// The shot playing is the victory shot.
    pub victory: bool,
    /// The victory shot is settled for this battle (played, skipped, or not applicable): the
    /// result screen may open.
    pub victory_done: bool,
    /// Real seconds since the battle ended (the hold before the shot).
    end_wait: f32,
    demo_frames: u32,
    pub active: bool,
    /// Played (or skipped) once for this battle.
    pub done: bool,
    t: f32,
    focus: Vec3,
    yaw: f32,
    saved_speed: f64,
    saved_fov: f32,
}

impl Cinematic {
    /// The result screen must wait: the victory shot is not settled yet.
    pub fn holds_result(&self) -> bool {
        !self.victory_done || (self.victory && self.active)
    }

    fn current_config(&self) -> CinematicConfig {
        if self.victory {
            self.victory_config.clone()
        } else {
            self.config.clone().unwrap_or_default()
        }
    }
}

#[derive(Component)]
pub struct CinematicBar;

/// Point and yaw of the victory shot: the general of `side` if still there, else the centre of
/// its present regiments (siege engines left out); `None` when nothing is left. The yaw is the
/// general's facing (0 for a centre).
pub fn victory_focus(sim: &sim_battle::BattleSim, side: sim_battle::SideId) -> Option<(Vec3, f32)> {
    let mut sum = Vec3::ZERO;
    let mut count = 0.0;
    for unit in sim.units().iter().filter(|u| {
        u.side == side
            && u.present()
            && !u.synthetic
            && u.category != data_model::UnitCategory::Siege
    }) {
        let y = sim.field().walk_height(unit.x, unit.z) as f32;
        let at = Vec3::new(unit.x as f32, y, unit.z as f32);
        if unit.is_general {
            return Some((at, unit.facing as f32));
        }
        sum += at;
        count += 1.0;
    }
    (count > 0.0).then(|| (sum / count, 0.0))
}

fn spawn_bars(commands: &mut Commands) {
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

/// `cinematic` (Godot `battle/cinematic`): the first-clash and victory shots; on without settings.
pub fn cinematic_on(settings: Option<&Settings>) -> bool {
    settings.is_none_or(|s| s.cinematic)
}

/// `cinematic_slowmo`: the slow motion during the first-clash shot.
pub fn slowmo_on(settings: Option<&Settings>) -> bool {
    settings.is_none_or(|s| s.cinematic_slowmo)
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
        let (base, victory, enabled) = CinematicConfig::load();
        state.config = Some(base);
        state.victory_config = victory;
        state.victory_enabled = enabled;
    }
    let config = state.current_config();
    if state.active {
        state.t += time.delta_secs();
        let skip = edges.escape
            || edges.enter
            || keys.just_pressed(KeyCode::Enter)
            || (state.victory && keys.just_pressed(KeyCode::Space))
            || buttons.get_just_pressed().next().is_some();
        if edges.escape {
            edges.escape = false; // the pause menu must not open for the skip
        }
        if skip || state.t >= config.duration_s || (session.snapshot.finished && !state.victory) {
            state.active = false;
            state.victory = false;
            let speed = state.saved_speed;
            session.speed = speed;
            for bar in &bars {
                commands.entity(bar).despawn();
            }
        }
        return;
    }
    // Dev hook: `CENT_ANS_VICTORY_DEMO=1` plays the battle to its end at once (proof of the
    // victory shot; the shot needs a player's win, vary `--seed`).
    if !state.victory_done
        && !session.snapshot.finished
        && std::env::var("CENT_ANS_VICTORY_DEMO").is_ok_and(|v| v != "0")
        && !deployment.active
    {
        state.demo_frames += 1;
        if state.demo_frames == 30 {
            let player = session.player;
            let sim = session.sim_mut();
            if let Some(player) = player {
                sim.set_ai(player, true); // the idle player would always lose
            }
            let mut guard = 0;
            while !sim.is_finished() && guard < 200_000 {
                sim.step();
                guard += 1;
            }
            let snapshot = crate::battle::scene::BattleSnapshot::of(session.sim_mut());
            session.snapshot = snapshot;
            info!("victory demo: battle played in {guard} steps");
        }
    }
    // Battle over: the victory shot, then the result screen.
    if !state.victory_done && session.snapshot.finished {
        let player = session.player;
        let wanted = state.victory_enabled
            && player.is_some()
            && !session.replaying
            && cinematic_on(settings.as_deref());
        let Some(player) = player.filter(|_| wanted) else {
            state.victory_done = true;
            return;
        };
        state.end_wait += time.delta_secs();
        if state.end_wait < VICTORY_HOLD_S {
            return;
        }
        let sim = session.sim_mut();
        let won = sim.outcome().is_some_and(|o| o.winner == player);
        let Some((focus, yaw)) = won.then(|| victory_focus(sim, player)).flatten() else {
            info!("victory shot: not played (player won: {won})");
            state.victory_done = true;
            return;
        };
        info!("victory shot: playing at {focus:?} yaw {yaw:.2}");
        state.victory_done = true;
        state.victory = true;
        state.done = true;
        state.active = true;
        state.t = 0.0;
        state.focus = focus;
        state.yaw = yaw;
        state.saved_speed = session.speed;
        state.saved_fov = match cameras.single() {
            Ok((Projection::Perspective(p), _)) => p.fov,
            _ => 55.0_f32.to_radians(),
        };
        spawn_bars(&mut commands);
        return;
    }
    if state.done || deployment.active || !cinematic_on(settings.as_deref()) {
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
    if slowmo_on(settings.as_deref()) {
        session.speed = f64::from(config.slowmo_scale);
    }
    if let Ok((Projection::Perspective(p), _)) = cameras.single() {
        state.saved_fov = p.fov;
    } else {
        state.saved_fov = 55.0_f32.to_radians();
    }
    spawn_bars(&mut commands);
}

/// Overrides the rendered camera while the shot plays (after the frame's transform of the
/// player's camera; the player's state is left alone), and gives the field of view back at the end.
pub fn drive_camera(
    state: Res<Cinematic>,
    mut session: ResMut<BattleSession>,
    mut cameras: Query<(&mut Transform, &mut Projection), With<BattleCamera>>,
) {
    if state.config.is_none() {
        return;
    }
    let config = state.current_config();
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
    fn cinematic_settings_gate_the_shot_and_the_slowmo() {
        let mut settings = Settings::default();
        assert!(cinematic_on(Some(&settings)) && slowmo_on(Some(&settings)));
        assert!(cinematic_on(None) && slowmo_on(None));
        settings.cinematic_slowmo = false;
        assert!(cinematic_on(Some(&settings)) && !slowmo_on(Some(&settings)));
        settings.cinematic = false;
        assert!(!cinematic_on(Some(&settings)));
    }

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
