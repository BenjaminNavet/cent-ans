//! Soldier rendering: every living figure of the simulation drawn by GPU instancing, positions
//! and facings read from `sim_battle::BattleSim::soldier_poses` and blended between the two
//! latest fixed steps.
//!
//! Ports the instancing core of game/scripts/battle/battle_soldiers.gd (poses from
//! `battle_sim_poses.rs`, blend of `battle_pose_lerp.rs`) and the figure meshes of
//! battle_meshes.gd as plain meshes tinted by side. Not ported yet: skinned/animated figures
//! (battle_skinned.gd), unit looks and faces (battle_unit_looks.gd), LOD rings, banners and
//! standards, formation outlines, blood and effects (battle_cavalry_gaits.gd,
//! battle_formation_outline.gd, battle_blood.gd, battle_effects.gd).
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

pub mod instancing;

use std::collections::HashMap;
use std::sync::Arc;

use bevy::prelude::*;
use data_model::UnitCategory;
use sim_battle::{BattleSim, SideId};

use crate::app::AppState;
use crate::battle::scene::BattleSession;
use instancing::{InstancedBatch, InstancingPlugin, MaterialParams};

/// Bytes of one instance record: x, y, z, yaw (degrees), scale, model index.
const RECORD_FLOATS: usize = 6;
/// A figure that moved more than this between two steps is teleporting: no blend (m).
const TELEPORT_M: f32 = 8.0;

/// Counters of the soldier renderer (dump section `battle`).
#[derive(Resource, Default, Debug, Clone)]
pub struct SoldierStats {
    /// Instances handed to the GPU this frame.
    pub drawn: usize,
    /// `[attacker foot, attacker mounted, defender foot, defender mounted]`.
    pub per_batch: [usize; 4],
}

/// One figure: `(x, y, z, facing)` as given by the simulation.
type Pose = [f32; 4];

/// Poses of the two latest simulation steps, by regiment id.
#[derive(Resource, Default)]
struct PoseFrames {
    ticks: Option<u64>,
    prev: HashMap<u32, Vec<Pose>>,
    cur: HashMap<u32, Vec<Pose>>,
    /// `(ticks, fraction bits)` of the records last written.
    written: Option<(u64, u32)>,
}

#[derive(Resource)]
struct SoldierBatches {
    entities: [Entity; 4],
    versions: [u64; 4],
}

pub struct BattleUnitsPlugin;

impl Plugin for BattleUnitsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InstancingPlugin)
            .init_resource::<SoldierStats>()
            .init_resource::<PoseFrames>()
            .add_systems(
                Update,
                (
                    spawn_batches.run_if(resource_added::<BattleSession>),
                    update_soldiers
                        .run_if(in_state(AppState::Battle))
                        .run_if(resource_exists::<BattleSession>)
                        .run_if(resource_exists::<SoldierBatches>),
                )
                    .chain()
                    .after(crate::battle::scene::StepSet),
            )
            .add_systems(OnExit(AppState::Battle), clear_units);
    }
}

fn clear_units(
    mut commands: Commands,
    mut stats: ResMut<SoldierStats>,
    mut frames: ResMut<PoseFrames>,
) {
    commands.remove_resource::<SoldierBatches>();
    *stats = SoldierStats::default();
    *frames = PoseFrames::default();
}

/// Foot soldier: a 1.7 m capsule.
pub fn foot_mesh() -> Mesh {
    Mesh::from(Capsule3d::new(0.22, 1.26)).translated_by(Vec3::Y * 0.85)
}

/// Horseman: body, legs and a rider.
pub fn mounted_mesh() -> Mesh {
    let mut mesh = Mesh::from(Cuboid::new(0.55, 0.9, 1.9)).translated_by(Vec3::Y * 0.85);
    let legs = Mesh::from(Cuboid::new(0.4, 0.45, 1.4)).translated_by(Vec3::Y * 0.22);
    let rider = Mesh::from(Capsule3d::new(0.22, 0.9)).translated_by(Vec3::Y * 1.97);
    mesh.merge(&legs).expect("same layout");
    mesh.merge(&rider).expect("same layout");
    mesh
}

fn side_index(side: SideId) -> usize {
    usize::from(side == SideId::Defender)
}

fn batch_index(side: SideId, mounted: bool) -> usize {
    side_index(side) * 2 + usize::from(mounted)
}

fn spawn_batches(
    mut commands: Commands,
    session: Res<BattleSession>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let foot = meshes.add(foot_mesh());
    let mounted = meshes.add(mounted_mesh());
    let mut entities = [Entity::PLACEHOLDER; 4];
    for side in SideId::BOTH {
        let [r, g, b] = session.colors[side_index(side)];
        let color = Color::srgb(r, g, b).to_linear();
        for is_mounted in [false, true] {
            let index = batch_index(side, is_mounted);
            entities[index] = commands
                .spawn((
                    Name::new(format!(
                        "Soldiers{}{}",
                        if side == SideId::Attacker {
                            "Attacker"
                        } else {
                            "Defender"
                        },
                        if is_mounted { "Mounted" } else { "Foot" }
                    )),
                    InstancedBatch {
                        mesh_handle: if is_mounted {
                            mounted.clone()
                        } else {
                            foot.clone()
                        },
                        records: Arc::new(Vec::new()),
                        count: 0,
                        texture_handle: None,
                        params: MaterialParams {
                            base_color: [color.red, color.green, color.blue, 1.0],
                            alpha_cutoff: 0.0,
                            roughness: 0.8,
                            metallic: 0.0,
                            double_sided: false,
                        },
                        version: 0,
                    },
                    DespawnOnExit(AppState::Battle),
                ))
                .id();
        }
    }
    commands.insert_resource(SoldierBatches {
        entities,
        versions: [0; 4],
    });
}

