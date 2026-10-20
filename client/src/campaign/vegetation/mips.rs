//! Mipmaps of the impostor atlases (Godot: `Image.generate_mipmaps`).
//!
//! The atlases (8 views x N rows of square cells) load without a mip chain: far away, minified
//! impostors shimmer. The chain is built once on a thread (box filter; the colour is weighted by
//! the alpha for the albedo so the transparent background does not bleed into the crowns) and
//! stops while a cell is still 4 texels wide, so a cell never mixes with its neighbour.

use std::sync::{Arc, Mutex};

use bevy::image::{ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;

use super::VegRender;

/// Texels a cell keeps at the last level.
const MIN_CELL: u32 = 4;

/// Number of levels (level 0 included) for a cell of `cell` texels.
pub fn level_count(cell: u32) -> u32 {
    let mut levels = 1;
    let mut size = cell;
    while size / 2 >= MIN_CELL {
        size /= 2;
        levels += 1;
    }
    levels
}

/// Next level of an RGBA8 image (`w`, `h` even). `weighted`: colour weighted by alpha.
pub fn downsample(src: &[u8], w: usize, h: usize, weighted: bool) -> Vec<u8> {
    let (nw, nh) = (w / 2, h / 2);
    let mut out = vec![0u8; nw * nh * 4];
    for y in 0..nh {
        for x in 0..nw {
            let mut sum = [0f32; 4];
            let mut weight = 0f32;
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let i = ((2 * y + dy) * w + 2 * x + dx) * 4;
                let a = src[i + 3] as f32;
                let k = if weighted { a } else { 1.0 };
                for c in 0..3 {
                    sum[c] += src[i + c] as f32 * k;
                }
                sum[3] += a;
                weight += k;
            }
            let o = (y * nw + x) * 4;
            for c in 0..3 {
                out[o + c] = if weight > 0.0 {
                    (sum[c] / weight).round() as u8
                } else {
                    0
                };
            }
            out[o + 3] = (sum[3] / 4.0).round() as u8;
        }
    }
    out
}

/// Whole chain, levels concatenated (level 0 first), `levels` levels in all.
pub fn chain(base: &[u8], w: usize, h: usize, levels: u32, weighted: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(base.len() * 4 / 3 + 16);
    out.extend_from_slice(base);
    let mut current = base.to_vec();
    let (mut cw, mut ch) = (w, h);
    for _ in 1..levels {
        current = downsample(&current, cw, ch, weighted);
        cw /= 2;
        ch /= 2;
        out.extend_from_slice(&current);
    }
    out
}

/// Next level of the normal atlas: every channel (the alpha holds the baked occlusion) is
/// averaged with the coverage of the albedo as weight, so the neutral background of the cells
/// (flat normal, no occlusion) does not wash the crowns out far away. `weights` is the coverage
/// (sum of the alpha of the children / 4, as the albedo's alpha chain) of the source level.
pub fn downsample_covered(src: &[u8], weights: &[f32], w: usize, h: usize) -> (Vec<u8>, Vec<f32>) {
    let (nw, nh) = (w / 2, h / 2);
    let mut out = vec![0u8; nw * nh * 4];
    let mut out_weights = vec![0f32; nw * nh];
    for y in 0..nh {
        for x in 0..nw {
            let mut sum = [0f32; 4];
            let mut total = 0f32;
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let t = (2 * y + dy) * w + 2 * x + dx;
                let k = weights[t];
                for (c, acc) in sum.iter_mut().enumerate() {
                    *acc += src[t * 4 + c] as f32 * k;
                }
                total += k;
            }
            let o = y * nw + x;
            out_weights[o] = total / 4.0;
            for (c, acc) in sum.iter().enumerate() {
                out[o * 4 + c] = if total > 0.0 {
                    (acc / total).round() as u8
                } else {
                    // Empty cell area: the neutral background of the atlas.
                    [128, 128, 255, 255][c]
                };
            }
        }
    }
    (out, out_weights)
}

/// Whole chain of the normal atlas weighted by the coverage `alpha` (albedo alpha, level 0).
pub fn chain_covered(base: &[u8], alpha: &[u8], w: usize, h: usize, levels: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity(base.len() * 4 / 3 + 16);
    out.extend_from_slice(base);
    let mut current = base.to_vec();
    let mut weights: Vec<f32> = alpha.iter().map(|a| *a as f32).collect();
    let (mut cw, mut ch) = (w, h);
    for _ in 1..levels {
        (current, weights) = downsample_covered(&current, &weights, cw, ch);
        cw /= 2;
        ch /= 2;
        out.extend_from_slice(&current);
    }
    out
}

type Slot = Arc<Mutex<Option<Vec<u8>>>>;

enum Job {
    Idle,
    Running(Slot, u32),
    Done,
}

/// Mip generation of the two atlases (albedo, normal).
#[derive(Resource)]
pub struct AtlasMips {
    jobs: [Job; 2],
    /// Levels of the atlases once finished (diagnostics).
    pub levels: u32,
}

