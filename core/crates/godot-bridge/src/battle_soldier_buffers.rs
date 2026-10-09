//! Buffer post-processing of the soldier renderer, moved out of
//! GDScript (`battle_soldiers.gd`) so that one native call replaces the
//! per-figure loops of the frame.
//!
//! Both functions work on `MultiMesh` transform buffers (12 floats per
//! figure: three rows of basis x, y, z then origin). No Godot type here:
//! plain Rust, testable with `cargo test`. Rendering only, no game rule.

/// Floats of one figure transform.
pub(crate) const STRIDE: usize = 12;
/// Floats of one `MultiMesh` instance with custom data (transform + 4).
pub(crate) const STRIDE_CUSTOM: usize = 16;

/// Indices of the basis (non-origin) floats of a transform.
const BASIS: [usize; 9] = [0, 1, 2, 4, 5, 6, 8, 9, 10];

/// Extra height (m) of the point tested against the camera frustum: the
/// middle of the figure, not its feet.
const VIEW_LIFT_M: f32 = 1.2;
/// Radius (m) of the sphere tested against the frustum (rider included).
const VIEW_RADIUS_M: f32 = 2.5;
/// Margin (m) added to the LOD radius: the shader slices exactly, the CPU
/// sort takes one metre more.
const RADIUS_MARGIN_M: f64 = 1.0;

/// Zeroes the basis of figure `slot` of each listed slot below `n` (a zero
/// basis folds the figure away: standard-bearer, musician, knocked-down).
pub(crate) fn fold_slots(buffer: &mut [f32], slots: &[i32], n: usize) {
    for &slot in slots {
        let Ok(slot) = usize::try_from(slot) else {
            continue;
        };
        let origin = slot * STRIDE;
        if slot >= n || origin + STRIDE > buffer.len() {
            continue;
        }
        for q in BASIS {
            buffer[origin + q] = 0.0;
        }
    }
}

/// The camera frustum as `[nx, ny, nz, d]` planes (Godot `Plane`: distance
/// of `p` is `n . p - d`, positive outside).
fn outside(planes: &[f32], p: [f32; 3]) -> bool {
    planes
        .as_chunks::<4>()
        .0
        .iter()
        .any(|plane| plane[0] * p[0] + plane[1] * p[1] + plane[2] * p[2] - plane[3] > VIEW_RADIUS_M)
}

/// FG5: transforms of the first `n` figures of `buffer` within `radius`
/// (plus the margin) of `camera` and inside the frustum, 16 floats each:
/// the transform then `[rank in buffer, 0, 0, 0]` (custom data, keeps face
/// and variant). Empty when none.
pub(crate) fn fine_near(
    buffer: &[f32],
    n: usize,
    camera: [f32; 3],
    radius: f64,
    planes: &[f32],
) -> Vec<f32> {
    let reach = radius + RADIUS_MARGIN_M;
    let reach_sq = reach * reach;
    let mut out = Vec::with_capacity(n.min(buffer.len() / STRIDE).min(64) * STRIDE_CUSTOM);
    for rank in 0..n.min(buffer.len() / STRIDE) {
        let o = rank * STRIDE;
        let (x, y, z) = (buffer[o + 3], buffer[o + 7], buffer[o + 11]);
        let dx = f64::from(x - camera[0]);
        let dy = f64::from(y - camera[1]);
        let dz = f64::from(z - camera[2]);
        if dx * dx + dy * dy + dz * dz < reach_sq && !outside(planes, [x, y + VIEW_LIFT_M, z]) {
            out.extend_from_slice(&buffer[o..o + STRIDE]);
            out.extend_from_slice(&[rank as f32, 0.0, 0.0, 0.0]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buffer(count: usize) -> Vec<f32> {
        let mut out = Vec::new();
        for i in 0..count {
            let f = i as f32;
            out.extend_from_slice(&[
                1.0,
                0.5,
                0.25,
                f * 2.0,
                0.0,
                1.0,
                0.0,
                0.0,
                -0.25,
                0.5,
                1.0,
                f,
            ]);
        }
        out
    }

    #[test]
    fn folds_only_the_basis_of_slots_below_n() {
        let mut b = buffer(4);
        fold_slots(&mut b, &[1, -1, 3, 9], 3);
        for q in BASIS {
            assert_eq!(b[STRIDE + q], 0.0);
            assert_eq!(b[3 * STRIDE + q], buffer(4)[3 * STRIDE + q], "slot 3 >= n");
        }
        assert_eq!(b[STRIDE + 3], 2.0, "origin kept");
        assert_eq!(b[0], 1.0);
    }

    #[test]
    fn fine_near_keeps_the_figures_in_range_with_their_rank() {
        let b = buffer(10); // figure i at (2i, 0, i)
        let out = fine_near(&b, 10, [0.0, 0.0, 0.0], 6.0, &[]);
        // reach 7: figures 0..=3 (3: 6^2+3^2 = 45 < 49), 4: 64+16 = 80.
        assert_eq!(out.len(), 4 * STRIDE_CUSTOM);
        assert_eq!(out[STRIDE], 0.0);
        assert_eq!(out[STRIDE_CUSTOM + STRIDE], 1.0);
        assert_eq!(
            &out[STRIDE_CUSTOM..STRIDE_CUSTOM + STRIDE],
            &b[STRIDE..2 * STRIDE]
        );
        assert!(fine_near(&b, 0, [0.0; 3], 6.0, &[]).is_empty());
    }

    #[test]
    fn fine_near_drops_figures_behind_a_frustum_plane() {
        let b = buffer(4);
        // Plane x = 3 facing +x (n . p - d > 2.5 outside: x > 5.5).
        let out = fine_near(&b, 4, [0.0, 0.0, 0.0], 100.0, &[1.0, 0.0, 0.0, 3.0]);
        assert_eq!(out.len(), 3 * STRIDE_CUSTOM, "figure at x = 6 is out");
    }
}