/// Regiments drawn as figures: field regiments with soldiers (not engines, not off-field reserve).
fn drawn_units(sim: &BattleSim) -> impl Iterator<Item = &sim_battle::Unit> {
    sim.units()
        .iter()
        .filter(|u| !u.synthetic && !u.reserve && u.category != UnitCategory::Siege)
}

/// Blends `prev` towards `cur` by `t`; a figure that jumped, or a regiment whose head count
/// changed, shows its current pose.
pub fn blend_poses(prev: Option<&[Pose]>, cur: &[Pose], t: f32) -> Vec<Pose> {
    let Some(prev) = prev.filter(|p| p.len() == cur.len() && t < 1.0) else {
        return cur.to_vec();
    };
    cur.iter()
        .zip(prev)
        .map(|(c, p)| {
            let jump = Vec3::new(c[0] - p[0], 0.0, c[2] - p[2]).length();
            if jump > TELEPORT_M {
                return *c;
            }
            let mut turn = c[3] - p[3];
            turn = (turn + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            [
                p[0] + (c[0] - p[0]) * t,
                p[1] + (c[1] - p[1]) * t,
                p[2] + (c[2] - p[2]) * t,
                p[3] + turn * t,
            ]
        })
        .collect()
}

fn push_record(records: &mut Vec<u8>, pose: &Pose) {
    let record: [f32; RECORD_FLOATS] = [pose[0], pose[1], pose[2], pose[3].to_degrees(), 1.0, 0.0];
    for value in record {
        records.extend_from_slice(&value.to_le_bytes());
    }
}

fn update_soldiers(
    session: Res<BattleSession>,
    mut frames: ResMut<PoseFrames>,
    mut batches: ResMut<SoldierBatches>,
    mut stats: ResMut<SoldierStats>,
    mut query: Query<&mut InstancedBatch>,
) {
    let sim = &*session.sim();
    let ticks = sim.ticks();
    if frames.ticks != Some(ticks) {
        let advanced = frames
            .ticks
            .is_some_and(|old| ticks > old && ticks - old <= 3);
        let old = std::mem::take(&mut frames.cur);
        frames.prev = if advanced { old } else { HashMap::new() };
        frames.cur = drawn_units(sim)
            .map(|unit| {
                let poses = sim
                    .soldier_poses(unit, 1.0)
                    .into_iter()
                    .map(|p| [p[0] as f32, p[1] as f32, p[2] as f32, p[3] as f32])
                    .collect();
                (unit.id, poses)
            })
            .collect();
        frames.ticks = Some(ticks);
        frames.written = None;
    }
    let fraction = if session.paused {
        1.0
    } else {
        session.step_fraction()
    };
    let key = (ticks, fraction.to_bits());
    if frames.written == Some(key) {
        return;
    }
    frames.written = Some(key);
    let mut records: [Vec<u8>; 4] = Default::default();
    let mut counts = [0usize; 4];
    for unit in drawn_units(sim) {
        let Some(cur) = frames.cur.get(&unit.id) else {
            continue;
        };
        let poses = blend_poses(frames.prev.get(&unit.id).map(Vec::as_slice), cur, fraction);
        let index = batch_index(unit.side, unit.mounted);
        counts[index] += poses.len();
        for pose in &poses {
            push_record(&mut records[index], pose);
        }
    }
    for (index, bytes) in records.into_iter().enumerate() {
        if let Ok(mut batch) = query.get_mut(batches.entities[index]) {
            batches.versions[index] += 1;
            batch.count = counts[index] as u32;
            batch.records = Arc::new(bytes);
            batch.version = batches.versions[index];
        }
    }
    stats.drawn = counts.iter().sum();
    stats.per_batch = counts;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blend_interpolates_and_skips_jumps() {
        let prev = [[0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0]];
        let cur = [[1.0, 0.0, 2.0, 0.5], [50.0, 0.0, 0.0, 0.0]];
        let out = blend_poses(Some(&prev), &cur, 0.5);
        assert_eq!(out[0], [0.5, 0.0, 1.0, 0.25]);
        assert_eq!(out[1], cur[1], "a jump is not blended");
        assert_eq!(blend_poses(Some(&prev[..1]), &cur, 0.5), cur.to_vec());
        assert_eq!(blend_poses(None, &cur, 0.5), cur.to_vec());
    }

    #[test]
    fn blend_turns_the_short_way() {
        let prev = [[0.0, 0.0, 0.0, 3.0]];
        let cur = [[0.0, 0.0, 0.0, -3.0]];
        let out = blend_poses(Some(&prev), &cur, 0.5);
        assert!(out[0][3].abs() > 3.0, "{}", out[0][3]);
    }

    #[test]
    fn record_is_24_bytes() {
        let mut bytes = Vec::new();
        push_record(&mut bytes, &[1.0, 2.0, 3.0, std::f32::consts::PI]);
        assert_eq!(bytes.len(), 24);
        assert_eq!(f32::from_le_bytes(bytes[12..16].try_into().unwrap()), 180.0);
    }
}
