//! Movement bubble: the cells the selected army reaches this turn (vivid) and next turn (pale),
//! drawn on the relief by a mask shader. Port of `army_movement_bubble.gd` and
//! `reachable_bubble.gdshader` (lot armies6; before, one quad per cell).
//!
//! The mask is `game_api::reach_mask` (one texel per navigation-grid cell, cropped to the area):
//! R = this turn, G = cost / budget, B = next turn. A grid mesh laid on the relief covers the
//! mask frame; the fragment shader (`shaders/bubble.wesl`) softens each mask with nine bilinear
//! taps then thresholds it at 0.5, which gives a rounded contour, an anti-aliased golden rim of
//! constant width on screen, an ink shadow outside and a veil that strengthens towards the edge
//! of the range; the next turn has its own paler tone. The mask gets a CPU mip chain so the
//! contour stays stable when zooming out (a cell smaller than a screen pixel).

use bevy::asset::RenderAssetUsages;
use bevy::asset::embedded_asset;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::reflect::TypePath;
use bevy::render::render_resource::{
    AsBindGroup, Extent3d, TextureDataOrder, TextureDimension, TextureFormat,
};
use bevy::shader::ShaderRef;
use game_api::ReachMask;

use super::projection::Ground;

/// Lift above the ground (`ArmyMovementBubble.LIFT`).
pub const LIFT: f32 = 0.35;
/// Step of the mesh grid in map pixels, and the cap on its quads per side (`MIN_STEP_PX`,
/// `MAX_QUADS`).
pub const MIN_STEP_PX: f32 = 4.0;
pub const MAX_QUADS: f32 = 96.0;

/// The bubble entity.
#[derive(Component)]
pub struct BubbleMesh;

/// Handles and counters of the bubble.
#[derive(Resource, Default)]
pub struct BubbleHandles {
    pub mesh: Option<Handle<Mesh>>,
    pub material: Option<Handle<BubbleMaterial>>,
    pub quads: usize,
    pub builds: u32,
    /// Mask frame in texels and mip levels of the last build (dump).
    pub mask_size: [u32; 2],
    pub mask_mips: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, bevy::render::render_resource::ShaderType)]
pub struct BubbleParams {
    pub fill_color: Vec4,
    pub rim_fill_color: Vec4,
    pub edge_color: Vec4,
    pub shadow_color: Vec4,
    pub next_fill_color: Vec4,
    pub next_edge_color: Vec4,
    /// xy origin of the mask in map pixels, zw its extent.
    pub frame: Vec4,
    /// xy texel size, z edge_px, w next_edge_px.
    pub texel: Vec4,
    /// x show_next, y depth_pull.
    pub flags: Vec4,
}

fn srgba(r: f32, g: f32, b: f32, a: f32) -> Vec4 {
    let c = Color::srgb(r, g, b).to_linear();
    Vec4::new(c.red, c.green, c.blue, a)
}

