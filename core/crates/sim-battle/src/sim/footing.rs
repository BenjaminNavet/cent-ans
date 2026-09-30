//! Figures on narrow footings: a bridge deck and a wall walk. The rules keep
//! the regiment's rectangle (contact, squeeze on a bridge); the renderer
//! would otherwise draw that rectangle as is, wider than the deck (men
//! walking on the water beside the bridge) or deeper than the wall walk
//! (men standing in the air on both sides of the wall). These layouts file
//! the same figures along the footing instead. Rendering only.

use super::BattleSim;
use crate::hydro::Bridge;
use crate::rng::jitter;
use crate::siege::SiegeWorks;
use crate::unit::{Unit, UnitState};

/// Margin kept between the outer files and the edge of a bridge deck (m).
const DECK_MARGIN: f64 = 0.5;
/// Free width of the wall walk: clear of the inner edge, and of the merlons
/// on the outer edge (m, see `battle_siege.gd`).
const WALK_INNER_MARGIN: f64 = 0.4;
const WALK_OUTER_MARGIN: f64 = 1.0;
/// Spacing of the ranks across the wall walk (m).
const WALK_RANK_SPACING: f64 = 0.75;
/// Two wall pieces meet when their ends are closer than this (m).
const JOINT_TOLERANCE: f64 = 0.5;

impl BattleSim {
    /// `(x, y, z, facing)` of the figures of `unit` when it stands on a wall
    /// walk or crosses a bridge, `None` elsewhere. `positions` are the
    /// formation's figures (their count and headings are kept).
    pub(super) fn footing_poses(
        &self,
        unit: &Unit,
        positions: &[(f64, f64, f64)],
    ) -> Option<Vec<[f64; 4]>> {
        if positions.is_empty() || unit.climbing.is_some() {
            return None;
        }
        if unit.on_wall {
            if let Some(works) = &self.siege {
                return wall_walk_poses(works, &self.field, unit, positions);
            }
        }
        let bridge = self.crossed_bridge(unit)?;
        Some(self.bridge_column_poses(bridge, unit, positions))
    }

    /// The bridge `unit` is crossing: its centre on the deck, or within half
    /// its depth of either end.
    fn crossed_bridge(&self, unit: &Unit) -> Option<&Bridge> {
        let (_, depth) = unit.extent();
        self.field.bridges.iter().find(|b| {
            let (along, across) = b.local(unit.x, unit.z);
            across.abs() <= b.width * 0.5 + DECK_MARGIN
                && along.abs() <= b.length * 0.5 + depth * 0.5
        })
    }

    /// A column along the bridge axis, as many files as the deck holds,
    /// centred on the regiment and led by its first rank.
    fn bridge_column_poses(
        &self,
        bridge: &Bridge,
        unit: &Unit,
        positions: &[(f64, f64, f64)],
    ) -> Vec<[f64; 4]> {
        let (sx, sz) = unit.spacing();
        let usable = (bridge.width - 2.0 * DECK_MARGIN).max(0.0);
        let files = ((usable / sx).floor() as usize + 1).max(1);
        let n = positions.len();
        let rows = n.div_ceil(files);
        let files = files.min(n);
        let (fx, fz) = unit.forward();
        let ahead = if fx * bridge.dir.0 + fz * bridge.dir.1 >= 0.0 {
            1.0
        } else {
            -1.0
        };
        let (along0, _) = bridge.local(unit.x, unit.z);
        let file_step = if files > 1 {
            (usable / (files - 1) as f64).min(sx)
        } else {
            0.0
        };
        let spread = if unit.state == UnitState::Routing {
            0.6
        } else {
            0.25
        };
        let id = u64::from(unit.id);
        positions
            .iter()
            .enumerate()
            .map(|(i, &(_, _, angle))| {
                let (row, file) = (i / files, i % files);
                let jx = jitter(id, i as u64 * 2) * spread;
                let jz = jitter(id, i as u64 * 2 + 1) * spread;
                let along = along0 + ahead * (((rows as f64) - 1.0) * 0.5 - row as f64) * sz + jz;
                let across = ((file as f64 - (files as f64 - 1.0) * 0.5) * file_step + jx)
                    .clamp(-usable * 0.5, usable * 0.5);
                let x = bridge.x + bridge.dir.0 * along - bridge.dir.1 * across;
                let z = bridge.z + bridge.dir.1 * along + bridge.dir.0 * across;
                [x, self.field.walk_height(x, z), z, angle]
            })
            .collect()
    }
}

