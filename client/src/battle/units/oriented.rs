//! Instanced draws of oriented meshes (arrows in flight or stuck in the ground, smoke puffs):
//! same pipeline as `instancing.rs` but the record carries a full direction instead of a yaw.
//! Record (8 f32): `x y z scale | dx dy dz 0`; the mesh's +Z axis is turned to `d`.

use std::collections::HashSet;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use bevy::{
    core_pipeline::core_3d::{Transparent3d, TransparentSortingInfo3d},
    ecs::{
        query::QueryItem,
        system::{SystemParamItem, lifetimeless::*},
    },
    mesh::{MeshVertexBufferLayoutRef, VertexBufferLayout},
    pbr::{
        MeshPipeline, MeshPipelineKey, MeshPipelineSystems, SetMeshViewBindGroup,
        SetMeshViewBindingArrayBindGroup, ViewKeyCache,
    },
    prelude::*,
    render::{
        Render, RenderApp, RenderStartup, RenderSystems,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::{RenderMesh, RenderMeshBufferInfo, allocator::MeshAllocator},
        render_asset::RenderAssets,
        render_phase::{
            AddRenderCommand, DrawFunctions, PhaseItem, PhaseItemExtraIndex, RenderCommand,
            RenderCommandResult, SetItemPipeline, TrackedRenderPass, ViewSortedRenderPhases,
        },
        render_resource::*,
        renderer::RenderDevice,
        sync_component::SyncComponent,
        sync_world::MainEntity,
        texture::{FallbackImage, GpuImage},
        view::ExtractedView,
    },
};

const SHADER_PATH: &str = "embedded://cent_ans_client/battle/units/oriented.wesl";
/// Instanced draws actually issued since the last reset (read and cleared once per frame).
pub static DRAWN_ORIENTED: AtomicU32 = AtomicU32::new(0);

/// Bytes of one record in the `.f32` files.
const RECORD_BYTES: usize = 32;

use super::instancing::MaterialParams;

/// One instanced draw of meshes laid along +Z and turned to a direction: records of 8 f32,
/// `x y z scale | dx dy dz 0` (position, size, unit direction of the mesh's +Z axis).
#[derive(Component, Clone)]
pub struct OrientedBatch {
    /// Strong handles keep the mesh and texture alive for the whole run.
    pub mesh_handle: Handle<Mesh>,
    pub records: Arc<Vec<u8>>,
    pub count: u32,
    pub texture_handle: Option<Handle<Image>>,
    pub params: MaterialParams,
    /// Bumped whenever `records` changes: the GPU buffer is rebuilt.
    pub version: u64,
}

impl SyncComponent<RenderApp> for OrientedBatch {
    type Target = Self;
}

impl ExtractComponent<RenderApp> for OrientedBatch {
    type QueryData = &'static OrientedBatch;
    type QueryFilter = ();
    type Out = Self;

    fn extract_component(item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self> {
        Some(item.clone())
    }
}

pub struct OrientedPlugin;

impl Plugin for OrientedPlugin {
    fn build(&self, app: &mut App) {
        bevy::asset::embedded_asset!(app, "oriented.wesl");
        app.add_plugins(ExtractComponentPlugin::<OrientedBatch>::default());
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .add_render_command::<Transparent3d, DrawOriented>()
            .init_resource::<SpecializedMeshPipelines<OrientedPipeline>>()
            .add_systems(RenderStartup, init_pipeline.after(MeshPipelineSystems))
            .add_systems(
                Render,
                (
                    queue_batches.in_set(RenderSystems::QueueMeshes),
                    prepare_buffers.in_set(RenderSystems::PrepareResources),
                    prepare_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                ),
            );
    }
}

#[derive(Resource)]
struct OrientedPipeline {
    shader: Handle<Shader>,
    mesh_pipeline: MeshPipeline,
    material_layout: BindGroupLayoutDescriptor,
}

fn init_pipeline(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mesh_pipeline: Res<MeshPipeline>,
) {
    use bevy::render::render_resource::binding_types::{sampler, texture_2d, uniform_buffer_sized};
    let material_layout = BindGroupLayoutDescriptor::new(
        "oriented_material_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                uniform_buffer_sized(false, BufferSize::new(32)),
            ),
        ),
    );
    commands.insert_resource(OrientedPipeline {
        shader: asset_server.load(SHADER_PATH),
        mesh_pipeline: mesh_pipeline.clone(),
        material_layout,
    });
}

