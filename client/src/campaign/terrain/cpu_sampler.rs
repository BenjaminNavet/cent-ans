//! CPU ground sampler fed to the lots that cannot use the [`super::surface::GroundHeight`]
//! `SystemParam` (they hold a plain `Arc<dyn Fn>` / trait object): `water::height::GroundHeight`
//! and `provinces::SurfaceHeight`. A snapshot of [`Surface`] + [`ReliefState`] (cheap clones: the
//! big arrays are `Arc`ed) is refreshed every frame in `PreUpdate`; the samplers read it.

use std::sync::{Arc, RwLock};

use bevy::prelude::*;

use super::relief_state::ReliefState;
use super::surface::Surface;
use crate::campaign::provinces::SurfaceHeight;
use crate::campaign::water::RebuildWater;
use crate::campaign::water::height::{GroundHeight, HeightSampler};

#[derive(Default)]
struct Snapshot {
    surface: Option<Surface>,
    relief: Option<ReliefState>,
    surface_version: u64,
}

/// Shared snapshot behind the injected samplers.
#[derive(Resource, Clone, Default)]
pub struct CpuSampler(Arc<RwLock<Snapshot>>);

impl CpuSampler {
    /// Displayed ground height (world units, >= 0); 0 before the terrain is loaded.
    pub fn display_height(&self, x: f32, z: f32) -> f32 {
        let snap = self.0.read().expect("cpu sampler");
        match (&snap.surface, &snap.relief) {
            (Some(surface), Some(relief)) => surface.display_height_at(relief, x, z),
            _ => 0.0,
        }
    }

    pub fn height_m(&self, x: f32, z: f32) -> f32 {
        let snap = self.0.read().expect("cpu sampler");
        snap.surface.as_ref().map_or(0.0, |s| s.height_m_at(x, z))
    }

    pub fn is_ready(&self) -> bool {
        self.0.read().expect("cpu sampler").surface.is_some()
    }
}

impl HeightSampler for CpuSampler {
    fn height_m(&self, x: f32, y: f32) -> f32 {
        CpuSampler::height_m(self, x, y)
    }

    fn surface_world(&self, x: f32, y: f32) -> f32 {
        self.display_height(x, y)
    }

    fn seat_height_m(&self, x: f32, y: f32) -> f32 {
        let snap = self.0.read().expect("cpu sampler");
        snap.surface
            .as_ref()
            .map_or(0.0, |s| s.highest_level_height_m(x, y))
    }
}

/// Installs the samplers into the water and province lots (every campaign visit).
pub fn install_samplers(mut commands: Commands, sampler: Res<CpuSampler>) {
    commands.insert_resource(GroundHeight(Arc::new(sampler.clone())));
    let for_picker = sampler.clone();
    commands.insert_resource(SurfaceHeight(Arc::new(move |x, z| {
        for_picker.display_height(x, z)
    })));
}

/// Refreshes the snapshot; the first time the terrain is ready, asks the water lot to rebuild.
pub fn sync_snapshot(
    sampler: Res<CpuSampler>,
    surface: Option<Res<Surface>>,
    relief: Option<Res<ReliefState>>,
    mut rebuild: MessageWriter<RebuildWater>,
) {
    let (Some(surface), Some(relief)) = (surface, relief) else {
        return;
    };
    if !surface.is_changed() && !relief.is_changed() {
        return;
    }
    let mut snap = sampler.0.write().expect("cpu sampler");
    let first = snap.surface.is_none();
    if first || snap.surface_version != surface.version {
        snap.surface = Some((*surface).clone());
        snap.surface_version = surface.version;
    }
    snap.relief = Some((*relief).clone());
    drop(snap);
    if first {
        rebuild.write(RebuildWater);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_sampler_is_flat_and_not_ready() {
        let sampler = CpuSampler::default();
        assert_eq!(sampler.display_height(10.0, 10.0), 0.0);
        assert!(!sampler.is_ready());
        assert_eq!(HeightSampler::surface_world(&sampler, 1.0, 2.0), 0.0);
    }
}
