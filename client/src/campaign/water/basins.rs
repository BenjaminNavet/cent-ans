//! Sea basin weight texture (port of `SeaBasins.texture`): one channel per basin (R, G, B, A in
//! file order), polygons filled on a coarse grid then blurred so limits fade.

use bevy::prelude::*;

use super::data::{MAX_BASINS, SeaBasins};
use super::polygon;

/// RGBA8 grid `(width, height, data)` of basin weights for a map of `map_size` pixels.
pub fn weight_grid(basins: &SeaBasins, map_size: Vec2) -> (u32, u32, Vec<u8>) {
    let cell = basins.cell_px.max(1) as f32;
    let width = (map_size.x / cell).ceil().max(1.0) as u32;
    let height = (map_size.y / cell).ceil().max(1.0) as u32;
    let mut data = vec![0u8; (width * height * 4) as usize];
    for (channel, basin) in basins.basins.iter().take(MAX_BASINS).enumerate() {
        let poly: Vec<Vec2> = basin
            .polygon_px
            .iter()
            .map(|p| Vec2::new(p[0], p[1]))
            .collect();
        if poly.len() < 3 {
            continue;
        }
        let (min, max) = poly.iter().fold((Vec2::MAX, Vec2::MIN), |(lo, hi), p| {
            (lo.min(*p), hi.max(*p))
        });
        let x0 = ((min.x / cell).floor().max(0.0)) as u32;
        let x1 = (((max.x / cell).ceil()) as u32).min(width);
        let y0 = ((min.y / cell).floor().max(0.0)) as u32;
        let y1 = (((max.y / cell).ceil()) as u32).min(height);
        for y in y0..y1 {
            for x in x0..x1 {
                let centre = Vec2::new((x as f32 + 0.5) * cell, (y as f32 + 0.5) * cell);
                if polygon::contains(&poly, centre) {
                    data[((y * width + x) * 4) as usize + channel] = 255;
                }
            }
        }
    }
    let radius = (basins.blur_cells / 2) as usize;
    if radius > 0 {
        for _ in 0..2 {
            box_blur(&mut data, width as usize, height as usize, radius);
        }
    }
    (width, height, data)
}

/// Separable box blur of an RGBA8 image, edges clamped.
fn box_blur(data: &mut [u8], width: usize, height: usize, radius: usize) {
    let mut tmp = data.to_vec();
    let span = (2 * radius + 1) as u32;
    for y in 0..height {
        for c in 0..4 {
            for x in 0..width {
                let mut sum = 0u32;
                for k in 0..=2 * radius {
                    let sx = (x + k).saturating_sub(radius).min(width - 1);
                    sum += data[(y * width + sx) * 4 + c] as u32;
                }
                tmp[(y * width + x) * 4 + c] = (sum / span) as u8;
            }
        }
    }
    for x in 0..width {
        for c in 0..4 {
            for y in 0..height {
                let mut sum = 0u32;
                for k in 0..=2 * radius {
                    let sy = (y + k).saturating_sub(radius).min(height - 1);
                    sum += tmp[(sy * width + x) * 4 + c] as u32;
                }
                data[(y * width + x) * 4 + c] = (sum / span) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::water::data::{Basin, BasinLook};

    fn basins(blur: u32) -> SeaBasins {
        SeaBasins {
            cell_px: 10,
            blur_cells: blur,
            default: BasinLook::default(),
            basins: vec![Basin {
                id: "a".into(),
                look: BasinLook::default(),
                polygon_px: vec![[0.0, 0.0], [50.0, 0.0], [50.0, 100.0], [0.0, 100.0]],
            }],
        }
    }

    #[test]
    fn fills_first_channel_only_inside_polygon() {
        let (w, h, data) = weight_grid(&basins(1), Vec2::new(100.0, 100.0));
        assert_eq!((w, h), (10, 10));
        assert_eq!(data[0], 255); // cell (0,0), channel R
        assert_eq!(data[1], 0);
        assert_eq!(data[(9 * 4) as usize], 0); // cell (9,0) outside
    }

    #[test]
    fn blur_softens_the_edge() {
        let (_, _, data) = weight_grid(&basins(4), Vec2::new(100.0, 100.0));
        let edge = data[(5 * 4) as usize];
        assert!(edge > 0 && edge < 255, "edge value {edge}");
    }

    #[test]
    fn no_basin_means_empty_grid() {
        let none = SeaBasins::default();
        let (_, _, data) = weight_grid(&none, Vec2::new(64.0, 64.0));
        assert!(data.iter().all(|&b| b == 0));
    }
}
