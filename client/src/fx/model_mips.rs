//! Mip chains for the textures of glTF models.
//!
//! Bevy's glTF loader hands over 2048 px textures with a single mip level: seen from the
//! campaign distances (a rock or a cart a few dozen pixels wide) the sampler then reads one
//! texel in a hundred and the model shimmers as salt-and-pepper noise (rocks "tramés", the cart
//! and its ox "criblés de trous"). Every `StandardMaterial` that appears gets the missing mip
//! levels (box filter on the CPU, a couple of images per frame) and a trilinear anisotropic
//! sampler. Images that already carry mips (the battle atlases) are left alone.

use std::collections::VecDeque;

use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on, futures_lite::future};

use crate::agent::dump::DumpAppExt;
use bevy::render::render_resource::{TextureDimension, TextureFormat};

/// Images mipped per frame (a 2048 px chain is about 20 ms in the dev profile).
const PER_FRAME: usize = 2;
/// Frames an image may stay unloaded in the queue before it is dropped.
const GIVE_UP_FRAMES: u32 = 1800;
/// Anisotropy of the mipped samplers.
const ANISOTROPY: u16 = 8;
/// Asset path prefix of the generated campaign models (ADR 0211): their albedo is baked and opaque, so
/// it is block-compressed (BC1) and kept on the GPU only (ADR 0776); 2048 px RGBA8 + mips was 21 MiB
/// on the GPU and 21 MiB more in RAM for each of ~66 models.
const COMPRESSED_PREFIXES: [&str; 2] = ["models/dn/", "models/props_ga/"];
/// Largest side kept for a compressed model texture (2048 px baked albedo seen at campaign
/// distances; 1024 px BC1 + mips is 0.7 MiB instead of 2.8 MiB).
const MAX_MODEL_SIDE: usize = 1024;
/// Smallest side (texels) worth compressing.
const MIN_COMPRESS_SIDE: usize = 256;

pub struct ModelMipsPlugin;

impl Plugin for ModelMipsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MipQueue>()
            .register_dump(dump)
            .add_systems(
                Update,
                (
                    queue_material_images,
                    generate_queued_mips,
                    collect_compressed,
                )
                    .chain(),
            );
    }
}

#[derive(Resource, Default)]
pub struct MipQueue {
    waiting: VecDeque<(AssetId<Image>, u32)>,
    seen: std::collections::HashSet<AssetId<Image>>,
    /// Images that got a chain (dump probe).
    done: usize,
    /// Background BC1 encodes: image id and the finished chain (`None`: not compressible).
    encoding: Vec<(AssetId<Image>, Task<Encoded>)>,
    /// Images stored as BC1 and dropped from RAM (dump probe).
    compressed: usize,
    /// Swapped images whose materials still bind the RGBA8 texture, with the frames left before
    /// the materials are flagged (the new `GpuImage` must be prepared first).
    stale_materials: Vec<(AssetId<Image>, u8)>,
    /// Materials flagged after a swap (dump probe).
    materials_refreshed: usize,
}

/// Result of a background encode.
enum Encoded {
    /// Block-compressed chain and the (possibly reduced) size of level 0.
    Bc1(Vec<u8>, UVec2),
    /// Translucent texels: the untouched RGBA8 pixels and their size, handed back.
    Rgba(Vec<u8>, UVec2),
}

fn spawn_encode(data: Vec<u8>, w: usize, h: usize) -> Task<Encoded> {
    AsyncComputeTaskPool::get().spawn(async move {
        match bc1_chain(&data, w, h, MAX_MODEL_SIDE) {
            Some((chain, size)) => Encoded::Bc1(chain, size),
            None => Encoded::Rgba(data, UVec2::new(w as u32, h as u32)),
        }
    })
}

/// Whether the asset path is one of the models whose albedo is compressed.
fn compressed_path(server: &AssetServer, id: AssetId<Image>) -> bool {
    server.get_path(id).is_some_and(|p| {
        COMPRESSED_PREFIXES
            .iter()
            .any(|prefix| p.path().starts_with(prefix))
    })
}

impl MipQueue {
    /// Asks for the mip levels of an image that is not part of a glTF material.
    pub fn request(&mut self, id: AssetId<Image>) {
        if self.seen.insert(id) {
            self.waiting.push_back((id, 0));
        }
    }
}

