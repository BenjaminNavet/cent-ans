//! CB1: formation width of a dragged `Move` and the pace of a grouped
//! order (see `crate::formation_width`).

use super::BattleSim;
use crate::formation_width::{split_widths, FormationWidthRules};
use crate::unit::{Formation, Unit, UnitState};

/// Tags given to a `match_speed` order without a `group_tag`: this bit set,
/// plus the lowest regiment id of the order (a locked group's tags, chosen
/// by the interface, stay below it).
pub const AUTO_GROUP_TAG: u32 = 0x8000_0000;

/// How a regiment walks under a `Move` order (CB1).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MoveShape {
    /// Frontage asked for on arrival (its share of the drag).
    pub width: Option<f64>,
    pub match_speed: bool,
    pub group_tag: Option<u32>,
}

/// The group tag of a `Move` of `units`: the one given, else with
/// `match_speed` one derived from the lowest regiment id.
pub(super) fn move_group_tag(units: &[u32], match_speed: bool, tag: Option<u32>) -> Option<u32> {
    match tag {
        Some(tag) => Some(tag),
        None if match_speed => units.iter().min().map(|&id| AUTO_GROUP_TAG | id),
        None => None,
    }
}

impl Unit {
    /// Pace of the regiment on open ground (m/s): its speed, running and
    /// formation, and fatigue, without terrain. The pace of a
    /// `match_speed` group is that of its slowest regiment.
    fn open_pace(&self) -> f64 {
        let mut pace = f64::from(self.stats.speed) * 0.04;
        if self.running {
            pace *= 2.0;
        }
        pace *= match self.formation {
            Formation::Column => 1.15,
            Formation::Square => 0.3,
            Formation::Line | Formation::Wedge => 1.0,
        };
        pace * (1.0 - self.fatigue / 200.0)
    }

    /// Walking under a `match_speed` group order still under way.
    fn pacing(&self) -> Option<u32> {
        (self.match_speed
            && self.destination.is_some()
            && !self.withdrawing
            && self.present()
            && self.state != UnitState::Routing)
            .then_some(self.group_tag)
            .flatten()
    }
}

impl BattleSim {
    /// Each regiment's share of a dragged `width` (in proportion to
    /// strength, `group_gap_m` between neighbours); `None` without width.
    pub(crate) fn move_widths(&self, units: &[u32], width: Option<f64>) -> Option<Vec<f64>> {
        let width = width.filter(|w| w.is_finite() && *w > 0.0)?;
        if units.len() == 1 {
            return Some(vec![width]);
        }
        let counts: Vec<u32> = units
            .iter()
            .map(|&id| self.units[id as usize].soldiers())
            .collect();
        Some(split_widths(
            &counts,
            width,
            FormationWidthRules::bundled().group_gap_m,
        ))
    }

    /// Frontages the regiments take for their shares `widths`.
    pub(crate) fn move_frontages(&self, units: &[u32], widths: Option<&[f64]>) -> Option<Vec<f64>> {
        let widths = widths?;
        Some(
            units
                .iter()
                .zip(widths)
                .map(|(&id, &w)| self.units[id as usize].extent_for_width(Some(w)).0)
                .collect(),
        )
    }

    /// Factor (≤ 1) bringing `unit` down to the pace of the slowest
    /// regiment walking under the same `match_speed` group order.
    pub(super) fn group_pace_factor(&self, unit: &Unit) -> f64 {
        let Some(tag) = unit.pacing() else {
            return 1.0;
        };
        let own = unit.open_pace();
        if own <= 0.0 {
            return 1.0;
        }
        let slowest = self
            .units
            .iter()
            .filter(|u| u.pacing() == Some(tag))
            .map(Unit::open_pace)
            .fold(own, f64::min);
        (slowest / own).min(1.0)
    }
}
