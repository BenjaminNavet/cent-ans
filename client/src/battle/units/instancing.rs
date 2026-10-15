//! Custom GPU instancing for vegetation: one entity per (mesh, material) carrying the raw record
//! buffer, drawn with a single instanced draw in the `Transparent3d` phase (sorted last, depth
//! tested). Pattern of Bevy's `custom_shader_instancing` example, lit by Bevy's PBR shader code.
//!
//! Lost compared with `StandardMaterial` entities: depth/normal prepass (so no SSAO on these
//! meshes), normal maps, per-instance frustum culling (like Godot's MultiMesh the whole batch is
//! drawn), motion vectors. Shadow casting is opt-in per batch (`cast_shadows`, opaque meshes:
//! depth only, as in `skinned.rs`).

use std::collections::{HashMap, HashSet};
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
        LightEntity, MeshPipeline, MeshPipelineKey, MeshPipelineSystems, PrepassPipeline,
        SetMeshViewBindGroup, SetMeshViewBindingArrayBindGroup, SetPrepassViewBindGroup,
        SetPrepassViewEmptyBindGroup, Shadow, ShadowBatchSetKey, ShadowBinKey, ViewKeyCache,
    },
    prelude::*,
    render::{
        Render, RenderApp, RenderStartup, RenderSystems,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        mesh::{RenderMesh, RenderMeshBufferInfo, allocator::MeshAllocator},
        render_asset::RenderAssets,
        render_phase::{
            AddRenderCommand, BinnedRenderPhaseType, DrawFunctions, InputUniformIndex, PhaseItem,
            PhaseItemExtraIndex, RenderCommand, RenderCommandResult, SetItemPipeline,
            TrackedRenderPass, ViewBinnedRenderPhases, ViewSortedRenderPhases,
        },
        render_resource::*,
        renderer::RenderDevice,
        sync_component::SyncComponent,
        sync_world::MainEntity,
        texture::{FallbackImage, GpuImage},
        view::{ExtractedView, RetainedViewEntity},
    },
};

const SHADER_PATH: &str = "embedded://cent_ans_client/battle/units/instanced.wesl";
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
    pub(super) fn to_bytes(self) -> Vec<u8> {
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
    /// Bumped whenever `records` changes: the GPU buffer is rebuilt.
    pub version: u64,
    /// Drawn into the directional shadow cascades too (opaque meshes only).
    pub cast_shadows: bool,
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
        bevy::asset::embedded_asset!(app, "instanced.wesl");
        app.add_plugins(ExtractComponentPlugin::<InstancedBatch>::default());
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .add_render_command::<Transparent3d, DrawInstanced>()
            .add_render_command::<Shadow, DrawInstancedShadow>()
            .init_resource::<SpecializedMeshPipelines<InstancedPipeline>>()
            .init_resource::<SpecializedMeshPipelines<InstancedShadowPipeline>>()
            .add_systems(
                RenderStartup,
                (
                    init_pipeline.after(MeshPipelineSystems),
                    init_shadow_pipeline
                        .after(init_pipeline)
                        .after(bevy::pbr::init_prepass_pipeline),
                ),
            )
            .add_systems(
                Render,
                (
                    queue_batches.in_set(RenderSystems::QueueMeshes),
                    queue_shadow_casters
                        .in_set(RenderSystems::QueueMeshes)
                        .after(bevy::pbr::queue_shadows),
                    prepare_buffers.in_set(RenderSystems::PrepareResources),
                    prepare_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                ),
            );
    }
}

#[derive(Resource, Clone)]
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
    batches: Query<(Entity, &InstancedBatch, Option<&InstanceBuffer>)>,
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
            label: Some("instanced records"),
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
        commands.entity(entity).insert(MaterialBindGroup(group));
    }
}

// --- Shadow casting (opt-in per batch) --------------------------------------------------------
//
// Same layout as the skinned figures' shadow pass: group 0 = prepass view (binding 0 is the light
// view's `View`, all the vertex stage reads), group 1 = empty, group 2 = our material (unused by
// the vertex stage). Depth only, no fragment stage: alpha-cut batches must not opt in.

#[derive(Resource)]
struct InstancedShadowPipeline {
    base: InstancedPipeline,
    view_layout: BindGroupLayoutDescriptor,
    empty_layout: BindGroupLayoutDescriptor,
    unclipped_depth: bool,
}