/// A strip along the wall walk, centred on the regiment and running on to
/// the next pieces of the enceinte; its first rank at the parapet.
fn wall_walk_poses(
    works: &SiegeWorks,
    field: &crate::field::Battlefield,
    unit: &Unit,
    positions: &[(f64, f64, f64)],
) -> Option<Vec<[f64; 4]>> {
    let start = (0..works.pieces.len()).min_by(|&a, &b| {
        let da = works.pieces[a].distance(unit.x, unit.z);
        let db = works.pieces[b].distance(unit.x, unit.z);
        da.total_cmp(&db)
    })?;
    let (sx, _) = unit.spacing();
    let usable = (works.thickness - WALK_INNER_MARGIN - WALK_OUTER_MARGIN).max(0.0);
    let ranks = ((usable / WALK_RANK_SPACING).floor() as usize + 1).max(1);
    let n = positions.len();
    let files = n.div_ceil(ranks);
    let ranks = ranks.min(n);
    // Across the walk: 0 on the centre line, positive outward.
    let centre = (WALK_INNER_MARGIN - WALK_OUTER_MARGIN) * 0.5;
    let piece = &works.pieces[start];
    let (cx, cz) = piece.closest_point(unit.x, unit.z);
    let s0 = (cx - piece.a.0).hypot(cz - piece.a.1);
    let id = u64::from(unit.id);
    let spread = if unit.state == UnitState::Routing {
        0.4
    } else {
        0.15
    };
    Some(
        positions
            .iter()
            .enumerate()
            .map(|(i, &(_, _, angle))| {
                let (rank, file) = (i / files, i % files);
                let jx = jitter(id, i as u64 * 2) * spread;
                let jz = jitter(id, i as u64 * 2 + 1) * spread;
                let s = s0 + (file as f64 - (files as f64 - 1.0) * 0.5) * sx + jx;
                let (p, (px, pz)) = walk_along(works, start, s);
                let piece = &works.pieces[p];
                let (nx, nz) = piece.outward();
                let off =
                    (centre + ((ranks as f64 - 1.0) * 0.5 - rank as f64) * WALK_RANK_SPACING + jz)
                        .clamp(-usable * 0.5 + centre, usable * 0.5 + centre);
                let top = field
                    .height(piece.a.0, piece.a.1)
                    .min(field.height(piece.b.0, piece.b.1))
                    + works.wall_height;
                [px + nx * off, top, pz + nz * off, angle]
            })
            .collect(),
    )
}

/// The piece and the point of the enceinte's centre line at distance `s`
/// from `a` of piece `start` (negative before it), following the joined
/// pieces; clamped at an open end.
fn walk_along(works: &SiegeWorks, start: usize, s: f64) -> (usize, (f64, f64)) {
    let pieces = &works.pieces;
    let near = |p: (f64, f64), q: (f64, f64)| (p.0 - q.0).hypot(p.1 - q.1) <= JOINT_TOLERANCE;
    // Current piece, entered at `from` (one of its ends), remaining distance.
    let (mut current, mut from, mut rest) = if s >= 0.0 {
        (start, pieces[start].a, s)
    } else {
        (start, pieces[start].b, pieces[start].length() - s)
    };
    let point = |p: usize, from: (f64, f64), d: f64| {
        let piece = &pieces[p];
        let to = if near(from, piece.a) {
            piece.b
        } else {
            piece.a
        };
        let len = piece.length().max(1e-9);
        let t = (d / len).clamp(0.0, 1.0);
        (from.0 + (to.0 - from.0) * t, from.1 + (to.1 - from.1) * t)
    };
    for _ in 0..pieces.len() {
        let len = pieces[current].length();
        if rest <= len {
            break;
        }
        let exit = if near(from, pieces[current].a) {
            pieces[current].b
        } else {
            pieces[current].a
        };
        let Some(next) = (0..pieces.len())
            .find(|&q| q != current && (near(pieces[q].a, exit) || near(pieces[q].b, exit)))
        else {
            break;
        };
        rest -= len;
        current = next;
        from = exit;
    }
    (current, point(current, from, rest))
}
