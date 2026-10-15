//! Forest edges (port of `foliage_variety` in `foliage_common.gdshaderinc`).
//!
//! On the ramp of the forest cover (`smoothstep(cover_low, cover_high, splat)`), trees are lower
//! (`edge_height`) and sparser (`edge_thin` of the row removed); field trees (zero cover) and
//! the heart of the massifs are untouched. Godot did it in the vertex shader of every tree; here
//! it is done once per tile on the worker thread, with the same formula. Godot also narrows the
//! crown by 0.85 on the edge: the impostor width is `height * ratio`, so the single height factor
//! (`edge_height` = 0.85 in `map_prop_scale.tres`) gives the same result.

use super::data::{Plane, style};
use super::mask::smoothstep;
use super::tiles::TreeInst;

/// Edge weight (0 in the field and at the heart of the massif, 1 in the middle of the ramp).
pub fn edge_weight(cover: f32, range: (f32, f32)) -> f32 {
    let d = smoothstep(range.0, range.1, cover);
    smoothstep(0.02, 0.2, d) * (1.0 - smoothstep(0.55, 0.9, d))
}

/// Height factor and keep test of a tree whose edge weight is `edge`.
pub fn edge_effect(edge: f32, seed: f32) -> (f32, bool) {
    let factor = 1.0 + (style::EDGE_HEIGHT - 1.0) * edge;
    let keep = edge * style::EDGE_THIN <= (seed * 91.7).fract();
    (factor, keep)
}

/// Lowers and thins the trees of a tile; returns the number removed.
pub fn apply(
    cover: &Plane,
    map_size: (f64, f64),
    range: (f32, f32),
    trees: &mut Vec<TreeInst>,
) -> usize {
    if cover.is_empty() {
        return 0;
    }
    let before = trees.len();
    trees.retain_mut(|tree| {
        let raw = cover.texel(map_size, tree.x as f64, tree.z as f64, 0);
        let edge = edge_weight(raw, range);
        if edge <= 0.0 {
            return true;
        }
        let (factor, keep) = edge_effect(edge, tree.seed);
        if keep {
            tree.height *= factor;
        }
        keep
    });
    before - trees.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_is_zero_in_the_field_and_in_the_heart() {
        let range = (0.22, 0.68);
        assert_eq!(edge_weight(0.0, range), 0.0);
        assert_eq!(edge_weight(1.0, range), 0.0);
        let mid = edge_weight(0.45, range);
        assert!(mid > 0.5, "{mid}");
    }

    #[test]
    fn edge_lowers_and_thins_deterministically() {
        let (factor, _) = edge_effect(1.0, 0.3);
        assert!((factor - style::EDGE_HEIGHT).abs() < 1e-6);
        // Thinned share on a full edge ~ EDGE_THIN.
        let removed = (0..1000)
            .filter(|i| !edge_effect(1.0, *i as f32 / 1000.0).1)
            .count();
        assert!((60..140).contains(&removed), "{removed}");
        assert!(edge_effect(0.0, 0.0).1);
    }

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
        let removed = apply(&cover, (2.0, 1.0), (0.22, 0.68), &mut trees);
        assert_eq!(removed, 0);
        assert_eq!(trees[0].height, 1.0);
        assert!(trees[1].height < 0.95);
    }
}
