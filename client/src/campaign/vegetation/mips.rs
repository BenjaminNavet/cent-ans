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
                let levels = level_count(w / super::render::ATLAS_VIEWS);
                let slot: Slot = Arc::default();
                let out = slot.clone();
                std::thread::Builder::new()
                    .name("vegetation-mips".into())
                    .spawn(move || {
                        let built = chain(&data, w as usize, h as usize, levels, weighted);
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
    fn chain_has_the_expected_size() {
        let base = vec![255u8; 16 * 8 * 4];
        let out = chain(&base, 16, 8, 3, false);
        assert_eq!(out.len(), (16 * 8 + 8 * 4 + 4 * 2) * 4);
    }
}
