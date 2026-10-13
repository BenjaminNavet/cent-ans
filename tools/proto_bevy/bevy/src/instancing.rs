//! Custom GPU instancing for vegetation: one entity per (mesh, material) carrying the raw record
//! buffer, drawn with a single instanced draw in the `Transparent3d` phase (sorted last, depth
//! tested). Pattern of Bevy's `custom_shader_instancing` example, lit by Bevy's PBR shader code.
//!
//! Lost compared with `StandardMaterial` entities: shadow casting, depth/normal prepass (so no
//! SSAO on these meshes), normal maps, per-instance frustum culling (like Godot's MultiMesh the
//! whole batch is drawn), motion vectors.

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

const SHADER_PATH: &str = "bevy/shaders/instanced.wesl";
/// Instanced draws actually issued since the last reset (read and cleared once per frame).
pub static DRAWN_BATCHES: AtomicU32 = AtomicU32::new(0);

/// Bytes of one record in the `.f32` files.
const RECORD_BYTES: usize = 24;

/// Material parameters mirrored by `Params` in `instanced.wesl` (32 bytes).
#[derive(Clone, Copy, Debug)]
pub struct MaterialParams {
    pub base_color: [f32; 4],
    /// `<= 0` means opaque.
    pub alpha_cutoff: f32,
    pub roughness: f32,
    pub metallic: f32,
    pub double_sided: bool,
}

impl MaterialParams {
    fn to_bytes(self) -> Vec<u8> {
        let floats = [
            self.base_color[0],
            self.base_color[1],
            self.base_color[2],
            self.base_color[3],
            self.alpha_cutoff,
            self.roughness,
            self.metallic,
            if self.double_sided { 1.0 } else { 0.0 },
        ];
        floats
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect()
    }
}

/// One instanced draw: a mesh, its material and the first `count` records.
#[derive(Component, Clone)]
pub struct InstancedBatch {
    /// Strong handles keep the mesh and texture alive for the whole run.
    pub mesh_handle: Handle<Mesh>,
    pub records: Arc<Vec<u8>>,
    pub count: u32,
    pub texture_handle: Option<Handle<Image>>,
    pub params: MaterialParams,
}

impl SyncComponent<RenderApp> for InstancedBatch {
    type Target = Self;
}

impl ExtractComponent<RenderApp> for InstancedBatch {
    type QueryData = &'static InstancedBatch;
    type QueryFilter = ();
    type Out = Self;

    fn extract_component(item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self> {
        Some(item.clone())
    }
}

pub struct InstancingPlugin;

impl Plugin for InstancingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(ExtractComponentPlugin::<InstancedBatch>::default());
        app.sub_app_mut(RenderApp)
            .add_render_command::<Transparent3d, DrawInstanced>()
            .init_resource::<SpecializedMeshPipelines<InstancedPipeline>>()
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
struct InstancedPipeline {
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
        "instanced_material_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                uniform_buffer_sized(false, BufferSize::new(32)),
            ),
        ),
    );
    commands.insert_resource(InstancedPipeline {
        shader: asset_server.load(SHADER_PATH),
        mesh_pipeline: mesh_pipeline.clone(),
        material_layout,
    });
}

impl SpecializedMeshPipeline for InstancedPipeline {
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
                    format: VertexFormat::Float32x2,
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
    pipeline: Res<InstancedPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<InstancedPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<RenderMesh>>,
    batches: Query<(Entity, &MainEntity, &InstancedBatch)>,
    mut phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    views: Query<&ExtractedView>,
    view_key_cache: Res<ViewKeyCache>,
    mut reported: Local<HashSet<(Entity, &'static str)>>,
) {
    let draw_function = draw_functions.read().id::<DrawInstanced>();
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
            if reported.insert((entity, "queued")) {
                info!("instanced batch {entity:?}: queued with pipeline {pipeline_id:?}");
            }
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
}

#[derive(Component)]
struct MaterialBindGroup(BindGroup);

fn prepare_buffers(
    mut commands: Commands,
    batches: Query<(Entity, &InstancedBatch), Without<InstanceBuffer>>,
    render_device: Res<RenderDevice>,
) {
    for (entity, batch) in &batches {
        let bytes = &batch.records[..batch.count as usize * RECORD_BYTES];
        let buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("instanced records"),
            contents: bytes,
            usage: BufferUsages::VERTEX,
        });
        info!(
            "instanced batch {entity:?}: instance buffer ready ({} instances)",
            batch.count
        );
        commands.entity(entity).insert(InstanceBuffer {
            buffer,
            count: batch.count,
        });
    }
}

fn prepare_bind_groups(
    mut commands: Commands,
    batches: Query<(Entity, &InstancedBatch), Without<MaterialBindGroup>>,
    images: Res<RenderAssets<GpuImage>>,
    fallback: Res<FallbackImage>,
    pipeline: Res<InstancedPipeline>,
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
            label: Some("instanced material params"),
            contents: &batch.params.to_bytes(),
            usage: BufferUsages::UNIFORM,
        });
        let group = render_device.create_bind_group(
            "instanced_material_bind_group",
            &pipeline_cache.get_bind_group_layout(&pipeline.material_layout),
            &BindGroupEntries::sequential((
                &gpu_image.texture_view,
                &gpu_image.sampler,
                uniform.as_entire_binding(),
            )),
        );
        info!("instanced batch {entity:?}: material bind group ready");
        commands.entity(entity).insert(MaterialBindGroup(group));
    }
}

type DrawInstanced = (
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
    type ItemQuery = (Read<InstanceBuffer>, Read<InstancedBatch>);

    fn render<'w>(
        _item: &P,
        _view: (),
        item_query: Option<(&'w InstanceBuffer, &'w InstancedBatch)>,
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
        DRAWN_BATCHES.fetch_add(1, Ordering::Relaxed);
        RenderCommandResult::Success
    }
}
