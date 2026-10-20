//! Forest edges (port of `foliage_variety` in `foliage_common.gdshaderinc`).
//!
//! On the ramp of the forest cover (`smoothstep(cover_low, cover_high, splat)`), trees are lower
//! (`edge_height`) and sparser (`edge_thin` of the row removed); field trees (zero cover) and
//! the heart of the massifs are untouched. Godot did it in the vertex shader of every tree; here
//! it is done once per tile on the worker thread, with the same formula. Godot also narrows the
//! crown by 0.85 on the edge: the impostor width is `height * ratio`, so the single height factor
//! (`edge_height` = 0.85 in `map_prop_scale.tres`) gives the same result.
//!
//! TS-T2 (Total War look): the edge is crisp. Below `edge.cut` of the ramp only a few trees stay,
//! grouped by small cells into lone trees and little groves standing in front of the forest; past
//! the cut every tree stays (lowered on the first band), no random thinning frays the edge.

use super::data::{Plane, style};
use super::look::ForestLook;
use super::mask::smoothstep;
use super::tiles::TreeInst;

/// Hash in [0, 1) of an integer cell.
fn cell_hash(x: i64, y: i64) -> f32 {
    let mut h = (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= h >> 31;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^= h >> 29;
    (h >> 40) as f32 / (1u64 << 24) as f32
}

/// Whether a tree in front of the edge (ramp position `d` below the cut) is kept: whole small
/// cells are kept (little groves), plus a few lone trees, more of them near the cut.
pub fn outlier_kept(x: f32, z: f32, seed: f32, d: f32, look: &ForestLook) -> bool {
    let near_cut = (d / look.edge_cut.max(1e-3)).clamp(0.0, 1.0);
    let share = look.outlier_share * (0.4 + 0.6 * near_cut);
    let g = look.outlier_group_px.max(0.1);
    let cell = cell_hash((x / g).floor() as i64, (z / g).floor() as i64);
    (cell < share && (seed * 13.7).fract() < 0.75) || (seed * 91.7).fract() < share * 0.25
}

/// Lowers the trees of a tile on the edge band and keeps only a few in front of the edge;
/// returns the number removed.
pub fn apply(
    cover: &Plane,
    map_size: (f64, f64),
    range: (f32, f32),
    look: &ForestLook,
    trees: &mut Vec<TreeInst>,
) -> usize {
    if cover.is_empty() {
        return 0;
    }
    let before = trees.len();
    trees.retain_mut(|tree| {
        let raw = cover.texel(map_size, tree.x as f64, tree.z as f64, 0);
        let d = smoothstep(range.0, range.1, raw);
        if d <= 0.0 {
            // Field trees (hedgerows, lone trees, groves) are untouched.
            return true;
        }
        if d < look.edge_cut {
            return outlier_kept(tree.x, tree.z, tree.seed, d, look);
        }
        let band = 1.0 - smoothstep(look.edge_cut, (look.edge_cut + 0.25).min(1.0), d);
        tree.height *= 1.0 + (style::EDGE_HEIGHT - 1.0) * band;
        true
    });
    before - trees.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(x: f32, seed: f32) -> TreeInst {
        TreeInst {
            x,
            z: 0.5,
            y: 0.0,
            yaw_deg: 0.0,
            height: 1.0,
            crown: 1.0,
            row: 0,
            seed,
            tint: [1.0; 3],
        }
    }

    #[test]
    fn apply_scales_trees_on_the_ramp_only() {
        // Two texels over a 2 x 1 map: cover 0 (field) then 0.45 (ramp).
        let cover = Plane {
            w: 2,
            h: 1,
            ch: 1,
            data: vec![0, (0.45 * 255.0) as u8],
        };
        let mut trees = vec![tree(0.5, 0.9), tree(1.5, 0.9)];
        let look = ForestLook {
            edge_cut: 0.45,
            ..ForestLook::default()
        };
        let removed = apply(&cover, (2.0, 1.0), (0.22, 0.68), &look, &mut trees);
        assert_eq!(removed, 0);
        assert_eq!(trees[0].height, 1.0);
        assert!(trees[1].height < 0.95);
    }

    #[test]
    fn few_trees_stand_in_front_of_the_edge() {
        let look = ForestLook::default();
        let kept = (0..4000)
            .filter(|i| {
                let x = (i % 64) as f32 * 0.37;
                let z = (i / 64) as f32 * 0.37;
                outlier_kept(x, z, (*i as f32 * 0.618).fract(), 0.2, &look)
            })
            .count();
        // About the outlier share (grouped), never the whole fringe.
        assert!((40..800).contains(&kept), "{kept}");
    }
}
