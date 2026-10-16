//! GPU-skinned, GPU-animated figure batches: one instanced draw per regiment and level of
//! detail. Pattern of `instancing.rs` (custom `Transparent3d` draw lit by Bevy's PBR code) with
//! the vertex stage doing the animation like game/shaders/battle_soldier_skinned.gdshader:
//! bone matrices come from a baked texture (`CAB1`), the clip and time of every soldier are
//! drawn on the GPU from the soldier's hash and the regiment clock.
//!
//! Instance record (12 f32, 48 bytes): `x y z yaw_deg | scale id anim_time far_blend |
//! volley_time highlight 0 0`. Per-batch parameters (livery, clip sets, looks, clip table) are a
//! uniform buffer rebuilt only when `params_version` changes.
//!
//! Arms (DA1): atlas `heraldry::ArmsAtlasHandle` (texture array, binding 4-5), layers and bust
//! box in the parameter block.
//!
//! Fine maps (FG3, `fine_maps.rs`): atlases of LOD0/LOD1 (bindings 6-9), shared detail tiles and
//! horse coat (10-15); the mesh carries the packed atlas uv (`ATTRIBUTE_ATLAS`, location 3).
//!
//! Lost compared with the Godot material: weave/mail/hammered procedural patterns, blood, severed parts, spear flames.

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

use super::anim::{AnimConfig, Mode};
use super::figures::{
    ATTRIBUTE_ATLAS, ATTRIBUTE_BONES, ATTRIBUTE_MASK, ATTRIBUTE_WEIGHTS, MAX_CLIPS,
};
use super::looks::MAX_COLOURS;

const SHADER_PATH: &str = "embedded://cent_ans_client/battle/units/skinned.wesl";
/// Skinned draws issued since the last reset.
pub static DRAWN_SKINNED: AtomicU32 = AtomicU32::new(0);
/// Instances drawn by those draws.
pub static DRAWN_FIGURES: AtomicU32 = AtomicU32::new(0);

/// Floats of one instance record.
pub const RECORD_FLOATS: usize = 12;
const RECORD_BYTES: usize = RECORD_FLOATS * 4;

// Offsets (in f32) inside the parameter block, mirrored by `Params` in `skinned.wesl`.
const P_LIVERY: usize = 0;
const P_TRIM: usize = 4;
const P_CUR_SET: usize = 8;
const P_PREV_SET: usize = 16;
const P_CUR: usize = 24;
const P_PREV: usize = 28;
const P_TIMING: usize = 32;
const P_JITTER: usize = 36;
const P_GA3: usize = 40;
const P_WEAR: usize = 44;
const P_PLAIN: usize = 48;
const P_COAT: usize = 64;
const P_EXTRA: usize = 80;
const P_FX: usize = 84;
const P_HITN: usize = 88;
const P_HIT_SET: usize = 92;
const P_ARMS: usize = 100;
const P_CHEST_BOX: usize = 104;
const P_CHEST: usize = 108;
const P_BADGE_CROSS: usize = 112;
const P_BADGE_PATCH: usize = 116;
const P_FINE: usize = 120;
/// Rider's lance: (bone, pennon share, steel occlusion unused), grip, axis, side (4 `vec4`).
const P_PROP: usize = 124;
const P_CLIPS: usize = 140;
/// Floats of the parameter block (131 `vec4`).
pub const PARAM_FLOATS: usize = P_CLIPS + MAX_CLIPS * 4;
const PARAM_BYTES: u64 = (PARAM_FLOATS * 4) as u64;

/// `steel_occlusion` uniform of the Godot soldier shader (CR2/CR4 specular occlusion of steel and
/// mail, 0 = SR1 look). Godot never overrides the default.
pub const STEEL_OCCLUSION: f32 = 1.0;
/// `pennon_share` uniform: share of the riders whose lance carries a pennon.
pub const PENNON_SHARE: f32 = 0.6;

/// Parameters of a batch.
#[derive(Clone, Debug)]
pub struct SkinParams {
    pub data: Vec<f32>,
}

