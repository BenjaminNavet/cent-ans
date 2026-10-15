//! Siege engines as glTF models (port of the placement part of `siege_engines_fx.gd`): rams,
//! siege towers and the static engines of the regiments (trebuchet, mangonel, bombard, chosen by
//! `data/fx/siege_engines.json` `engines`) stand at the position and heading of their unit in the
//! core. The models are in metres (`game/assets/models/siege/<model>.glb`, front along +z).
//!
//! Animation: a ram jolts forward at each of its blows (`RamStrike`); the rolling wheels, the
//! trebuchet arm, the bombard recoil and the crew are not ported (see the module doc of
//! [`super`]).

use std::collections::HashMap;
use std::f32::consts::FRAC_PI_2;

use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};
use sim_battle::SiegeFxKind;

use super::{EngineKind, SiegeFrame, SiegeStats};
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

#[derive(Resource, Default)]
struct EngineIndex(HashMap<u32, Entity>);

pub struct EnginesPlugin;

impl Plugin for EnginesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EngineIndex>()
            .add_systems(
                Update,
                sync_engines
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