impl Default for BubbleParams {
    /// Uniform defaults of `reachable_bubble.gdshader`.
    fn default() -> Self {
        Self {
            fill_color: srgba(0.98, 0.88, 0.58, 0.05),
            rim_fill_color: srgba(1.0, 0.82, 0.40, 0.20),
            edge_color: srgba(1.0, 0.86, 0.42, 1.0),
            shadow_color: srgba(0.16, 0.09, 0.03, 0.55),
            next_fill_color: srgba(0.90, 0.88, 0.80, 0.13),
            next_edge_color: srgba(0.96, 0.92, 0.78, 0.85),
            frame: Vec4::new(0.0, 0.0, 1.0, 1.0),
            texel: Vec4::new(0.01, 0.01, 1.6, 1.4),
            flags: Vec4::new(1.0, 0.004, 0.0, 0.0),
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct BubbleMaterial {
    #[uniform(0)]
    pub params: BubbleParams,
    #[texture(1)]
    #[sampler(2)]
    pub mask: Option<Handle<Image>>,
}

impl Material for BubbleMaterial {
    fn vertex_shader() -> ShaderRef {
        "embedded://cent_ans_client/campaign/armies/shaders/bubble.wesl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "embedded://cent_ans_client/campaign/armies/shaders/bubble.wesl".into()
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
        Ok(())
    }
}

pub struct BubblePlugin;

impl Plugin for BubblePlugin {
    fn build(&self, app: &mut App) {
        if app.world().contains_resource::<Assets<Mesh>>() {
            embedded_asset!(app, "shaders/bubble.wesl");
            app.add_plugins(MaterialPlugin::<BubbleMaterial>::default());
        }
    }
}

/// Material parameters for a mask frame.
pub fn params_for(mask: &ReachMask) -> BubbleParams {
    BubbleParams {
        frame: Vec4::new(mask.origin[0], mask.origin[1], mask.size[0], mask.size[1]),
        texel: Vec4::new(
            1.0 / mask.width.max(1) as f32,
            1.0 / mask.height.max(1) as f32,
            1.6,
            1.4,
        ),
        flags: Vec4::new(if mask.next_cells > 0 { 1.0 } else { 0.0 }, 0.004, 0.0, 0.0),
        ..BubbleParams::default()
    }
}

/// RGBA8 data of the mask with a box-filtered mip chain down to 1x1 (levels one after another).
/// Returns the data and the number of levels.
pub fn mask_with_mips(rgb: &[u8], width: u32, height: u32) -> (Vec<u8>, u32) {
    let (mut w, mut h) = (width.max(1) as usize, height.max(1) as usize);
    let mut level: Vec<u8> = rgb
        .chunks(3)
        .flat_map(|t| [t[0], t[1], t[2], 255])
        .collect();
    let mut data = level.clone();
    let mut levels = 1;
    while w > 1 || h > 1 {
        let (nw, nh) = ((w / 2).max(1), (h / 2).max(1));
        let mut next = vec![0u8; nw * nh * 4];
        for y in 0..nh {
            for x in 0..nw {
                for c in 0..4 {
                    let mut sum = 0u32;
                    let mut count = 0u32;
                    for dy in 0..2 {
                        for dx in 0..2 {
                            let (sx, sy) = ((x * 2 + dx).min(w - 1), (y * 2 + dy).min(h - 1));
                            sum += u32::from(level[(sy * w + sx) * 4 + c]);
                            count += 1;
                        }
                    }
                    next[(y * nw + x) * 4 + c] = (sum / count) as u8;
                }
            }
        }
        data.extend_from_slice(&next);
        level = next;
        (w, h) = (nw, nh);
        levels += 1;
    }
    (data, levels)
}

/// Mask image (linear RGBA8, bilinear, clamped) of a reach mask.
pub fn mask_image(mask: &ReachMask) -> (Image, u32) {
    use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
    let (data, levels) = mask_with_mips(&mask.bytes, mask.width, mask.height);
    let mut image = Image::default();
    image.texture_descriptor.size = Extent3d {
        width: mask.width.max(1),
        height: mask.height.max(1),
        depth_or_array_layers: 1,
    };
    image.texture_descriptor.dimension = TextureDimension::D2;
    image.texture_descriptor.format = TextureFormat::Rgba8Unorm;
    image.texture_descriptor.mip_level_count = levels;
    image.data_order = TextureDataOrder::LayerMajor;
    image.data = Some(data);
    image.asset_usage = RenderAssetUsages::RENDER_WORLD;
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::ClampToEdge,
        address_mode_v: ImageAddressMode::ClampToEdge,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        mipmap_filter: ImageFilterMode::Linear,
        ..default()
    });
    (image, levels)
}

/// Grid mesh laid on the relief over the mask frame (`_build_mesh`): step at least
/// [`MIN_STEP_PX`], at most [`MAX_QUADS`] quads per side. Returns the mesh and its quads.
pub fn build_mesh(origin: Vec2, size: Vec2, ground: &impl Ground) -> (Mesh, usize) {
    let step = MIN_STEP_PX.max(size.x.max(size.y) / MAX_QUADS);
    let nx = ((size.x / step).ceil() as usize).max(1);
    let nz = ((size.y / step).ceil() as usize).max(1);
    let end = origin + size;
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity((nx + 1) * (nz + 1));
    for j in 0..=nz {
        let z = (origin.y + j as f32 * step).min(end.y);
        for i in 0..=nx {
            let x = (origin.x + i as f32 * step).min(end.x);
            positions.push([x, ground.height(x, z) + LIFT, z]);
        }
    }
    let mut indices: Vec<u32> = Vec::with_capacity(nx * nz * 6);
    let row = (nx + 1) as u32;
    for j in 0..nz as u32 {
        for i in 0..nx as u32 {
            let a = j * row + i;
            let b = a + row;
            indices.extend_from_slice(&[a, a + 1, b + 1, a, b + 1, b]);
        }
    }
    let count = positions.len();
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; count]);
    mesh.insert_indices(Indices::U32(indices));
    (mesh, nx * nz)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::armies::projection::HeightSampler;

    #[test]
    fn grid_covers_the_frame_on_the_ground() {
        let ground = HeightSampler::new(|x, _| x * 0.01);
        let (mesh, quads) = build_mesh(Vec2::new(100.0, 100.0), Vec2::new(40.0, 20.0), &ground);
        assert_eq!(quads, 10 * 5);
        assert_eq!(mesh.count_vertices(), 11 * 6);
        assert_eq!(mesh.indices().unwrap().len(), quads * 6);
    }

    #[test]
    fn huge_frames_are_capped() {
        let (_, quads) = build_mesh(
            Vec2::ZERO,
            Vec2::new(4000.0, 4000.0),
            &HeightSampler::flat(),
        );
        assert!(quads <= 97 * 97);
    }

    #[test]
    fn mips_go_down_to_one_texel() {
        let rgb: Vec<u8> = (0..5 * 3 * 3).map(|i| i as u8).collect();
        let (data, levels) = mask_with_mips(&rgb, 5, 3);
        // 5x3 + 2x1 + 1x1 texels of 4 bytes.
        assert_eq!(levels, 3);
        assert_eq!(data.len(), (15 + 2 + 1) * 4);
    }

    #[test]
    fn full_mask_stays_full_in_the_mips() {
        let rgb = vec![255u8; 4 * 4 * 3];
        let (data, levels) = mask_with_mips(&rgb, 4, 4);
        assert_eq!(levels, 3);
        assert!(data.iter().all(|b| *b == 255));
    }
}
