//! Ribbon meshing of one E2 tile: fine rivers and draped roads (`fine_ribbon_job.gd`), built on
//! the async compute pool then installed on the main thread (`FineGeoLayer`).
//!
//! Heights in METRES in `position.y`: the shaders drape them on the exaggerated relief
//! (`relief.wesl`), so a change of exaggeration needs no rebuild. Ground under the ribbon: a
//! snapshot of the resident relief pages, re-read when the finest page level under the tile
//! changes.
//!
//! Rivers: vertices doubled on the axis (the shader pushes them by the half width, at least a
//! screen pixel); NORMAL = signed perpendicular; UV0 = (world width, side +-1); UV1 = (distance
//! downstream, display rank); COLOR = (divagating, tidal, wetland, intermittent). Height: water
//! level `z`, raised above the surface where the bed is not carved.
//! Roads: same layout; UV1 = (distance, real width m); COLOR = (main, causeway, paved, computed).
//! Height: max(surface, tile `z`) + lift.
//!
//! Not ported: settlement covers and their gate bridges (`covers` is always empty since VT,
//! ADR 0138), the open zones of the 1:1 cities (VH, no 1:1 city in the Bevy client): every custom
//! zone cuts the water like Godot's closed `zones`.

use std::sync::Arc;
use std::time::Instant;

use bevy::asset::RenderAssetUsages;
use bevy::math::{Vec2, Vec3};
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

use super::cafv::{
    CafvTile, FLAG_COMPUTED_ROAD, FLAG_DIVAGATING, FLAG_INTERMITTENT, FLAG_MAIN_ROAD, FLAG_TIDAL,
    FLAG_WETLAND,
};
use super::store::tile_rect;
use crate::campaign::terrain::pyramid::{ReliefPyramid, col_of_key, level_of_key, row_of_key};
use crate::campaign::terrain::surface::Surface;

/// Lift (m) of the water above the surface where the bed is not carved, and of the roads.
pub const RIVER_LIFT_M: f32 = 0.35;
pub const ROAD_LIFT_M: f32 = 0.25;
/// Maximum step (world units) of the densified roads: they follow the relief facets.
pub const ROAD_STEP: f32 = 0.08;

/// Inputs of one tile build.
pub struct FineRibbonJob {
    pub key: i32,
    pub river_tile: Option<Arc<CafvTile>>,
    pub road_tile: Option<Arc<CafvTile>>,
    /// Resident pages around the tile (and the world heightmap, unused here).
    pub surface: Option<Surface>,
    /// Custom zones (x, z, radius): no generic water inside.
    pub zones: Vec<[f32; 3]>,
    /// Cities and towns (x, z, radius of the paved streets).
    pub towns: Vec<[f32; 3]>,
    pub meters_per_unit: f32,
    pub min_order: u32,
}

/// Output of one tile build.
pub struct FineRibbonOutput {
    pub key: i32,
    pub river: Option<Mesh>,
    pub road: Option<Mesh>,
    pub river_points: usize,
    pub road_points: usize,
    pub finest: i32,
    pub build_ms: f32,
}

#[derive(Default)]
struct Arrays {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    uv0: Vec<[f32; 2]>,
    uv1: Vec<[f32; 2]>,
    colors: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl Arrays {
    fn push(&mut self, p: Vec2, h: f32, perp: Vec3, uv0: [f32; 2], uv1: [f32; 2], c: [f32; 4]) {
        for side in [1.0, -1.0] {
            self.positions.push([p.x, h, p.y]);
            self.normals.push((perp * side).to_array());
            self.uv0.push([uv0[0], side * uv0[1]]);
            self.uv1.push(uv1);
            self.colors.push(c);
        }
    }

    fn strip(&mut self, base: u32, count: usize) {
        for i in 0..count.saturating_sub(1) as u32 {
            let a = base + i * 2;
            self.indices
                .extend_from_slice(&[a, a + 1, a + 3, a, a + 3, a + 2]);
        }
    }

    fn into_mesh(self) -> Option<Mesh> {
        if self.indices.is_empty() {
            return None;
        }
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uv0);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, self.uv1);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, self.colors);
        mesh.insert_indices(Indices::U32(self.indices));
        Some(mesh)
    }
}

