//! Mip chains for the textures of glTF models.
//!
//! Bevy's glTF loader hands over 2048 px textures with a single mip level: seen from the
//! campaign distances (a rock or a cart a few dozen pixels wide) the sampler then reads one
//! texel in a hundred and the model shimmers as salt-and-pepper noise (rocks "tramés", the cart
//! and its ox "criblés de trous"). Every `StandardMaterial` that appears gets the missing mip
//! levels (box filter on the CPU, a couple of images per frame) and a trilinear anisotropic
//! sampler. Images that already carry mips (the battle atlases) are left alone.

use std::collections::VecDeque;

use bevy::image::{ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;

use crate::agent::dump::DumpAppExt;
use bevy::render::render_resource::{TextureDimension, TextureFormat};

/// Images mipped per frame (a 2048 px chain is about 20 ms in the dev profile).
const PER_FRAME: usize = 2;
/// Frames an image may stay unloaded in the queue before it is dropped.
const GIVE_UP_FRAMES: u32 = 1800;
/// Anisotropy of the mipped samplers.
const ANISOTROPY: u16 = 8;

pub struct ModelMipsPlugin;

impl Plugin for ModelMipsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MipQueue>()
            .register_dump(dump)
            .add_systems(Update, (queue_material_images, generate_queued_mips).chain());
    }
}

#[derive(Resource, Default)]
struct MipQueue {
    waiting: VecDeque<(AssetId<Image>, u32)>,
    seen: std::collections::HashSet<AssetId<Image>>,
    /// Images that got a chain (dump probe).
    done: usize,
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

fn generate_queued_mips(mut queue: ResMut<MipQueue>, mut images: ResMut<Assets<Image>>) {
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
        budget -= 1;
        if let Some(mut image) = images.get_mut(id)
            && add_mips(&mut image)
        {
            queue.done += 1;
        }
    }
}

fn dump(world: &World) -> (String, serde_json::Value) {
    let q = world.resource::<MipQueue>();
    (
        "model_mips".into(),
        serde_json::json!({"mipped": q.done, "waiting": q.waiting.len(), "seen": q.seen.len()}),
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
    let mut descriptor = match &image.sampler {
        ImageSampler::Descriptor(d) => d.clone(),
        ImageSampler::Default => ImageSamplerDescriptor::linear(),
    };
    descriptor.mipmap_filter = bevy::image::ImageFilterMode::Linear;
    descriptor.min_filter = bevy::image::ImageFilterMode::Linear;
    descriptor.mag_filter = bevy::image::ImageFilterMode::Linear;
    descriptor.anisotropy_clamp = ANISOTROPY;
    image.sampler = ImageSampler::Descriptor(descriptor);
    true
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
    fn levels_average_their_texels() {
        let mut base = vec![0u8; 2 * 2 * 4];
        base[0] = 255; // one bright texel of four
        let chain = rgba8_mip_chain(&base, 2, 2);
        assert_eq!(chain[16], 64);
    }
}
