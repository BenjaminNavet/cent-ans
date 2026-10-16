//! Instanced draws of the campaign vegetation with their own shader (`billboard.wesl`): tree
//! impostors (camera-facing card, azimuth view, seasonal tint, wind), reed clumps and ponds.
//! Pattern of `battle::units::instancing` (one `Transparent3d` draw per batch, lit by Bevy's PBR
//! code) with a 40-byte record, two textures (albedo, normal atlas) and a parameter block the
//! main world can refresh (season weights) without rebuilding the records.
//!
//! The pipeline of the battle lot cannot draw billboards (it only rotates about Y), hence this
//! copy. Like Godot's MultiMesh the whole batch is drawn (no per-instance culling).

use std::collections::HashSet;
use std::sync::Arc;

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

const SHADER_PATH: &str = "embedded://cent_ans_client/campaign/vegetation/billboard.wesl";

/// Bytes of one record: x, y, z, yaw (deg), scale, 0, seed, r, g, b.
pub const RECORD_BYTES: usize = 40;
/// Bytes of the parameter block (`Params` of `billboard.wesl`: twelve `vec4`).
const PARAM_BYTES: u64 = 240;

/// What a batch draws (`params.frame.w`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Impostor = 0,
    Reeds = 1,
    Pond = 2,
    /// Near tree: the normalised glb mesh of the species (`foliage_model.gdshader`).
    Model = 3,
    /// Soft shadow of a tree on the ground (alpha-blended ellipse cast along the sun), in place
    /// of the shadow maps Godot gives its tree MultiMeshes (`generalised_shadow_distance`).
    Shadow = 4,
}

/// Pipeline key: the mesh key plus the blended variant (ground shadows).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct VegKey {
    mesh: MeshPipelineKey,
    blended: bool,
}

/// Parameter block mirrored by `Params` in `billboard.wesl`.
#[derive(Clone, Copy, Debug)]
pub struct Params {
    /// Season weights (spring, summer, autumn, winter).
    pub season: Vec4,
    /// rgb multiplier, season class of the species (0 oak, 1 beech, 2 conifer, 3 hedge, 4 gold).
    pub tint: Vec4,
    /// Atlas views, rows, row of the species, width / height ratio of the crown.
    pub atlas: Vec4,
    /// Baking frame ortho, foot, alpha cutoff, mode.
    pub frame: Vec4,
    /// Wind bend, calm, light, albedo gain.
    pub wind: Vec4,
    /// Reeds: clump width and height (world units), sway, far scale.
    pub extra: Vec4,
    /// Reeds, ponds: scale near and far distances, pond far scale.
    pub range: Vec4,
    /// Ground lighting copied from the terrain material (`TerrainParams`), so the trees are lit
    /// and fogged like the ground: direction to the sun (w = 1 when set), sun colour, ambient
    /// colour, distance fog colour and strength, fog begin and end.
    pub sun_dir: Vec4,
    pub sun_color: Vec4,
    pub ambient: Vec4,
    pub fog: Vec4,
    pub fog_range: Vec4,
    /// Colour grade of the ground (saturation, contrast, shadow saturation, strength), shadow lift
    /// and highlight tint: Godot graded the whole frame, trees included.
    pub grade: Vec4,
    pub lift: Vec4,
    pub highlights: Vec4,
}

impl Params {
    pub fn new(mode: Mode) -> Params {
        Params {
            season: Vec4::new(1.0, 0.0, 0.0, 0.0),
            tint: Vec4::new(1.0, 1.0, 1.0, 0.0),
            atlas: Vec4::new(8.0, 1.0, 0.0, 1.0),
            frame: Vec4::new(1.7, 0.2588, 0.5, mode as u32 as f32),
            wind: Vec4::new(0.07, 0.018, 0.1, 1.6),
            extra: Vec4::new(1.0, 1.0, 0.25, 1.0),
            range: Vec4::new(3.0, 60.0, 0.0, 1.0),
            sun_dir: Vec4::ZERO,
            sun_color: Vec4::ZERO,
            ambient: Vec4::ZERO,
            fog: Vec4::ZERO,
            fog_range: Vec4::ZERO,
            grade: Vec4::ZERO,
            lift: Vec4::ZERO,
            highlights: Vec4::ONE,
        }
    }

