//! CPU surface of the relief: resident pages of the pyramid over the world heightmap. Port of
//! `ReliefQuadtree.surface_height_at` / `sample_pages_m` and of the displayed height
//! (`MapData.display_height`). Used for meshing nodes, the camera ground and by other lots
//! (picking, objects placed on the ground) through [`GroundHeight`].

use std::collections::HashMap;
use std::sync::Arc;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::pyramid::{ReliefPyramid, TILE_PX, key_of, tile_units};
use super::relief_floor::HeightGrid;
use super::relief_state::ReliefState;

/// One decoded 512 x 512 page (raw 16-bit heights).
pub type Page = Arc<Vec<u16>>;

/// Bilinear altitude (m) at pixel coordinates of a page, clamped to the edge
/// (`ReliefQuadtree._bilinear`, same arithmetic).
pub fn page_bilinear_m(page: &[u16], fx: f32, fy: f32, h_min: f32, h_range: f32) -> f32 {
    let max = TILE_PX as f32 - 1.0;
    let fx = fx.clamp(0.0, max);
    let fy = fy.clamp(0.0, max);
    let i = (fx as usize).min(TILE_PX - 2);
    let j = (fy as usize).min(TILE_PX - 2);
    let (tx, ty) = (fx - i as f32, fy - j as f32);
    let o = j * TILE_PX + i;
    let (a, b) = (page[o] as f32, page[o + 1] as f32);
    let (c, d) = (page[o + TILE_PX] as f32, page[o + TILE_PX + 1] as f32);
    let top = a + (b - a) * tx;
    let v = (top + (c + (d - c) * tx - top) * ty) / 65535.0;
    h_min + v * h_range
}

/// Resident pages and the fallback heightmap.
#[derive(Resource, Clone)]
pub struct Surface {
    pub heights: Arc<HeightGrid>,
    pub pyramid: Arc<ReliefPyramid>,
    pages: HashMap<i64, Page>,
    /// Bumped at every arrival or eviction.
    pub version: u64,
}

impl Surface {
    pub fn new(heights: Arc<HeightGrid>, pyramid: Arc<ReliefPyramid>) -> Surface {
        Surface {
            heights,
            pyramid,
            pages: HashMap::new(),
            version: 0,
        }
    }

    pub fn insert_page(&mut self, key: i64, page: Page) {
        self.pages.insert(key, page);
        self.version += 1;
    }

    pub fn remove_page(&mut self, key: i64) -> bool {
        let removed = self.pages.remove(&key).is_some();
        if removed {
            self.version += 1;
        }
        removed
    }

    pub fn page(&self, key: i64) -> Option<&Page> {
        self.pages.get(&key)
    }

    pub fn page_keys(&self) -> impl Iterator<Item = i64> + '_ {
        self.pages.keys().copied()
    }

    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// Resident pages per level (E0..).
    pub fn pages_per_level(&self) -> Vec<usize> {
        let mut counts = vec![0usize; self.pyramid.max_level as usize + 1];
        for &key in self.pages.keys() {
            counts[super::pyramid::level_of_key(key) as usize] += 1;
        }
        counts
    }

    /// Altitude (m) in a given page at the map point (x, z).
    pub fn page_height_m(&self, key: i64, x: f32, z: f32) -> Option<f32> {
        let page = self.pages.get(&key)?;
        let level = super::pyramid::level_of_key(key);
        let (ox, oz) = ReliefPyramid::tile_origin(
            level,
            super::pyramid::col_of_key(key),
            super::pyramid::row_of_key(key),
        );
        let px_units = (tile_units(level) / TILE_PX as f64) as f32;
        Some(page_bilinear_m(
            page,
            (x - ox as f32) / px_units - 0.5,
            (z - oz as f32) / px_units - 0.5,
            self.pyramid.h_min_m,
            self.pyramid.h_range_m,
        ))
    }

    /// Altitude (m) of the finest resident page under (x, z); `None` when no page covers it.
    pub fn finest_page_height_m(&self, x: f32, z: f32) -> Option<f32> {
        for level in (0..=self.pyramid.max_level).rev() {
            let (col, row) = ReliefPyramid::tile_at(level, x as f64, z as f64);
            if col < 0 || row < 0 {
                return None;
            }
            let key = key_of(level as i64, col, row);
            if let Some(h) = self.page_height_m(key, x, z) {
                return Some(h);
            }
        }
        None
    }

    /// Altitude (m): finest resident page, else the world heightmap.
    pub fn height_m_at(&self, x: f32, z: f32) -> f32 {
        self.finest_page_height_m(x, z)
            .unwrap_or_else(|| self.heights.height_m_at(x, z))
    }

    /// Altitude (m) as sampled by a node built on page `fine` (`-1`: heightmap). Falls back to the
    /// heightmap when the page is not resident.
    pub fn node_height_m(&self, fine: i64, x: f32, z: f32) -> f32 {
        if fine >= 0
            && let Some(h) = self.page_height_m(fine, x, z)
        {
            return h;
        }
        self.heights.height_m_at(x, z)
    }

    /// Displayed height (world units) at (x, z), never below sea level (`surface_height_at`).
    pub fn display_height_at(&self, relief: &ReliefState, x: f32, z: f32) -> f32 {
        relief.display_height(self.height_m_at(x, z), x, z).max(0.0)
    }
}