impl Default for SkinParams {
    fn default() -> Self {
        let mut data = vec![0.0; PARAM_FLOATS];
        // Safe defaults: one clip, no jitter, one variant.
        data[P_CUR + 2] = 1.0;
        data[P_PREV + 2] = 1.0;
        data[P_PREV + 3] = -1000.0;
        data[P_TIMING] = 0.35;
        data[P_TIMING + 1] = 6.0;
        data[P_TIMING + 2] = 1.0;
        data[P_JITTER + 2] = 24.0;
        data[P_JITTER + 3] = 1.0;
        // No fine maps (FG3) until the figure names an atlas layer.
        data[P_FINE] = -1.0;
        data[P_FINE + 1] = super::fine_maps::FINE_DISTANCE;
        data[P_FINE + 2] = STEEL_OCCLUSION;
        // No rider's lance (bone -1), Godot's defaults for the rest.
        data[P_PROP] = -1.0;
        data[P_PROP + 1] = PENNON_SHARE;
        data[P_PROP + 6] = 1.0;
        data[P_PROP + 8] = 1.0;
        // No arms atlas; Godot's default bust box and banneret share.
        data[P_ARMS..P_ARMS + 4].copy_from_slice(&[-1.0; 4]);
        data[P_CHEST_BOX..P_CHEST_BOX + 4].copy_from_slice(&[0.0, 1.22, 0.3, 0.38]);
        data[P_CHEST..P_CHEST + 4].copy_from_slice(&[
            -0.1,
            0.3,
            super::heraldry::VASSAL_SHARE,
            0.0,
        ]);
        Self { data }
    }
}

impl SkinParams {
    pub fn set_livery(&mut self, rgb: [f32; 3], share: f32) {
        self.data[P_LIVERY..P_LIVERY + 3].copy_from_slice(&rgb);
        self.data[P_LIVERY + 3] = share;
    }

    /// Arms of the regiment (atlas layers, bust box, chest arms or livery cross).
    pub fn set_arms(&mut self, arms: &super::heraldry::RegimentArms) {
        for (slot, layer) in arms.layers.iter().enumerate() {
            self.data[P_ARMS + slot] = *layer as f32;
        }
        if let Some(chest) = arms.chest {
            self.data[P_CHEST_BOX..P_CHEST_BOX + 2].copy_from_slice(&chest.center);
            self.data[P_CHEST_BOX + 2..P_CHEST_BOX + 4].copy_from_slice(&chest.size);
            self.data[P_CHEST..P_CHEST + 2].copy_from_slice(&chest.depth);
        }
        self.data[P_CHEST + 3] = if arms.chest_arms { 1.0 } else { 0.0 };
        let (cross, patch) = match arms.badge {
            Some(badge) => (Some(badge.cross), badge.patch),
            None => (None, None),
        };
        for (at, colour) in [(P_BADGE_CROSS, cross), (P_BADGE_PATCH, patch)] {
            match colour {
                Some(rgb) => {
                    self.data[at..at + 3].copy_from_slice(&rgb);
                    self.data[at + 3] = 1.0;
                }
                None => self.data[at..at + 4].copy_from_slice(&[0.0; 4]),
            }
        }
    }

    /// Atlas layer of the regiment's lord or faction (`-1`: no arms).
    pub fn arms_layer(&self) -> i32 {
        self.data[P_ARMS] as i32
    }

    pub fn livery_share(&self) -> f32 {
        self.data[P_LIVERY + 3]
    }

    pub fn set_trim(&mut self, rgb: [f32; 3]) {
        self.data[P_TRIM..P_TRIM + 3].copy_from_slice(&rgb);
    }

    pub fn set_rig(&mut self, fps: f32, variants: u32, size_jitter: f32, reload_s: f32) {
        self.data[P_JITTER + 2] = fps;
        self.data[P_JITTER + 3] = variants.max(1) as f32;
        self.data[P_TIMING + 3] = size_jitter;
        self.data[P_TIMING + 1] = reload_s;
    }

    /// Turns the batch into camera-facing quads of `width` x `height` metres (impostors).
    pub fn set_impostor(&mut self, width: f32, height: f32) {
        self.data[P_EXTRA + 2] = width;
        self.data[P_EXTRA + 3] = height;
    }

    /// Fade between two exchanges of a melee cycle (s, 0 = hard cut).
    pub fn set_cycle_blend(&mut self, seconds: f32) {
        self.data[P_FX] = seconds;
    }

    /// Reaction clips to losses (`hit` set, at most 8) and how long each plays.
    pub fn set_hit_set(&mut self, clips: &[u32], seconds: f32, share: f32) {
        for slot in 0..MAX_SETSLOTS {
            self.data[P_HIT_SET + slot] = clips.get(slot).copied().unwrap_or(0) as f32;
        }
        self.data[P_HITN] = clips.len().min(MAX_SETSLOTS) as f32;
        self.data[P_FX + 1] = -1000.0;
        self.data[P_FX + 2] = seconds;
        self.data[P_FX + 3] = share;
    }

    /// Instant (regiment clock) of the latest losses: starts the reactions.
    pub fn set_hit_time(&mut self, time: f32) {
        self.data[P_FX + 1] = time;
    }

    /// Current time read by fixed-mode batches (corpses): `cur.w`.
    pub fn set_now(&mut self, now: f32) {
        self.data[P_CUR + 3] = now;
    }

