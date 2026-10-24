//! Ground cache of the campaign map (ADR 0742).
//!
//! The regional ground of `terrain.wesl` (soil layers, painted fields, close detail, forest floor
//! and canopy: `regional_ground`) costs ~20 ms a frame at 1080p when computed per pixel, while it
//! depends on the map point only. Each relief node (`lod.rs`) therefore owns one layer of an atlas
//! (`texture_2d_array`, N x N texels over the node square) where that ground is baked once:
//! albedo + forest share (Rgba8UnormSrgb), normal offset (Rg16Float). The main pass, compiled with
//! `GROUND_CACHE`, reads the layer instead of computing the block.
//!
//! * Main world: [`GroundCache`] hands out the layers, numbers the bakes (tickets) and lists the
//!   jobs; `lod.rs` shows a node only once its first bake is done.
//! * Render world: the bake pass draws the node mesh flat over a scratch target (`bake_vertex` /
//!   `bake_fragment` of `terrain.wesl`, the material bind group in group 3, empty groups 0-2),
//!   then copies it into the node layer, a few nodes a frame.
//! * Invalidation: a signature of what the block reads (season, ground, fields, close detail,
//!   texture handles) bumps the generation, and every node is baked again.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

use bevy::asset::RenderAssetUsages;
use bevy::core_pipeline::Core3dSystems;
use bevy::core_pipeline::schedule::Core3d;
use bevy::pbr::{MATERIAL_BIND_GROUP_INDEX, MeshPipeline, MeshPipelineKey, PreparedMaterial};
use bevy::prelude::*;
use bevy::render::erased_render_asset::ErasedRenderAssets;
use bevy::render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy::render::extract_resource::{ExtractResource, ExtractResourcePlugin};
use bevy::render::material_bind_groups::MaterialBindGroupAllocators;
use bevy::render::mesh::allocator::MeshAllocator;
use bevy::render::mesh::{RenderMesh, RenderMeshBufferInfo};
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{
    BindGroup, BindGroupLayoutDescriptor, CachedRenderPipelineId, ColorTargetState, ColorWrites,
    Extent3d, FragmentState, ImageSubresourceRange, LoadOp, Operations, Origin3d, PipelineCache,
    PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor,
    StoreOp, TexelCopyTextureInfo, Texture, TextureAspect, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension,
    VertexState,
};
use bevy::render::render_resource::{PrimitiveTopology, SpecializedMeshPipeline};
use bevy::render::renderer::{RenderContext, RenderDevice, ViewQuery};
use bevy::render::texture::GpuImage;
use bevy::render::{Render, RenderApp, RenderStartup, RenderSystems};
use bevy::shader::Shader;
use bevy::shader::ShaderDefVal;

use super::material::{SHADER_PATH, TerrainMaterial, perf_defs};

/// Baked albedo (rgb) + forest share (a).
pub const ALBEDO_FORMAT: TextureFormat = TextureFormat::Rgba8UnormSrgb;
/// Baked normal offset (x, z).
pub const DN_FORMAT: TextureFormat = TextureFormat::Rg16Float;
/// Baked relief shade: sun visibility (r), horizon occlusion (g) (ADR 0745).
pub const SHADE_FORMAT: TextureFormat = TextureFormat::Rg8Unorm;
/// Layers of the atlas: more than the nodes drawn at once (selected + retiring).
pub const LAYERS: u32 = 448;
/// Nodes baked per frame at most once every node is shown (a refresh after an invalidation).
const REFRESH_PER_FRAME: usize = 2;
/// Nodes baked per frame at most while some node waits for its first bake.
const FIRST_PER_FRAME: usize = 10;
/// Shortest delay between two invalidations (s): gliding seasons rebake at this pace at most.
const INVALIDATE_MIN_S: f64 = 1.0;

/// One bake: the node mesh into its layer.
#[derive(Clone, Debug)]
pub struct BakeJob {
    pub layer: u32,
    pub ticket: u64,
    pub mesh: AssetId<Mesh>,
    /// Footprint in the real view (baked before the pre-baked border).
    pub visible: bool,
}

