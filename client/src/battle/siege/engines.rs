//! Siege engines as glTF models (port of `siege_engines_fx.gd`): rams, siege towers and the
//! static engines of the regiments (trebuchet, mangonel, bombard, chosen by
//! `data/fx/siege_engines.json` `engines`) stand at the position and heading of their unit in the
//! core. The models are in metres (`game/assets/models/siege/<model>.glb`, front along +z) with
//! named parts.
//!
//! Animation (`animation.rs` holds the maths):
//! - a ram jolts forward at each of its blows (`RamStrike`);
//! - trebuchet: the arm, sling and counterweight swing when the regiment fires, the stone leaves
//!   the sling at the release phase, then the arm oscillates and the winch brings it back as the
//!   core's reload runs; the swing starts early (`predict`) so the stone leaves at the shot;
//! - mangonel: the arm slaps, the winch winds;
//! - bombard: the barrel recoils along its axis, the mantlet is raised while the regiment fires;
//! - the servants ([`super::crew`]) stand at the posts of the layout of each engine and make the
//!   gesture of their role from the reload.
//!
//! Not ported: the rolling wheels and body sway of the ram and the tower, the simplified `_lod`
//! meshes, the GA3 variants (the procedural models carry the named parts), the ammunition
//! carriers, the release points of the projectiles.

use std::collections::HashMap;
use std::f32::consts::FRAC_PI_2;

use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};
use sim_battle::SiegeFxKind;

use super::animation::{self, EngineSettings};
use super::crew::{CrewEntry, CrewFrame, EngineSettingsRes};
use super::{EngineKind, EngineState, SiegeFrame, SiegeStats};
use crate::app::AppState;
use crate::battle::scene::BattleSession;

/// Seconds of the forward jolt of the ram at a blow.
const JOLT_S: f32 = 0.45;
/// Seconds between two blows of a ram (`ram_period` of the siege data, 3 s by default).
const RAM_PERIOD_S: f32 = 3.0;
/// Peak of the jolt (m).
const JOLT_M: f32 = 1.1;

#[derive(Component)]
pub struct EngineView {
    pub unit: u32,
    pub model: String,
    pub handle: Handle<WorldAsset>,
    pub jolt: f32,
    /// Generated GA3 variant laid in place of `model` (same named parts); `None`: the base glb.
    pub variant: Option<String>,
    /// Animation clock of the last blow of a ram (the beam swing is paced on it).
    pub last_strike: Option<f32>,
}

/// Roll and sway of a rolling engine, and its named parts (once the scene has spawned).
#[derive(Default)]
pub struct MoverState {
    pub resolved: bool,
    pub wheels: Vec<Entity>,
    pub pivot: Option<Entity>,
    pub body: Option<Entity>,
    pub roll: f32,
    pub sway: f32,
    pub beam_offset: f32,
}

/// Named nodes of a static engine's model.
#[derive(Default, Clone, Copy)]
pub struct Parts {
    pub resolved: bool,
    pub arm: Option<Entity>,
    pub sling: Option<Entity>,
    pub stone: Option<Entity>,
    pub counterweight: Option<Entity>,
    pub winch: Option<Entity>,
    pub barrel: Option<Entity>,
    pub mantlet: Option<Entity>,
    /// Rest position of the barrel along its axis.
    pub barrel_rest_z: f32,
}

/// Animation state of one engine.
#[derive(Component)]
pub struct EngineAnim {
    /// Animation clock at which the swing (or the shot) began.
    pub swing: f32,
    pub fired: bool,
    pub predicted: bool,
    pub last_reload: f32,
    /// Raised share of the mantlet (0..1).
    pub mantlet: f32,
    pub parts: Parts,
    /// Last position and last time the engine moved (rams and towers: the pushers).
    pub last_pos: Vec3,
    pub moved_at: f32,
    pub mover: MoverState,
}

impl EngineAnim {
    fn new(pos: Vec3) -> Self {
        Self {
            swing: -1000.0,
            fired: false,
            predicted: false,
            last_reload: 0.0,
            mantlet: 0.0,
            parts: Parts::default(),
            last_pos: pos,
            moved_at: -1000.0,
            mover: MoverState::default(),
        }
    }
}

#[derive(Resource, Default)]
struct EngineIndex(HashMap<u32, Entity>);

pub struct EnginesPlugin;

