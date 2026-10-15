//! Standards carried by the bearers: close to the field, the regiment's marker flag gives way to
//! a smaller standard held by a figure of the front rank (the slots of
//! `Unit::standard_slots`); a fallen standard lies on the ground where it fell.
//! Light port of game/scripts/battle/battle_standards.gd: the cloth and pole are those of the
//! marker banner of the regiment (`banners.rs`), no separate bearer rig, no musicians, no
//! trophies carried by the captor, no sounds.

use std::collections::HashMap;

use bevy::camera::visibility::NoFrustumCulling;
use bevy::prelude::*;
use sim_battle::BattleSim;
use sim_battle::unit::StandardState;

use super::banners::{BANNER_HEIGHT, Banners, banner_scale};
use super::wind::BattleWind;
use super::{PoseFrames, drawn_units};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::scene::BattleSession;
use crate::battle::scene::camera::BattleCameraState;

/// Marker scale at or under which the carried standard replaces the marker
/// (`marker_hide_scale` of battle_finish.json).
const MARKER_HIDE_SCALE: f32 = 1.05;
/// Size of a carried standard against the marker flag.
const CARRIED_SCALE: f32 = 0.5;
/// Roll (rad) of a standard lying on the ground.
const FALLEN_ROLL: f32 = 1.35;

struct Carried {
    root: Entity,
}

#[derive(Resource, Default)]
pub struct CarriedStandards {
    map: HashMap<u32, Carried>,
    /// Standards drawn this frame by state: carried, fallen.
    pub shown: [usize; 2],
    /// Markers hidden in favour of a standard.
    pub markers_hidden: usize,
}

/// Index of the figure carrying the standard of `unit` among `n` drawn figures.
pub fn bearer_slot(
    unit: &sim_battle::Unit,
    sim: &BattleSim,
    index: usize,
    n: usize,
) -> Option<usize> {
    let bearers = sim.standard_bearers(index);
    if bearers == 0 || n == 0 {
        return None;
    }
    unit.standard_slots(1.0, 1)
        .first()
        .map(|slot| (*slot as usize).min(n - 1))
}

#[allow(clippy::too_many_arguments)]
fn update_standards(
    mut commands: Commands,
    mut session: ResMut<BattleSession>,
    banners: Res<Banners>,
    frames: Res<PoseFrames>,
    wind: Option<Res<BattleWind>>,
    camera_state: Option<Res<BattleCameraState>>,
    mut carried: ResMut<CarriedStandards>,
    mut transforms: Query<&mut Transform>,
    mut visibilities: Query<&mut Visibility>,
) {
    let sim = &*session.sim_mut();
    let scale = banner_scale(camera_state.map_or(150.0, |c| c.distance));
    let close = scale <= MARKER_HIDE_SCALE;
    let (Some((pole_mesh, wood)), Some((finial_mesh, gold))) =
        (banners.pole.clone(), banners.finial.clone())
    else {
        return;
    };
    let wind = wind.map_or(BattleWind::default(), |w| *w);
    let mut shown = [0usize; 2];
    let mut hidden = 0;
    let indices: HashMap<u32, usize> = sim
        .units()
        .iter()
        .enumerate()
        .map(|(i, u)| (u.id, i))
        .collect();
    for unit in drawn_units(sim) {
        let Some(banner) = banners.map.get(&unit.id) else {
            continue;
        };
        let Some(poses) = frames.cur.get(&unit.id) else {
            continue;
        };
        // Where the standard is, and whether it stands.
        let placement = match unit.standard {
            StandardState::Carried => indices
                .get(&unit.id)
                .and_then(|i| bearer_slot(unit, sim, *i, poses.len()))
                .and_then(|slot| poses.get(slot))
                .map(|p| (Vec3::new(p[0], p[1], p[2]), false)),
            StandardState::Fallen { x, z, .. } | StandardState::Lost { x, z } => {
                let y = sim.field().height(x, z) as f32;
                Some((Vec3::new(x as f32, y, z as f32), true))
            }
            StandardState::Captured { .. } => None,
        };
        let entry = carried.map.get(&unit.id).map(|c| c.root);
        let show = close && placement.is_some();
        if close && let Ok(mut marker) = visibilities.get_mut(banner.root) {
            // The marker gives way whenever the regiment's standard is handled here.
            *marker = Visibility::Hidden;
            hidden += 1;
        }
        let (Some((position, fallen)), true) = (placement, show) else {
            if let Some(root) = entry
                && let Ok(mut visibility) = visibilities.get_mut(root)
            {
                *visibility = Visibility::Hidden;
            }
            continue;
        };
        let root = match entry {
            Some(root) => root,
            None => {
                let (mesh, material) = (banner.mesh.clone(), banner.material.clone());
                let root = commands
                    .spawn((
                        Name::new(format!("Standard{}", unit.id)),
                        Transform::default(),
                        Visibility::Hidden,
                        DespawnOnExit(AppState::Battle),
                    ))
                    .with_children(|parent| {
                        parent.spawn((
                            Mesh3d(pole_mesh.clone()),
                            MeshMaterial3d(wood.clone()),
                            NoFrustumCulling,
                        ));
                        parent.spawn((
                            Mesh3d(finial_mesh.clone()),
                            MeshMaterial3d(gold.clone()),
                            NoFrustumCulling,
                        ));
                        parent.spawn((
                            Mesh3d(mesh),
                            MeshMaterial3d(material),
                            Transform::from_xyz(0.03, BANNER_HEIGHT - 0.05, 0.0),
                            NoFrustumCulling,
                        ));
                    })
                    .id();
                carried.map.insert(unit.id, Carried { root });
                root
            }
        };
        if let Ok(mut transform) = transforms.get_mut(root) {
            transform.translation = position;
            let yaw = Quat::from_rotation_y(wind.downwind_yaw());
            // A fallen standard lies along the wind with its cloth on the ground.
            transform.rotation = if fallen {
                yaw * Quat::from_rotation_z(-FALLEN_ROLL)
            } else {
                yaw
            };
            transform.scale = Vec3::splat(CARRIED_SCALE);
        }
        if let Ok(mut visibility) = visibilities.get_mut(root) {
            *visibility = Visibility::Inherited;
        }
        shown[usize::from(fallen)] += 1;
    }
    carried.shown = shown;
    carried.markers_hidden = hidden;
}

fn clear_standards(mut carried: ResMut<CarriedStandards>) {
    *carried = CarriedStandards::default();
}

fn dump_standards(world: &World) -> (String, serde_json::Value) {
    let Some(c) = world.get_resource::<CarriedStandards>() else {
        return ("battle_standards".into(), serde_json::Value::Null);
    };
    (
        "battle_standards".into(),
        serde_json::json!({
            "carried_shown": c.shown[0],
            "fallen_shown": c.shown[1],
            "markers_hidden": c.markers_hidden,
            "entities": c.map.len(),
        }),
    )
}

pub struct StandardsPlugin;

impl Plugin for StandardsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CarriedStandards>()
            .register_dump(dump_standards)
            .add_systems(
                Update,
                update_standards
                    .run_if(in_state(AppState::Battle))
                    .run_if(resource_exists::<BattleSession>)
                    .after(super::banners::update_banners),
            )
            .add_systems(OnExit(AppState::Battle), clear_standards);
    }
}