    fn to_bytes(self) -> Vec<u8> {
        [
            self.season,
            self.tint,
            self.atlas,
            self.frame,
            self.wind,
            self.extra,
            self.range,
            self.sun_dir,
            self.sun_color,
            self.ambient,
            self.fog,
            self.fog_range,
            self.grade,
            self.lift,
            self.highlights,
        ]
        .iter()
        .flat_map(|v| v.to_array())
        .flat_map(|x| x.to_le_bytes())
        .collect()
    }
}

/// One instanced draw: a mesh, its textures, its parameters and the first `count` records.
#[derive(Component, Clone)]
pub struct VegBatch {
    pub mesh_handle: Handle<Mesh>,
    pub records: Arc<Vec<u8>>,
    pub count: u32,
    pub albedo: Handle<Image>,
    /// Normal atlas (impostors); the albedo stands in when `None`.
    pub normal: Option<Handle<Image>>,
    pub params: Params,
    /// Bumped whenever `records` changes: the GPU buffer is rebuilt.
    pub version: u64,
    /// Bumped whenever `params` changes: the uniform is rebuilt.
    pub params_version: u64,
}

/// Appends one record to `out`.
pub fn push_record(
    out: &mut Vec<u8>,
    position: Vec3,
    yaw_deg: f32,
    scale: f32,
    seed: f32,
    tint: [f32; 3],
) {
    push_weighted(out, position, yaw_deg, scale, 0.0, seed, tint);
}

/// Appends one record with a model-zone weight (trees: share of the tree taken by its model in
/// the dithered model / impostor band, `model_zone_weight` of `foliage_common.gdshaderinc`).
pub fn push_weighted(
    out: &mut Vec<u8>,
    position: Vec3,
    yaw_deg: f32,
    scale: f32,
    weight: f32,
    seed: f32,
    tint: [f32; 3],
) {
    for value in [
        position.x, position.y, position.z, yaw_deg, scale, weight, seed, tint[0], tint[1], tint[2],
    ] {
        out.extend_from_slice(&value.to_le_bytes());
    }
}

impl SyncComponent<RenderApp> for VegBatch {
    type Target = Self;
}

impl ExtractComponent<RenderApp> for VegBatch {
    type QueryData = &'static VegBatch;
    type QueryFilter = ();
    type Out = Self;

    fn extract_component(item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self> {
        Some(item.clone())
    }
}

pub struct BillboardPlugin;

impl Plugin for BillboardPlugin {
    fn build(&self, app: &mut App) {
        bevy::asset::embedded_asset!(app, "billboard.wesl");
        app.add_plugins(ExtractComponentPlugin::<VegBatch>::default());
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .add_render_command::<Transparent3d, DrawVeg>()
            .init_resource::<SpecializedMeshPipelines<VegPipeline>>()
            .add_systems(RenderStartup, init_pipeline.after(MeshPipelineSystems))
            .add_systems(
                Render,
                (
                    queue_batches.in_set(RenderSystems::QueueMeshes),
                    prepare_buffers
                        .in_set(RenderSystems::PrepareResources)
                        .run_if(|batches: Query<(), With<VegBatch>>| !batches.is_empty()),
                    prepare_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                ),
            );
    }
}

#[derive(Resource)]
struct VegPipeline {
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
        "veg_material_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::VERTEX_FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_2d(TextureSampleType::Float { filterable: true }),
                uniform_buffer_sized(false, BufferSize::new(PARAM_BYTES)),
            ),
        ),
    );
    commands.insert_resource(VegPipeline {
        shader: asset_server.load(SHADER_PATH),
        mesh_pipeline: mesh_pipeline.clone(),
        material_layout,
    });
}

impl SpecializedMeshPipeline for VegPipeline {
    type Key = VegKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key.mesh, layout)?;
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
        });
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.shader = self.shader.clone();
            if key.blended {
                for target in fragment.targets.iter_mut().flatten() {
                    target.blend = Some(BlendState::ALPHA_BLENDING);
                }
            }
        }
        if key.blended
            && let Some(depth) = descriptor.depth_stencil.as_mut()
        {
            depth.depth_write_enabled = Some(false);
        }
        // Cards are seen from both sides.
        descriptor.primitive.cull_mode = None;
        // Keep the view groups 0 and 1, drop the per-mesh group, add the material at group 2.
        descriptor.layout.truncate(2);
        descriptor.layout.push(self.material_layout.clone());
        Ok(descriptor)
    }
}

