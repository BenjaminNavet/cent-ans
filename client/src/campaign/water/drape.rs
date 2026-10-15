//! Draping of the water and province overlays on the dynamic exaggerated relief.
//!
//! Meshes of the lot carry the altitude in METRES in `position.y`; the vertex shaders convert it to
//! the displayed height with `relief_display_height` (module `campaign/terrain/relief.wesl`), so a
//! change of camera zoom (which changes the relief) needs no mesh rebuild. Each draped material
//! binds the shared [`ReliefParams`] uniform and the floor texture in its own bind group and
//! implements [`Drape`]; [`CampaignDrapePlugin`] copies [`ReliefGpu`] into every such material.

use bevy::prelude::*;

use crate::campaign::terrain::relief_gpu::{ReliefGpu, ReliefParams};

/// A material that binds the shared relief resources.
pub trait Drape: Asset {
    fn set_relief(&mut self, params: ReliefParams, floor: &Handle<Image>);
    fn relief(&self) -> (ReliefParams, &Handle<Image>);
}

/// Copies [`ReliefGpu`] into the materials of type `M` when it changed or when materials were
/// added since the last run.
pub fn sync_drape<M: Drape>(
    gpu: Option<Res<ReliefGpu>>,
    mut materials: ResMut<Assets<M>>,
    mut known: Local<usize>,
) {
    let Some(gpu) = gpu else {
        return;
    };
    let count = materials.len();
    if !gpu.is_changed() && count == *known {
        return;
    }
    *known = count;
    let ids: Vec<AssetId<M>> = materials.ids().collect();
    for id in ids {
        let stale = materials.get(id).is_some_and(|m| {
            let (params, floor) = m.relief();
            params != gpu.params || *floor != gpu.floor
        });
        if stale && let Some(mut material) = materials.get_mut(id) {
            material.set_relief(gpu.params, &gpu.floor);
        }
    }
}

/// Registers [`sync_drape`] for a material type.
pub fn add_drape<M: Drape>(app: &mut App) {
    app.add_systems(PostUpdate, sync_drape::<M>);
}
