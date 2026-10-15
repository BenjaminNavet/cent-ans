//! Lighting of the vegetation copied from the terrain material.
//!
//! Godot lights the foliage and the ground with the same scene light; here the ground has its own
//! shading (`terrain.wesl`: ambient + sun x N.L, distance fog), so the vegetation batches take the
//! same sun, ambient and fog uniforms (`TerrainParams`) every time they change, and a tree reads
//! against the ground as in Godot instead of through Bevy's PBR exposure.

use bevy::prelude::*;

use super::billboard::VegBatch;
use crate::campaign::terrain::material::TerrainMaterial;

/// Light uniforms of the vegetation (`sun_dir`, `sun_color`, `ambient`, `fog`, `fog_range`).
#[derive(Resource, Default, Clone, Copy, PartialEq, Debug)]
pub struct VegLight {
    pub sun_dir: Vec4,
    pub sun_color: Vec4,
    pub ambient: Vec4,
    pub fog: Vec4,
    pub fog_range: Vec4,
}

impl VegLight {
    pub fn of(params: &crate::campaign::terrain::material::TerrainParams) -> VegLight {
        VegLight {
            sun_dir: params.sun_dir.truncate().normalize_or(Vec3::Y).extend(1.0),
            sun_color: params.sun_color,
            ambient: params.ambient,
            fog: params.fog,
            fog_range: Vec4::new(params.map.z, params.edge.w, 0.0, 0.0),
        }
    }
}

pub(super) fn sync_light(
    terrain: Query<&MeshMaterial3d<TerrainMaterial>>,
    materials: Res<Assets<TerrainMaterial>>,
    mut light: ResMut<VegLight>,
    mut batches: Query<&mut VegBatch>,
) {
    if let Some(material) = terrain.iter().next().and_then(|m| materials.get(&m.0)) {
        let current = VegLight::of(&material.params);
        if current != *light {
            *light = current;
        }
    }
    if light.sun_dir.w < 0.5 {
        return;
    }
    for mut batch in &mut batches {
        let p = &batch.params;
        if p.sun_dir == light.sun_dir
            && p.sun_color == light.sun_color
            && p.ambient == light.ambient
            && p.fog == light.fog
            && p.fog_range == light.fog_range
        {
            continue;
        }
        let p = &mut batch.params;
        p.sun_dir = light.sun_dir;
        p.sun_color = light.sun_color;
        p.ambient = light.ambient;
        p.fog = light.fog;
        p.fog_range = light.fog_range;
        batch.params_version += 1;
    }
}
