//! Paved streets of an emblematic siege town (port of `landmark_siege_town.gd`): the great streets
//! of the plan (Saint-Jacques and la Harpe in Paris, Cheapside in London...) laid between the
//! houses as 8 m ribbons cut every 4 m so they follow the ground, 0.12 m above it. Pure mesh
//! building; `works.rs` spawns it with the atlas masonry matter (cobbles).

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

/// Width of a street (m).
pub const STREET_WIDTH: f32 = 8.0;
/// Length of a ribbon quad (m).
pub const STEP_M: f32 = 4.0;
/// Height above the ground (m).
pub const LIFT_M: f32 = 0.12;

/// Mesh of all the `streets` (polylines in battle metres) laid on `ground`, and its quad count;
/// `None` when no segment is long enough.
pub fn street_mesh(
    streets: &[Vec<(f64, f64)>],
    ground: impl Fn(f64, f64) -> f32,
) -> Option<(Mesh, usize)> {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut quads = 0;
    for street in streets {
        for pair in street.windows(2) {
            let a = Vec2::new(pair[0].0 as f32, pair[0].1 as f32);
            let b = Vec2::new(pair[1].0 as f32, pair[1].1 as f32);
            let length = a.distance(b);
            if length < 0.5 {
                continue;
            }
            let direction = (b - a) / length;
            let side = Vec2::new(-direction.y, direction.x) * STREET_WIDTH * 0.5;
            let steps = ((length / STEP_M).ceil() as usize).max(1);
            let lifted = |p: Vec2| [p.x, ground(f64::from(p.x), f64::from(p.y)) + LIFT_M, p.y];
            for k in 0..steps {
                let p0 = a.lerp(b, k as f32 / steps as f32);
                let p1 = a.lerp(b, (k + 1) as f32 / steps as f32);
                let base = positions.len() as u32;
                for corner in [p0 - side, p0 + side, p1 + side, p1 - side] {
                    positions.push(lifted(corner));
                }
                indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
                quads += 1;
            }
        }
    }
    if quads == 0 {
        return None;
    }
    // Ribbons are flat enough to share an upward normal.
    let normals = vec![[0.0, 1.0, 0.0]; positions.len()];
    // Cobbles are an atlas matter coded in the vertex alpha (`Rubble`), untinted.
    let alpha = crate::campaign::towns::atlas::layer_alpha("Rubble");
    let colors = vec![[0.9, 0.88, 0.85, alpha]; positions.len()];
    let mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices));
    Some((mesh, quads))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_street_is_cut_every_four_metres_and_follows_the_ground() {
        let streets = vec![vec![(0.0, 0.0), (10.0, 0.0), (10.0, 0.2)]];
        let (mesh, quads) = street_mesh(&streets, |x, _| x as f32 * 0.5).unwrap();
        // 10 m -> 3 quads; the 0.2 m stub is skipped.
        assert_eq!(quads, 3);
        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .unwrap()
            .as_float3()
            .unwrap();
        assert_eq!(positions.len(), 12);
        // Every vertex stands 0.12 m above the ground, and the ribbon is 8 m wide.
        for p in positions {
            assert!((p[1] - (p[0] * 0.5 + LIFT_M)).abs() < 1e-5);
        }
        let zs: Vec<f32> = positions.iter().map(|p| p[2]).collect();
        let width = zs.iter().cloned().fold(f32::MIN, f32::max)
            - zs.iter().cloned().fold(f32::MAX, f32::min);
        assert!((width - STREET_WIDTH).abs() < 1e-4);
    }

    #[test]
    fn no_streets_no_mesh() {
        assert!(street_mesh(&[], |_, _| 0.0).is_none());
        assert!(street_mesh(&[vec![(0.0, 0.0), (0.1, 0.0)]], |_, _| 0.0).is_none());
    }
}