    pub fn set_blend_time(&mut self, seconds: f32) {
        self.data[P_TIMING] = seconds;
    }

    /// Clip table of the rig: `(start, frames, loops)` per sorted clip name.
    pub fn set_clips(&mut self, clips: &[(u32, u32, bool)]) {
        for (i, (start, frames, looped)) in clips.iter().take(MAX_CLIPS).enumerate() {
            let at = P_CLIPS + i * 4;
            self.data[at] = *start as f32;
            self.data[at + 1] = (*frames).max(1) as f32;
            self.data[at + 2] = if *looped { 1.0 } else { 0.0 };
        }
    }

    /// Albedo of a generated figure (`ga3`): tint luminances and head height.
    pub fn set_ga3(&mut self, lum: [f32; 2], head_y: f32, has_albedo: bool) {
        self.data[P_GA3] = lum[0].max(1e-3);
        self.data[P_GA3 + 1] = lum[1].max(1e-3);
        self.data[P_GA3 + 2] = head_y;
        self.data[P_GA3 + 3] = if has_albedo { 1.0 } else { 0.0 };
    }

    /// FG3 fine maps of the figure: atlas layer (`< 0`: none) and the shared distance limit of
    /// the maps (m, `fine_distance`).
    pub fn set_fine(&mut self, atlas_layer: i32) {
        self.data[P_FINE] = atlas_layer as f32;
        self.data[P_FINE + 1] = super::fine_maps::FINE_DISTANCE;
    }

    /// `steel_occlusion` (0 = no specular occlusion, 1 = Godot's default).
    pub fn set_steel_occlusion(&mut self, value: f32) {
        self.data[P_FINE + 2] = value.clamp(0.0, 1.0);
    }

    pub fn steel_occlusion(&self) -> f32 {
        self.data[P_FINE + 2]
    }

    /// Rider's lance (`_setup_lance`): rig bone of the `Prop`, grip, axis and side in the rest
    /// pose. `None`: no lance (bone -1).
    pub fn set_prop(&mut self, prop: Option<&super::library::LanceSpec>) {
        match prop {
            Some(l) => {
                self.data[P_PROP] = l.bone as f32;
                self.data[P_PROP + 4..P_PROP + 7].copy_from_slice(&l.grip);
                self.data[P_PROP + 8..P_PROP + 11].copy_from_slice(&l.axis);
                self.data[P_PROP + 12..P_PROP + 15].copy_from_slice(&l.side);
            }
            None => self.data[P_PROP] = -1.0,
        }
    }

    pub fn prop_bone(&self) -> i32 {
        self.data[P_PROP] as i32
    }

    pub fn set_wear(&mut self, mud_height: f32, weathering: f32) {
        self.data[P_WEAR + 2] = mud_height;
        self.data[P_WEAR + 3] = weathering;
    }

    pub fn set_plain(&mut self, colours: &[[f32; 3]; MAX_COLOURS], count: usize) {
        for (i, c) in colours.iter().enumerate() {
            self.data[P_PLAIN + i * 4..P_PLAIN + i * 4 + 3].copy_from_slice(c);
        }
        self.data[P_WEAR] = count as f32;
    }

    pub fn set_coats(&mut self, colours: &[[f32; 3]; MAX_COLOURS], count: usize) {
        for (i, c) in colours.iter().enumerate() {
            self.data[P_COAT + i * 4..P_COAT + i * 4 + 3].copy_from_slice(c);
        }
        self.data[P_WEAR + 1] = count as f32;
    }

    fn write_config(&mut self, set_at: usize, block_at: usize, config: &AnimConfig) {
        for slot in 0..MAX_SETSLOTS {
            self.data[set_at + slot] = config.set.get(slot).copied().unwrap_or(0) as f32;
        }
        self.data[block_at] = config.speed;
        self.data[block_at + 1] = match config.mode {
            Mode::Loop => 0.0,
            Mode::Cycle => 1.0,
            Mode::Volley => 2.0,
            Mode::Pick => 3.0,
        };
        self.data[block_at + 2] = config.set.len().max(1) as f32;
    }

    /// Makes `config` the current animation; the previous one becomes the source of a fade
    /// starting at `now` (regiment clock). `None` for the first configuration.
    pub fn set_config(&mut self, config: &AnimConfig, now: f32, fade: bool) {
        if fade {
            let (cur_set, prev_set) = (P_CUR_SET, P_PREV_SET);
            let current: Vec<f32> = self.data[cur_set..cur_set + MAX_SETSLOTS].to_vec();
            self.data[prev_set..prev_set + MAX_SETSLOTS].copy_from_slice(&current);
            let cur_block: Vec<f32> = self.data[P_CUR..P_CUR + 4].to_vec();
            self.data[P_PREV..P_PREV + 3].copy_from_slice(&cur_block[..3]);
            self.data[P_JITTER + 1] = self.data[P_JITTER];
            self.data[P_EXTRA] = self.data[P_CUR + 3];
            self.data[P_EXTRA + 1] = self.data[P_TIMING + 2];
            self.data[P_PREV + 3] = now;
        } else {
            self.data[P_PREV + 3] = -1000.0;
        }
        self.write_config(P_CUR_SET, P_CUR, config);
        self.data[P_CUR + 3] = config.cycle;
        self.data[P_TIMING + 2] = config.release;
        self.data[P_JITTER] = config.jitter;
    }
}

