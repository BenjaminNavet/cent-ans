//! Formation width set by a right-drag (lot CB1, spec
//! `docs/superpowers/specs/2026-09-27-controles-bataille-tw-design.md`,
//! « Formation au glisser », plan écart 5).
//!
//! A drag gives the frontage of a regiment in Line: [`Unit::line_files`]
//! (files = width / lateral spacing) replaces the default depth of the Line
//! (4 ranks of foot, 3 of shooters, 2 of horse). The ranks that follow stay
//! within per-class bounds (`data/rules/formation_width.json`: pikemen keep
//! at least 4 ranks, archers 2...), so a width out of bounds is brought back
//! to them; every soldier keeps a place (ranks × files ≥ soldiers). A group
//! drag shares the width between the regiments in proportion to their
//! strength ([`split_widths`]).

use data_model::{Ability, UnitCategory};
use serde::Deserialize;

use crate::unit::Unit;

/// Rank bounds of one class of troops.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RankBounds {
    pub min_ranks: u32,
    pub max_ranks: u32,
}

/// Classes of `data/rules/formation_width.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WidthClasses {
    pub pikemen: RankBounds,
    pub infantry: RankBounds,
    pub ranged: RankBounds,
    pub cavalry: RankBounds,
    pub siege: RankBounds,
}

/// `data/rules/formation_width.json`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormationWidthRules {
    #[serde(default)]
    pub description: String,
    /// Space between two regiments of a dragged group line (metres),
    /// taken off the drag before it is shared out.
    pub group_gap_m: f64,
    pub classes: WidthClasses,
}

data_model::bundled_rules!(FormationWidthRules, "rules/formation_width.json");

impl FormationWidthRules {

    /// Rank bounds of `unit`: engines, then horsemen, then pikemen
    /// (`pike_square`), then shooters, then foot.
    pub fn bounds(&self, unit: &Unit) -> RankBounds {
        let c = &self.classes;
        let b = if unit.category == UnitCategory::Siege {
            c.siege
        } else if unit.mounted {
            c.cavalry
        } else if unit.has(Ability::PikeSquare) {
            c.pikemen
        } else if unit.category == UnitCategory::Ranged {
            c.ranged
        } else {
            c.infantry
        };
        RankBounds {
            min_ranks: b.min_ranks.max(1),
            max_ranks: b.max_ranks.max(b.min_ranks).max(1),
        }
    }
}

/// `(ranks, files)` of a Line of `n` soldiers asked `files` wide, the
/// ranks brought within `bounds` (never more ranks than soldiers).
pub fn line_shape(n: u32, files: u32, bounds: RankBounds) -> (u32, u32) {
    let n = n.max(1);
    let files = files.clamp(1, n);
    let ranks = n.div_ceil(files);
    let clamped = ranks.clamp(bounds.min_ranks, bounds.max_ranks).min(n);
    if clamped == ranks {
        (ranks, files)
    } else {
        (clamped, n.div_ceil(clamped))
    }
}

/// Shares a group drag `total` metres wide between regiments of `counts`
/// soldiers (in their left-to-right order): `gap` metres between two
/// neighbours, the rest in proportion to strength. Mirrored by
/// `FormationDrag.split_widths` on the Godot side (deployment).
pub fn split_widths(counts: &[u32], total: f64, gap: f64) -> Vec<f64> {
    if counts.is_empty() {
        return Vec::new();
    }
    let n = counts.len() as f64;
    let share = (total - gap * (n - 1.0)).max(0.0);
    let sum: f64 = counts.iter().map(|&c| f64::from(c.max(1))).sum();
    counts
        .iter()
        .map(|&c| share * f64::from(c.max(1)) / sum)
        .collect()
}

impl Unit {
    /// CB1: files of a Line `width` metres wide, brought within the rank
    /// bounds of the regiment (`None` for a width that is not a positive
    /// number: the order leaves the formation as it is).
    pub fn files_for_width(&self, width: f64) -> Option<u32> {
        if !(width.is_finite() && width > 0.0) {
            return None;
        }
        let n = self.soldiers().max(1);
        let (sx, _) = self.spacing_in(self.drag_formation());
        let asked = ((width / sx).round() as u32).max(1);
        let bounds = FormationWidthRules::bundled().bounds(self);
        let (_, files) = line_shape(n, asked, bounds);
        Some(files)
    }

    /// CB1: frontage and depth (metres) of the Line this regiment takes
    /// when ordered `width` metres wide (its present extent without width).
    pub fn extent_for_width(&self, width: Option<f64>) -> (f64, f64) {
        let Some(files) = width.and_then(|w| self.files_for_width(w)) else {
            return self.extent();
        };
        let bounds = FormationWidthRules::bundled().bounds(self);
        let (ranks, files) = line_shape(self.soldiers(), files, bounds);
        let (sx, sz) = self.spacing_in(self.drag_formation());
        (f64::from(files) * sx, f64::from(ranks) * sz)
    }

    /// RJ-a: the formation a drag leaves the regiment in: its own when
    /// line-shaped (`width_adjustable`), the default line otherwise.
    pub fn drag_formation(&self) -> crate::unit::Formation {
        if self.formation.def().width_adjustable {
            self.formation
        } else {
            crate::unit::Formation::default_formation()
        }
    }

    /// CB1: takes the Line `width` metres wide (no change without width).
    pub fn set_width(&mut self, width: Option<f64>) {
        if let Some(files) = width.and_then(|w| self.files_for_width(w)) {
            // RJ-a: a line-shaped formation keeps its name under a drag;
            // any other goes back to the default line.
            if !self.formation.def().width_adjustable {
                self.formation = self.drag_formation();
                self.reform = None;
            }
            self.line_files = Some(files);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_rules_load() {
        let rules = FormationWidthRules::bundled();
        assert!(rules.classes.pikemen.min_ranks >= 4);
        assert!(rules.group_gap_m >= 0.0);
    }

    #[test]
    fn line_shape_keeps_every_soldier() {
        let b = RankBounds {
            min_ranks: 2,
            max_ranks: 8,
        };
        for n in 1..200 {
            for files in 1..120 {
                let (r, f) = line_shape(n, files, b);
                assert!(r * f >= n, "n {n} files {files}");
                assert!(r <= b.max_ranks.max(1));
                assert!(r >= b.min_ranks.min(n));
            }
        }
    }

    #[test]
    fn split_widths_is_proportional() {
        let w = split_widths(&[100, 300], 110.0, 10.0);
        assert!((w[0] - 25.0).abs() < 1e-9 && (w[1] - 75.0).abs() < 1e-9);
        assert!(split_widths(&[], 10.0, 1.0).is_empty());
    }
}
