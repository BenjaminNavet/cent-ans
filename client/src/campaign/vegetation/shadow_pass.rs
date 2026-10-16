//! Real cast shadows of the campaign trees: the tree batches (impostors and near models) marked
//! [`VegShadowCaster`] are drawn depth-only into the cascaded shadow maps of the sun
//! (`shadow_cast.wesl`), as Godot's tree MultiMeshes (`cast_shadow` on, up to
//! `generalised_shadow_distance`). The terrain, the towns and the trees themselves sample those
//! maps, so the shadows fall on the relief and on the neighbouring crowns. Replaces the soft
//! ellipses drawn on the ground before (vegvis).
//!
//! Bevy's shadow phase is binned and retained: the batches are added as non-mesh items of the
//! directional light views (`BinnedRenderPhaseType::NonMesh`, the draw function does its own
//! instancing) and removed again each frame before being re-added.

use std::collections::HashMap;

use bevy::{
    ecs::query::QueryItem,
    mesh::{MeshVertexBufferLayoutRef, VertexBufferLayout},
    pbr::{
        LightEntity, MeshPipelineKey, PrepassPipeline, SetPrepassViewBindGroup,
        SetPrepassViewEmptyBindGroup, Shadow, ShadowBatchSetKey, ShadowBinKey,
    },
    prelude::*,
    render::{
        Render, RenderApp, RenderStartup, RenderSystems,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::{RenderMesh, allocator::MeshAllocator},
        render_asset::RenderAssets,
        render_phase::{
            AddRenderCommand, BinnedRenderPhaseType, DrawFunctions, InputUniformIndex,
            SetItemPipeline, ViewBinnedRenderPhases,
        },
        render_resource::*,
        renderer::RenderDevice,
        sync_component::SyncComponent,
        sync_world::MainEntity,
        view::{ExtractedView, RetainedViewEntity},
    },
};

use super::billboard::{DrawBatch, RECORD_BYTES, SetMaterialBindGroup, VegBatch, VegPipeline};

const SHADER_PATH: &str = "embedded://cent_ans_client/campaign/vegetation/shadow_cast.wesl";

/// Marker of the vegetation batches drawn into the sun's shadow maps (tree impostors, models).
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct VegShadowCaster;

impl SyncComponent<RenderApp> for VegShadowCaster {
    type Target = Self;
}

impl ExtractComponent<RenderApp> for VegShadowCaster {
    type QueryData = &'static VegShadowCaster;
    type QueryFilter = ();
    type Out = Self;

    fn extract_component(_item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self> {
        Some(VegShadowCaster)
    }
}

pub struct VegShadowPassPlugin;

impl Plugin for VegShadowPassPlugin {
    fn build(&self, app: &mut App) {
        bevy::asset::embedded_asset!(app, "shadow_cast.wesl");
        app.add_plugins(ExtractComponentPlugin::<VegShadowCaster>::default());
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .add_render_command::<Shadow, DrawVegShadow>()
            .init_resource::<SpecializedMeshPipelines<VegShadowPipeline>>()
            .init_resource::<QueuedShadowCasters>()
            .add_systems(
                RenderStartup,
                init_pipeline
                    .after(super::billboard::init_pipeline)
                    .after(bevy::pbr::init_prepass_pipeline),
            )
            .add_systems(
                Render,
                queue_shadow_batches
                    .in_set(RenderSystems::QueueMeshes)
                    .run_if(resource_exists::<VegShadowPipeline>),
            );
    }
}

#[derive(Resource)]
pub struct VegShadowPipeline {
    shader: Handle<Shader>,
    view_layout: BindGroupLayoutDescriptor,
    empty_layout: BindGroupLayoutDescriptor,
    material_layout: BindGroupLayoutDescriptor,
    depth_clip_control: bool,
}

fn init_pipeline(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    prepass: Option<Res<PrepassPipeline>>,
    veg: Option<Res<VegPipeline>>,
    render_device: Res<RenderDevice>,
) {
    let (Some(prepass), Some(veg)) = (prepass, veg) else {
        warn!("vegetation shadows: prepass or vegetation pipeline missing, trees cast no shadow");
        return;
    };
    commands.insert_resource(VegShadowPipeline {
        shader: asset_server.load(SHADER_PATH),
        view_layout: prepass.view_layout_no_motion_vectors.clone(),
        empty_layout: prepass.empty_layout.clone(),
        material_layout: veg.material_layout.clone(),
        depth_clip_control: render_device
            .features()
            .contains(WgpuFeatures::DEPTH_CLIP_CONTROL),
    });
}

/// Pipeline key: topology bits of the mesh and whether the light view is orthographic
/// (directional: depth unclipped).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct VegShadowKey {
    mesh: MeshPipelineKey,
}

