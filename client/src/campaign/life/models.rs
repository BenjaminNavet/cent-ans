//! Glb scenes of the living map: spawn, size fitting and fade (no pop, ADR 0354).
//!
//! An instance is `Root` (placement, heading, fade scale) -> `Model` (yaw fix, fitted scale, lift)
//! -> `SceneRoot`. The native size of a glb is measured once (from the `Aabb`s of its meshes,
//! cached in [`ModelMetrics`]); the scale then gives it a fixed world size. Nothing appears or
//! disappears abruptly: instances grow from zero and shrink before being despawned.

use std::collections::HashMap;

use bevy::camera::primitives::Aabb;
use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};

/// What dimension of the model is fitted to the requested size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FitAxis {
    /// Longest horizontal side.
    Length,
    /// No measure: `target` is the uniform scale of the file (metres of the glb as they are).
    Native,
}

/// Native extent of a glb (metres of the file) and its lowest point.
#[derive(Debug, Clone, Copy)]
pub struct Metrics {
    pub horizontal: f32,
    pub min_y: f32,
}

#[derive(Resource, Default)]
pub struct ModelMetrics {
    pub by_path: HashMap<String, Metrics>,
}

/// Model child waiting for its size (`target` world units along `axis`).
#[derive(Component)]
pub struct FitModel {
    pub path: String,
    pub axis: FitAxis,
    pub target: f32,
    /// Extra uniform factor applied on top of the fit (1 = exact).
    pub factor: f32,
}

/// Root of one instance: grows to `target` (0..1) at `rate` per second.
#[derive(Component)]
pub struct Fade {
    pub current: f32,
    pub target: f32,
    pub base_scale: f32,
}

impl Fade {
    pub fn new(base_scale: f32) -> Self {
        Self {
            current: 0.0,
            target: 1.0,
            base_scale,
        }
    }
}

impl Fade {
    /// Eased visible share in [0, 1] (what `apply_fades` writes into the scale).
    pub fn visible(&self) -> f32 {
        let c = self.current.clamp(0.0, 1.0);
        c * c * (3.0 - 2.0 * c)
    }
}

/// Seconds of a full fade.
pub const FADE_SECONDS: f32 = 0.6;

/// Spawns the `Model` child (scene + fit) without a root; the caller parents it. `yaw_fix` turns
/// the model so that its front points along +X (heading axis of this module).
pub fn spawn_model(
    commands: &mut Commands,
    assets: &AssetServer,
    metrics: &ModelMetrics,
    path: &str,
    axis: FitAxis,
    target: f32,
    yaw_fix: f32,
) -> Entity {
    let handle: Handle<WorldAsset> =
        assets.load(GltfAssetLabel::Scene(0).from_asset(path.to_string()));
    let cached = if axis == FitAxis::Native {
        Some(Metrics {
            horizontal: 1.0,
            min_y: 0.0,
        })
    } else {
        metrics.by_path.get(path).copied()
    };
    let (scale, lift, visible) = match cached {
        Some(m) => {
            let k = fit_scale(m, axis, target);
            (k, -m.min_y * k, true)
        }
        None => (1.0, 0.0, false),
    };
    let model = commands
        .spawn((
            Name::new("life:model"),
            Transform::from_translation(Vec3::Y * lift)
                .with_rotation(Quat::from_rotation_y(yaw_fix))
                .with_scale(Vec3::splat(scale)),
            if visible {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            },
            FitModel {
                path: path.to_string(),
                axis,
                target,
                factor: 1.0,
            },
        ))
        .with_children(|p| {
            p.spawn((Name::new("life:scene"), WorldAssetRoot(handle)));
        })
        .id();
    if cached.is_some() {
        commands.entity(model).try_insert(Fitted);
    }
    model
}

/// Spawns the instance root (placement, heading, fade) holding one model; returns the root.
pub fn spawn_instance(
    commands: &mut Commands,
    assets: &AssetServer,
    metrics: &ModelMetrics,
    path: &str,
    axis: FitAxis,
    target: f32,
    yaw_fix: f32,
    position: Vec3,
    heading: Quat,
    extra: impl Bundle,
) -> Entity {
    let model = spawn_model(commands, assets, metrics, path, axis, target, yaw_fix);
    commands
        .spawn((
            Name::new("life:instance"),
            Transform::from_translation(position)
                .with_rotation(heading)
                .with_scale(Vec3::splat(1e-4)),
            Visibility::Inherited,
            Fade::new(1.0),
            extra,
        ))
        .add_child(model)
        .id()
}

