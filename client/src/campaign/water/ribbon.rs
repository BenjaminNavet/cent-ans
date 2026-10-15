//! Ribbon meshes along polylines in map pixels (port of `polyline_mesh.gd`, the mesh part of
//! `rivers_renderer.gd` and `RoadRenderer.ribbon_arrays`). Pure data, no Bevy render types, so it
//! is unit-tested; [`RibbonBuffers::into_mesh`] converts to a Bevy `Mesh`.
//!
//! Two layouts:
//! * [`screen_ribbon`]: every vertex sits on the centre line, `normal` = unit perpendicular
//!   (signed by side), `uv = (world width, side +-1)`; the vertex shader widens the ribbon to a
//!   minimum width in screen pixels (`terrain_line`, `road_line`, `river_water`).
//! * [`flat_ribbon`]: vertices are pushed out by half the width, `uv = (distance along, side)`
//!   (dirt road ribbons, `road.gdshader`).

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use super::height::HeightSampler;

/// Raw vertex streams of a ribbon mesh.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct RibbonBuffers {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    /// Second UV set (rivers: distance downstream, importance). Empty when unused.
    pub uvs2: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

impl RibbonBuffers {
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty()
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Appends `other`, shifting its indices.
    pub fn extend(&mut self, other: RibbonBuffers) {
        let base = self.positions.len() as u32;
        self.positions.extend(other.positions);
        self.normals.extend(other.normals);
        self.uvs.extend(other.uvs);
        self.uvs2.extend(other.uvs2);
        self.indices
            .extend(other.indices.into_iter().map(|i| i + base));
    }

    /// Bevy mesh; `None` when empty. UV set 1 is added only when present.
    pub fn into_mesh(self) -> Option<Mesh> {
        if self.is_empty() {
            return None;
        }
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD,
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs);
        if !self.uvs2.is_empty() {
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, self.uvs2);
        }
        mesh.insert_indices(Indices::U32(self.indices));
        Some(mesh)
    }
}

/// One polyline with a width per point (world units, same length as `points`).
#[derive(Debug, Clone)]
pub struct RibbonLine<'a> {
    pub points: &'a [Vec2],
    pub widths: &'a [f32],
    /// Written to `uv2.y` (rivers: importance 0-6).
    pub importance: f32,
}

/// Direction at point `i` (central difference, clamped at the ends), `RIGHT` when degenerate.
pub fn tangent_at(points: &[Vec2], i: usize) -> Vec2 {
    let prev = points[i.saturating_sub(1)];
    let next = points[(i + 1).min(points.len() - 1)];
    let dir = (next - prev).normalize_or_zero();
    if dir == Vec2::ZERO { Vec2::X } else { dir }
}

/// Perpendicular `(-dy, dx)` of a direction.
fn perp(dir: Vec2) -> Vec2 {
    Vec2::new(-dir.y, dir.x)
}

fn push_quads(indices: &mut Vec<u32>, base: u32, count: usize) {
    for i in 0..count.saturating_sub(1) as u32 {
        let a = base + i * 2;
        indices.extend_from_slice(&[a, a + 1, a + 3, a, a + 3, a + 2]);
    }
}

/// Centre-line ribbons for the screen-width vertex shaders. `lift` is added to the ground height.
pub fn screen_ribbon(lines: &[RibbonLine], ground: &dyn HeightSampler, lift: f32) -> RibbonBuffers {
    let mut out = RibbonBuffers::default();
    for line in lines {
        let count = line.points.len();
        if count < 2 {
            continue;
        }
        let base = out.positions.len() as u32;
        let mut along = 0.0;
        for i in 0..count {
            let p = line.points[i];
            if i > 0 {
                along += p.distance(line.points[i - 1]);
            }
            let n = perp(tangent_at(line.points, i));
            let y = ground.surface_world(p.x, p.y) + lift;
            let width = line.widths.get(i).copied().unwrap_or(0.0);
            for side in [1.0_f32, -1.0] {
                out.positions.push([p.x, y, p.y]);
                out.normals.push([n.x * side, 0.0, n.y * side]);
                out.uvs.push([width, side]);
                out.uvs2.push([along, line.importance]);
            }
        }
        push_quads(&mut out.indices, base, count);
    }
    out
}

/// Intermediate points every `step` (the ribbon follows the terrain facets).
pub fn densify(points: &[Vec2], step: f32) -> Vec<Vec2> {
    let Some(&first) = points.first() else {
        return Vec::new();
    };
    let mut out = vec![first];
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let n = ((a.distance(b) / step).ceil() as usize).max(1);
        for k in 1..=n {
            out.push(a.lerp(b, k as f32 / n as f32));
        }
    }
    out
}

