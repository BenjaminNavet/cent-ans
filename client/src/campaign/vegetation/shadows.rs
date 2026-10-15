//! Ground shadows of the campaign trees.
//!
//! Godot casts the tree MultiMeshes into its shadow maps up to `generalised_shadow_distance`
//! (70 units of camera distance, `Vegetation._shadow_distance`). The instanced vegetation
//! pipeline has no shadow pass, so each tree batch gets a twin batch of soft ellipses cast along
//! the sun on the ground (`Mode::Shadow` of `billboard.wesl`), sharing its records, faded out
//! with the camera distance.

use std::collections::HashMap;

use bevy::prelude::*;

use super::billboard::{Mode, Params, VegBatch};
use super::data::style::CROWN_WIDEN;
use super::{VegRender, VegView, VegWorld};
use crate::app::AppState;

/// Full shadows up to this camera distance, none beyond `FADE_END` (`generalised_shadow_distance`).
pub const FADE_START: f32 = 60.0;
pub const FADE_END: f32 = 90.0;
/// Share of the ground light removed under a crown (direct sun lost, sky light kept).
pub const STRENGTH: f32 = 0.42;

/// Opacity of the shadows for a camera at `distance` of its focus.
pub fn opacity(distance: f32) -> f32 {
    let t = ((distance - FADE_START) / (FADE_END - FADE_START)).clamp(0.0, 1.0);
    STRENGTH * (1.0 - t * t * (3.0 - 2.0 * t))
}

/// Twin shadow batches of the tree batches: `(row, model)` -> entity, with the copied version.
#[derive(Resource, Default)]
pub struct TreeShadows {
    pub entities: HashMap<(u16, bool), (Entity, u64)>,
    pub opacity: f32,
}

pub(super) fn reset(mut commands: Commands, mut shadows: ResMut<TreeShadows>) {
    for (_, (entity, _)) in shadows.entities.drain() {
        commands.entity(entity).try_despawn();
    }
}

pub(super) fn sync_shadows(
    mut commands: Commands,
    view: Res<VegView>,
    world: Option<Res<VegWorld>>,
    render: Res<VegRender>,
    mut shadows: ResMut<TreeShadows>,
    mut batches: Query<&mut VegBatch>,
    suns: Query<&GlobalTransform, With<DirectionalLight>>,
) {
    // Direction to the sun (a directional light shines along its forward axis).
    let to_sun = suns
        .iter()
        .next()
        .map_or(Vec3::new(0.3, 0.8, 0.5), |t| -t.forward().as_vec3())
        .normalize_or(Vec3::Y);
    let Some(world) = world else { return };
    let alpha = if view.active {
        opacity(view.distance)
    } else {
        0.0
    };
    shadows.opacity = alpha;
    let quad = render.assets.quad.clone();
    let shadows = &mut *shadows;
    shadows.entities.retain(|key, (entity, _)| {
        let keep = alpha > 0.0 && render.entities.contains_key(key);
        if !keep {
            commands.entity(*entity).try_despawn();
        }
        keep
    });
    let Some(quad) = quad.filter(|_| alpha > 0.0) else {
        return;
    };
    for (key, source) in &render.entities {
        let Ok(src) = batches.get(*source) else {
            continue;
        };
        let (records, count, version, albedo) = (
            src.records.clone(),
            src.count,
            src.version,
            src.albedo.clone(),
        );
        match shadows.entities.get_mut(key) {
            Some((entity, copied)) => {
                let Ok(mut batch) = batches.get_mut(*entity) else {
                    continue;
                };
                if *copied != version {
                    batch.records = records;
                    batch.count = count;
                    batch.version += 1;
                    *copied = version;
                }
                let range = to_sun.extend(alpha);
                if batch.params.range.distance_squared(range) > 1e-6 {
                    batch.params.range = range;
                    batch.params_version += 1;
                }
            }
            None => {
                let ratio = world
                    .0
                    .width_ratio
                    .get(key.0 as usize)
                    .copied()
                    .unwrap_or(1.0);
                let mut params = Params::new(Mode::Shadow);
                params.extra.x = ratio * CROWN_WIDEN;
                params.extra.y = f32::from(u8::from(key.1));
                params.range = to_sun.extend(alpha);
                let entity = commands
                    .spawn((
                        Name::new("TreeShadows"),
                        VegBatch {
                            mesh_handle: quad.clone(),
                            records,
                            count,
                            albedo,
                            normal: None,
                            params,
                            version: 1,
                            params_version: 0,
                        },
                        DespawnOnExit(AppState::Campaign),
                    ))
                    .id();
                shadows.entities.insert(*key, (entity, version));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadows_fade_with_the_camera_distance() {
        assert_eq!(opacity(20.0), STRENGTH);
        assert_eq!(opacity(FADE_END + 1.0), 0.0);
        let mid = opacity((FADE_START + FADE_END) * 0.5);
        assert!(mid > 0.0 && mid < STRENGTH);
    }
}