/// Unit direction along a polyline at `i` (central difference), +X when degenerate.
fn direction(pts: &[Vec2], i: usize) -> Vec2 {
    let n = pts.len();
    let d = pts[(i + 1).min(n - 1)] - pts[i.saturating_sub(1)];
    d.try_normalize().unwrap_or(Vec2::X)
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Finest resident page level whose tile touches the rectangle, -1 without page.
pub fn finest_level(surface: &Surface, rect: [f32; 4]) -> i32 {
    surface
        .page_keys()
        .filter(|&k| {
            let level = level_of_key(k);
            let (ox, oz) = ReliefPyramid::tile_origin(level, col_of_key(k), row_of_key(k));
            let size = crate::campaign::terrain::pyramid::tile_units(level);
            let (ox, oz) = (ox as f32, oz as f32);
            ox < rect[2] && ox + size as f32 > rect[0] && oz < rect[3] && oz + size as f32 > rect[1]
        })
        .map(level_of_key)
        .max()
        .unwrap_or(-1)
}

impl FineRibbonJob {
    /// Surface (m) under a point: finest resident page, `None` outside the pages.
    fn surface_m(&self, x: f32, z: f32) -> Option<f32> {
        self.surface.as_ref()?.finest_page_height_m(x, z)
    }

    fn in_zone(&self, p: Vec2) -> bool {
        self.zones
            .iter()
            .any(|z| Vec2::new(z[0], z[1]).distance_squared(p) < z[2] * z[2])
    }

    pub fn run(self) -> FineRibbonOutput {
        let t0 = Instant::now();
        let (col, row) = (self.key & 0xFFF, self.key >> 12);
        let finest = self
            .surface
            .as_ref()
            .map_or(-1, |s| finest_level(s, tile_rect(col, row)));
        let (river, river_points) = match &self.river_tile {
            Some(t) => self.build_rivers(t),
            None => (None, 0),
        };
        let (road, road_points) = match &self.road_tile {
            Some(t) => self.build_roads(t),
            None => (None, 0),
        };
        FineRibbonOutput {
            key: self.key,
            river,
            road,
            river_points,
            road_points,
            finest,
            build_ms: t0.elapsed().as_secs_f32() * 1000.0,
        }
    }

    fn build_rivers(&self, tile: &CafvTile) -> (Option<Mesh>, usize) {
        let mut out = Arrays::default();
        for li in 0..tile.lines() {
            let rank = tile.line_rank[li];
            if rank < self.min_order {
                continue;
            }
            let flags = tile.line_flags[li];
            let bit = |f: u32| if flags & f != 0 { 1.0 } else { 0.0 };
            let color = [
                bit(FLAG_DIVAGATING),
                bit(FLAG_TIDAL),
                bit(FLAG_WETLAND),
                bit(FLAG_INTERMITTENT),
            ];
            // Pieces cut by the custom zones (cut at the last point outside).
            let mut piece: Vec<usize> = Vec::new();
            for k in tile.range(li) {
                let p = Vec2::new(tile.x[k], tile.y[k]);
                if !self.zones.is_empty() && self.in_zone(p) {
                    self.river_piece(&mut out, tile, &piece, rank, color);
                    piece.clear();
                } else {
                    piece.push(k);
                }
            }
            self.river_piece(&mut out, tile, &piece, rank, color);
        }
        let points = out.positions.len() / 2;
        (out.into_mesh(), points)
    }

    fn river_piece(
        &self,
        out: &mut Arrays,
        tile: &CafvTile,
        piece: &[usize],
        rank: u32,
        color: [f32; 4],
    ) {
        if piece.len() < 2 {
            return;
        }
        let pts: Vec<Vec2> = piece
            .iter()
            .map(|&k| Vec2::new(tile.x[k], tile.y[k]))
            .collect();
        let base = out.positions.len() as u32;
        let mut along = 0.0;
        for (i, &k) in piece.iter().enumerate() {
            let p = pts[i];
            if i > 0 {
                along += p.distance(pts[i - 1]);
            }
            let dir = direction(&pts, i);
            let perp = Vec3::new(-dir.y, 0.0, dir.x);
            let mut h = tile.z[k];
            if let Some(ground) = self.surface_m(p.x, p.y) {
                h = h.max(ground + RIVER_LIFT_M);
            }
            let width = tile.w[k] / self.meters_per_unit;
            out.push(p, h, perp, [width, 1.0], [along, rank as f32], color);
        }
        out.strip(base, piece.len());
    }

    fn build_roads(&self, tile: &CafvTile) -> (Option<Mesh>, usize) {
        let mut out = Arrays::default();
        for li in 0..tile.lines() {
            let range = tile.range(li);
            if range.len() < 2 {
                continue;
            }
            let flags = tile.line_flags[li];
            let bit = |f: u32| if flags & f != 0 { 1.0 } else { 0.0 };
            // Densified: one point every ROAD_STEP (the ribbon follows the displayed relief).
            let mut pts: Vec<Vec2> = Vec::new();
            let mut zs: Vec<f32> = Vec::new();
            let mut ws: Vec<f32> = Vec::new();
            for k in range.clone() {
                let p = Vec2::new(tile.x[k], tile.y[k]);
                if k > range.start {
                    let q = *pts.last().expect("previous point");
                    let steps = (q.distance(p) / ROAD_STEP).ceil() as usize;
                    for m in 1..steps {
                        let t = m as f32 / steps as f32;
                        pts.push(q.lerp(p, t));
                        zs.push(tile.z[k - 1] + (tile.z[k] - tile.z[k - 1]) * t);
                        ws.push(tile.w[k - 1] + (tile.w[k] - tile.w[k - 1]) * t);
                    }
                }
                pts.push(p);
                zs.push(tile.z[k]);
                ws.push(tile.w[k]);
            }
            let base = out.positions.len() as u32;
            let mut along = 0.0;
            for i in 0..pts.len() {
                let p = pts[i];
                if i > 0 {
                    along += p.distance(pts[i - 1]);
                }
                let dir = direction(&pts, i);
                let perp = Vec3::new(-dir.y, 0.0, dir.x);
                let h = self.surface_m(p.x, p.y).map_or(zs[i], |g| g.max(zs[i])) + ROAD_LIFT_M;
                let color = [
                    bit(FLAG_MAIN_ROAD),
                    bit(FLAG_WETLAND),
                    self.paved(p),
                    bit(FLAG_COMPUTED_ROAD),
                ];
                out.push(
                    p,
                    h,
                    perp,
                    [ws[i] / self.meters_per_unit, 1.0],
                    [along, ws[i]],
                    color,
                );
            }
            out.strip(base, pts.len());
        }
        let points = out.positions.len() / 2;
        (out.into_mesh(), points)
    }

    /// Paved street in and around towns (1 at the core, fading out to the radius).
    fn paved(&self, p: Vec2) -> f32 {
        self.towns
            .iter()
            .filter_map(|t| {
                let d = Vec2::new(t[0], t[1]).distance(p);
                (d < t[2]).then(|| 1.0 - smoothstep(t[2] * 0.6, t[2], d))
            })
            .fold(0.0, f32::max)
    }
}

#[cfg(test)]
mod tests {
    use super::super::cafv::{LAYER_RIVERS, LAYER_ROADS, ORDER_SHIFT};
    use super::*;

    fn job(river: Option<CafvTile>, road: Option<CafvTile>) -> FineRibbonJob {
        FineRibbonJob {
            key: 0,
            river_tile: river.map(Arc::new),
            road_tile: road.map(Arc::new),
            surface: None,
            zones: Vec::new(),
            towns: Vec::new(),
            meters_per_unit: 100.0,
            min_order: 3,
        }
    }

    fn river(flags: u32, w: f32) -> CafvTile {
        CafvTile::single_line(
            LAYER_RIVERS,
            0,
            0,
            flags,
            &[
                [1.0, 1.0, 20.0, w],
                [2.0, 1.0, 19.0, w],
                [3.0, 1.0, 18.0, w],
            ],
        )
    }

    #[test]
    fn river_ribbon_layout() {
        let out = job(Some(river(5 << ORDER_SHIFT | FLAG_TIDAL, 50.0)), None).run();
        let mesh = out.river.expect("river mesh");
        assert_eq!(out.river_points, 3);
        assert_eq!(mesh.indices().unwrap().len(), 12);
        let Some(bevy::mesh::VertexAttributeValues::Float32x2(uv0)) =
            mesh.attribute(Mesh::ATTRIBUTE_UV_0)
        else {
            panic!("uv0");
        };
        // Width in world units (50 m / 100 m per unit), sides +1 / -1.
        assert_eq!(uv0[0], [0.5, 1.0]);
        assert_eq!(uv0[1], [0.5, -1.0]);
        let Some(bevy::mesh::VertexAttributeValues::Float32x2(uv1)) =
            mesh.attribute(Mesh::ATTRIBUTE_UV_1)
        else {
            panic!("uv1");
        };
        // Rank = max(order 5, width rank 6), distance downstream.
        assert_eq!(uv1[4], [2.0, 6.0]);
        let Some(bevy::mesh::VertexAttributeValues::Float32x4(colors)) =
            mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("colors");
        };
        assert_eq!(colors[0], [0.0, 1.0, 0.0, 0.0]);
        // Water level in metres without pages.
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions");
        };
        assert_eq!(pos[0][1], 20.0);
    }

    #[test]
    fn small_orders_and_zones_are_skipped() {
        // Width 3 m, order 2: below the minimum order.
        assert!(
            job(Some(river(2 << ORDER_SHIFT, 3.0)), None)
                .run()
                .river
                .is_none()
        );
        let mut j = job(Some(river(5 << ORDER_SHIFT, 50.0)), None);
        j.zones.push([2.0, 1.0, 0.5]);
        // The middle point is in the zone: two pieces of one point each, nothing drawn.
        assert!(j.run().river.is_none());
    }

    #[test]
    fn roads_are_densified_and_paved_in_towns() {
        let road = CafvTile::single_line(
            LAYER_ROADS,
            0,
            0,
            FLAG_MAIN_ROAD,
            &[[0.0, 0.0, 10.0, 6.0], [0.8, 0.0, 12.0, 6.0]],
        );
        let mut j = job(None, Some(road));
        j.towns.push([0.0, 0.0, 0.5]);
        let out = j.run();
        // 0.8 / 0.08 = 10 steps -> 11 points.
        assert_eq!(out.road_points, 11);
        let mesh = out.road.unwrap();
        let Some(bevy::mesh::VertexAttributeValues::Float32x4(colors)) =
            mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("colors");
        };
        assert_eq!(colors[0], [1.0, 0.0, 1.0, 0.0]);
        assert_eq!(colors[21][2], 0.0);
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions");
        };
        assert!((pos[0][1] - (10.0 + ROAD_LIFT_M)).abs() < 1e-4);
    }
}