impl Plugin for EnginesPlugin {
    fn build(&self, app: &mut App) {
        let settings = crate::paths::find_root()
            .ok()
            .and_then(|root| EngineSettings::load(&crate::paths::data_dir(&root)));
        if let Some(settings) = settings {
            app.insert_resource(EngineSettingsRes(std::sync::Arc::new(settings)));
        }
        app.init_resource::<EngineIndex>()
            .add_systems(
                Update,
                (sync_engines, animate_engines)
                    .chain()
                    .after(super::read_frame)
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>),
            )
            .add_systems(
                OnExit(AppState::Battle),
                |mut index: ResMut<EngineIndex>| {
                    index.0.clear();
                },
            );
    }
}

/// True if `relative` exists under the asset root (and `CENT_ANS_NO_GA3` is not set, the
/// `--no-ga3` of Godot).
fn asset_exists(relative: &str) -> bool {
    if std::env::var_os("CENT_ANS_NO_GA3").is_some() {
        return false;
    }
    crate::paths::find_root()
        .map(|root| crate::paths::asset_root(&root).join(relative).is_file())
        .unwrap_or(false)
}

/// Heading of the model for a sim facing (front +z -> `(cos f, sin f)`).
pub fn model_yaw(facing: f32) -> f32 {
    FRAC_PI_2 - facing
}

#[allow(clippy::too_many_arguments)]
fn sync_engines(
    mut commands: Commands,
    assets: Res<AssetServer>,
    time: Res<Time>,
    frame: Res<SiegeFrame>,
    mut index: ResMut<EngineIndex>,
    mut stats: ResMut<SiegeStats>,
    settings: Option<Res<EngineSettingsRes>>,
    mut views: Query<(&mut EngineView, &mut Transform)>,
) {
    if !frame.active {
        return;
    }
    let ga3 = settings.as_ref().map(|s| &s.0.ga3);
    for event in &frame.events {
        if let SiegeFxKind::RamStrike { unit, .. } = event.kind
            && let Some(entity) = index.0.get(&unit)
            && let Ok((mut view, _)) = views.get_mut(*entity)
        {
            view.jolt = JOLT_S;
            view.last_strike = Some(frame.clock);
        }
    }
    // Spawn the new, move the known.
    for engine in &frame.engines {
        let heading = Quat::from_rotation_y(model_yaw(engine.facing));
        if let Some(entity) = index.0.get(&engine.unit).copied()
            && let Ok((mut view, mut transform)) = views.get_mut(entity)
        {
            let mut offset = Vec3::ZERO;
            // A GA3 / Blender ram swings its beam on the pendulum (`animate_engines`), no jolt.
            if view.jolt > 0.0 && engine.kind != EngineKind::Ram {
                view.jolt = (view.jolt - time.delta_secs()).max(0.0);
                let k = (view.jolt / JOLT_S * std::f32::consts::PI).sin();
                offset = heading * Vec3::Z * (JOLT_M * k);
            }
            transform.translation = engine.pos + offset;
            transform.rotation = heading;
            continue;
        }
        let variant = ga3.and_then(|ga3| {
            animation::ga3_variant(engine.kind.model(), ga3, |name| {
                asset_exists(&format!("models/siege/{name}.glb"))
            })
        });
        let file = variant
            .clone()
            .unwrap_or_else(|| engine.kind.model().to_owned());
        let path = format!("models/siege/{file}.glb");
        let handle: Handle<WorldAsset> = assets.load(GltfAssetLabel::Scene(0).from_asset(path));
        let entity = commands
            .spawn((
                Name::new(format!("SiegeEngine{}", engine.unit)),
                EngineView {
                    unit: engine.unit,
                    model: engine.kind.model().to_owned(),
                    handle: handle.clone(),
                    jolt: 0.0,
                    variant,
                    last_strike: None,
                },
                EngineAnim::new(engine.pos),
                WorldAssetRoot(handle),
                Transform::from_translation(engine.pos).with_rotation(heading),
                Visibility::default(),
                DespawnOnExit(AppState::Battle),
            ))
            .id();
        index.0.insert(engine.unit, entity);
        stats.engine_models += 1;
        stats.engine_files.insert(engine.unit, file.clone());
        if file != engine.kind.model() {
            stats.engine_ga3 += 1;
        }
    }
    // Gone: destroyed, left the field.
    let alive: Vec<u32> = frame.engines.iter().map(|e| e.unit).collect();
    index.0.retain(|unit, entity| {
        let keep = alive.contains(unit);
        if !keep {
            commands.entity(*entity).despawn();
        }
        keep
    });
    stats.engine_models_loaded = views
        .iter()
        .filter(|(view, _)| assets.is_loaded_with_dependencies(&view.handle))
        .count();
    let _ = EngineKind::Ram;
}

