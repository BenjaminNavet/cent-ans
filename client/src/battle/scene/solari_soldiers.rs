//! Shadow proxies of the soldiers for the Solari ray queries (lot solari-soldats).
//!
//! Soldiers are instanced and skinned on the GPU (`units/skinned.rs`): they have no entity and no
//! BLAS, so under Solari (which replaces the sun cascades by ray queries) they cast no shadow. Here
//! each soldier that the raster draws (visible, or kept for its shadow by `units/crowd.rs`) gets a
//! PROXY: an entity with `RaytracingMesh3d` (one tiny shared box mesh, hence one shared BLAS, per
//! kind: foot soldier / horseman) and a dark `StandardMaterial`, but no `Mesh3d`, so it is invisible
//! to the raster. Its transform follows the soldier's record (position, heading).
//!
//! Cost control:
//! - the source is the regiment batches (a few dozen entities), never the proxies or the soldiers;
//! - a batch signature (entity, version, counts) skips the whole update when nothing changed;
//! - the pool only grows (up to `soldier_proxy_budget`), idle proxies are parked below the ground,
//!   nothing is allocated per frame; a proxy whose pose is unchanged is not written, so the TLAS
//!   (which copies only changed transforms) is left alone for still soldiers;
//! - over budget, the shadow-only soldiers are dropped first, then the tail of the visible ones.

use bevy::prelude::*;
use bevy::solari::prelude::RaytracingMesh3d;

use super::solari::{SolariSettings, raytracing_mesh};
use crate::agent::dump::DumpAppExt;
use crate::app::AppState;
use crate::battle::units::crowd::ProxyCaster;
use crate::battle::units::skinned::{RECORD_FLOATS, SkinnedBatch, SkinnedColourCount};

/// Bytes of one instance record (`RECORD_FLOATS` little-endian `f32`).
const RECORD_BYTES: usize = RECORD_FLOATS * 4;

/// Height (m) at which idle proxies wait: below the sunk ground substitute, so they shade nothing.
const PARK_Y: f32 = -80.0;

/// Pose of a proxy: `[x, y, z, yaw in degrees]`.
type Pose = [f32; 4];

/// Marks a soldier proxy entity.
#[derive(Component)]
pub struct SoldierProxy;

/// Pool of proxies of one kind. `E` is the entity handle (generic so the logic is testable).
struct Pool<E: Copy> {
    /// Every proxy ever created with the pose last written to it.
    slots: Vec<(E, Pose)>,
    /// Number of leading slots in use.
    active: usize,
}

impl<E: Copy> Default for Pool<E> {
    fn default() -> Self {
        Self {
            slots: Vec::new(),
            active: 0,
        }
    }
}

/// What one [`Pool::sync`] did.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct SyncStats {
    spawned: usize,
    written: usize,
}

impl<E: Copy> Pool<E> {
    /// Makes slot `i` follow `targets[i]`; slots past the end are parked at `parked`.
    fn sync(
        &mut self,
        targets: &[Pose],
        parked: Pose,
        mut spawn: impl FnMut(Pose) -> E,
        mut write: impl FnMut(E, Pose),
    ) -> SyncStats {
        let mut stats = SyncStats::default();
        for (i, &target) in targets.iter().enumerate() {
            match self.slots.get_mut(i) {
                Some((entity, pose)) => {
                    if *pose != target {
                        write(*entity, target);
                        *pose = target;
                        stats.written += 1;
                    }
                }
                None => {
                    self.slots.push((spawn(target), target));
                    stats.spawned += 1;
                }
            }
        }
        let end = self.active.min(self.slots.len());
        for i in targets.len()..end {
            let (entity, pose) = &mut self.slots[i];
            if *pose != parked {
                write(*entity, parked);
                *pose = parked;
                stats.written += 1;
            }
        }
        self.active = targets.len();
        stats
    }
}