/// Marker: the model child has its final scale.
#[derive(Component)]
pub struct Fitted;

pub fn fit_scale(m: Metrics, axis: FitAxis, target: f32) -> f32 {
    let native = match axis {
        FitAxis::Length => m.horizontal,
        FitAxis::Native => 1.0,
    };
    if native > 1e-4 { target / native } else { 1.0 }
}

/// Meshes of the living map's models never receive shadows: their triangles are sub-pixel from
/// the campaign cameras and flicker in and out of the cascaded shadow (black and white static on
/// carts, oxen and herds).
pub fn unshadow_model_meshes(
    mut commands: Commands,
    added: Query<Entity, (Added<Mesh3d>, Without<bevy::light::NotShadowReceiver>)>,
    parents: Query<&ChildOf>,
    models: Query<(), With<FitModel>>,
) {
    for entity in &added {
        let mut at = entity;
        while let Ok(parent) = parents.get(at) {
            at = parent.parent();
            if models.contains(at) {
                commands
                    .entity(entity)
                    .try_insert(bevy::light::NotShadowReceiver);
                break;
            }
        }
    }
}

/// Measures models whose meshes are ready and gives them their final scale.
pub fn fit_models(
    mut commands: Commands,
    mut metrics: ResMut<ModelMetrics>,
    pending: Query<(Entity, &FitModel), Without<Fitted>>,
    children: Query<&Children>,
    meshes: Query<(&Aabb, &GlobalTransform), With<Mesh3d>>,
    globals: Query<&GlobalTransform>,
    mut fit_transforms: Query<&mut Transform, With<FitModel>>,
    mut visibility: Query<&mut Visibility>,
) {
    for (entity, fit) in &pending {
        // Native bounds in the frame of the model child, with its own scale removed.
        let Ok(model_global) = globals.get(entity) else {
            continue;
        };
        let inverse = model_global.affine().inverse();
        let mut lo = Vec3::splat(f32::MAX);
        let mut hi = Vec3::splat(f32::MIN);
        let mut found = false;
        for d in children.iter_descendants(entity) {
            let Ok((aabb, global)) = meshes.get(d) else {
                continue;
            };
            found = true;
            let c = Vec3::from(aabb.center);
            let h = Vec3::from(aabb.half_extents);
            for sx in [-1.0, 1.0] {
                for sy in [-1.0, 1.0] {
                    for sz in [-1.0, 1.0] {
                        let corner = global
                            .affine()
                            .transform_point3(c + h * Vec3::new(sx, sy, sz));
                        let local = inverse.transform_point3(corner);
                        lo = lo.min(local);
                        hi = hi.max(local);
                    }
                }
            }
        }
        if !found || hi.x < lo.x {
            continue;
        }
        // `inverse` removes the model's yaw and scale, so these are native metres in the
        // model frame (front along the model's own axes before the yaw fix is applied).
        let size = hi - lo;
        let m = Metrics {
            horizontal: size.x.max(size.z),
            min_y: lo.y,
        };
        metrics.by_path.insert(fit.path.clone(), m);
        let k = fit_scale(m, fit.axis, fit.target) * fit.factor;
        if let Ok(mut t) = fit_transforms.get_mut(entity) {
            t.scale = Vec3::splat(k);
            t.translation.y = -m.min_y * k;
        }
        if let Ok(mut v) = visibility.get_mut(entity) {
            *v = Visibility::Inherited;
        }
        commands.entity(entity).try_insert(Fitted);
    }
}

/// Moves every fade toward its target and writes the root scale; despawns faded-out roots.
pub fn apply_fades(
    time: Res<Time>,
    mut commands: Commands,
    mut index: ResMut<super::stream::LifeIndex>,
    mut query: Query<(
        Entity,
        &mut Fade,
        &mut Transform,
        Option<&super::stream::LifeItem>,
    )>,
) {
    let step = time.delta_secs() / FADE_SECONDS;
    for (entity, mut fade, mut transform, item) in &mut query {
        let t = fade.target;
        if fade.current < t {
            fade.current = (fade.current + step).min(t);
        } else if fade.current > t {
            fade.current = (fade.current - step).max(t);
        }
        let eased = fade.current * fade.current * (3.0 - 2.0 * fade.current);
        transform.scale = Vec3::splat(fade.base_scale * eased.max(1e-4));
        if fade.target <= 0.0 && fade.current <= 0.0 {
            if let Some(item) = item {
                index.0.remove(&item.key);
            }
            commands.entity(entity).despawn();
        }
    }
}