type PartQuery<'w, 's> =
    Query<'w, 's, (&'static mut Transform, &'static mut Visibility), Without<EngineView>>;

/// Finds the named nodes under `root` (once the scene has spawned).
fn resolve_parts(
    root: Entity,
    children: &Query<&Children>,
    names: &Query<&Name>,
    transforms: &PartQuery,
) -> Parts {
    let mut parts = Parts::default();
    for entity in children.iter_descendants(root) {
        let Ok(name) = names.get(entity) else {
            continue;
        };
        let slot = match name.as_str() {
            "Arm" => &mut parts.arm,
            "Sling" => &mut parts.sling,
            "Stone" => &mut parts.stone,
            "Counterweight" => &mut parts.counterweight,
            "Winch" => &mut parts.winch,
            "Barrel" => &mut parts.barrel,
            "Mantlet" => &mut parts.mantlet,
            _ => continue,
        };
        *slot = Some(entity);
    }
    if let Some(barrel) = parts.barrel
        && let Ok((transform, _)) = transforms.get(barrel)
    {
        parts.barrel_rest_z = transform.translation.z;
    }
    parts.resolved = parts.arm.is_some() || parts.barrel.is_some();
    parts
}

fn rotate_x(transforms: &mut PartQuery, entity: Option<Entity>, angle: f32) {
    if let Some(entity) = entity
        && let Ok((mut transform, _)) = transforms.get_mut(entity)
    {
        transform.rotation = Quat::from_rotation_x(angle);
    }
}

