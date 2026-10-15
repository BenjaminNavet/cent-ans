//! Mesh of one quadtree node (a patch of `quads` x `quads` quads plus a skirt).
//!
//! Godot displaces a shared patch in the vertex shader from page textures. Here the patch is
//! sampled on the CPU from the resident pages when the node appears (or when its page changes),
//! and the dynamic vertical exaggeration stays in the vertex shader (`terrain.wesl`): per-vertex
//! attributes carry the altitude in metres and the relief fields, so a change of vertical scale
//! needs no rebuild.
//!
//! Attributes: `POSITION` = (x, altitude m, z) in map units, `NORMAL` = (dh/dx, dh/dz, skirt
//! depth), `UV_0` = (valley floor m, base m), `UV_1` = (squash factor k, massif lift D m),
//! `COLOR` = (geomorph delta m, vertex spacing, 0, 0): altitude of the parent (coarser) level at
//! this vertex minus its own, blended in by `terrain.wesl` as the node nears its switch distance.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use super::relief_state::ReliefState;
use super::surface::Surface;

/// What defines the geometry of a node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeSpec {
    /// North-west corner (x, z) in map units.
    pub origin: (f32, f32),
    /// Vertex spacing.
    pub spacing: f32,
    /// Quads per side (64 for a node, 32 for a quadrant drawn at the parent level).
    pub quads: u32,
    /// Page the heights come from (`-1`: world heightmap).
    pub fine: i64,
    /// Skirt depth in world units (hides cracks between pages and levels).
    pub skirt: f32,
}

/// Number of vertices and triangles of a patch.
pub fn patch_counts(quads: u32) -> (usize, usize) {
    let side = quads as usize + 1;
    (
        side * side + 4 * side,
        quads as usize * quads as usize * 2 + 4 * quads as usize * 2,
    )
}

