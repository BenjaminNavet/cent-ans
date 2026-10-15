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
/// Peak of the jolt (m).
const JOLT_M: f32 = 1.1;

#[derive(Component)]
pub struct EngineView {
    pub unit: u32,
    pub model: String,
    pub handle: Handle<WorldAsset>,
    pub jolt: f32,
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

/// Heading of the model for a sim facing (front +z -> `(cos f, sin f)`).
pub fn model_yaw(facing: f32) -> f32 {
    FRAC_PI_2 - facing
}

fn sync_engines(
    mut commands: Commands,
    assets: Res<AssetServer>,
    time: Res<Time>,
    frame: Res<SiegeFrame>,
    mut index: ResMut<EngineIndex>,
    mut stats: ResMut<SiegeStats>,
    mut views: Query<(&mut EngineView, &mut Transform)>,
) {
    if !frame.active {
        return;
    }
    for event in &frame.events {
        if let SiegeFxKind::RamStrike { unit, .. } = event.kind
            && let Some(entity) = index.0.get(&unit)
            && let Ok((mut view, _)) = views.get_mut(*entity)
        {
            view.jolt = JOLT_S;
        }
    }
    // Spawn the new, move the known.
    for engine in &frame.engines {
        let heading = Quat::from_rotation_y(model_yaw(engine.facing));
        if let Some(entity) = index.0.get(&engine.unit).copied()
            && let Ok((mut view, mut transform)) = views.get_mut(entity)
        {
            let mut offset = Vec3::ZERO;
            if view.jolt > 0.0 {
                view.jolt = (view.jolt - time.delta_secs()).max(0.0);
                let k = (view.jolt / JOLT_S * std::f32::consts::PI).sin();
                offset = heading * Vec3::Z * (JOLT_M * k);
            }
            transform.translation = engine.pos + offset;
            transform.rotation = heading;
            continue;
        }
        let path = format!("models/siege/{}.glb", engine.kind.model());
        let handle: Handle<WorldAsset> = assets.load(GltfAssetLabel::Scene(0).from_asset(path));
        let entity = commands
            .spawn((
                Name::new(format!("SiegeEngine{}", engine.unit)),
                EngineView {
                    unit: engine.unit,
                    model: engine.kind.model().to_owned(),
                    handle: handle.clone(),
                    jolt: 0.0,
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