fn init_shadow_pipeline(
    mut commands: Commands,
    base: Res<InstancedPipeline>,
    prepass: Res<PrepassPipeline>,
) {
    commands.insert_resource(InstancedShadowPipeline {
        base: base.clone(),
        view_layout: prepass.view_layout_no_motion_vectors.clone(),
        empty_layout: prepass.empty_layout.clone(),
        unclipped_depth: prepass.depth_clip_control_supported,
    });
}

impl SpecializedMeshPipeline for InstancedShadowPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.base.specialize(key, layout)?;
        descriptor.label = Some("instanced_shadow_pipeline".into());
        descriptor.layout = vec![
            self.view_layout.clone(),
            self.empty_layout.clone(),
            self.base.material_layout.clone(),
        ];
        descriptor.fragment = None;
        descriptor.primitive.unclipped_depth = self.unclipped_depth;
        descriptor.primitive.cull_mode = None;
        descriptor.depth_stencil = Some(DepthStencilState {
            format: bevy::core_pipeline::core_3d::CORE_3D_DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(CompareFunction::GreaterEqual),
            stencil: StencilState::default(),
            bias: DepthBiasState::default(),
        });
        descriptor.multisample = MultisampleState::default();
        Ok(descriptor)
    }
}

/// Instanced batches put in the cascades' phases this frame (batches x cascades).
pub static INSTANCED_SHADOW_CASTERS: AtomicU32 = AtomicU32::new(0);

#[allow(clippy::too_many_arguments)]
fn queue_shadow_casters(
    draw_functions: Res<DrawFunctions<Shadow>>,
    pipeline: Res<InstancedShadowPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<InstancedShadowPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<RenderMesh>>,
    allocator: Res<MeshAllocator>,
    batches: Query<(Entity, &MainEntity, &InstancedBatch)>,
    mut phases: ResMut<ViewBinnedRenderPhases<Shadow>>,
    lights: Query<(&LightEntity, &ExtractedView)>,
    mut queued: Local<HashMap<RetainedViewEntity, Vec<MainEntity>>>,
    mut reported: Local<bool>,
) {
    let draw_function = draw_functions.read().id::<DrawInstancedShadow>();
    let mut casters = 0;
    for (light, view) in &lights {
        if !matches!(light, LightEntity::Directional { .. }) {
            continue;
        }
        let Some(phase) = phases.get_mut(&view.retained_view_entity) else {
            continue;
        };
        let previous = queued.entry(view.retained_view_entity).or_default();
        for main_entity in previous.drain(..) {
            phase.remove(main_entity);
        }
        for (entity, main_entity, batch) in &batches {
            if !batch.cast_shadows || batch.count == 0 || batch.params.alpha_cutoff > 0.0 {
                continue;
            }
            let mesh_id = batch.mesh_handle.id();
            let (Some(mesh), Some(slabs)) = (meshes.get(mesh_id), allocator.mesh_slabs(&mesh_id))
            else {
                continue;
            };
            let key = MeshPipelineKey::from_primitive_topology_and_strip_index(
                mesh.primitive_topology(),
                mesh.index_format(),
            ) | MeshPipelineKey::DEPTH_PREPASS;
            let pipeline_id =
                match pipelines.specialize(&pipeline_cache, &pipeline, key, &mesh.layout) {
                    Ok(id) => id,
                    Err(error) => {
                        if !*reported {
                            *reported = true;
                            error!("instanced shadow: specialization failed: {error}");
                        }
                        continue;
                    }
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
            previous.push(*main_entity);
            casters += 1;
        }
    }
    INSTANCED_SHADOW_CASTERS.store(casters, Ordering::Relaxed);
}

type DrawInstancedShadow = (
    SetItemPipeline,
    SetPrepassViewBindGroup<0>,
    SetPrepassViewEmptyBindGroup<1>,
    SetMaterialBindGroup<2>,
    DrawBatch<false>,
);

type DrawInstanced = (
    SetItemPipeline,
    SetMeshViewBindGroup<0>,
    SetMeshViewBindingArrayBindGroup<1>,
    SetMaterialBindGroup<2>,
    DrawBatch<true>,
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

/// `MAIN`: a main-view draw (counted in `DRAWN_BATCHES`), else a shadow-cascade draw.
struct DrawBatch<const MAIN: bool>;

impl<P: PhaseItem, const MAIN: bool> RenderCommand<P> for DrawBatch<MAIN> {
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
        if MAIN {
            DRAWN_BATCHES.fetch_add(1, Ordering::Relaxed);
        }
        RenderCommandResult::Success
    }
}