/// FG3 atlas layer stored in a parameter block (`-1`: none).
pub fn fine_layer_of(params: &[f32]) -> i32 {
    params[P_FINE] as i32
}

const MAX_SETSLOTS: usize = 8;

/// One instanced skinned draw.
#[derive(Component, Clone)]
pub struct SkinnedBatch {
    /// Current level of detail (switch the handle to change it).
    pub mesh_handle: Handle<Mesh>,
    pub albedo: Option<Handle<Image>>,
    pub bones: Handle<Image>,
    pub params: Arc<Vec<f32>>,
    pub params_version: u64,
    pub records: Arc<Vec<u8>>,
    pub count: u32,
    pub version: u64,
}

impl SyncComponent<RenderApp> for SkinnedBatch {
    type Target = Self;
}

impl ExtractComponent<RenderApp> for SkinnedBatch {
    type QueryData = &'static SkinnedBatch;
    type QueryFilter = ();
    type Out = Self;

    fn extract_component(item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self> {
        Some(item.clone())
    }
}

/// Appends one record to `bytes`.
pub fn push_record(bytes: &mut Vec<u8>, record: &[f32; RECORD_FLOATS]) {
    for value in record {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
}

pub struct SkinnedPlugin;

impl Plugin for SkinnedPlugin {
    fn build(&self, app: &mut App) {
        bevy::asset::embedded_asset!(app, "skinned.wesl");
        app.add_plugins(ExtractComponentPlugin::<SkinnedBatch>::default());
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .add_render_command::<Transparent3d, DrawSkinned>()
            .add_render_command::<Shadow, DrawSkinnedShadow>()
            .init_resource::<SpecializedMeshPipelines<SkinnedPipeline>>()
            .init_resource::<SpecializedMeshPipelines<SkinnedShadowPipeline>>()
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
                    prepare_buffers
                        .in_set(RenderSystems::PrepareResources)
                        .run_if(|batches: Query<(), With<SkinnedBatch>>| !batches.is_empty()),
                    prepare_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                ),
            );
    }
}

#[derive(Resource, Clone)]
struct SkinnedPipeline {
    shader: Handle<Shader>,
    mesh_pipeline: MeshPipeline,
    material_layout: BindGroupLayoutDescriptor,
}

fn init_pipeline(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mesh_pipeline: Res<MeshPipeline>,
) {
    use bevy::render::render_resource::binding_types::{
        sampler, texture_2d, texture_2d_array, uniform_buffer_sized,
    };
    let material_layout = BindGroupLayoutDescriptor::new(
        "skinned_material_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::VERTEX_FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_2d(TextureSampleType::Float { filterable: false }),
                uniform_buffer_sized(false, BufferSize::new(PARAM_BYTES)),
                texture_2d_array(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                // FG3: atlas LOD0, atlas LOD1 (clamped sampler), detail tiles, detail albedo,
                // horse coat (repeating sampler).
                texture_2d_array(TextureSampleType::Float { filterable: true }),
                texture_2d_array(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                texture_2d_array(TextureSampleType::Float { filterable: true }),
                texture_2d_array(TextureSampleType::Float { filterable: true }),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    );
    commands.insert_resource(SkinnedPipeline {
        shader: asset_server.load(SHADER_PATH),
        mesh_pipeline: mesh_pipeline.clone(),
        material_layout,
    });
}

impl SpecializedMeshPipeline for SkinnedPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.mesh_pipeline.specialize(key, layout)?;
        descriptor.vertex.shader = self.shader.clone();
        let mesh_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_UV_0.at_shader_location(2),
            ATTRIBUTE_ATLAS.at_shader_location(3),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(5),
            ATTRIBUTE_BONES.at_shader_location(10),
            ATTRIBUTE_WEIGHTS.at_shader_location(11),
            ATTRIBUTE_MASK.at_shader_location(12),
        ])?;
        descriptor.vertex.buffers = vec![
            mesh_layout,
            VertexBufferLayout {
                array_stride: RECORD_BYTES as u64,
                step_mode: VertexStepMode::Instance,
                attributes: vec![
                    VertexAttribute {
                        format: VertexFormat::Float32x4,
                        offset: 0,
                        shader_location: 13,
                    },
                    VertexAttribute {
                        format: VertexFormat::Float32x4,
                        offset: 16,
                        shader_location: 14,
                    },
                    VertexAttribute {
                        format: VertexFormat::Float32x4,
                        offset: 32,
                        shader_location: 15,
                    },
                ],
            },
        ];
        if let Some(fragment) = descriptor.fragment.as_mut() {
            fragment.shader = self.shader.clone();
        }
        descriptor.layout.truncate(2);
        descriptor.layout.push(self.material_layout.clone());
        Ok(descriptor)
    }
}

