//! Model prototypes: a glb is instanced once as a hidden scene, its mesh primitives (mesh handle,
//! material handle, transform relative to the scene root) are read back, and every placed prop
//! then spawns those primitives directly (`spawn_prims`) instead of a whole scene.
//!
//! Why: a glb scene per prop costs a scene root, the node hierarchy and a scene-spawn pass per
//! instance, which capped the live props at 1800. The primitives of every instance of a model
//! share the same `Handle<Mesh>` and `Handle<StandardMaterial>` (labelled assets of the glb), so
//! Bevy's automatic batching draws them as GPU instances (one draw per mesh and material, the
//! per-instance transforms in the instance buffer) and an instance is only `1 + primitives`
//! entities with no hierarchy below. Godot did the same with one `MultiMesh` per model and cell.

use std::collections::HashMap;
use std::sync::Arc;

use bevy::camera::primitives::Aabb;
use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};

use crate::app::AppState;

/// Frames a prototype may stay unloaded before it is given up.
const GIVE_UP_FRAMES: u32 = 1800;

/// One mesh primitive of a model.
#[derive(Clone)]
pub struct ProtoPrim {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
    /// Transform relative to the model root.
    pub transform: Transform,
}

/// A model: its primitives and its bounding box in the model root space.
pub struct Proto {
    pub prims: Vec<ProtoPrim>,
    pub lo: Vec3,
    pub hi: Vec3,
}

/// Answer of [`Prototypes::request`].
pub enum Lookup {
    Ready(Arc<Proto>),
    Pending,
    Failed,
}

enum State {
    Loading {
        root: Entity,
        handle: Handle<WorldAsset>,
        frames: u32,
    },
    Ready(Arc<Proto>),
    Failed,
}

#[derive(Resource, Default)]
pub struct Prototypes {
    map: HashMap<String, State>,
    /// Albedo gain per model path (`albedo_gain` of the prop), applied once when it is captured.
    gains: HashMap<String, f32>,
}

/// Root of a loading prototype scene.
#[derive(Component)]
pub struct ProtoRoot {
    pub path: String,
}

impl Prototypes {
    /// The model of `path` (a glb under the asset root); starts loading it on the first call.
    pub fn request(&mut self, path: &str, commands: &mut Commands, server: &AssetServer) -> Lookup {
        match self.map.get(path) {
            Some(State::Ready(p)) => return Lookup::Ready(p.clone()),
            Some(State::Failed) => return Lookup::Failed,
            Some(State::Loading { .. }) => return Lookup::Pending,
            None => {}
        }
        let handle: Handle<WorldAsset> =
            server.load(GltfAssetLabel::Scene(0).from_asset(path.to_string()));
        let root = commands
            .spawn((
                Name::new("countryside:prototype"),
                // Hidden and far away: only its descendants' components are read.
                Transform::from_xyz(0.0, -1.0e6, 0.0),
                Visibility::Hidden,
                WorldAssetRoot(handle.clone()),
                ProtoRoot {
                    path: path.to_string(),
                },
                DespawnOnExit(AppState::Campaign),
            ))
            .id();
        self.map.insert(
            path.to_string(),
            State::Loading {
                root,
                handle,
                frames: 0,
            },
        );
        Lookup::Pending
    }

    /// Records the albedo gain of a model (before its first request): the dark weathered textures
    /// of the windmills read as black silhouettes under the campaign light without it.
    pub fn set_gain(&mut self, path: &str, gain: f32) {
        if (gain - 1.0).abs() > 1e-4 && !self.gains.contains_key(path) {
            self.gains.insert(path.to_string(), gain);
        }
    }

    pub fn ready_count(&self) -> usize {
        self.map
            .values()
            .filter(|s| matches!(s, State::Ready(_)))
            .count()
    }

    pub fn loading_count(&self) -> usize {
        self.map
            .values()
            .filter(|s| matches!(s, State::Loading { .. }))
            .count()
    }

    pub fn failed_count(&self) -> usize {
        self.map
            .values()
            .filter(|s| matches!(s, State::Failed))
            .count()
    }

    /// Drops the loading entries (their scene roots go with the campaign state).
    pub fn reset(&mut self) {
        self.map.retain(|_, s| matches!(s, State::Ready(_)));
    }
}