/// One regiment draw read for this frame.
struct Source<'a> {
    mounted: bool,
    records: &'a [u8],
    /// Records the raster shadow passes draw.
    count: usize,
    /// Leading records also drawn in colour (in the frustum).
    visible: usize,
}

/// Position and heading of record `index`.
fn record_pose(records: &[u8], index: usize) -> Pose {
    let at = |field: usize| {
        let o = index * RECORD_BYTES + field * 4;
        f32::from_le_bytes([records[o], records[o + 1], records[o + 2], records[o + 3]])
    };
    [at(0), at(1), at(2), at(3)]
}

/// Poses of the proxies to place, split by kind, at most `budget` in all: every visible soldier
/// first (regiment order), then the shadow-only ones.
fn collect(
    sources: &[Source],
    budget: usize,
    foot: &mut Vec<Pose>,
    mounted: &mut Vec<Pose>,
) -> usize {
    foot.clear();
    mounted.clear();
    let mut left = budget;
    for shadow_pass in [false, true] {
        for source in sources {
            let usable = source.count.min(source.records.len() / RECORD_BYTES);
            let visible = source.visible.min(usable);
            let range = if shadow_pass {
                visible..usable
            } else {
                0..visible
            };
            let target = if source.mounted {
                &mut *mounted
            } else {
                &mut *foot
            };
            for index in range.take(left) {
                target.push(record_pose(source.records, index));
                left -= 1;
            }
        }
    }
    budget - left
}

/// Box mesh data standing on y = 0 (centred on x and z), flat normals: 24 vertices, 12 triangles.
fn box_data(size: Vec3) -> (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<u32>) {
    let half = size * 0.5;
    let mut positions = Vec::with_capacity(24);
    let mut normals = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    // (normal, u axis, v axis): u x v = normal.
    let faces = [
        (Vec3::X, Vec3::Y, Vec3::Z),
        (Vec3::NEG_X, Vec3::Z, Vec3::Y),
        (Vec3::Y, Vec3::Z, Vec3::X),
        (Vec3::NEG_Y, Vec3::X, Vec3::Z),
        (Vec3::Z, Vec3::X, Vec3::Y),
        (Vec3::NEG_Z, Vec3::Y, Vec3::X),
    ];
    for (normal, u, v) in faces {
        let base = positions.len() as u32;
        for (su, sv) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let corner = (normal + u * su + v * sv) * half + Vec3::Y * half.y;
            positions.push(corner.to_array());
            normals.push(normal.to_array());
        }
        indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    (positions, normals, indices)
}