fn queue_material_images(
    mut events: MessageReader<AssetEvent<StandardMaterial>>,
    materials: Res<Assets<StandardMaterial>>,
    mut queue: ResMut<MipQueue>,
) {
    for event in events.read() {
        let AssetEvent::Added { id } = event else {
            continue;
        };
        let Some(m) = materials.get(*id) else {
            continue;
        };
        for tex in [
            &m.base_color_texture,
            &m.metallic_roughness_texture,
            &m.normal_map_texture,
            &m.occlusion_texture,
            &m.emissive_texture,
        ]
        .into_iter()
        .flatten()
        {
            if queue.seen.insert(tex.id()) {
                queue.waiting.push_back((tex.id(), 0));
            }
        }
    }
}

fn generate_queued_mips(
    mut queue: ResMut<MipQueue>,
    mut images: ResMut<Assets<Image>>,
    server: Res<AssetServer>,
) {
    let mut budget = PER_FRAME;
    for _ in 0..queue.waiting.len() {
        if budget == 0 {
            break;
        }
        let Some((id, waited)) = queue.waiting.pop_front() else {
            break;
        };
        // `get` first: a mutable access flags the asset as modified (GPU re-upload).
        let Some(image) = images.get(id) else {
            if waited < GIVE_UP_FRAMES {
                queue.waiting.push_back((id, waited + 1));
            }
            continue;
        };
        if !needs_mips(image) {
            continue;
        }
        // Opaque baked albedo of the campaign models: chain + BC1 on a background thread. The
        // pixels are moved, not cloned (16 MiB each); the uploaded RGBA8 texture is kept meanwhile
        // (an image without data reuses its previous GPU texture).
        if compressible(image) && compressed_path(&server, id) {
            let (w, h) = (image.width() as usize, image.height() as usize);
            if let Some(data) = images.get_mut(id).and_then(|mut image| image.data.take()) {
                queue.encoding.push((id, spawn_encode(data, w, h)));
            }
            continue;
        }
        budget -= 1;
        let started = std::time::Instant::now();
        if let Some(mut image) = images.get_mut(id)
            && add_mips(&mut image)
        {
            queue.done += 1;
            crate::agent::texture_census::record(
                format!("model_mips {}x{}", image.width(), image.height()),
                started,
                image.data.as_ref().map_or(0, Vec::len),
            );
        }
    }
}

/// Installs the finished BC1 chains: the image becomes `Bc1RgbaUnormSrgb` with its full mip chain and
/// keeps no CPU copy (`RENDER_WORLD` only: Bevy drops `data` once uploaded). A chain that could not be
/// built (translucent texels) falls back to the plain RGBA8 chain.
///
/// The material bind groups built on the RGBA8 texture are not rebuilt by the image swap alone:
/// the materials that sample a swapped image are flagged as changed a few frames later, once the
/// new `GpuImage` exists. (A 4 x 4 grey placeholder installed before the first upload left the
/// campaign models uniformly grey for good, 28/10, ADR 0776 amendment.)
fn collect_compressed(
    mut queue: ResMut<MipQueue>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    refresh_stale_materials(&mut queue, &mut materials);
    let mut pending = Vec::new();
    for (id, mut task) in std::mem::take(&mut queue.encoding) {
        let Some(result) = block_on(future::poll_once(&mut task)) else {
            pending.push((id, task));
            continue;
        };
        let Some(mut image) = images.get_mut(id) else {
            continue;
        };
        queue.stale_materials.push((id, MATERIAL_REFRESH_FRAMES));
        match result {
            Encoded::Bc1(chain, size) => {
                let (w, h) = (size.x, size.y);
                image.texture_descriptor.size.width = w;
                image.texture_descriptor.size.height = h;
                image.texture_descriptor.format = TextureFormat::Bc1RgbaUnormSrgb;
                image.texture_descriptor.mip_level_count = mip_levels(w, h);
                image.data = Some(chain);
                set_trilinear(&mut image);
                image.asset_usage = RenderAssetUsages::RENDER_WORLD;
                queue.compressed += 1;
            }
            Encoded::Rgba(data, size) => {
                image.texture_descriptor.size.width = size.x;
                image.texture_descriptor.size.height = size.y;
                image.data = Some(data);
                if add_mips(&mut image) {
                    queue.done += 1;
                    // Translucent albedo stays RGBA8 but only on the GPU.
                    image.asset_usage = RenderAssetUsages::RENDER_WORLD;
                }
            }
        }
    }
    queue.encoding = pending;
}