/// Main-world state of the cache (present only while the cache is on).
#[derive(Resource)]
pub struct GroundCache {
    pub texels: u32,
    pub albedo: Handle<Image>,
    pub dn: Handle<Image>,
    pub shade: Handle<Image>,
    free: Vec<u32>,
    /// Layers handed out since the last publish: cleared by the render world (alpha 0 = not
    /// baked) before any node draws them, so a reused layer never shows its previous node.
    fresh: Vec<u32>,
    /// Number of the last publish (the render world clears `fresh` once per publish).
    serial: u64,
    next_ticket: u64,
    /// Bumped by an invalidation: every node with an older generation is baked again.
    pub generation: u64,
    signature: Option<u64>,
    shade_signature: u64,
    /// Rebakes of the whole cache caused by the relief shade (sun, scale, settings).
    pub shade_rebakes: u64,
    pub shade_enabled: bool,
    last_invalidation: f64,
    /// Bakes reported done by the render world (layer, ticket).
    pub done: Arc<Mutex<Vec<(u32, u64)>>>,
    pub bakes_done: u64,
    pub pending: usize,
    /// A node found no free layer (it is drawn live-less: the atlas is too small).
    pub exhausted: u64,
}

impl GroundCache {
    pub fn new(texels: u32, images: &mut Assets<Image>) -> GroundCache {
        let mut atlas = |format: TextureFormat| {
            let mut image = Image::new_uninit(
                Extent3d {
                    width: texels,
                    height: texels,
                    depth_or_array_layers: LAYERS,
                },
                TextureDimension::D2,
                format,
                RenderAssetUsages::RENDER_WORLD,
            );
            image.texture_descriptor.usage =
                TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST;
            image.texture_view_descriptor = Some(TextureViewDescriptor {
                dimension: Some(TextureViewDimension::D2Array),
                ..default()
            });
            images.add(image)
        };
        GroundCache {
            texels,
            albedo: atlas(ALBEDO_FORMAT),
            dn: atlas(DN_FORMAT),
            shade: atlas(SHADE_FORMAT),
            free: (0..LAYERS).rev().collect(),
            fresh: Vec::new(),
            serial: 0,
            next_ticket: 0,
            generation: 0,
            signature: None,
            shade_signature: 0,
            shade_rebakes: 0,
            shade_enabled: false,
            last_invalidation: f64::NEG_INFINITY,
            done: Arc::new(Mutex::new(Vec::new())),
            bakes_done: 0,
            pending: 0,
            exhausted: 0,
        }
    }

    /// A free layer for a new node (-1 when the atlas is full).
    pub fn alloc(&mut self) -> i32 {
        match self.free.pop() {
            Some(layer) => {
                self.fresh.push(layer);
                layer as i32
            }
            None => {
                self.exhausted += 1;
                -1
            }
        }
    }

    pub fn release(&mut self, layer: i32) {
        if layer >= 0 {
            self.free.push(layer as u32);
        }
    }

    pub fn ticket(&mut self) -> u64 {
        self.next_ticket += 1;
        self.next_ticket
    }

    pub fn take_done(&self) -> Vec<(u32, u64)> {
        self.done
            .lock()
            .map(|mut done| std::mem::take(&mut *done))
            .unwrap_or_default()
    }

    pub fn used_layers(&self) -> usize {
        LAYERS as usize - self.free.len()
    }
}

/// Bake order of a frame, copied to the render world.
#[derive(Resource, Clone, ExtractResource, Default)]
#[extract_app(RenderApp)]
pub struct GroundBakeQueue {
    pub texels: u32,
    pub albedo: Handle<Image>,
    pub dn: Handle<Image>,
    pub shade: Handle<Image>,
    pub material: AssetId<TerrainMaterial>,
    pub jobs: Vec<BakeJob>,
    /// Layers to clear before this frame's bakes, and the publish they belong to.
    pub clears: Vec<u32>,
    pub serial: u64,
    pub done: Arc<Mutex<Vec<(u32, u64)>>>,
}

/// Marks the campaign camera: the bake runs once per frame in its Core3d schedule.
#[derive(Component, ExtractComponent, Clone, Copy, Default)]
#[extract_app(RenderApp)]
pub struct GroundCacheView;

