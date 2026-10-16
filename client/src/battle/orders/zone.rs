//! Deployment zone of the player drawn on the ground (port of
//! game/scripts/battle/deployment_zone.gd and game/shaders/deployment_zone.gdshader, A1-02).
//!
//! A grid mesh draped on the relief (10 m cells, 0.6 m lift) with a shader (`zone.wesl`): bright
//! gold edge line, inner halo, hatching in a border band and a nearly invisible veil, which fade
//! with the camera distance. Replaces the plain outline of the first deployment port. The rectangle
//! comes from the core (`deployment_zones`); no rule here.

use bevy::asset::{RenderAssetUsages, embedded_asset};
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;
use sim_battle::Battlefield;

use super::deploy::Deployment;
use crate::battle::scene::BattleSession;

/// Edge colour of the zone (pale gold).
pub const EDGE: Color = Color::srgb(1.0, 0.86, 0.42);
const STEP: f32 = 10.0;
const LIFT: f32 = 0.6;

#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub struct ZoneParams {
    pub rect: Vec4,
    pub edge_color: Vec4,
    pub shape: Vec4,
    pub fade: Vec4,
}

impl ZoneParams {
    pub fn new(zone: [f32; 4]) -> Self {
        let edge = LinearRgba::from(EDGE);
        Self {
            rect: Vec4::from(zone),
            edge_color: Vec4::new(edge.red, edge.green, edge.blue, 1.0),
            // line 0.9 m, glow 7 m, hatch band 22 m, hatch period 5 m.
            shape: Vec4::new(0.9, 7.0, 22.0, 5.0),
            // interior alpha, fade near / far (m).
            fade: Vec4::new(0.03, 140.0, 700.0, 0.0),
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct ZoneMaterial {
    #[uniform(0)]
    pub params: ZoneParams,
}

impl Material for ZoneMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://cent_ans_client/battle/orders/zone.wesl".into()
    }
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = None;
        if let Some(depth) = descriptor.depth_stencil.as_mut() {
            depth.depth_write_enabled = Some(false);
        }
        Ok(())
    }
}

pub struct ZonePlugin;

impl Plugin for ZonePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "zone.wesl");
        app.add_plugins(MaterialPlugin::<ZoneMaterial>::default());
    }
}

/// A zone mesh on the field.
#[derive(Component)]
pub struct ZoneMesh;

/// Grid mesh of `zone` `[x0, z0, x1, z1]` draped on the walkable height.
pub fn zone_mesh(field: &Battlefield, zone: [f32; 4]) -> Option<Mesh> {
    let (width, depth) = (zone[2] - zone[0], zone[3] - zone[1]);
    if width <= 0.0 || depth <= 0.0 {
        return None;
    }
    let nx = ((width / STEP).ceil() as usize).max(1);
    let nz = ((depth / STEP).ceil() as usize).max(1);
    let mut positions = Vec::with_capacity((nx + 1) * (nz + 1));
    for j in 0..=nz {
        for i in 0..=nx {
            let x = zone[0] + width * i as f32 / nx as f32;
            let z = zone[1] + depth * j as f32 / nz as f32;
            let y = field.walk_height(f64::from(x), f64::from(z)) as f32 + LIFT;
            positions.push([x, y, z]);
        }
    }
    let mut indices = Vec::with_capacity(nx * nz * 6);
    let stride = (nx + 1) as u32;
    for j in 0..nz as u32 {
        for i in 0..nx as u32 {
            let a = j * stride + i;
            indices.extend([a, a + stride, a + 1, a + 1, a + stride, a + stride + 1]);
        }
    }
    let normals = vec![[0.0, 1.0, 0.0]; positions.len()];
    let uvs = vec![[0.0, 0.0]; positions.len()];
    Some(
        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(Indices::U32(indices)),
    )
}

/// Spawns the zone meshes while the deployment phase is active and removes them afterwards.
pub fn sync_zone(
    mut commands: Commands,
    mut deployment: ResMut<Deployment>,
    mut session: ResMut<BattleSession>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ZoneMaterial>>,
    existing: Query<Entity, With<ZoneMesh>>,
) {
    let present = !existing.is_empty();
    if deployment.zone_meshes != existing.iter().count() {
        deployment.zone_meshes = existing.iter().count();
    }
    if deployment.active && !present {
        for zone in &deployment.zones {
            let Some(mesh) = zone_mesh(session.sim_mut().field(), *zone) else {
                continue;
            };
            commands.spawn((
                Name::new("DeploymentZone"),
                ZoneMesh,
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(materials.add(ZoneMaterial {
                    params: ZoneParams::new(*zone),
                })),
                Transform::default(),
                bevy::light::NotShadowCaster,
                bevy::light::NotShadowReceiver,
                DespawnOnExit(crate::app::AppState::Battle),
            ));
        }
    } else if !deployment.active && present {
        for entity in &existing {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn params_carry_the_godot_defaults() {
        let p = ZoneParams::new([10.0, 20.0, 110.0, 70.0]);
        assert_eq!(p.rect, Vec4::new(10.0, 20.0, 110.0, 70.0));
        assert_eq!(p.shape, Vec4::new(0.9, 7.0, 22.0, 5.0));
        assert_eq!(p.fade.x, 0.03);
        assert_eq!((p.fade.y, p.fade.z), (140.0, 700.0));
    }
}