/// Frames between an image swap and the refresh of the materials that sample it.
const MATERIAL_REFRESH_FRAMES: u8 = 2;

/// Flags the materials of the images swapped `MATERIAL_REFRESH_FRAMES` ago as modified, so that
/// their bind groups are rebuilt on the new texture.
fn refresh_stale_materials(queue: &mut MipQueue, materials: &mut Assets<StandardMaterial>) {
    if queue.stale_materials.is_empty() {
        return;
    }
    let mut due = std::collections::HashSet::new();
    queue.stale_materials.retain_mut(|(id, frames)| {
        if *frames == 0 {
            due.insert(*id);
            false
        } else {
            *frames -= 1;
            true
        }
    });
    if due.is_empty() {
        return;
    }
    let stale: Vec<_> = materials
        .iter()
        .filter(|(_, m)| {
            m.base_color_texture
                .as_ref()
                .is_some_and(|t| due.contains(&t.id()))
        })
        .map(|(id, _)| id)
        .collect();
    for id in stale {
        // A mutable access flags the material as modified: its bind group is rebuilt.
        if materials.get_mut(id).is_some() {
            queue.materials_refreshed += 1;
        }
    }
}

/// Opaque sRGB RGBA8 texture whose sides are multiples of the BC block.
fn compressible(image: &Image) -> bool {
    let (w, h) = (image.width() as usize, image.height() as usize);
    needs_mips(image)
        && image.texture_descriptor.format == TextureFormat::Rgba8UnormSrgb
        && w % 4 == 0
        && h % 4 == 0
        && w.min(h) >= MIN_COMPRESS_SIDE
        && image.data.as_ref().is_some_and(|d| d.len() == w * h * 4)
}

/// Full BC1 mip chain of an RGBA8 image (level 0 first, reduced by box filter until its larger side
/// is at most `max_side`), `None` when a texel is not opaque.
fn bc1_chain(rgba: &[u8], w: usize, h: usize, max_side: usize) -> Option<(Vec<u8>, UVec2)> {
    use crate::campaign::terrain::bc1;
    if rgba.as_chunks::<4>().0.iter().any(|p| p[3] != 255) {
        return None;
    }
    let mut out = Vec::new();
    let mut level = rgba.to_vec();
    let (mut lw, mut lh) = (w, h);
    while lw.max(lh) > max_side && lw % 8 == 0 && lh % 8 == 0 {
        let (nw, nh) = (lw / 2, lh / 2);
        level = box_down(&level, lw, lh, nw, nh);
        (lw, lh) = (nw, nh);
    }
    let size = UVec2::new(lw as u32, lh as u32);
    loop {
        out.extend(bc1::encode_level(&level, lw, lh));
        if lw <= 1 && lh <= 1 {
            break;
        }
        let (nw, nh) = ((lw / 2).max(1), (lh / 2).max(1));
        level = box_down(&level, lw, lh, nw, nh);
        (lw, lh) = (nw, nh);
    }
    Some((out, size))
}

/// One box-filtered mip level.
fn box_down(prev: &[u8], pw: usize, ph: usize, nw: usize, nh: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(nw * nh * 4);
    for y in 0..nh {
        for x in 0..nw {
            let (x0, y0) = ((x * 2).min(pw - 1), (y * 2).min(ph - 1));
            let (x1, y1) = ((x * 2 + 1).min(pw - 1), (y * 2 + 1).min(ph - 1));
            for c in 0..4 {
                let at = |xx: usize, yy: usize| u32::from(prev[(yy * pw + xx) * 4 + c]);
                out.push(((at(x0, y0) + at(x1, y0) + at(x0, y1) + at(x1, y1) + 2) / 4) as u8);
            }
        }
    }
    out
}

fn dump(world: &World) -> (String, serde_json::Value) {
    let q = world.resource::<MipQueue>();
    (
        "model_mips".into(),
        serde_json::json!({"mipped": q.done, "waiting": q.waiting.len(), "seen": q.seen.len(),
            "bc1_compressed": q.compressed, "bc1_encoding": q.encoding.len(),
            "materials_refreshed": q.materials_refreshed}),
    )
}