/// Handles shared by every proxy.
struct Shared {
    foot: Handle<Mesh>,
    mounted: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

/// Pools, caches and the counters of the dump.
#[derive(Resource, Default)]
pub struct SoldierProxies {
    foot: Pool<Entity>,
    mounted: Pool<Entity>,
    shared: Option<Shared>,
    /// Batch signature of the last update: (batch, version, count, visible).
    signature: Vec<(Entity, u64, u32, u32)>,
    foot_poses: Vec<Pose>,
    mounted_poses: Vec<Pose>,
    /// Scratch for the sources of the frame (batch, mounted), reused.
    order: Vec<(Entity, bool)>,
    /// Counters for the dump.
    stats: ProxyStats,
}

#[derive(Default, Clone, Copy)]
struct ProxyStats {
    updates: u64,
    skipped: u64,
    last_written: usize,
    last_wanted: usize,
    dropped_by_budget: usize,
}

pub fn reset_proxies(mut proxies: ResMut<SoldierProxies>) {
    *proxies = SoldierProxies::default();
}

fn transform_of(pose: Pose) -> Transform {
    Transform {
        translation: Vec3::new(pose[0], pose[1], pose[2]),
        rotation: Quat::from_rotation_y(pose[3].to_radians()),
        scale: Vec3::ONE,
    }
}

/// Dump section `solari_soldiers`.
pub fn dump_soldiers(world: &World) -> (String, serde_json::Value) {
    let value = match world.get_resource::<SoldierProxies>() {
        Some(p) => serde_json::json!({
            "proxies_foot": p.foot.slots.len(),
            "proxies_mounted": p.mounted.slots.len(),
            "active_foot": p.foot.active,
            "active_mounted": p.mounted.active,
            "updates": p.stats.updates,
            "skipped_unchanged": p.stats.skipped,
            "last_written": p.stats.last_written,
            "last_wanted": p.stats.last_wanted,
            "dropped_by_budget": p.stats.dropped_by_budget,
        }),
        None => serde_json::Value::Null,
    };
    ("solari_soldiers".to_owned(), value)
}

pub fn register(app: &mut App) {
    app.register_dump(dump_soldiers);
}

/// Creates the shared meshes and material once.
fn shared(
    slot: &mut Option<Shared>,
    settings: &SolariSettings,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> (Handle<Mesh>, Handle<Mesh>, Handle<StandardMaterial>) {
    let shared = slot.get_or_insert_with(|| {
        let mut make = |size: [f32; 3]| {
            let (positions, normals, indices) = box_data(Vec3::from(size));
            meshes.add(raytracing_mesh(positions, normals, indices))
        };
        Shared {
            foot: make(settings.soldier_foot_size_m),
            mounted: make(settings.soldier_mounted_size_m),
            material: materials.add(StandardMaterial {
                base_color: Color::linear_rgb(0.12, 0.1, 0.09),
                perceptual_roughness: 0.9,
                reflectance: 0.04,
                ..default()
            }),
        }
    });
    (
        shared.foot.clone(),
        shared.mounted.clone(),
        shared.material.clone(),
    )
}

/// Keeps one proxy per soldier the raster draws, within the budget.
#[allow(clippy::too_many_arguments)]
pub fn sync_soldier_proxies(
    mut commands: Commands,
    settings: Res<SolariSettings>,
    mut proxies: ResMut<SoldierProxies>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    batches: Query<(
        Entity,
        &SkinnedBatch,
        Option<&SkinnedColourCount>,
        &ProxyCaster,
    )>,
    mut transforms: Query<&mut Transform, With<SoldierProxy>>,
) {
    let proxies = &mut *proxies;
    // Stable order (entity id) so unchanged regiments keep their slots.
    proxies.order.clear();
    proxies.order.extend(
        batches
            .iter()
            .filter(|(_, batch, _, _)| batch.count > 0)
            .map(|(entity, _, _, caster)| (entity, caster.mounted)),
    );
    proxies.order.sort_unstable_by_key(|(entity, _)| *entity);

    let signature_of = |entity: Entity| {
        let (_, batch, colour, _) = batches.get(entity).expect("listed above");
        let visible = colour.map_or(batch.count, |c| c.0.min(batch.count));
        (entity, batch.version, batch.count, visible)
    };
    let same = proxies.signature.len() == proxies.order.len()
        && proxies
            .signature
            .iter()
            .zip(&proxies.order)
            .all(|(old, (entity, _))| *old == signature_of(*entity));
    if same {
        proxies.stats.skipped += 1;
        return;
    }
    proxies.signature.clear();
    for i in 0..proxies.order.len() {
        let entity = proxies.order[i].0;
        proxies.signature.push(signature_of(entity));
    }

    let sources: Vec<Source> = proxies
        .order
        .iter()
        .map(|&(entity, mounted)| {
            let (_, batch, colour, _) = batches.get(entity).expect("listed above");
            Source {
                mounted,
                records: &batch.records,
                count: batch.count as usize,
                visible: colour.map_or(batch.count, |c| c.0) as usize,
            }
        })
        .collect();
    let available: usize = sources
        .iter()
        .map(|s| s.count.min(s.records.len() / RECORD_BYTES))
        .sum();
    let budget = if settings.soldiers_in_tlas {
        settings.soldier_proxy_budget
    } else {
        0
    };
    let placed = collect(
        &sources,
        budget,
        &mut proxies.foot_poses,
        &mut proxies.mounted_poses,
    );
    drop(sources);
    proxies.stats.last_wanted = placed;
    proxies.stats.dropped_by_budget = available - placed;

    let (foot_mesh, mounted_mesh, material) =
        shared(&mut proxies.shared, &settings, &mut meshes, &mut materials);
    let parked_foot = [0.0, PARK_Y, 0.0, 0.0];
    let SoldierProxies {
        foot,
        mounted,
        foot_poses,
        mounted_poses,
        ..
    } = proxies;
    let mut written = 0;
    let mut write = |entity: Entity, pose: Pose| {
        if let Ok(mut transform) = transforms.get_mut(entity) {
            *transform = transform_of(pose);
        }
    };
    for (pool, poses, mesh) in [
        (foot, &*foot_poses, foot_mesh),
        (mounted, &*mounted_poses, mounted_mesh),
    ] {
        let stats = pool.sync(
            poses,
            parked_foot,
            |pose| {
                commands
                    .spawn((
                        SoldierProxy,
                        RaytracingMesh3d(mesh.clone()),
                        MeshMaterial3d(material.clone()),
                        transform_of(pose),
                        DespawnOnExit(AppState::Battle),
                    ))
                    .id()
            },
            &mut write,
        );
        written += stats.written + stats.spawned;
    }
    proxies.stats.updates += 1;
    proxies.stats.last_written = written;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(x: f32, y: f32, z: f32, yaw: f32) -> Vec<u8> {
        let mut floats = [0.0f32; RECORD_FLOATS];
        floats[..4].copy_from_slice(&[x, y, z, yaw]);
        floats.iter().flat_map(|f| f.to_le_bytes()).collect()
    }

    fn records(poses: &[Pose]) -> Vec<u8> {
        poses
            .iter()
            .flat_map(|p| record(p[0], p[1], p[2], p[3]))
            .collect()
    }

    const PARK: Pose = [0.0, PARK_Y, 0.0, 0.0];

    #[test]
    fn pool_spawns_then_reuses_and_parks() {
        let mut pool = Pool::<u32>::default();
        let mut next = 0;
        let mut spawned = Vec::new();
        let mut written = Vec::new();
        let a = [1.0, 0.0, 0.0, 0.0];
        let b = [2.0, 0.0, 0.0, 0.0];
        let c = [3.0, 0.0, 0.0, 0.0];
        let stats = pool.sync(
            &[a, b, c],
            PARK,
            |p| {
                next += 1;
                spawned.push((next, p));
                next
            },
            |e, p| written.push((e, p)),
        );
        assert_eq!(
            stats,
            SyncStats {
                spawned: 3,
                written: 0
            }
        );
        assert_eq!(pool.active, 3);
        // Same targets: nothing written (still soldiers do not touch the TLAS).
        let stats = pool.sync(&[a, b, c], PARK, |_| unreachable!(), |_, _| unreachable!());
        assert_eq!(stats, SyncStats::default());
        // One moved, one removed from the tail: one write + one park, no spawn.
        let moved = [1.5, 0.0, 0.0, 10.0];
        let stats = pool.sync(
            &[moved, b],
            PARK,
            |_| unreachable!(),
            |e, p| written.push((e, p)),
        );
        assert_eq!(
            stats,
            SyncStats {
                spawned: 0,
                written: 2
            }
        );
        assert_eq!(written, vec![(1, moved), (3, PARK)]);
        assert_eq!(pool.active, 2);
        // Back to three: the parked slot is reused, not respawned.
        written.clear();
        let stats = pool.sync(
            &[moved, b, c],
            PARK,
            |_| unreachable!(),
            |e, p| written.push((e, p)),
        );
        assert_eq!(
            stats,
            SyncStats {
                spawned: 0,
                written: 1
            }
        );
        assert_eq!(written, vec![(3, c)]);
        // Fully emptied: everything parks once, then nothing more.
        let stats = pool.sync(&[], PARK, |_| unreachable!(), |_, _| {});
        assert_eq!(stats.written, 3);
        let stats = pool.sync(&[], PARK, |_| unreachable!(), |_, _| unreachable!());
        assert_eq!(stats, SyncStats::default());
        assert_eq!(pool.slots.len(), 3);
    }

    #[test]
    fn records_decode_to_poses() {
        let bytes = records(&[[1.0, 2.0, 3.0, 90.0], [4.0, 5.0, 6.0, 180.0]]);
        assert_eq!(record_pose(&bytes, 1), [4.0, 5.0, 6.0, 180.0]);
    }

    #[test]
    fn collect_puts_visible_first_and_drops_shadow_only_over_budget() {
        let a = records(&[[0.0; 4], [1.0; 4], [2.0; 4]]); // 2 visible + 1 shadow-only
        let b = records(&[[10.0; 4], [11.0; 4]]); // horsemen, 1 visible + 1 shadow-only
        let sources = [
            Source {
                mounted: false,
                records: &a,
                count: 3,
                visible: 2,
            },
            Source {
                mounted: true,
                records: &b,
                count: 2,
                visible: 1,
            },
        ];
        let (mut foot, mut mounted) = (Vec::new(), Vec::new());
        assert_eq!(collect(&sources, 10, &mut foot, &mut mounted), 5);
        assert_eq!(foot, vec![[0.0; 4], [1.0; 4], [2.0; 4]]);
        assert_eq!(mounted, vec![[10.0; 4], [11.0; 4]]);
        // Budget 3: both visible foot + the visible horseman; shadow-only soldiers are dropped first.
        assert_eq!(collect(&sources, 3, &mut foot, &mut mounted), 3);
        assert_eq!(foot, vec![[0.0; 4], [1.0; 4]]);
        assert_eq!(mounted, vec![[10.0; 4]]);
        // Budget 0 (soldiers_in_tlas off): nothing.
        assert_eq!(collect(&sources, 0, &mut foot, &mut mounted), 0);
        assert!(foot.is_empty() && mounted.is_empty());
    }

    #[test]
    fn collect_clamps_counts_to_the_record_bytes() {
        let a = records(&[[7.0; 4]]);
        let sources = [Source {
            mounted: false,
            records: &a,
            count: 5,
            visible: 9,
        }];
        let (mut foot, mut mounted) = (Vec::new(), Vec::new());
        assert_eq!(collect(&sources, 100, &mut foot, &mut mounted), 1);
    }

    #[test]
    fn box_is_a_closed_twelve_triangle_solid_standing_on_the_ground() {
        let (positions, normals, indices) = box_data(Vec3::new(0.5, 1.8, 0.4));
        assert_eq!(
            (positions.len(), normals.len(), indices.len()),
            (24, 24, 36)
        );
        let min_y = positions.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
        let max_y = positions.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
        assert!(min_y.abs() < 1e-6 && (max_y - 1.8).abs() < 1e-6);
        // Every triangle is wound counter-clockwise seen from outside (geometric normal = vertex normal).
        for t in indices.chunks(3) {
            let [a, b, c] = [t[0], t[1], t[2]].map(|i| Vec3::from(positions[i as usize]));
            let geometric = (b - a).cross(c - a).normalize();
            assert!(geometric.dot(Vec3::from(normals[t[0] as usize])) > 0.99);
        }
    }

    #[test]
    fn yaw_rotates_like_the_skinning_shader() {
        // skinned.wesl rotate_y: x' = c x + s z, z' = -s x + c z.
        let rotation = transform_of([0.0, 0.0, 0.0, 90.0]).rotation;
        let v = rotation * Vec3::X;
        assert!((v - Vec3::new(0.0, 0.0, -1.0)).length() < 1e-5);
    }
}
