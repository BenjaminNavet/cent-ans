//! CAFV v1 binary tile (`cafv_tile.gd`, `docs/geo.md` § « Hydrographie fine »): lines of the fine
//! river network (layer 1) or of the draped roads (layer 2) of one E2 tile (64 world units).
//!
//! Little-endian header `4s H H H H I I I I` (28 bytes: magic, version, layer, level, pad, col,
//! row, lines, points), then `u32 x 4` per line (feature, first point, point count, flags), then
//! the `x`, `y`, `z`, `w` arrays as `f32 x n`. `z` = water level or road surface (m, not
//! exaggerated), `w` = width (m). Read-only once parsed: safe to share between threads.

pub const MAGIC: &[u8; 4] = b"CAFV";
pub const HEADER_BYTES: usize = 28;
pub const LAYER_RIVERS: u16 = 1;
pub const LAYER_ROADS: u16 = 2;
/// Side of an E2 tile in world units.
pub const TILE_UNITS: f32 = 64.0;

pub const FLAG_DIVAGATING: u32 = 1;
pub const FLAG_TIDAL: u32 = 2;
pub const FLAG_INTERMITTENT: u32 = 4;
pub const FLAG_RECTIFIED: u32 = 8;
pub const FLAG_COARSE: u32 = 16;
/// River: marsh; road: causeway through a wetland.
pub const FLAG_WETLAND: u32 = 32;
pub const FLAG_MAIN_ROAD: u32 = 64;
pub const FLAG_COMPUTED_ROAD: u32 = 128;
pub const ORDER_SHIFT: u32 = 24;
pub const SOURCE_SHIFT: u32 = 28;

#[derive(Debug, Clone, Default)]
pub struct CafvTile {
    pub layer: u16,
    pub level: u16,
    pub col: i32,
    pub row: i32,
    pub line_feature: Vec<u32>,
    pub line_start: Vec<u32>,
    pub line_count: Vec<u32>,
    pub line_flags: Vec<u32>,
    /// Display rank: max(Strahler order, order equivalent to the widest point) for rivers, 9 for
    /// roads (the computed order underestimates the large rivers: Seine at Rouen order 4, 1 km).
    pub line_rank: Vec<u32>,
    pub x: Vec<f32>,
    pub y: Vec<f32>,
    pub z: Vec<f32>,
    pub w: Vec<f32>,
}

/// Strahler order equivalent to a width (`river_widths.json` -> `strahler_width_m`).
pub fn rank_of_width(width_m: f32) -> u32 {
    match width_m {
        w if w >= 220.0 => 9,
        w if w >= 120.0 => 8,
        w if w >= 60.0 => 7,
        w if w >= 30.0 => 6,
        w if w >= 16.0 => 5,
        w if w >= 9.0 => 4,
        w if w >= 5.0 => 3,
        _ => 2,
    }
}

pub fn order_of(flags: u32) -> u32 {
    (flags >> ORDER_SHIFT) & 0xF
}

impl CafvTile {
    pub fn lines(&self) -> usize {
        self.line_start.len()
    }

    pub fn points(&self) -> usize {
        self.x.len()
    }

    /// Point range of line `i`, clamped to the point arrays.
    pub fn range(&self, i: usize) -> std::ops::Range<usize> {
        let s = self.line_start[i] as usize;
        let e = (s + self.line_count[i] as usize).min(self.points());
        s.min(e)..e
    }

