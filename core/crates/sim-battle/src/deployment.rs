//! Initial placement of the player's regiments during the deployment phase
//! (lot SC BT8: moved from `DeploymentController.plan` in GDScript).
//!
//! A right click (`p0 == p1`, or a drag shorter than [`MIN_DRAG_M`]) gathers
//! the selection around the point, each regiment keeping its facing; a
//! right-drag lays them along the line `p0 -> p1`, front turned away from the
//! camera. The drag length is the whole frontage, shared in proportion to
//! strength ([`split_widths`]); the regiments keep their present left-to-right
//! order along the drag, and each takes the width the core retains for it.

use crate::formation_width::split_widths;

/// Metres between two neighbouring regiments of a deployment line.
pub(crate) const UNIT_GAP: f64 = 6.0;
/// A drag shorter than this is a plain click.
pub(crate) const MIN_DRAG_M: f64 = 2.0;

/// What the placement needs to know of one regiment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeployUnit {
    pub x: f64,
    pub z: f64,
    pub facing: f64,
    pub soldiers: u32,
    /// Present extent (frontage, depth) in metres.
    pub width: f64,
    pub depth: f64,
}

impl Default for DeployUnit {
    /// An unknown regiment.
    fn default() -> Self {
        Self {
            x: 0.0,
            z: 0.0,
            facing: 0.0,
            soldiers: 1,
            width: 30.0,
            depth: 6.0,
        }
    }
}

/// One regiment's place.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeploySpot {
    pub id: u32,
    pub x: f64,
    pub z: f64,
    /// `NaN`: keep the present facing.
    pub facing: f64,
    /// Width asked of the core (0 without a drag).
    pub share: f64,
    /// Extent retained (frontage, depth).
    pub width: f64,
    pub depth: f64,
}