/// Reads the primitives of the prototype scenes that finished spawning, then removes them.
pub fn capture_prototypes(
    mut commands: Commands,
    mut protos: ResMut<Prototypes>,
    server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    transforms: Query<&Transform>,
    prims: Query<(
        &Mesh3d,
        &MeshMaterial3d<StandardMaterial>,
        &Transform,
        Option<&Aabb>,
    )>,
) {
    let loading: Vec<(String, Entity)> = protos
        .map
        .iter()
        .filter_map(|(path, s)| match s {
            State::Loading { root, .. } => Some((path.clone(), *root)),
            _ => None,
        })
        .collect();
    for (path, root) in loading {
        let mut found: Vec<ProtoPrim> = Vec::new();
        let mut lo = Vec3::splat(f32::MAX);
        let mut hi = Vec3::splat(f32::MIN);
        for d in children.iter_descendants(root) {
            let Ok((mesh, material, local, aabb)) = prims.get(d) else {
                continue;
            };
            // Transform relative to the root: the chain of the local transforms up to it.
            let mut t = *local;
            let mut at = d;
            while let Ok(parent) = parents.get(at) {
                let p = parent.parent();
                if p == root {
                    break;
                }
                if let Ok(pt) = transforms.get(p) {
                    t = pt.mul_transform(t);
                }
                at = p;
            }
            if let Some(aabb) = aabb {
                let centre = Vec3::from(aabb.center);
                let half = Vec3::from(aabb.half_extents);
                for sx in [-1.0, 1.0] {
                    for sy in [-1.0, 1.0] {
                        for sz in [-1.0, 1.0] {
                            let corner = t.transform_point(centre + half * Vec3::new(sx, sy, sz));
                            lo = lo.min(corner);
                            hi = hi.max(corner);
                        }
                    }
                }
            }
            found.push(ProtoPrim {
                mesh: mesh.0.clone(),
                material: material.0.clone(),
                transform: t,
            });
        }
        let Some(State::Loading { handle, frames, .. }) = protos.map.get_mut(&path) else {
            continue;
        };
        if found.is_empty() {
            *frames += 1;
            let failed = matches!(
                server.get_load_state(handle.id()),
                Some(bevy::asset::LoadState::Failed(_))
            );
            if failed || *frames > GIVE_UP_FRAMES {
                warn!("countryside: model {path} could not be loaded");
                commands.entity(root).despawn();
                protos.map.insert(path, State::Failed);
            }
            continue;
        }
        if hi.x < lo.x {
            lo = Vec3::ZERO;
            hi = Vec3::ZERO;
        }
        commands.entity(root).despawn();
        if let Some(gain) = protos.gains.get(&path).copied() {
            let mut seen = Vec::new();
            for prim in &found {
                if seen.contains(&prim.material.id()) {
                    continue;
                }
                seen.push(prim.material.id());
                if let Some(mut material) = materials.get_mut(&prim.material) {
                    let c = material.base_color.to_linear();
                    material.base_color =
                        Color::linear_rgba(c.red * gain, c.green * gain, c.blue * gain, c.alpha);
                }
            }
        }
        protos.map.insert(
            path,
            State::Ready(Arc::new(Proto {
                prims: found,
                lo,
                hi,
            })),
        );
    }
}

/// Spawns the primitives of `proto` as plain entities with `extra` on each, `offset` applied in the
/// model space before the primitive's own transform. Returns them (to be children of the prop).
pub fn spawn_prims<B: Bundle + Clone>(
    commands: &mut Commands,
    proto: &Proto,
    offset: Vec3,
    extra: B,
) -> Vec<Entity> {
    proto
        .prims
        .iter()
        .map(|p| {
            commands
                .spawn((
                    Mesh3d(p.mesh.clone()),
                    MeshMaterial3d(p.material.clone()),
                    Transform::from_translation(offset).mul_transform(p.transform),
                    Visibility::Inherited,
                    bevy::light::NotShadowCaster,
                    extra.clone(),
                ))
                .id()
        })
        .collect()
}

pub fn reset(mut protos: ResMut<Prototypes>) {
    protos.reset();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offset_composes_before_the_primitive_transform() {
        let p = Transform::from_xyz(1.0, 2.0, 3.0);
        let t = Transform::from_translation(Vec3::new(-1.0, 0.0, 0.0)).mul_transform(p);
        assert_eq!(t.translation, Vec3::new(0.0, 2.0, 3.0));
    }

    #[test]
    fn unknown_paths_are_not_counted_ready() {
        let protos = Prototypes::default();
        assert_eq!(protos.ready_count(), 0);
        assert_eq!(protos.loading_count(), 0);
    }
}
