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

/// Copies [`ReliefGpu`] into the materials of type `M` whose copy is stale. Checked every frame:
/// a material replaced in place (same asset count, `ReliefGpu` unchanged) kept the default zero
/// uniform, which drew every river and road at sea level, buried under the land (showfix, 16/10).
pub fn sync_drape<M: Drape>(gpu: Option<Res<ReliefGpu>>, mut materials: ResMut<Assets<M>>) {
    let Some(gpu) = gpu else {
        return;
    };
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

/// Source of the shared relief module (`terrain/relief.wesl`).
const RELIEF_SOURCE: &str = include_str!("../terrain/relief.wesl");

/// A draped shader with the relief module inlined: the `import` line is dropped and the module's
/// functions are prepended, so the shader does not depend on cross-module resolution at runtime
/// (an unresolved import silently keeps the pipeline pending).
pub fn with_relief(source: &str) -> String {
    const IMPORT: &str = "import cent_ans_client::campaign::terrain::relief";
    if !source.contains(IMPORT) {
        return source.to_owned();
    }
    let lines: Vec<&str> = source
        .lines()
        .filter(|l| !l.trim_start().starts_with(IMPORT))
        .collect();
    // WESL wants every `import` before the first declaration: the module goes after the last one.
    let after_imports = lines
        .iter()
        .rposition(|l| l.trim_start().starts_with("import "))
        .map_or(0, |i| i + 1);
    format!(
        "{}\n{RELIEF_SOURCE}\n{}",
        lines[..after_imports].join("\n"),
        lines[after_imports..].join("\n")
    )
}