impl SpecializedMeshPipeline for OrientedPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;
        descriptor.vertex.shader = self.shader.clone();
        descriptor.vertex.buffers.push(VertexBufferLayout {
            array_stride: RECORD_BYTES as u64,
            step_mode: VertexStepMode::Instance,
            attributes: vec![
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 0,
                    shader_location: 8,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: 16,
                    shader_location: 9,
                },
            ],
        });
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.shader = self.shader.clone();
        }
        // Keep the view groups 0 and 1, drop the per-mesh group, add the material at group 2.
        descriptor.layout.truncate(2);
        descriptor.layout.push(self.material_layout.clone());
        Ok(descriptor)
    }
}

#[allow(clippy::too_many_arguments)]
fn queue_batches(
    draw_functions: Res<DrawFunctions<Transparent3d>>,
    pipeline: Res<OrientedPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<OrientedPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<RenderMesh>>,
    batches: Query<(Entity, &MainEntity, &OrientedBatch)>,
    mut phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    views: Query<&ExtractedView>,
    view_key_cache: Res<ViewKeyCache>,
    mut reported: Local<HashSet<(Entity, &'static str)>>,
) {
    let draw_function = draw_functions.read().id::<DrawOriented>();
    for view in &views {
        let Some(phase) = phases.get_mut(&view.retained_view_entity) else {
            continue;
        };
        let Some(&view_key) = view_key_cache.get(&view.retained_view_entity) else {
            continue;
        };
        for (entity, main_entity, batch) in &batches {
            let Some(mesh) = meshes.get(batch.mesh_handle.id()) else {
                if reported.insert((entity, "mesh not prepared")) {
                    warn!("instanced batch {entity:?}: render mesh not ready yet");
                }
                continue;
            };
            let key = view_key
                | MeshPipelineKey::from_primitive_topology_and_strip_index(
                    mesh.primitive_topology(),
                    mesh.index_format(),
                );
            let pipeline_id = match pipelines.specialize(
                &pipeline_cache,
                &pipeline,
                key,
                &mesh.layout,
            ) {
                Ok(id) => id,
                Err(error) => {
                    if reported.insert((entity, "specialize failed")) {
                        error!(
                            "instanced batch {entity:?}: pipeline specialization failed: {error}"
                        );
                    }
                    continue;
                }
            };
            phase.add_retained(Transparent3d {
                sorting_info: TransparentSortingInfo3d::Sorted {
                    mesh_center: Vec3::ZERO,
                    depth_bias: 0.0,
                },
                entity: (entity, *main_entity),
                pipeline: pipeline_id,
                draw_function,
                distance: 0.0,
                batch_range: 0..1,
                extra_index: PhaseItemExtraIndex::None,
                indexed: true,
            });
        }
    }
}

#[derive(Component)]
struct InstanceBuffer {
    buffer: Buffer,
    count: u32,
    version: u64,
}

#[derive(Component)]
struct MaterialBindGroup(BindGroup);

fn prepare_buffers(
    mut commands: Commands,
    batches: Query<(Entity, &OrientedBatch, Option<&InstanceBuffer>)>,
    render_device: Res<RenderDevice>,
) {
    for (entity, batch, existing) in &batches {
        if existing.is_some_and(|buffer| buffer.version == batch.version) {
            continue;
        }
        if batch.count == 0 {
            commands.entity(entity).remove::<InstanceBuffer>();
            continue;
        }
        let bytes = &batch.records[..batch.count as usize * RECORD_BYTES];
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("oriented records"),
            contents: bytes,
            usage: BufferUsages::VERTEX,
        });
        commands.entity(entity).insert(InstanceBuffer {
            buffer,
            count: batch.count,
            version: batch.version,
        });
    }
}