/// Places for `ids` (selection order) around/along `p0 -> p1` (ground
/// points `(x, z)`), `camera` the camera's ground point. `unit` gives a
/// regiment's state, `extent(id, share)` the size it takes for a drag share.
pub fn plan_deployment(
    ids: &[u32],
    p0: (f64, f64),
    p1: (f64, f64),
    camera: (f64, f64),
    unit: impl Fn(u32) -> DeployUnit,
    extent: impl Fn(u32, f64) -> (f64, f64),
) -> Vec<DeploySpot> {
    if ids.is_empty() {
        return Vec::new();
    }
    let drag = (p1.0 - p0.0, p1.1 - p0.1);
    let drag_len = drag.0.hypot(drag.1);
    let dragged = drag_len >= MIN_DRAG_M;
    let center = ((p0.0 + p1.0) * 0.5, (p0.1 + p1.1) * 0.5);
    let mut facing = f64::NAN;
    let along;
    let mut order: Vec<u32> = ids.to_vec();
    let mut shares: Vec<f64> = Vec::new();
    if dragged {
        let mut normal = (-drag.1 / drag_len, drag.0 / drag_len);
        if normal.0 * (center.0 - camera.0) + normal.1 * (center.1 - camera.1) < 0.0 {
            normal = (-normal.0, -normal.1);
        }
        facing = normal.0.atan2(normal.1);
        along = (drag.0 / drag_len, drag.1 / drag_len);
        let mut keyed: Vec<(f64, usize)> = ids
            .iter()
            .enumerate()
            .map(|(k, &id)| {
                let u = unit(id);
                (u.x * along.0 + u.z * along.1, k)
            })
            .collect();
        keyed.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        order = keyed.iter().map(|&(_, k)| ids[k]).collect();
        let counts: Vec<u32> = order.iter().map(|&id| unit(id).soldiers).collect();
        shares = split_widths(&counts, drag_len, UNIT_GAP);
    } else {
        let f = unit(ids[0]).facing;
        along = (f.cos(), -f.sin());
    }
    let sizes: Vec<(f64, f64)> = order
        .iter()
        .enumerate()
        .map(|(k, &id)| {
            if dragged {
                extent(id, shares[k])
            } else {
                let u = unit(id);
                (u.width, u.depth)
            }
        })
        .collect();
    let total: f64 = sizes.iter().map(|s| s.0 + UNIT_GAP).sum::<f64>() - UNIT_GAP;
    let mut cursor = -total * 0.5;
    order
        .iter()
        .enumerate()
        .map(|(k, &id)| {
            let offset = cursor + sizes[k].0 * 0.5;
            cursor += sizes[k].0 + UNIT_GAP;
            DeploySpot {
                id,
                x: center.0 + along.0 * offset,
                z: center.1 + along.1 * offset,
                facing,
                share: if dragged { shares[k] } else { 0.0 },
                width: sizes[k].0,
                depth: sizes[k].1,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn units(id: u32) -> DeployUnit {
        DeployUnit {
            x: f64::from(id) * 10.0,
            z: 0.0,
            facing: 0.5,
            soldiers: 100 * (id + 1),
            width: 20.0 + f64::from(id),
            depth: 6.0,
        }
    }

    fn near(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-6, "{a} != {b}");
    }

    #[test]
    fn click_gathers_around_point_keeping_facing() {
        // Reference taken from DeploymentController.plan (GDScript): facing 0.5,
        // along = (cos .5, -sin .5), widths 20 and 21, gap 6.
        let spots = plan_deployment(
            &[0, 1],
            (100.0, 50.0),
            (100.0, 50.0),
            (0.0, 0.0),
            units,
            |_, _| (0.0, 0.0),
        );
        assert_eq!(spots.len(), 2);
        let (c, s) = (0.5_f64.cos(), 0.5_f64.sin());
        // total = 20 + 6 + 21 = 47; first offset = -23.5 + 10 = -13.5.
        near(spots[0].x, 100.0 - 13.5 * c);
        near(spots[0].z, 50.0 + 13.5 * s);
        // second offset = -23.5 + 26 + 10.5 = 13.0.
        near(spots[1].x, 100.0 + 13.0 * c);
        near(spots[1].z, 50.0 - 13.0 * s);
        assert!(spots[0].facing.is_nan() && spots[0].share == 0.0);
        near(spots[1].width, 21.0);
    }

    #[test]
    fn drag_orders_left_to_right_and_faces_away_from_camera() {
        // Drag from (0,0) to (100,0), camera at z = -50: front towards +z.
        let spots = plan_deployment(
            &[1, 0],
            (0.0, 0.0),
            (100.0, 0.0),
            (50.0, -50.0),
            units,
            |_, share| (share, 4.0),
        );
        assert_eq!(spots.iter().map(|s| s.id).collect::<Vec<_>>(), vec![0, 1]);
        near(spots[0].facing, 0.0);
        // 100 - 6 = 94 shared 100:200.
        near(spots[0].share, 94.0 / 3.0);
        near(spots[1].share, 94.0 * 2.0 / 3.0);
        // total = 94 + 6 = 100: cursor starts at -50; centre 50.
        near(spots[0].x, 50.0 - 50.0 + spots[0].share * 0.5);
        near(
            spots[1].x,
            50.0 - 50.0 + spots[0].share + 6.0 + spots[1].share * 0.5,
        );
        near(spots[0].z, 0.0);
        // Camera on the other side flips the facing.
        let flipped = plan_deployment(
            &[0],
            (0.0, 0.0),
            (100.0, 0.0),
            (50.0, 50.0),
            units,
            |_, s| (s, 4.0),
        );
        near(flipped[0].facing, std::f64::consts::PI);
    }

    #[test]
    fn short_drag_is_a_click_and_empty_selection_is_empty() {
        let spots = plan_deployment(&[0], (0.0, 0.0), (1.0, 0.0), (0.0, 0.0), units, |_, _| {
            (0.0, 0.0)
        });
        assert!(spots[0].facing.is_nan());
        assert!(
            plan_deployment(&[], (0.0, 0.0), (9.0, 0.0), (0.0, 0.0), units, |_, _| (
                0.0, 0.0
            ))
            .is_empty()
        );
    }
}
