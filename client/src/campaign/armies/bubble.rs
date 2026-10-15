//! Movement bubble: the cells the selected army reaches this turn (vivid) and next turn (pale),
//! laid on the relief as one translucent mesh. Port of `army_movement_bubble.gd` +
//! `reachable_bubble.gdshader` without the mask shader: one quad per navgrid cell (capped),
//! rebuilt when the selection, the sim or the displayed relief changes.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use super::projection::Ground;

/// Lift above the ground (`ArmyMovementBubble.LIFT`).
pub const LIFT: f32 = 0.35;
/// Cap on the quads of the mesh.
pub const MAX_QUADS: usize = 9000;
pub const THIS_TURN: [f32; 4] = [0.30, 0.55, 0.95, 0.38];
pub const NEXT_TURN: [f32; 4] = [0.45, 0.65, 0.95, 0.16];

/// The bubble entity.
#[derive(Component)]
pub struct BubbleMesh;

/// Handles of the bubble.
#[derive(Resource, Default)]
pub struct BubbleHandles {
    pub mesh: Option<Handle<Mesh>>,
    pub quads: usize,
    pub builds: u32,
}

/// Mesh of the cells (`this_turn` and `next_turn` centres in map pixels, cells `size` px wide).
/// Returns the mesh and the number of quads.
pub fn build_mesh(
    this_turn: &[Vec2],
    next_turn: &[Vec2],
    size: f32,
    ground: &impl Ground,
) -> (Mesh, usize) {
    let total = this_turn.len() + next_turn.len();
    let stride = total.div_ceil(MAX_QUADS).max(1);
    let half = size * 0.5 * stride.min(3) as f32;
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<[f32; 4]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut quad = |center: Vec2, color: [f32; 4]| {
        let base = positions.len() as u32;
        for (dx, dz) in [(-half, -half), (half, -half), (half, half), (-half, half)] {
            let (x, z) = (center.x + dx, center.y + dz);
            positions.push([x, ground.height(x, z) + LIFT, z]);
            colors.push(color);
        }
        indices.extend([base, base + 2, base + 1, base, base + 3, base + 2]);
    };
    for cell in next_turn.iter().step_by(stride) {
        quad(*cell, NEXT_TURN);
    }
    for cell in this_turn.iter().step_by(stride) {
        quad(*cell, THIS_TURN);
    }
    let quads = positions.len() / 4;
    let count = positions.len();
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; count]);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(Indices::U32(indices));
    (mesh, quads)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::armies::projection::HeightSampler;

    #[test]
    fn one_quad_per_cell_on_the_ground() {
        let ground = HeightSampler::new(|x, _| x * 0.01);
        let this = vec![Vec2::new(100.0, 100.0), Vec2::new(104.0, 100.0)];
        let next = vec![Vec2::new(108.0, 100.0)];
        let (mesh, quads) = build_mesh(&this, &next, 4.0, &ground);
        assert_eq!(quads, 3);
        assert_eq!(mesh.count_vertices(), 12);
        assert_eq!(mesh.indices().unwrap().len(), 18);
    }

    #[test]
    fn huge_areas_are_capped() {
        let cells: Vec<Vec2> = (0..40000).map(|i| Vec2::new(i as f32, 0.0)).collect();
        let (_, quads) = build_mesh(&cells, &[], 4.0, &HeightSampler::flat());
        assert!(quads <= MAX_QUADS + 1);
    }
}