/// Flat dirt ribbons of constant `width`, vertices pushed out by half the width, `uv = (along, side)`.
pub fn flat_ribbon(
    runs: &[(Vec<Vec2>, f32)],
    sample_step: f32,
    ground: &dyn HeightSampler,
    lift: f32,
) -> RibbonBuffers {
    let mut out = RibbonBuffers::default();
    for (points, width) in runs {
        if points.len() < 2 {
            continue;
        }
        let dense = densify(points, sample_step);
        let half = width * 0.5;
        let base = out.positions.len() as u32;
        let mut along = 0.0;
        for i in 0..dense.len() {
            if i > 0 {
                along += dense[i].distance(dense[i - 1]);
            }
            let offset = perp(tangent_at(&dense, i)) * half;
            for (edge, side) in [(dense[i] + offset, 1.0_f32), (dense[i] - offset, -1.0)] {
                let y = ground.surface_world(edge.x, edge.y) + lift;
                out.positions.push([edge.x, y, edge.y]);
                out.normals.push([0.0, 1.0, 0.0]);
                out.uvs.push([along, side]);
            }
        }
        push_quads(&mut out.indices, base, dense.len());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::water::height::FlatGround;

    fn line() -> Vec<Vec2> {
        vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0),
        ]
    }

    #[test]
    fn screen_ribbon_counts_and_layout() {
        let points = line();
        let widths = [0.2, 0.3, 0.4];
        let buffers = screen_ribbon(
            &[RibbonLine {
                points: &points,
                widths: &widths,
                importance: 4.0,
            }],
            &FlatGround::default(),
            0.1,
        );
        assert_eq!(buffers.positions.len(), 6);
        assert_eq!(buffers.triangle_count(), 4);
        // Both sides share the centre-line position, normals are opposite unit perpendiculars.
        assert_eq!(buffers.positions[0], buffers.positions[1]);
        assert_eq!(buffers.positions[0], [0.0, 0.1, 0.0]);
        assert_eq!(buffers.normals[0], [0.0, 0.0, 1.0]);
        assert_eq!(buffers.normals[1], [0.0, 0.0, -1.0]);
        assert_eq!(buffers.uvs[2], [0.3, 1.0]);
        assert_eq!(buffers.uvs2[4], [20.0, 4.0]);
        assert_eq!(&buffers.indices[..6], &[0, 1, 3, 0, 3, 2]);
        assert!(
            buffers
                .indices
                .iter()
                .all(|&i| (i as usize) < buffers.positions.len())
        );
    }

    #[test]
    fn short_lines_are_skipped_and_extend_shifts_indices() {
        let one = [Vec2::ZERO];
        let ground = FlatGround::default();
        assert!(
            screen_ribbon(
                &[RibbonLine {
                    points: &one,
                    widths: &[0.1],
                    importance: 0.0
                }],
                &ground,
                0.0
            )
            .is_empty()
        );
        let points = line();
        let widths = [0.1; 3];
        let single = screen_ribbon(
            &[RibbonLine {
                points: &points,
                widths: &widths,
                importance: 0.0,
            }],
            &ground,
            0.0,
        );
        let mut both = single.clone();
        both.extend(single.clone());
        assert_eq!(both.positions.len(), 12);
        assert_eq!(both.indices[12], single.indices[0] + 6);
    }

    #[test]
    fn flat_ribbon_offsets_by_half_width_and_follows_ground() {
        let runs = vec![(vec![Vec2::new(0.0, 0.0), Vec2::new(4.0, 0.0)], 1.0)];
        let ground = FlatGround { altitude_m: 100.0 };
        let buffers = flat_ribbon(&runs, 2.0, &ground, 0.06);
        // 4 px at step 2 -> 3 points -> 6 vertices.
        assert_eq!(buffers.positions.len(), 6);
        let y = 0.6 + 0.06;
        assert_eq!(buffers.positions[0], [0.0, y, 0.5]);
        assert_eq!(buffers.positions[1], [0.0, y, -0.5]);
        assert_eq!(buffers.uvs[4], [4.0, 1.0]);
        assert_eq!(buffers.normals[0], [0.0, 1.0, 0.0]);
    }

    #[test]
    fn densify_keeps_endpoints() {
        let d = densify(&[Vec2::ZERO, Vec2::new(5.0, 0.0)], 2.0);
        assert_eq!(d.len(), 4);
        assert_eq!(d[3], Vec2::new(5.0, 0.0));
        assert!(densify(&[], 1.0).is_empty());
    }

    #[test]
    fn degenerate_tangent_falls_back_to_x() {
        let p = [Vec2::ONE, Vec2::ONE];
        assert_eq!(tangent_at(&p, 0), Vec2::X);
    }

    #[test]
    fn into_mesh_is_none_when_empty() {
        assert!(RibbonBuffers::default().into_mesh().is_none());
    }
}