/// Writes this frame's jobs for the render world (`lod.rs` fills `jobs`, first bakes first).
pub fn publish_jobs(
    cache: &mut GroundCache,
    queue: &mut GroundBakeQueue,
    material: AssetId<TerrainMaterial>,
    mut jobs: Vec<(bool, BakeJob)>,
) {
    let first = jobs.iter().filter(|(first, _)| *first).count();
    jobs.sort_by_key(|(first, job)| (!*first, !job.visible));
    let knob = |name: &str, default: usize| {
        crate::campaign::perf_switch::value(name).map_or(default, |v| v as usize)
    };
    let budget = if first > 0 {
        knob("GC_FIRST", FIRST_PER_FRAME)
    } else {
        knob("GC_REFRESH", REFRESH_PER_FRAME)
    };
    cache.pending = jobs.len();
    queue.texels = cache.texels;
    queue.albedo = cache.albedo.clone();
    queue.dn = cache.dn.clone();
    queue.shade = cache.shade.clone();
    queue.material = material;
    queue.done = cache.done.clone();
    cache.serial += 1;
    queue.serial = cache.serial;
    queue.clears = std::mem::take(&mut cache.fresh);
    queue.jobs = jobs.into_iter().take(budget).map(|(_, job)| job).collect();
}

/// Signature of the relief shade inputs: quantised sun, scale, settings, heightfield handle.
fn shade_signature(material: &TerrainMaterial) -> u64 {
    let mut hasher = DefaultHasher::new();
    format!("{:?}", material.shade).hash(&mut hasher);
    material
        .height_lod
        .as_ref()
        .map(Handle::id)
        .hash(&mut hasher);
    hasher.finish()
}

/// Keeps the shade uniform of the material in step with the settings, the sun and the relief
/// scale (all quantised in [`shade::params`]); writes only a real change.
pub fn sync_shade(
    settings: Option<Res<super::shade::ReliefShadeSettings>>,
    mut commands: Commands,
    cache: Option<ResMut<GroundCache>>,
    handle: Option<Res<super::TerrainMaterialHandle>>,
    mut materials: ResMut<Assets<TerrainMaterial>>,
) {
    let (Some(mut cache), Some(handle)) = (cache, handle) else {
        return;
    };
    let Some(settings) = settings else {
        commands.insert_resource(super::shade::ReliefShadeSettings::load());
        return;
    };
    let Some(material) = materials.get(&handle.0) else {
        return;
    };
    let wanted = super::shade::params(
        &settings,
        material.params.sun_dir.truncate(),
        material.params.scale.x,
        material.params.scale.y,
        material.height_lod.is_some(),
    );
    cache.shade_enabled = wanted.sun.w > 0.5;
    if material.shade != wanted
        && let Some(mut material) = materials.get_mut(&handle.0)
    {
        material.shade = wanted;
    }
}

/// Signature of what the regional ground reads; a change rebakes every node.
fn signature(material: &TerrainMaterial) -> u64 {
    let mut hasher = DefaultHasher::new();
    // Gliding season weights: quantised, so a slow glide rebakes in steps.
    let weights = (material.season.weights * 64.0).round();
    format!(
        "{:?}{:?}{:?}{:?}{:?}{:?}{:?}{:?}",
        material.params.map,
        weights,
        material.season.forest_floor,
        material.season.patches,
        material.season.summer_gold,
        material.ground,
        material.fields,
        material.gd,
    )
    .hash(&mut hasher);
    for handle in [
        &material.colormap,
        &material.bg_albedo,
        &material.bg_normal,
        &material.ab_dist,
        &material.splat,
        &material.micro_albedo,
        &material.parcels_albedo,
        &material.parcels_normal,
        &material.landuse,
        &material.terroir,
    ] {
        handle.as_ref().map(Handle::id).hash(&mut hasher);
    }
    hasher.finish()
}

