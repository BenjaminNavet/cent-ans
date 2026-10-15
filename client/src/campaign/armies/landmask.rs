//! Real land/sea mask for the orders (`MapData.is_land_px`): `data/map/land_mask.png` (8-bit
//! grey, 255 = land, one texel per map pixel). Without the file (tests, broken install) the
//! displayed height <= 0 stands in, as before.

use std::sync::Arc;

use bevy::prelude::*;

use crate::campaign::terrain::surface::Surface;
use crate::paths;

#[derive(Resource, Default, Clone)]
pub struct LandMask {
    width: usize,
    height: usize,
    bytes: Arc<Vec<u8>>,
}

impl LandMask {
    pub fn new(width: usize, height: usize, bytes: Vec<u8>) -> Self {
        Self {
            width,
            height,
            bytes: Arc::new(bytes),
        }
    }

    pub fn is_loaded(&self) -> bool {
        !self.bytes.is_empty()
    }

    pub fn size(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    /// Land at map pixel `(x, y)`; outside the map counts as sea.
    pub fn is_land(&self, x: f32, y: f32) -> bool {
        if x < 0.0 || y < 0.0 {
            return false;
        }
        let (ix, iy) = (x as usize, y as usize);
        ix < self.width && iy < self.height && self.bytes[iy * self.width + ix] > 127
    }
}

/// Decodes an 8-bit grey PNG.
pub fn decode(file: impl std::io::Read + std::io::Seek) -> Option<LandMask> {
    let mut reader = png::Decoder::new(std::io::BufReader::new(file))
        .read_info()
        .ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    if info.color_type != png::ColorType::Grayscale || info.bit_depth != png::BitDepth::Eight {
        warn!("armies: land_mask.png is not 8-bit grey");
        return None;
    }
    buf.truncate(info.buffer_size());
    Some(LandMask::new(
        info.width as usize,
        info.height as usize,
        buf,
    ))
}

pub fn load_land_mask(mut commands: Commands) {
    let mask = paths::find_root()
        .ok()
        .and_then(|root| std::fs::File::open(paths::data_dir(&root).join("map/land_mask.png")).ok())
        .and_then(decode)
        .unwrap_or_default();
    commands.insert_resource(mask);
}

/// Open water at `(x, z)`: the mask when loaded, else the displayed height.
pub fn is_water(mask: &LandMask, surface: Option<&Surface>, x: f32, z: f32) -> bool {
    if mask.is_loaded() {
        !mask.is_land(x, z)
    } else {
        surface.is_some_and(|s| s.heights.height_m_at(x, z) <= 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_reads_land_and_sea() {
        let mask = LandMask::new(2, 2, vec![255, 0, 0, 255]);
        assert!(mask.is_land(0.5, 0.5));
        assert!(!mask.is_land(1.5, 0.5));
        assert!(mask.is_land(1.5, 1.5));
        assert!(!mask.is_land(-1.0, 0.0));
        assert!(!mask.is_land(5.0, 0.0));
        assert!(!LandMask::default().is_loaded());
    }
}