    /// Decodes a tile; `None` for an unexpected format. `offset_tiles`: origin offset of the cache
    /// in E2 tiles (`root_origin_tiles` x 4, ADR 0115) added to the header and to the points.
    pub fn parse(bytes: &[u8], offset_tiles: (i32, i32)) -> Option<CafvTile> {
        if bytes.len() < HEADER_BYTES || &bytes[0..4] != MAGIC {
            return None;
        }
        let u16_at = |o: usize| u16::from_le_bytes([bytes[o], bytes[o + 1]]);
        let u32_at =
            |o: usize| u32::from_le_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]]);
        if u16_at(4) != 1 {
            return None;
        }
        let n_lines = u32_at(20) as usize;
        let n_points = u32_at(24) as usize;
        let expected = HEADER_BYTES + n_lines * 16 + n_points * 16;
        if bytes.len() < expected {
            return None;
        }
        let mut tile = CafvTile {
            layer: u16_at(6),
            level: u16_at(8),
            col: u32_at(12) as i32 + offset_tiles.0,
            row: u32_at(16) as i32 + offset_tiles.1,
            ..Default::default()
        };
        for i in 0..n_lines {
            let o = HEADER_BYTES + i * 16;
            tile.line_feature.push(u32_at(o));
            tile.line_start.push(u32_at(o + 4));
            tile.line_count.push(u32_at(o + 8));
            tile.line_flags.push(u32_at(o + 12));
        }
        let base = HEADER_BYTES + n_lines * 16;
        let span = n_points * 4;
        let floats = |start: usize| -> Vec<f32> {
            bytes[start..start + span]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|c| f32::from_le_bytes(*c))
                .collect()
        };
        tile.x = floats(base);
        tile.y = floats(base + span);
        tile.z = floats(base + span * 2);
        tile.w = floats(base + span * 3);
        if offset_tiles != (0, 0) {
            let dx = offset_tiles.0 as f32 * TILE_UNITS;
            let dy = offset_tiles.1 as f32 * TILE_UNITS;
            tile.x.iter_mut().for_each(|v| *v += dx);
            tile.y.iter_mut().for_each(|v| *v += dy);
        }
        tile.line_rank = (0..n_lines)
            .map(|i| {
                if tile.layer != LAYER_RIVERS {
                    return 9;
                }
                let widest = tile.range(i).map(|k| tile.w[k]).fold(0.0, f32::max);
                order_of(tile.line_flags[i]).max(rank_of_width(widest))
            })
            .collect();
        Some(tile)
    }

    /// Encodes a tile (tests and tools): inverse of [`CafvTile::parse`] without offset.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        for v in [1u16, self.layer, self.level, 0] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        for v in [
            self.col as u32,
            self.row as u32,
            self.lines() as u32,
            self.points() as u32,
        ] {
            out.extend_from_slice(&v.to_le_bytes());
        }
        for i in 0..self.lines() {
            for v in [
                self.line_feature[i],
                self.line_start[i],
                self.line_count[i],
                self.line_flags[i],
            ] {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        for arr in [&self.x, &self.y, &self.z, &self.w] {
            for v in arr.iter() {
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        out
    }

    /// One-line tile (tests): points `(x, y, z, w)` with the given flags.
    pub fn single_line(layer: u16, col: i32, row: i32, flags: u32, pts: &[[f32; 4]]) -> CafvTile {
        let mut tile = CafvTile {
            layer,
            level: 2,
            col,
            row,
            line_feature: vec![0],
            line_start: vec![0],
            line_count: vec![pts.len() as u32],
            line_flags: vec![flags],
            ..Default::default()
        };
        for p in pts {
            tile.x.push(p[0]);
            tile.y.push(p[1]);
            tile.z.push(p[2]);
            tile.w.push(p[3]);
        }
        CafvTile::parse(&tile.encode(), (0, 0)).unwrap_or(tile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_rank() {
        let flags = 4 << ORDER_SHIFT | FLAG_TIDAL;
        let tile = CafvTile::single_line(
            LAYER_RIVERS,
            3,
            5,
            flags,
            &[[1.0, 2.0, 10.0, 8.0], [2.0, 3.0, 9.0, 130.0]],
        );
        assert_eq!(
            (tile.col, tile.row, tile.lines(), tile.points()),
            (3, 5, 1, 2)
        );
        // Width 130 m -> rank 8, above the computed order 4.
        assert_eq!(tile.line_rank[0], 8);
        assert_eq!(tile.line_flags[0], flags);
        let shifted = CafvTile::parse(&tile.encode(), (4, 0)).unwrap();
        assert_eq!(shifted.col, 7);
        assert!((shifted.x[0] - (1.0 + 256.0)).abs() < 1e-4);
    }

    #[test]
    fn rejects_bad_bytes() {
        assert!(CafvTile::parse(b"NOPE", (0, 0)).is_none());
        let mut bytes = CafvTile::single_line(LAYER_ROADS, 0, 0, 0, &[[0.0; 4]; 2]).encode();
        bytes.truncate(bytes.len() - 1);
        assert!(CafvTile::parse(&bytes, (0, 0)).is_none());
    }

    #[test]
    fn repository_tile_parses_when_the_cache_exists() {
        let Ok(root) = crate::paths::find_root() else {
            return;
        };
        let dir = std::path::Path::new(&std::env::var("HOME").unwrap_or_default())
            .join("dev/cent-ans-raw/pyramid-reelle/hydro_fine/E2");
        let path = dir.join("57_0.bin");
        let _ = root;
        let Ok(bytes) = std::fs::read(&path) else {
            return;
        };
        let tile = CafvTile::parse(&bytes, (0, 0)).expect("CAFV tile");
        assert_eq!(tile.layer, LAYER_RIVERS);
        assert_eq!((tile.col, tile.row), (57, 0));
        assert!(tile.points() > 0);
    }
}