/// Builds the node mesh.
pub fn build_node_mesh(spec: &NodeSpec, surface: &Surface, relief: &ReliefState) -> Mesh {
    let quads = spec.quads as usize;
    let side = quads + 1;
    // Heights with a one-vertex border for the central-difference slopes.
    let wide = side + 2;
    let mut heights = vec![0.0f32; wide * wide];
    for j in 0..wide {
        for i in 0..wide {
            let x = spec.origin.0 + (i as f32 - 1.0) * spec.spacing;
            let z = spec.origin.1 + (j as f32 - 1.0) * spec.spacing;
            heights[j * wide + i] = surface.node_height_m(spec.fine, x, z);
        }
    }
    let at = |i: usize, j: usize| heights[(j + 1) * wide + (i + 1)];
    let (vertex_count, triangle_count) = patch_counts(spec.quads);
    let mut positions = Vec::with_capacity(vertex_count);
    let mut slopes = Vec::with_capacity(vertex_count);
    let mut fields_a = Vec::with_capacity(vertex_count);
    let mut fields_b = Vec::with_capacity(vertex_count);
    let mut morphs: Vec<[f32; 4]> = Vec::with_capacity(vertex_count);
    let mut push = |i: usize, j: usize, skirt: f32| {
        let x = spec.origin.0 + i as f32 * spec.spacing;
        let z = spec.origin.1 + j as f32 * spec.spacing;
        let h = at(i, j);
        let dx = (heights[(j + 1) * wide + (i + 2)] - heights[(j + 1) * wide + i])
            / (2.0 * spec.spacing);
        let dz = (heights[(j + 2) * wide + (i + 1)] - heights[j * wide + (i + 1)])
            / (2.0 * spec.spacing);
        let fields = relief.fields_at(x, z);
        positions.push([x, h, z]);
        slopes.push([dx, dz, skirt]);
        fields_a.push([fields.x, fields.y]);
        fields_b.push([fields.z, relief.lift_at(x, z)]);
        // Parent level: vertices on odd indices sit on the line between the even ones.
        let at_signed = |a: isize, b: isize| heights[(b + 1) as usize * wide + (a + 1) as usize];
        let (ii, jj) = (i as isize, j as isize);
        let coarse = match (i % 2, j % 2) {
            (0, 0) => h,
            (1, 0) => 0.5 * (at_signed(ii - 1, jj) + at_signed(ii + 1, jj)),
            (0, 1) => 0.5 * (at_signed(ii, jj - 1) + at_signed(ii, jj + 1)),
            _ => 0.5 * (at_signed(ii - 1, jj - 1) + at_signed(ii + 1, jj + 1)),
        };
        morphs.push([coarse - h, spec.spacing, 0.0, 0.0]);
    };
    for j in 0..side {
        for i in 0..side {
            push(i, j, 0.0);
        }
    }
    // Skirt vertices: north, south, west, east edges (same attributes, dropped in the shader).
    for i in 0..side {
        push(i, 0, spec.skirt);
    }
    for i in 0..side {
        push(i, quads, spec.skirt);
    }
    for j in 0..side {
        push(0, j, spec.skirt);
    }
    for j in 0..side {
        push(quads, j, spec.skirt);
    }

    let mut indices: Vec<u32> = Vec::with_capacity(triangle_count * 3);
    for j in 0..quads {
        for i in 0..quads {
            let a = (j * side + i) as u32;
            let b = a + 1;
            let c = a + side as u32;
            let d = c + 1;
            // Counter-clockwise seen from above (+Y).
            indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    let base = (side * side) as u32;
    let s = side as u32;
    for i in 0..quads as u32 {
        // North edge j = 0.
        skirt_quad(&mut indices, i, i + 1, base + i, base + i + 1);
        // South edge j = quads.
        let south = (quads * side) as u32 + i;
        skirt_quad(
            &mut indices,
            south + 1,
            south,
            base + s + i + 1,
            base + s + i,
        );
        // West edge i = 0.
        skirt_quad(
            &mut indices,
            (i + 1) * s,
            i * s,
            base + 2 * s + i + 1,
            base + 2 * s + i,
        );
        // East edge i = quads.
        let east = i * s + s - 1;
        skirt_quad(
            &mut indices,
            east,
            east + s,
            base + 3 * s + i,
            base + 3 * s + i + 1,
        );
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, slopes);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, fields_a);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, fields_b);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, morphs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Two-sided quad hanging below an edge segment (the material is rendered without culling).
fn skirt_quad(indices: &mut Vec<u32>, top_a: u32, top_b: u32, low_a: u32, low_b: u32) {
    indices.extend_from_slice(&[top_a, low_a, top_b, top_b, low_a, low_b]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::terrain::profile::{CloseCameraProfile, ReliefProfile};
    use crate::campaign::terrain::pyramid::ReliefPyramid;
    use crate::campaign::terrain::relief_floor::HeightGrid;
    use crate::campaign::terrain::tres::Tres;
    use std::sync::Arc;

    fn world() -> (Surface, ReliefState) {
        // Slope of 1 m per map unit along x.
        let width = 128;
        let mut data = Vec::new();
        for _y in 0..width {
            for x in 0..width {
                data.push(((x as f32 * 1.0 + 200.0) / 5000.0 * 65535.0) as u16);
            }
        }
        let heights = Arc::new(HeightGrid {
            width,
            height: width,
            data,
            h_min_m: -200.0,
            h_range_m: 5000.0,
        });
        let surface = Surface::new(heights, Arc::new(ReliefPyramid::test_new(1, 1, 0)));
        let relief = ReliefState::new(
            ReliefProfile::from_tres(&Tres::default()),
            CloseCameraProfile::from_tres(&Tres::default()),
        );
        (surface, relief)
    }

    #[test]
    fn patch_has_the_expected_counts_and_slopes() {
        let (surface, relief) = world();
        let spec = NodeSpec {
            origin: (16.0, 16.0),
            spacing: 1.0,
            quads: 32,
            fine: -1,
            skirt: 0.5,
        };
        let mesh = build_node_mesh(&spec, &surface, &relief);
        let (vertices, triangles) = patch_counts(32);
        assert_eq!(mesh.count_vertices(), vertices);
        assert_eq!(mesh.indices().unwrap().len(), triangles * 3);
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(slopes)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("normals missing");
        };
        // dh/dx = 1 m per unit, dh/dz = 0, skirt flag only on the skirt vertices.
        assert!((slopes[5][0] - 1.0).abs() < 0.05, "{}", slopes[5][0]);
        assert!(slopes[5][1].abs() < 1e-3);
        assert_eq!(slopes[5][2], 0.0);
        assert_eq!(slopes[slopes.len() - 1][2], 0.5);
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions missing");
        };
        assert_eq!(positions[0][0], 16.0);
        assert!((positions[0][1] - 16.0).abs() < 0.2, "{}", positions[0][1]);
        // Geomorph: flat in z, slope 1 in x -> the parent level reproduces a linear ramp exactly.
        let Some(bevy::mesh::VertexAttributeValues::Float32x4(morphs)) =
            mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("morph attribute missing");
        };
        assert!(morphs[5][0].abs() < 0.05, "{}", morphs[5][0]);
        assert_eq!(morphs[5][1], 1.0);
        // Every index is in range.
        let max = mesh.indices().unwrap().iter().max().unwrap();
        assert!(max < vertices);
    }
}