/// Bumps the generation when the regional inputs of the ground changed.
pub fn invalidate(
    time: Res<Time>,
    cache: Option<ResMut<GroundCache>>,
    handle: Option<Res<super::TerrainMaterialHandle>>,
    materials: Res<Assets<TerrainMaterial>>,
    mut events: MessageReader<AssetEvent<TerrainMaterial>>,
) {
    let modified = events.read().count() > 0;
    let (Some(mut cache), Some(handle)) = (cache, handle) else {
        return;
    };
    // Bench only: rebake every node continuously (`CENT_ANS_GC_STRESS=1`).
    if crate::campaign::perf_switch::value("GC_STRESS").is_some() {
        cache.generation += 1;
    }
    let now = time.elapsed_secs_f64();
    if !modified && cache.signature.is_some() {
        return;
    }
    if now - cache.last_invalidation < INVALIDATE_MIN_S && cache.signature.is_some() {
        return;
    }
    let Some(material) = materials.get(&handle.0) else {
        return;
    };
    let sig = signature(material);
    let shade_sig = shade_signature(material);
    if cache.signature != Some(sig) || cache.shade_signature != shade_sig {
        if cache.signature.is_some() {
            cache.generation += 1;
            if cache.shade_signature != shade_sig {
                cache.shade_rebakes += 1;
            }
        }
        cache.signature = Some(sig);
        cache.shade_signature = shade_sig;
        cache.last_invalidation = now;
    }
}

/// Puts / removes the bake marker on the campaign camera.
pub fn mark_camera(
    mut commands: Commands,
    cache: Option<Res<GroundCache>>,
    cameras: Query<(Entity, Has<GroundCacheView>), With<crate::campaign::camera::CampaignCamera>>,
) {
    for (camera, has) in &cameras {
        if cache.is_some() && !has {
            commands.entity(camera).insert(GroundCacheView);
        } else if cache.is_none() && has {
            commands.entity(camera).remove::<GroundCacheView>();
        }
    }
}

pub struct GroundCachePlugin;

impl Plugin for GroundCachePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GroundBakeQueue>().add_plugins((
            ExtractResourcePlugin::<GroundBakeQueue>::default(),
            ExtractComponentPlugin::<GroundCacheView>::default(),
        ));
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .add_systems(RenderStartup, init_bake_pipeline)
            .add_systems(Render, prepare_bake.in_set(RenderSystems::Prepare))
            .add_systems(Core3d, bake_pass.in_set(Core3dSystems::Prepass));
    }
}

#[derive(Resource)]
struct BakePipeline {
    shader: Handle<Shader>,
    empty: BindGroupLayoutDescriptor,
    empty_group: Option<BindGroup>,
    id: Option<CachedRenderPipelineId>,
    scratch: Option<Scratch>,
}

struct Scratch {
    texels: u32,
    albedo: Texture,
    albedo_view: TextureView,
    dn: Texture,
    dn_view: TextureView,
    shade: Texture,
    shade_view: TextureView,
}

fn init_bake_pipeline(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(BakePipeline {
        shader: asset_server.load(SHADER_PATH),
        empty: BindGroupLayoutDescriptor::new("ground_bake_empty", &[]),
        empty_group: None,
        id: None,
        scratch: None,
    });
}