impl SpecializedMeshPipeline for VegShadowPipeline {
    type Key = VegShadowKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mesh_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
        ])?;
        let instances = VertexBufferLayout {
            array_stride: RECORD_BYTES as u64,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 8,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: 16,
                    shader_location: 9,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: 28,
                    shader_location: 10,
                },
            ],
        };
        let ortho = key.mesh.contains(MeshPipelineKey::UNCLIPPED_DEPTH_ORTHO);
        let emulate = ortho && !self.depth_clip_control;
        let mut defs: Vec<bevy::shader::ShaderDefVal> = Vec::new();
        if emulate {
            defs.push("UNCLIPPED_DEPTH_ORTHO_EMULATION".into());
        }
        Ok(RenderPipelineDescriptor {
            label: Some("vegetation_shadow_pipeline".into()),
            layout: vec![
                self.view_layout.clone(),
                self.empty_layout.clone(),
                self.material_layout.clone(),
            ],
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: defs.clone(),
                entry_point: Some("vertex".into()),
                buffers: vec![mesh_layout, instances],
                ..default()
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs: defs,
                entry_point: Some(
                    if emulate {
                        "fragment_unclipped"
                    } else {
                        "fragment"
                    }
                    .into(),
                ),
                targets: vec![],
                ..default()
            }),
            primitive: PrimitiveState {
                topology: key.mesh.primitive_topology(),
                strip_index_format: None,
                cull_mode: None,
                unclipped_depth: ortho && self.depth_clip_control,
                ..default()
            },
            depth_stencil: Some(DepthStencilState {
                format: bevy::core_pipeline::core_3d::CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(CompareFunction::GreaterEqual),
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState::default(),
            ..default()
        })
    }
}

/// Batches added to each shadow view last frame (removed before being re-added).
#[derive(Resource, Default)]
pub struct QueuedShadowCasters {
    by_view: HashMap<RetainedViewEntity, Vec<MainEntity>>,
}

#[allow(clippy::too_many_arguments)]
fn queue_shadow_batches(
    draw_functions: Res<DrawFunctions<Shadow>>,
    pipeline: Res<VegShadowPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<VegShadowPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<RenderMesh>>,
    allocator: Res<MeshAllocator>,
    batches: Query<(Entity, &MainEntity, &VegBatch), With<VegShadowCaster>>,
    mut phases: ResMut<ViewBinnedRenderPhases<Shadow>>,
    lights: Query<(&LightEntity, &ExtractedView)>,
    mut queued: ResMut<QueuedShadowCasters>,
) {
    let draw_function = draw_functions.read().id::<DrawVegShadow>();
    if crate::campaign::perf_switch::off("vegshadow") {
        return;
    }
    let mut seen = Vec::new();
    for (light, view) in &lights {
        if !matches!(light, LightEntity::Directional { .. }) {
            continue;
        }
        let Some(phase) = phases.get_mut(&view.retained_view_entity) else {
            continue;
        };
        seen.push(view.retained_view_entity);
        for main in queued
            .by_view
            .remove(&view.retained_view_entity)
            .unwrap_or_default()
        {
            phase.remove(main);
        }
        let mut added = Vec::new();
        for (entity, main_entity, batch) in &batches {
            if batch.count == 0 {
                continue;
            }
            let mesh_id = batch.mesh_handle.id();
            let (Some(mesh), Some(slabs)) = (meshes.get(mesh_id), allocator.mesh_slabs(&mesh_id))
            else {
                continue;
            };
            let key = VegShadowKey {
                mesh: MeshPipelineKey::from_primitive_topology_and_strip_index(
                    mesh.primitive_topology(),
                    mesh.index_format(),
                ) | MeshPipelineKey::DEPTH_PREPASS
                    | if view.clip_from_view.w_axis.w == 1.0 {
                        MeshPipelineKey::UNCLIPPED_DEPTH_ORTHO
                    } else {
                        MeshPipelineKey::NONE
                    },
            };
            let Ok(pipeline_id) =
                pipelines.specialize(&pipeline_cache, &pipeline, key, &mesh.layout)
            else {
                continue;
            };
            phase.add(
                ShadowBatchSetKey {
                    pipeline: pipeline_id,
                    draw_function,
                    material_bind_group_index: None,
                    slabs,
                },
                ShadowBinKey {
                    asset_id: mesh_id.untyped(),
                },
                (entity, *main_entity),
                InputUniformIndex::default(),
                BinnedRenderPhaseType::NonMesh,
            );
            added.push(*main_entity);
        }
        queued.by_view.insert(view.retained_view_entity, added);
    }
    queued.by_view.retain(|view, _| seen.contains(view));
}

type DrawVegShadow = (
    SetItemPipeline,
    SetPrepassViewBindGroup<0>,
    SetPrepassViewEmptyBindGroup<1>,
    SetMaterialBindGroup<2>,
    DrawBatch,
);

#[cfg(test)]
mod tests {
    #[test]
    fn shader_declares_both_fragment_entry_points() {
        let text = include_str!("shadow_cast.wesl");
        assert!(text.contains("fn fragment("));
        assert!(text.contains("fn fragment_unclipped("));
        assert!(text.contains("@group(0) @binding(1) var<uniform> globals"));
    }
}