#[allow(clippy::too_many_arguments)]
fn queue_batches(
    draw_functions: Res<DrawFunctions<Transparent3d>>,
    pipeline: Res<SkinnedPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<SkinnedPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<RenderMesh>>,
    batches: Query<(Entity, &MainEntity, &SkinnedBatch)>,
    mut phases: ResMut<ViewSortedRenderPhases<Transparent3d>>,
    views: Query<&ExtractedView>,
    view_key_cache: Res<ViewKeyCache>,
    mut reported: Local<HashSet<(Entity, &'static str)>>,
) {
    let draw_function = draw_functions.read().id::<DrawSkinned>();
    for view in &views {
        let Some(phase) = phases.get_mut(&view.retained_view_entity) else {
            continue;
        };
        let Some(&view_key) = view_key_cache.get(&view.retained_view_entity) else {
            continue;
        };
        for (entity, main_entity, batch) in &batches {
            let Some(mesh) = meshes.get(batch.mesh_handle.id()) else {
                continue;
            };
            let key = view_key
                | MeshPipelineKey::from_primitive_topology_and_strip_index(
                    mesh.primitive_topology(),
                    mesh.index_format(),
                );
            let pipeline_id =
                match pipelines.specialize(&pipeline_cache, &pipeline, key, &mesh.layout) {
                    Ok(id) => id,
                    Err(error) => {
                        if reported.insert((entity, "specialize failed")) {
                            error!("skinned batch {entity:?}: specialization failed: {error}");
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
    arms: Option<AssetId<Image>>,
    /// Fine maps were bound (FG3).
    fine: bool,
}

fn prepare_buffers(
    mut commands: Commands,
    batches: Query<(Entity, &SkinnedBatch, Option<&InstanceBuffer>)>,
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
            label: Some("skinned records"),
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

#[allow(clippy::too_many_arguments)]
fn prepare_bind_groups(
    mut commands: Commands,
    batches: Query<(Entity, &SkinnedBatch, Option<&MaterialBindGroup>)>,
    images: Res<RenderAssets<GpuImage>>,
    fallback: Res<FallbackImage>,
    pipeline: Res<SkinnedPipeline>,
    pipeline_cache: Res<PipelineCache>,
    render_device: Res<RenderDevice>,
    atlas: Option<Res<super::heraldry::ArmsAtlasHandle>>,
    fine_maps: Option<Res<super::fine_maps::FineMaps>>,
) {
    let arms_id = atlas.as_ref().and_then(|a| a.0.as_ref()).map(|h| h.id());
    let arms_image = match arms_id {
        Some(id) => images.get(id),
        None => None,
    };
    // Fine maps: all five uploaded, else the batch is bound to fallbacks with `layer = -1`.
    let fine_images = fine_maps.as_ref().and_then(|m| {
        Some([
            images.get(m.atlas0.as_ref()?.id())?,
            images.get(m.atlas1.as_ref()?.id())?,
            images.get(m.detail.as_ref()?.id())?,
            images.get(m.detail_albedo.as_ref()?.id())?,
            images.get(m.horse.as_ref()?.id())?,
        ])
    });
    let fine_ready = fine_images.is_some();
    for (entity, batch, existing) in &batches {
        if existing.is_some_and(|g| {
            g.params_version == batch.params_version && g.arms == arms_id && g.fine == fine_ready
        }) {
            continue;
        }
        // Wait for the atlas upload rather than caching a group without arms.
        if arms_id.is_some() && arms_image.is_none() {
            continue;
        }
        let arms = arms_image.unwrap_or(&fallback.d2_array);
        let albedo = match &batch.albedo {
            Some(handle) => match images.get(handle.id()) {
                Some(image) => image,
                None => continue,
            },
            None => &fallback.d2,
        };
        let Some(bones) = images.get(batch.bones.id()) else {
            continue;
        };
        let mut params = batch.params.as_ref().clone();
        if !fine_ready {
            params[P_FINE] = -1.0;
        }
        let bytes: Vec<u8> = params
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect();
        let [atlas0, atlas1, detail, detail_albedo, horse] = match fine_images {
            Some(set) => set,
            None => [
                &fallback.d2_array,
                &fallback.d2_array,
                &fallback.d2_array,
                &fallback.d2_array,
                &fallback.d2,
            ],
        };
        let uniform = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("skinned params"),
            contents: &bytes,
            usage: BufferUsages::UNIFORM,
        });
        let group = render_device.create_bind_group(
            "skinned_material_bind_group",
            &pipeline_cache.get_bind_group_layout(&pipeline.material_layout),
            &BindGroupEntries::sequential((
                &albedo.texture_view,
                &albedo.sampler,
                &bones.texture_view,
                uniform.as_entire_binding(),
                &arms.texture_view,
                &arms.sampler,
                &atlas0.texture_view,
                &atlas1.texture_view,
                &atlas0.sampler,
                &detail.texture_view,
                &detail_albedo.texture_view,
                &horse.texture_view,
                &detail.sampler,
            )),
        );
        commands.entity(entity).insert(MaterialBindGroup {
            group,
            params_version: batch.params_version,
            arms: arms_id,
            fine: fine_ready,
        });
    }
}

// --- Shadow casting --------------------------------------------------------------------------
//
// The figures are drawn into every directional shadow cascade with the same vertex stage (bone
// texture animation), as a non-mesh item of Bevy's binned `Shadow` phase. Layout of Bevy's
// prepass: group 0 = prepass view (its binding 0 is the light view's `View`, which is all the
// vertex stage reads through `mesh_view_bindings::view`), group 1 = empty, group 2 = our
// material. Depth only (no fragment stage): the impostor batches (alpha-cut quads) cast none.

/// Shadow pipeline of the skinned figures.
#[derive(Resource)]
struct SkinnedShadowPipeline {
    base: SkinnedPipeline,
    view_layout: BindGroupLayoutDescriptor,
    empty_layout: BindGroupLayoutDescriptor,
    unclipped_depth: bool,
}

fn init_shadow_pipeline(
    mut commands: Commands,
    base: Res<SkinnedPipeline>,
    prepass: Res<PrepassPipeline>,
) {
    commands.insert_resource(SkinnedShadowPipeline {
        base: base.clone(),
        view_layout: prepass.view_layout_no_motion_vectors.clone(),
        empty_layout: prepass.empty_layout.clone(),
        unclipped_depth: prepass.depth_clip_control_supported,
    });
    UNCLIPPED_DEPTH.store(prepass.depth_clip_control_supported, Ordering::Relaxed);
}

impl SpecializedMeshPipeline for SkinnedShadowPipeline {
    type Key = MeshPipelineKey;

    fn specialize(
        &self,
        key: Self::Key,
        layout: &MeshVertexBufferLayoutRef,
    ) -> Result<RenderPipelineDescriptor, SpecializedMeshPipelineError> {
        let mut descriptor = self.base.specialize(key, layout)?;
        descriptor.label = Some("skinned_shadow_pipeline".into());
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

/// Batches put in each cascade's phase last frame (removed before re-adding).
#[derive(Default)]
struct QueuedCasters(HashMap<RetainedViewEntity, Vec<MainEntity>>);

/// Shadow draws actually issued since the last reset (pipeline ready).
pub static SHADOW_DRAWS: AtomicU32 = AtomicU32::new(0);
/// Whether the GPU clamps depth natively (`DEPTH_CLIP_CONTROL`).
pub static UNCLIPPED_DEPTH: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

struct CountShadowDraw;

impl<P: PhaseItem> RenderCommand<P> for CountShadowDraw {
    type Param = ();
    type ViewQuery = ();
    type ItemQuery = ();

    fn render<'w>(
        _item: &P,
        _view: (),
        _entity: Option<()>,
        _param: (),
        _pass: &mut TrackedRenderPass<'w>,
    ) -> RenderCommandResult {
        SHADOW_DRAWS.fetch_add(1, Ordering::Relaxed);
        RenderCommandResult::Success
    }
}

/// Shadow casters actually queued this frame (batches x cascades).
pub static SHADOW_CASTERS: AtomicU32 = AtomicU32::new(0);

#[allow(clippy::too_many_arguments)]
fn queue_shadow_casters(
    draw_functions: Res<DrawFunctions<Shadow>>,
    pipeline: Res<SkinnedShadowPipeline>,
    mut pipelines: ResMut<SpecializedMeshPipelines<SkinnedShadowPipeline>>,
    pipeline_cache: Res<PipelineCache>,
    meshes: Res<RenderAssets<RenderMesh>>,
    allocator: Res<MeshAllocator>,
    batches: Query<(Entity, &MainEntity, &SkinnedBatch)>,
    mut phases: ResMut<ViewBinnedRenderPhases<Shadow>>,
    lights: Query<(&LightEntity, &ExtractedView)>,
    mut queued: Local<QueuedCasters>,
    mut reported: Local<bool>,
    mut disabled: Local<Option<bool>>,
) {
    let draw_function = draw_functions.read().id::<DrawSkinnedShadow>();
    let mut casters = 0;
    // Dev hook for proof diffs: `CENT_ANS_NO_FIGURE_SHADOWS=1` queues none.
    let off = *disabled.get_or_insert_with(|| std::env::var("CENT_ANS_NO_FIGURE_SHADOWS").is_ok());
    for (light, view) in &lights {
        if !matches!(light, LightEntity::Directional { .. }) {
            continue;
        }
        let Some(phase) = phases.get_mut(&view.retained_view_entity) else {
            continue;
        };
        let previous = queued.0.entry(view.retained_view_entity).or_default();
        for main_entity in previous.drain(..) {
            phase.remove(main_entity);
        }
        for (entity, main_entity, batch) in batches.iter().filter(|_| !off) {
            // Impostors (alpha-cut camera-facing quads) cast no shadow.
            if batch.params.get(P_EXTRA + 2).copied().unwrap_or(0.0) > 0.0 || batch.count == 0 {
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
                            error!("skinned shadow: specialization failed: {error}");
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
    SHADOW_CASTERS.store(casters, Ordering::Relaxed);
}

type DrawSkinnedShadow = (
    SetItemPipeline,
    SetPrepassViewBindGroup<0>,
    SetPrepassViewEmptyBindGroup<1>,
    SetMaterialBindGroup<2>,
    DrawBatch,
    CountShadowDraw,
);

type DrawSkinned = (
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
    type ItemQuery = (Read<InstanceBuffer>, Read<SkinnedBatch>);

    fn render<'w>(
        _item: &P,
        _view: (),
        item_query: Option<(&'w InstanceBuffer, &'w SkinnedBatch)>,
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
        DRAWN_SKINNED.fetch_add(1, Ordering::Relaxed);
        DRAWN_FIGURES.fetch_add(instances.count, Ordering::Relaxed);
        RenderCommandResult::Success
    }
}

#[cfg(test)]
mod tests {
    type V3 = [f32; 3];

    fn sub_dot(a: V3, b: V3) -> f32 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }

    fn cross(a: V3, b: V3) -> V3 {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    }

    fn scale(a: V3, k: f32) -> V3 {
        [a[0] * k, a[1] * k, a[2] * k]
    }

    fn add(a: V3, b: V3) -> V3 {
        [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
    }

    fn normalized(a: V3) -> V3 {
        scale(a, 1.0 / sub_dot(a, a).sqrt())
    }

    /// Rust mirror of `fg3_perturb` in skinned.wesl (tangent frame solved from the derivatives).
    fn perturb_reference(
        n: V3,
        dp1: V3,
        dp2: V3,
        duv1: [f32; 2],
        duv2: [f32; 2],
        nxy: [f32; 2],
    ) -> V3 {
        let det = duv1[0] * duv2[1] - duv1[1] * duv2[0];
        let sgn = if det >= 0.0 { 1.0 } else { -1.0 };
        let t = scale(add(scale(dp1, duv2[1]), scale(dp2, -duv1[1])), sgn);
        let b = scale(add(scale(dp2, duv1[0]), scale(dp1, -duv2[0])), sgn);
        let m = sub_dot(t, t).max(sub_dot(b, b));
        let inv = m.sqrt().recip();
        normalized(add(add(scale(t, inv * nxy[0]), scale(b, inv * nxy[1])), n))
    }

    /// The previous (Schuler) cotangent frame, kept to show why the sign is now explicit.
    fn schuler_reference(
        n: V3,
        dp1: V3,
        dp2: V3,
        duv1: [f32; 2],
        duv2: [f32; 2],
        nxy: [f32; 2],
    ) -> V3 {
        let dp2perp = cross(dp2, n);
        let dp1perp = cross(n, dp1);
        let t = add(scale(dp2perp, duv1[0]), scale(dp1perp, duv2[0]));
        let b = add(scale(dp2perp, duv1[1]), scale(dp1perp, duv2[1]));
        let m = sub_dot(t, t).max(sub_dot(b, b));
        let inv = m.sqrt().recip();
        normalized(add(add(scale(t, inv * nxy[0]), scale(b, inv * nxy[1])), n))
    }

    /// A map tilted towards +u (red > 0.5) must lean the normal towards the world direction in
    /// which u grows, whatever the direction of the screen y axis and the mirroring of the uv.
    #[test]
    fn tangent_space_normals_follow_the_uv_direction() {
        let n = [0.0, 0.0, 1.0];
        let k = 0.01;
        // u grows towards world +x, v (up) towards world +y.
        for y_down in [true, false] {
            let sy = if y_down { -1.0 } else { 1.0 }; // world y change per screen y step
            let dp1 = [k, 0.0, 0.0];
            let dp2 = [0.0, sy * k, 0.0];
            let duv1 = [k, 0.0];
            let duv2 = [0.0, sy * k];
            let r = perturb_reference(n, dp1, dp2, duv1, duv2, [0.5, 0.0]);
            assert!(
                r[0] > 0.3 && r[1].abs() < 1e-5,
                "+u -> +x ({y_down}): {r:?}"
            );
            let r = perturb_reference(n, dp1, dp2, duv1, duv2, [0.0, 0.5]);
            assert!(
                r[1] > 0.3 && r[0].abs() < 1e-5,
                "+v -> +y ({y_down}): {r:?}"
            );
            // Mirrored uv (u towards -x): the tilt follows u.
            let r = perturb_reference(n, dp1, dp2, [-k, 0.0], duv2, [0.5, 0.0]);
            assert!(r[0] < -0.3, "mirrored u ({y_down}): {r:?}");
        }
        // The previous formula flipped with the screen y direction (hence the explicit sign).
        let a = schuler_reference(
            n,
            [k, 0.0, 0.0],
            [0.0, -k, 0.0],
            [k, 0.0],
            [0.0, -k],
            [0.5, 0.0],
        );
        let b = schuler_reference(
            n,
            [k, 0.0, 0.0],
            [0.0, k, 0.0],
            [k, 0.0],
            [0.0, k],
            [0.5, 0.0],
        );
        assert!(a[0] * b[0] < 0.0, "{a:?} {b:?}");
    }

    #[test]
    fn steel_occlusion_and_lance_defaults_match_godot() {
        let mut p = SkinParams::default();
        assert_eq!(p.steel_occlusion(), 1.0);
        assert_eq!(p.prop_bone(), -1);
        assert_eq!(p.data[P_PROP + 1], 0.6);
        p.set_steel_occlusion(0.4);
        assert!((p.steel_occlusion() - 0.4).abs() < 1e-6);
        p.set_prop(Some(&super::super::library::LanceSpec {
            bone: 9,
            grip: [1.0, 2.0, 3.0],
            axis: [0.0, 0.0, 1.0],
            side: [-1.0, 0.0, 0.0],
        }));
        assert_eq!(p.prop_bone(), 9);
        assert_eq!(p.data[P_PROP + 5], 2.0);
        assert_eq!(p.data[P_PROP + 12], -1.0);
    }

    use super::*;
    use crate::battle::units::anim::{AnimConfig, Mode};

    fn config(set: &[u32], mode: Mode) -> AnimConfig {
        AnimConfig {
            key: "t".into(),
            names: vec![],
            set: set.to_vec(),
            mode,
            speed: 1.5,
            cycle: 2.0,
            release: 0.3,
            jitter: 0.03,
        }
    }

    #[test]
    fn param_block_is_131_vec4() {
        assert_eq!(PARAM_FLOATS, 131 * 4);
        assert_eq!(SkinParams::default().data.len(), PARAM_FLOATS);
    }

    #[test]
    fn config_swap_keeps_previous_as_fade_source() {
        let mut p = SkinParams::default();
        p.set_config(&config(&[3, 4], Mode::Cycle), 0.0, false);
        assert_eq!(p.data[P_CUR_SET..P_CUR_SET + 3], [3.0, 4.0, 0.0]);
        assert_eq!(p.data[P_CUR + 1], 1.0);
        assert_eq!(p.data[P_CUR + 2], 2.0);
        assert_eq!(p.data[P_PREV + 3], -1000.0);
        p.set_config(&config(&[7], Mode::Loop), 12.5, true);
        assert_eq!(p.data[P_PREV_SET], 3.0);
        assert_eq!(p.data[P_PREV + 1], 1.0, "previous mode kept");
        assert_eq!(p.data[P_PREV + 3], 12.5);
        assert_eq!(p.data[P_CUR_SET], 7.0);
        assert_eq!(p.data[P_CUR + 1], 0.0);
    }

    #[test]
    fn clip_table_and_record() {
        let mut p = SkinParams::default();
        p.set_clips(&[(10, 20, true), (30, 0, false)]);
        assert_eq!(p.data[P_CLIPS..P_CLIPS + 3], [10.0, 20.0, 1.0]);
        assert_eq!(p.data[P_CLIPS + 4 + 1], 1.0, "zero frames clamp to one");
        let mut bytes = Vec::new();
        push_record(&mut bytes, &[0.0; RECORD_FLOATS]);
        assert_eq!(bytes.len(), RECORD_BYTES);
    }
}