#[allow(clippy::too_many_arguments)]
fn queue_batches(
    draw_functions: Res<DrawFunctions<Transparent3d>>,
    pipeline: Res<VegPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<VegPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<RenderMesh>>,
    batches: Query<(Entity, &MainEntity, &VegBatch)>,
    mut phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    views: Query<&ExtractedView>,
    view_key_cache: Res<ViewKeyCache>,
    mut reported: Local<HashSet<(Entity, &'static str)>>,
) {
    let draw_function = draw_functions.read().id::<DrawVeg>();
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
                    warn!("vegetation batch {entity:?}: render mesh not ready yet");
                }
                continue;
            };
            let key = VegKey {
                mesh: view_key
                    | MeshPipelineKey::from_primitive_topology_and_strip_index(
                        mesh.primitive_topology(),
                        mesh.index_format(),
                    ),
                blended: batch.params.frame.w > Mode::Shadow as u32 as f32 - 0.5,
            };
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
                            "vegetation batch {entity:?}: pipeline specialization failed: {error}"
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
struct MaterialBindGroup {
    group: BindGroup,
    params_version: u64,
}

fn prepare_buffers(
    mut commands: Commands,
    batches: Query<(Entity, &VegBatch, Option<&InstanceBuffer>)>,
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
            label: Some("vegetation records"),
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
    batches: Query<(Entity, &VegBatch, Option<&MaterialBindGroup>)>,
    images: Res<RenderAssets<GpuImage>>,
    fallback: Res<FallbackImage>,
    pipeline: Res<VegPipeline>,
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
) {
    for (entity, batch, existing) in &batches {
        if existing.is_some_and(|g| g.params_version == batch.params_version) {
            continue;
        }
        let Some(albedo) = images.get(batch.albedo.id()) else {
            continue; // texture not uploaded yet
        };
        let normal = match &batch.normal {
            Some(handle) => match images.get(handle.id()) {
                Some(image) => &image.texture_view,
                None => continue,
            },
            None => &albedo.texture_view,
        };
        let _ = &fallback;
        let uniform = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("vegetation params"),
            contents: &batch.params.to_bytes(),
            usage: BufferUsages::UNIFORM,
        });
        let group = render_device.create_bind_group(
            "veg_material_bind_group",
            &pipeline_cache.get_bind_group_layout(&pipeline.material_layout),
            &BindGroupEntries::sequential((
                &albedo.texture_view,
                &albedo.sampler,
                normal,
                uniform.as_entire_binding(),
            )),
        );
        commands.entity(entity).insert(MaterialBindGroup {
            group,
            params_version: batch.params_version,
        });
    }
}

type DrawVeg = (
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
        pass.set_bind_group(I, &group.group, &[]);
        RenderCommandResult::Success
    }
}

struct DrawBatch;

impl<P: PhaseItem> RenderCommand<P> for DrawBatch {
    type Param = (SRes<RenderAssets<RenderMesh>>, SRes<MeshAllocator>);
    type ViewQuery = ();
    type ItemQuery = (Read<InstanceBuffer>, Read<VegBatch>);

    fn render<'w>(
        _item: &P,
        _view: (),
        item_query: Option<(&'w InstanceBuffer, &'w VegBatch)>,
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
        RenderCommandResult::Success
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_params_have_the_declared_sizes() {
        let mut out = Vec::new();
        push_record(&mut out, Vec3::new(1.0, 2.0, 3.0), 45.0, 1.5, 0.3, [1.0; 3]);
        assert_eq!(out.len(), RECORD_BYTES);
        assert_eq!(
            Params::new(Mode::Reeds).to_bytes().len() as u64,
            PARAM_BYTES
        );
        assert_eq!(Params::new(Mode::Pond).frame.w, 2.0);
    }

    /// `billboard.wesl` compiles (WESL -> WGSL) against the PBR imports it uses.
    #[test]
    fn shader_text_declares_the_record_layout() {
        let text = include_str!("billboard.wesl");
        for needle in [
            "@location(8)",
            "@location(9)",
            "@location(10)",
            "struct Params",
        ] {
            assert!(text.contains(needle), "{needle}");
        }
    }
}