fn prepare_bind_groups(
    mut commands: Commands,
    batches: Query<(Entity, &OrientedBatch), Without<MaterialBindGroup>>,
    images: Res<RenderAssets<GpuImage>>,
    fallback: Res<FallbackImage>,
    pipeline: Res<OrientedPipeline>,
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
) {
    for (entity, batch) in &batches {
        let gpu_image = match &batch.texture_handle {
            Some(id) => {
                let Some(image) = images.get(id.id()) else {
                    continue; // texture not uploaded yet
                };
                image
            }
            None => &fallback.d2,
        };
        let uniform = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("oriented material params"),
            contents: &batch.params.to_bytes(),
            usage: BufferUsages::UNIFORM,
        });
        let group = render_device.create_bind_group(
            "oriented_material_bind_group",
            &pipeline_cache.get_bind_group_layout(&pipeline.material_layout),
            &BindGroupEntries::sequential((
                &gpu_image.texture_view,
                &gpu_image.sampler,
                uniform.as_entire_binding(),
            )),
        );
        commands.entity(entity).insert(MaterialBindGroup(group));
    }
}

type DrawOriented = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshViewBindingArrayBindGroup<1>,
    SetMaterialBindGroup<2>,
    DrawBatch,
);

struct SetMaterialBindGroup<const I: usize>;

impl<P: PhaseItem, const I: usize> RenderCommand<P> for SetMaterialBindGroup<I> {
    type Param = ();
    type ViewQuery = ();
    type ItemQuery = Read<MaterialBindGroup>;

    fn render<'w>(
        _item: &P,
        _view: (),
        group: Option<&'w MaterialBindGroup>,
        _param: SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let Some(group) = group else {
            return RenderCommandResult::Skip;
        };
        pass.set_bind_group(I, &group.0, &[]);
        RenderCommandResult::Success
    }
}

struct DrawBatch;

impl<P: PhaseItem> RenderCommand<P> for DrawBatch {
    type Param = (SRes<RenderAssets<RenderMesh>>, SRes<MeshAllocator>);
    type ViewQuery = ();
    type ItemQuery = (Read<InstanceBuffer>, Read<OrientedBatch>);

    fn render<'w>(
        _item: &P,
        _view: (),
        item_query: Option<(&'w InstanceBuffer, &'w OrientedBatch)>,
        (meshes, allocator): SystemParamItem<'w, '_, Self::Param>,
        pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        let allocator = allocator.into_inner();
        let Some((instances, batch)) = item_query else {
            return RenderCommandResult::Skip;
        };
        let mesh_id = batch.mesh_handle.id();
        let Some(gpu_mesh) = meshes.into_inner().get(mesh_id) else {
            return RenderCommandResult::Skip;
        };
        let Some(vertex_slice) = allocator.mesh_vertex_slice(&mesh_id) else {
            return RenderCommandResult::Skip;
        };
        pass.set_vertex_buffer(0, vertex_slice.buffer.slice(..));
        pass.set_vertex_buffer(1, instances.buffer.slice(..));
        match &gpu_mesh.buffer_info {
            RenderMeshBufferInfo::Indexed {
                index_format,
                count,
            } => {
                let Some(index_slice) = allocator.mesh_index_slice(&mesh_id) else {
                    return RenderCommandResult::Skip;
                };
                pass.set_index_buffer(index_slice.buffer.slice(..), *index_format);
                pass.draw_indexed(
                    index_slice.range.start..(index_slice.range.start + count),
                    vertex_slice.range.start as i32,
                    0..instances.count,
                );
            }
            RenderMeshBufferInfo::NonIndexed => {
                pass.draw(vertex_slice.range, 0..instances.count);
            }
        }
        DRAWN_ORIENTED.fetch_add(1, Ordering::Relaxed);
        RenderCommandResult::Success
    }
}