fn needs_mips(image: &Image) -> bool {
    image.texture_descriptor.mip_level_count == 1
        && image.texture_descriptor.dimension == TextureDimension::D2
        && image.texture_descriptor.size.depth_or_array_layers == 1
        && matches!(
            image.texture_descriptor.format,
            TextureFormat::Rgba8UnormSrgb | TextureFormat::Rgba8Unorm
        )
        && image.width().max(image.height()) >= 4
        && image.data.is_some()
}

/// Number of levels of a `w` x `h` texture down to 1x1.
pub fn mip_levels(w: u32, h: u32) -> u32 {
    32 - w.max(h).max(1).leading_zeros()
}

/// RGBA8 mip chain (box filter), all levels concatenated, level 0 first.
pub fn rgba8_mip_chain(base: &[u8], w: usize, h: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(base.len() * 4 / 3 + 16);
    out.extend_from_slice(base);
    let (mut pw, mut ph) = (w, h);
    let mut start = 0;
    while pw > 1 || ph > 1 {
        let (nw, nh) = ((pw / 2).max(1), (ph / 2).max(1));
        let prev = out[start..start + pw * ph * 4].to_vec();
        start += pw * ph * 4;
        for y in 0..nh {
            for x in 0..nw {
                let (x0, y0) = ((x * 2).min(pw - 1), (y * 2).min(ph - 1));
                let (x1, y1) = ((x * 2 + 1).min(pw - 1), (y * 2 + 1).min(ph - 1));
                for c in 0..4 {
                    let at = |xx: usize, yy: usize| u32::from(prev[(yy * pw + xx) * 4 + c]);
                    out.push(((at(x0, y0) + at(x1, y0) + at(x0, y1) + at(x1, y1) + 2) / 4) as u8);
                }
            }
        }
        (pw, ph) = (nw, nh);
    }
    out
}

fn add_mips(image: &mut Image) -> bool {
    let (w, h) = (image.width() as usize, image.height() as usize);
    let Some(data) = image.data.take() else {
        return false;
    };
    if data.len() != w * h * 4 {
        image.data = Some(data);
        return false;
    }
    image.data = Some(rgba8_mip_chain(&data, w, h));
    image.texture_descriptor.mip_level_count = mip_levels(w as u32, h as u32);
    set_trilinear(image);
    true
}

fn set_trilinear(image: &mut Image) {
    let mut descriptor = match &image.sampler {
        ImageSampler::Descriptor(d) => d.clone(),
        ImageSampler::Default => ImageSamplerDescriptor::linear(),
    };
    descriptor.mipmap_filter = bevy::image::ImageFilterMode::Linear;
    descriptor.min_filter = bevy::image::ImageFilterMode::Linear;
    descriptor.mag_filter = bevy::image::ImageFilterMode::Linear;
    descriptor.anisotropy_clamp = ANISOTROPY;
    image.sampler = ImageSampler::Descriptor(descriptor);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_has_every_level_down_to_one_pixel() {
        let base = vec![200u8; 8 * 4 * 4];
        let chain = rgba8_mip_chain(&base, 8, 4);
        // 8x4 + 4x2 + 2x1 + 1x1
        assert_eq!(chain.len(), (32 + 8 + 2 + 1) * 4);
        assert_eq!(mip_levels(8, 4), 4);
        assert!(chain.iter().all(|&b| b == 200));
    }

    #[test]
    fn bc1_chain_covers_every_level_and_rejects_translucency() {
        let opaque = vec![255u8; 16 * 16 * 4];
        let (chain, size) = bc1_chain(&opaque, 16, 16, 64).expect("opaque");
        assert_eq!(size, UVec2::new(16, 16));
        // 16x16 (16 blocks) + 8x8 (4) + 4x4 (1) + 2x2 (1) + 1x1 (1), 8 bytes each
        assert_eq!(chain.len(), (16 + 4 + 1 + 1 + 1) * 8);
        let mut translucent = opaque;
        translucent[3] = 10;
        assert!(bc1_chain(&translucent, 16, 16, 64).is_none());
        let (reduced, size) = bc1_chain(&vec![255u8; 32 * 32 * 4], 32, 32, 16).expect("opaque");
        assert_eq!(size, UVec2::new(16, 16));
        assert_eq!(reduced.len(), (16 + 4 + 1 + 1 + 1) * 8);
    }

    #[test]
    fn levels_average_their_texels() {
        let mut base = vec![0u8; 2 * 2 * 4];
        base[0] = 255; // one bright texel of four
        let chain = rgba8_mip_chain(&base, 2, 2);
        assert_eq!(chain[16], 64);
    }
}