impl Default for AtlasMips {
    fn default() -> Self {
        AtlasMips {
            jobs: [Job::Idle, Job::Idle],
            levels: 1,
        }
    }
}

impl AtlasMips {
    pub fn finished(&self) -> usize {
        self.jobs.iter().filter(|j| matches!(j, Job::Done)).count()
    }
}

/// Starts the chain of each loaded atlas and installs it when ready.
pub(super) fn build_atlas_mips(
    render: Res<VegRender>,
    mut mips: ResMut<AtlasMips>,
    mut images: ResMut<Assets<Image>>,
) {
    let handles = [
        (render.assets.atlas.clone(), true),
        (render.assets.normal.clone(), false),
    ];
    for (n, (handle, weighted)) in handles.into_iter().enumerate() {
        let Some(handle) = handle else { continue };
        match &mips.jobs[n] {
            Job::Done => {}
            Job::Idle => {
                let Some(image) = images.get(&handle) else {
                    continue;
                };
                let (w, h) = (image.width(), image.height());
                let Some(data) = image.data.clone() else {
                    continue;
                };
                let usable = image.texture_descriptor.mip_level_count == 1
                    && data.len() == (w * h * 4) as usize
                    && w % 2 == 0;
                if !usable {
                    mips.jobs[n] = Job::Done;
                    continue;
                }
                // The normal atlas is weighted by the coverage of the albedo (its own alpha is the
                // occlusion): wait for the albedo, whose level 0 stays first in its data.
                let coverage: Option<Vec<u8>> = if weighted {
                    None
                } else {
                    let Some(albedo) = render.assets.atlas.as_ref().and_then(|a| images.get(a))
                    else {
                        continue;
                    };
                    let texels = (w * h) as usize;
                    let Some(bytes) = albedo.data.as_ref().filter(|d| {
                        albedo.width() == w && albedo.height() == h && d.len() >= texels * 4
                    }) else {
                        mips.jobs[n] = Job::Done;
                        continue;
                    };
                    Some(
                        bytes[..texels * 4]
                            .as_chunks::<4>()
                            .0
                            .iter()
                            .map(|p| p[3])
                            .collect(),
                    )
                };
                let levels = level_count(w / super::render::ATLAS_VIEWS);
                let slot: Slot = Arc::default();
                let out = slot.clone();
                std::thread::Builder::new()
                    .name("vegetation-mips".into())
                    .spawn(move || {
                        let built = match &coverage {
                            Some(alpha) => {
                                chain_covered(&data, alpha, w as usize, h as usize, levels)
                            }
                            None => chain(&data, w as usize, h as usize, levels, weighted),
                        };
                        *out.lock().expect("mip slot") = Some(built);
                    })
                    .ok();
                mips.jobs[n] = Job::Running(slot, levels);
            }
            Job::Running(slot, levels) => {
                let levels = *levels;
                let Some(data) = slot.lock().expect("mip slot").take() else {
                    continue;
                };
                if let Some(mut image) = images.get_mut(&handle) {
                    image.data = Some(data);
                    image.texture_descriptor.mip_level_count = levels;
                    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                        mag_filter: ImageFilterMode::Linear,
                        min_filter: ImageFilterMode::Linear,
                        mipmap_filter: ImageFilterMode::Linear,
                        anisotropy_clamp: 4,
                        ..default()
                    });
                    mips.levels = levels;
                }
                mips.jobs[n] = Job::Done;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_stop_at_four_texel_cells() {
        assert_eq!(level_count(256), 7);
        assert_eq!(level_count(4), 1);
        assert_eq!(level_count(8), 2);
    }

    #[test]
    fn downsample_weights_colour_by_alpha() {
        // 2x2: one opaque red texel, three transparent black ones.
        let mut src = vec![0u8; 16];
        src[0] = 200;
        src[3] = 255;
        let out = downsample(&src, 2, 2, true);
        assert_eq!(out.len(), 4);
        assert_eq!(out[0], 200);
        assert_eq!(out[3], 64);
        let plain = downsample(&src, 2, 2, false);
        assert_eq!(plain[0], 50);
    }

    #[test]
    fn covered_chain_ignores_the_empty_background() {
        // 2x2 normal texels: one crown texel (tilted normal, occlusion 200) and three background
        // texels (flat normal, no occlusion) with no coverage.
        let src = [
            200, 100, 240, 200, 128, 128, 255, 255, 128, 128, 255, 255, 128, 128, 255, 255,
        ];
        let weights = [255.0, 0.0, 0.0, 0.0];
        let (out, w) = downsample_covered(&src, &weights, 2, 2);
        assert_eq!(out, vec![200, 100, 240, 200]);
        assert_eq!(w, vec![255.0 / 4.0]);
        let (empty, _) = downsample_covered(&src, &[0.0; 4], 2, 2);
        assert_eq!(empty, vec![128, 128, 255, 255]);
    }

    #[test]
    fn chain_has_the_expected_size() {
        let base = vec![255u8; 16 * 8 * 4];
        let out = chain(&base, 16, 8, 3, false);
        assert_eq!(out.len(), (16 * 8 + 8 * 4 + 4 * 2) * 4);
    }
}