/// System parameter for the displayed ground height, for any lot that places objects on the map.
/// Absent resources (terrain not loaded yet) give `None`.
#[derive(SystemParam)]
pub struct GroundHeight<'w> {
    surface: Option<Res<'w, Surface>>,
    relief: Option<Res<'w, ReliefState>>,
}

impl GroundHeight<'_> {
    /// Displayed ground height (world units, >= 0) at the map point (x, z).
    pub fn at(&self, x: f32, z: f32) -> Option<f32> {
        Some(
            self.surface
                .as_ref()?
                .display_height_at(self.relief.as_ref()?, x, z),
        )
    }

    pub fn is_ready(&self) -> bool {
        self.surface.is_some() && self.relief.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_bilinear_matches_relief_lod_reference() {
        let mut page = vec![0u16; TILE_PX * TILE_PX];
        for (i, v) in page.iter_mut().enumerate() {
            *v = ((i * 7919) % 65536) as u16;
        }
        let bytes: Vec<u8> = page.iter().flat_map(|v| v.to_le_bytes()).collect();
        for (fx, fy) in [
            (0.0, 0.0),
            (10.25, 3.75),
            (511.0, 511.0),
            (-4.0, 600.0),
            (255.5, 100.1),
        ] {
            let mine = page_bilinear_m(&page, fx, fy, -200.0, 5000.0) as f64;
            let reference = relief_lod::page_bilinear(&bytes, fx as f64, fy as f64, -200.0, 5000.0);
            assert!(
                (mine - reference).abs() < 0.05,
                "{fx},{fy}: {mine} vs {reference}"
            );
        }
    }

    #[test]
    fn falls_back_to_the_heightmap_without_pages() {
        let heights = Arc::new(HeightGrid {
            width: 8,
            height: 8,
            data: vec![32768; 64],
            h_min_m: 0.0,
            h_range_m: 1000.0,
        });
        let pyramid = Arc::new(ReliefPyramid::test_new(1, 1, 1));
        let mut surface = Surface::new(heights, pyramid);
        assert!((surface.height_m_at(3.0, 3.0) - 500.0).abs() < 0.1);
        assert_eq!(
            surface.node_height_m(-1, 3.0, 3.0),
            surface.height_m_at(3.0, 3.0)
        );
        assert!(surface.finest_page_height_m(3.0, 3.0).is_none());
        surface.insert_page(key_of(0, 0, 0), Arc::new(vec![65535; TILE_PX * TILE_PX]));
        assert!((surface.height_m_at(3.0, 3.0) - 4800.0).abs() < 0.1);
        assert_eq!(surface.pages_per_level(), vec![1, 0]);
        assert!(surface.remove_page(key_of(0, 0, 0)));
        assert!(surface.version >= 2);
    }
}