fn show(transforms: &mut PartQuery, entity: Option<Entity>, visible: bool) {
    if let Some(entity) = entity
        && let Ok((_, mut visibility)) = transforms.get_mut(entity)
    {
        *visibility = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

/// Servant entries of an engine (`_crew_engine` and the pushers of `_update_mover`).
fn crew_of(
    crew: &mut CrewFrame,
    settings: &EngineSettings,
    engine: &EngineState,
    model: &str,
    act_of: &dyn Fn(&str) -> &'static str,
    pushing: bool,
) {
    let Some(layout) = settings.crew.layouts.get(model) else {
        return;
    };
    let heading = model_yaw(engine.facing);
    let rotation = Quat::from_rotation_y(heading);
    for (k, servant) in layout.iter().enumerate() {
        let mut act = act_of(&servant.role).to_owned();
        let (mut x, mut z, mut yaw_deg) = (servant.x, servant.z, servant.yaw_deg);
        if servant.role == "pusher" {
            act = if pushing {
                "pusher".to_owned()
            } else if model == "ram" {
                "pusher_still".to_owned()
            } else {
                "idle".to_owned()
            };
        }
        if act == "idle"
            && let Some(rest) = servant.rest
        {
            // At rest out of the muzzle blast (bombard).
            (x, z, yaw_deg) = (rest[0], rest[1], rest[2]);
        }
        crew.entries.push(CrewEntry {
            key: format!("e{}/{}", engine.unit, k),
            pos: engine.pos + rotation * Vec3::new(x, 0.0, z),
            yaw: heading + yaw_deg.to_radians(),
            act,
            engine: model.to_owned(),
            index: k,
            figure: servant.figure,
            side: engine.side,
        });
    }
}

/// Poses the named parts of the static engines from the core's reload and shots, and lists the
/// servants.
#[allow(clippy::too_many_arguments)]
pub fn animate_engines(
    time: Res<Time>,
    frame: Res<SiegeFrame>,
    settings: Option<Res<EngineSettingsRes>>,
    mut crew: ResMut<CrewFrame>,
    mut stats: ResMut<SiegeStats>,
    mut views: Query<(Entity, &EngineView, &mut EngineAnim)>,
    children: Query<&Children>,
    names: Query<&Name>,
    mut transforms: PartQuery,
) {
    crew.entries.clear();
    stats.engine_poses.clear();
    let Some(settings) = settings else { return };
    if !frame.active {
        return;
    }
    let settings = &settings.0;
    let clock = frame.clock;
    let by_unit: HashMap<u32, &EngineState> = frame.engines.iter().map(|e| (e.unit, e)).collect();
    for (entity, view, mut anim) in &mut views {
        let Some(engine) = by_unit.get(&view.unit).copied() else {
            continue;
        };
        let EngineKind::Static(model) = &engine.kind else {
            // Rams and towers: the pushers push while the engine moves.
            let forward = Quat::from_rotation_y(model_yaw(engine.facing)) * Vec3::Z;
            let delta = engine.pos - anim.last_pos;
            let moved = if delta.length() > 20.0 {
                0.0
            } else {
                delta.dot(forward)
            };
            anim.last_pos = engine.pos;
            if moved.abs() > 0.001 {
                anim.moved_at = clock;
            }
            let pushing = clock - anim.moved_at < 1.0;
            animate_mover(
                entity,
                view,
                &mut anim,
                engine,
                moved,
                clock,
                settings,
                &children,
                &names,
                &mut transforms,
                &mut stats,
            );
            crew_of(
                &mut crew,
                settings,
                engine,
                view.model.as_str(),
                &|_| "idle",
                pushing,
            );
            continue;
        };
        if !anim.parts.resolved {
            anim.parts = resolve_parts(entity, &children, &names, &transforms);
        }
        let parts = anim.parts;
        let period = engine.reload_period.max(0.1);
        let reload = engine.reload;
        match model.as_str() {
            "trebuchet" | "mangonel" => {
                let trebuchet = model == "trebuchet";
                let (swing_s, settle_s, margin, predict, lead) = if trebuchet {
                    let c = &settings.trebuchet;
                    (c.swing_s, c.settle_s, c.ready_margin_s, c.predict, c.lead())
                } else {
                    let c = &settings.mangonel;
                    (c.swing_s, c.settle_s, c.ready_margin_s, c.predict, c.lead())
                };
                // Early swing: the stone leaves at the shot.
                if predict
                    && reload > 0.0
                    && reload <= lead
                    && engine.shooting
                    && clock - anim.swing >= swing_s + settle_s
                {
                    anim.swing = clock - (lead - reload);
                    anim.predicted = true;
                    anim.fired = true;
                    stats.swings += 1;
                    stats.predicted_swings += 1;
                }
                // The core fired (its reload restarted from the period).
                if reload > anim.last_reload + 0.5 {
                    let since = clock - anim.swing;
                    if anim.predicted && since >= 0.0 && since <= lead + 0.6 {
                        anim.swing = clock - lead;
                    } else {
                        anim.swing = clock;
                        stats.swings += 1;
                    }
                    anim.predicted = false;
                    anim.fired = true;
                }
                anim.last_reload = reload;
                let tau = clock - anim.swing;
                let share = animation::wound(swing_s, settle_s, margin, period, reload, anim.fired);
                if trebuchet {
                    let pose = settings.trebuchet.pose(tau, share);
                    rotate_x(&mut transforms, parts.arm, pose.arm);
                    rotate_x(&mut transforms, parts.sling, pose.sling);
                    rotate_x(&mut transforms, parts.counterweight, pose.counterweight);
                    if let Some(winch) = pose.winch {
                        rotate_x(&mut transforms, parts.winch, winch);
                    }
                    show(&mut transforms, parts.stone, pose.stone_visible);
                    stats.engine_poses.push(serde_json::json!({
                        "unit": engine.unit, "model": model, "arm_deg": pose.arm.to_degrees(),
                        "sling_deg": pose.sling.to_degrees(), "stone": pose.stone_visible,
                        "tau": tau, "wound": share, "parts": parts.resolved,
                    }));
                } else {
                    let pose = settings.mangonel.pose(tau, share);
                    rotate_x(&mut transforms, parts.arm, pose.arm);
                    if let Some(winch) = pose.winch {
                        rotate_x(&mut transforms, parts.winch, winch);
                    }
                    show(&mut transforms, parts.stone, pose.stone_visible);
                    stats.engine_poses.push(serde_json::json!({
                        "unit": engine.unit, "model": model, "arm_deg": pose.arm.to_degrees(),
                        "stone": pose.stone_visible, "tau": tau, "wound": share,
                        "parts": parts.resolved,
                    }));
                }
                let settled = tau >= swing_s + settle_s;
                let reloading = anim.fired && reload > 0.0;
                let phase = 1.0 - reload / period;
                crew_of(
                    &mut crew,
                    settings,
                    engine,
                    model,
                    &|role| {
                        animation::servant_act(
                            &settings.crew,
                            model,
                            role,
                            reloading,
                            settled,
                            share,
                            tau,
                            phase,
                            &settings.bombard,
                        )
                    },
                    false,
                );
            }
            "bombard" => {
                let c = &settings.bombard;
                if reload > anim.last_reload + 0.5 {
                    anim.swing = clock;
                    anim.fired = true;
                    stats.swings += 1;
                }
                anim.last_reload = reload;
                let tau = clock - anim.swing;
                let back = c.recoil_share(tau, anim.fired);
                if let Some(barrel) = parts.barrel
                    && let Ok((mut transform, _)) = transforms.get_mut(barrel)
                {
                    transform.translation.z = parts.barrel_rest_z - c.recoil_m * back;
                }
                // The mantlet is up while the regiment fires, down to reload under cover.
                let want = if engine.shooting { 1.0 } else { 0.0 };
                let step = (time.delta_secs() / c.mantlet_s.max(0.05)).max(0.0);
                anim.mantlet += (want - anim.mantlet).clamp(-step, step);
                rotate_x(
                    &mut transforms,
                    parts.mantlet,
                    c.mantlet_angle(anim.mantlet),
                );
                stats.engine_poses.push(serde_json::json!({
                    "unit": engine.unit, "model": model, "recoil": back,
                    "barrel_z": parts.barrel_rest_z - c.recoil_m * back,
                    "mantlet": anim.mantlet, "tau": tau, "parts": parts.resolved,
                }));
                let reloading = anim.fired && reload > 0.0;
                let phase = 1.0 - reload / period;
                crew_of(
                    &mut crew,
                    settings,
                    engine,
                    model,
                    &|role| {
                        animation::servant_act(
                            &settings.crew,
                            model,
                            role,
                            reloading,
                            true,
                            1.0,
                            tau,
                            phase,
                            &settings.bombard,
                        )
                    },
                    false,
                );
            }
            _ => {}
        }
    }
}

/// Rolls the wheels, sways the body and swings the beam of a ram (`_update_mover` of
/// `siege_engines_fx.gd`, `_update_rams` of `siege_assault_fx.gd`). The variant's settings
/// (bigger wheels, longer ropes of the GA3 ram) replace the common ones.
#[allow(clippy::too_many_arguments)]
fn animate_mover(
    entity: Entity,
    view: &EngineView,
    anim: &mut EngineAnim,
    engine: &EngineState,
    moved: f32,
    clock: f32,
    settings: &EngineSettings,
    children: &Query<&Children>,
    names: &Query<&Name>,
    transforms: &mut PartQuery,
    stats: &mut SiegeStats,
) {
    let is_ram = engine.kind == EngineKind::Ram;
    let base = if is_ram {
        &settings.ram
    } else {
        &settings.siege_tower
    };
    let c = match &view.variant {
        Some(_) => base.with_override(&settings.ga3[view.model.as_str()]),
        None => base.clone(),
    };
    let state = &mut anim.mover;
    if !state.resolved {
        for part in children.iter_descendants(entity) {
            let Ok(name) = names.get(part) else { continue };
            match name.as_str() {
                n if n.starts_with("Wheel") => state.wheels.push(part),
                "BeamPivot" => state.pivot = Some(part),
                "Shed" if is_ram => state.body = Some(part),
                "Body" if !is_ram => state.body = Some(part),
                _ => {}
            }
        }
        state.resolved = !state.wheels.is_empty() || state.pivot.is_some() || state.body.is_some();
    }
    state.roll += moved / c.wheel_radius.max(0.1);
    state.sway += moved.abs() * c.sway_per_m * std::f32::consts::TAU;
    for wheel in &state.wheels {
        rotate_x(transforms, Some(*wheel), state.roll);
    }
    let moving = moved.abs() > 0.001;
    if let Some(body) = state.body
        && let Ok((mut transform, _)) = transforms.get_mut(body)
    {
        let amp = c.sway_deg.to_radians();
        let (z, x) = if moving {
            (amp * state.sway.sin(), amp * 0.5 * (state.sway * 0.5).sin())
        } else {
            (0.0, 0.0)
        };
        let (mut rx, _, mut rz) = transform.rotation.to_euler(EulerRot::XYZ);
        rz += (z - rz) * 0.2;
        rx += (x - rx) * 0.2;
        transform.rotation = Quat::from_euler(EulerRot::XYZ, rx, 0.0, rz);
    }
    if is_ram && let Some(pivot) = state.pivot {
        // Blows fall at the end of each period: the beam is back before it, then strikes.
        let offset = match view.last_strike {
            Some(at) if clock - at < RAM_PERIOD_S * 1.5 => {
                animation::ram_offset(((clock - at) / RAM_PERIOD_S).rem_euclid(1.0))
            }
            _ => 0.0,
        };
        state.beam_offset = offset;
        rotate_x(transforms, Some(pivot), c.pivot_angle(offset));
        stats.engine_poses.push(serde_json::json!({
            "unit": engine.unit, "model": view.model, "variant": view.variant,
            "beam_offset": offset, "pivot_deg": c.pivot_angle(offset).to_degrees(),
            "wheels": state.wheels.len(), "roll": state.roll, "wheel_radius": c.wheel_radius,
            "beam_drop": c.beam_drop,
        }));
    }
}