#[allow(clippy::too_many_arguments)]
fn prepare_bake(
    queue: Option<Res<GroundBakeQueue>>,
    pipeline: Option<ResMut<BakePipeline>>,
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
    meshes: Res<RenderAssets<RenderMesh>>,
    materials: Res<ErasedRenderAssets<PreparedMaterial>>,
    mesh_pipeline: Res<MeshPipeline>,
) {
    let (Some(queue), Some(mut pipeline)) = (queue, pipeline) else {
        return;
    };
    if queue.jobs.is_empty() || queue.texels == 0 {
        return;
    }
    if pipeline.empty_group.is_none() {
        let layout = pipeline_cache.get_bind_group_layout(&pipeline.empty);
        pipeline.empty_group =
            Some(render_device.create_bind_group("ground_bake_empty", &layout, &[]));
    }
    if pipeline.scratch.as_ref().map(|s| s.texels) != Some(queue.texels) {
        let target = |label: &'static str, format: TextureFormat| {
            let texture = render_device.create_texture(&TextureDescriptor {
                label: Some(label),
                size: Extent3d {
                    width: queue.texels,
                    height: queue.texels,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = texture.create_view(&TextureViewDescriptor::default());
            (texture, view)
        };
        let (albedo, albedo_view) = target("ground_bake_albedo", ALBEDO_FORMAT);
        let (dn, dn_view) = target("ground_bake_dn", DN_FORMAT);
        let (shade, shade_view) = target("ground_bake_shade", SHADE_FORMAT);
        pipeline.scratch = Some(Scratch {
            texels: queue.texels,
            albedo,
            albedo_view,
            dn,
            dn_view,
            shade,
            shade_view,
        });
    }
    if pipeline.id.is_some() {
        return;
    }
    let Some(material_layout) = materials
        .get(queue.material)
        .and_then(|m| m.properties.material_layout.clone())
    else {
        return;
    };
    let Some(mesh) = queue.jobs.iter().find_map(|job| meshes.get(job.mesh)) else {
        return;
    };
    let Ok(vertex_layout) = mesh.layout.0.get_layout(&[
        Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
        Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
        Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
        Mesh::ATTRIBUTE_UV_1.at_shader_location(3),
        Mesh::ATTRIBUTE_COLOR.at_shader_location(5),
    ]) else {
        warn!("ground cache: node mesh lacks a vertex attribute, bake disabled");
        return;
    };
    // Defs of the ground module as the material pipeline compiles it (view bindings included,
    // unused by the bake entry points), without `GROUND_CACHE`.
    let key = MeshPipelineKey::from_msaa_samples(1)
        | MeshPipelineKey::DISTANCE_FOG
        | MeshPipelineKey::from_primitive_topology_and_strip_index(
            PrimitiveTopology::TriangleList,
            None,
        );
    let Ok(base) = mesh_pipeline.specialize(key, &mesh.layout) else {
        warn!("ground cache: mesh pipeline specialization failed, bake disabled");
        return;
    };
    let mut defs = base.fragment.map(|f| f.shader_defs).unwrap_or_default();
    defs.push(ShaderDefVal::UInt(
        "MATERIAL_BIND_GROUP".into(),
        MATERIAL_BIND_GROUP_INDEX as u32,
    ));
    defs.push(ShaderDefVal::UInt("GC_TEXELS".into(), queue.texels));
    defs.extend(perf_defs());
    let target = |format: TextureFormat| {
        Some(ColorTargetState {
            format,
            blend: None,
            write_mask: ColorWrites::ALL,
        })
    };
    let descriptor = RenderPipelineDescriptor {
        label: Some("ground_bake_pipeline".into()),
        layout: vec![
            pipeline.empty.clone(),
            pipeline.empty.clone(),
            pipeline.empty.clone(),
            material_layout,
        ],
        vertex: VertexState {
            shader: pipeline.shader.clone(),
            shader_defs: defs.clone(),
            entry_point: Some("bake_vertex".into()),
            buffers: vec![vertex_layout],
            ..default()
        },
        fragment: Some(FragmentState {
            shader: pipeline.shader.clone(),
            shader_defs: defs,
            entry_point: Some("bake_fragment".into()),
            targets: vec![
                target(ALBEDO_FORMAT),
                target(DN_FORMAT),
                target(SHADE_FORMAT),
            ],
            ..default()
        }),
        primitive: PrimitiveState {
            cull_mode: None,
            ..default()
        },
        ..default()
    };
    pipeline.id = Some(pipeline_cache.queue_render_pipeline(descriptor));
}

#[allow(clippy::too_many_arguments)]
fn bake_pass(
    _view: ViewQuery<&GroundCacheView>,
    queue: Option<Res<GroundBakeQueue>>,
    pipeline: Option<Res<BakePipeline>>,
    pipeline_cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<RenderMesh>>,
    mesh_allocator: Res<MeshAllocator>,
    images: Res<RenderAssets<GpuImage>>,
    materials: Res<ErasedRenderAssets<PreparedMaterial>>,
    allocators: Res<MaterialBindGroupAllocators>,
    mut cleared_serial: Local<u64>,
    mut ctx: RenderContext,
) {
    let Some(queue) = queue else {
        return;
    };
    // Fresh layers first, once per publish (an unchanged queue is not extracted again).
    if queue.serial != *cleared_serial {
        *cleared_serial = queue.serial;
        if let Some(albedo_atlas) = images.get(&queue.albedo) {
            let encoder = ctx.command_encoder();
            for &layer in &queue.clears {
                encoder.clear_texture(
                    &albedo_atlas.texture,
                    &ImageSubresourceRange {
                        aspect: TextureAspect::All,
                        base_mip_level: 0,
                        mip_level_count: None,
                        base_array_layer: layer,
                        array_layer_count: Some(1),
                    },
                );
            }
        }
    }
    let Some(pipeline) = pipeline else {
        return;
    };
    if queue.jobs.is_empty() {
        return;
    }
    let (Some(id), Some(scratch), Some(empty)) = (
        pipeline.id,
        pipeline.scratch.as_ref(),
        pipeline.empty_group.as_ref(),
    ) else {
        return;
    };
    let Some(render_pipeline) = pipeline_cache.get_render_pipeline(id) else {
        return;
    };
    let (Some(albedo_atlas), Some(dn_atlas), Some(shade_atlas)) = (
        images.get(&queue.albedo),
        images.get(&queue.dn),
        images.get(&queue.shade),
    ) else {
        return;
    };
    let Some(material) = materials.get(queue.material) else {
        return;
    };
    let Some(bind_group) = allocators
        .get(&std::any::TypeId::of::<TerrainMaterial>())
        .and_then(|allocator| allocator.get(material.binding.group))
        .and_then(|slab| slab.bind_group().cloned())
    else {
        return;
    };
    let mut done = Vec::new();
    for job in &queue.jobs {
        let Some(mesh) = meshes.get(job.mesh) else {
            continue;
        };
        let RenderMeshBufferInfo::Indexed {
            count,
            index_format,
        } = mesh.buffer_info
        else {
            continue;
        };
        let (Some(vertices), Some(indices)) = (
            mesh_allocator.mesh_vertex_slice(&job.mesh),
            mesh_allocator.mesh_index_slice(&job.mesh),
        ) else {
            continue;
        };
        {
            let descriptor = RenderPassDescriptor {
                label: Some("ground_bake_pass"),
                color_attachments: &[
                    Some(clear_attachment(&scratch.albedo_view)),
                    Some(clear_attachment(&scratch.dn_view)),
                    Some(clear_attachment(&scratch.shade_view)),
                ],
                ..default()
            };
            let mut pass = ctx.command_encoder().begin_render_pass(&descriptor);
            pass.set_pipeline(render_pipeline);
            for group in 0..3 {
                pass.set_bind_group(group, empty, &[]);
            }
            pass.set_bind_group(3, &bind_group, &[]);
            pass.set_vertex_buffer(0, *vertices.buffer.slice(..));
            pass.set_index_buffer(*indices.buffer.slice(..), index_format);
            pass.draw_indexed(
                indices.range.start..indices.range.start + count,
                vertices.range.start as i32,
                0..1,
            );
        }
        let size = Extent3d {
            width: scratch.texels,
            height: scratch.texels,
            depth_or_array_layers: 1,
        };
        let encoder = ctx.command_encoder();
        for (source, atlas) in [
            (&scratch.albedo, &albedo_atlas.texture),
            (&scratch.dn, &dn_atlas.texture),
            (&scratch.shade, &shade_atlas.texture),
        ] {
            encoder.copy_texture_to_texture(
                source.as_image_copy(),
                TexelCopyTextureInfo {
                    texture: atlas,
                    mip_level: 0,
                    origin: Origin3d {
                        x: 0,
                        y: 0,
                        z: job.layer,
                    },
                    aspect: TextureAspect::All,
                },
                size,
            );
        }
        done.push((job.layer, job.ticket));
    }
    if let Ok(mut shared) = queue.done.lock() {
        shared.extend(done);
    }
}

fn clear_attachment(view: &TextureView) -> RenderPassColorAttachment<'_> {
    RenderPassColorAttachment {
        view,
        depth_slice: None,
        resolve_target: None,
        ops: Operations {
            load: LoadOp::Clear(Default::default()),
            store: StoreOp::Store,
        },
    }
}
