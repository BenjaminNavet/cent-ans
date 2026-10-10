//! Staggered reinforcements (F5d): beyond the regiments a side may field at
//! once ([`crate::scale::BattleScale::max_on_field`], EP1: by tier of head
//! count, [`MAX_ON_FIELD`] on the standard field) a side
//! keeps the surplus off the field ([`Unit::reserve`]); each time one of its
//! fighting regiments is destroyed, routs or leaves, the first waiting one
//! (id order, deterministic) marches in from the side's edge of the field
//! (a garrison's from the central square).

use super::BattleSim;
use crate::setup::{EntryEdge, SideId};
use crate::unit::{Unit, UnitState};

/// Regiments a side may field at once on the standard field (the general's
/// always among them); EP1: the battle reads [`BattleSim::max_on_field`].
pub const MAX_ON_FIELD: usize = 40;

fn fighting(unit: &Unit) -> bool {
    unit.present() && !unit.synthetic && unit.state != UnitState::Routing && !unit.withdrawing
}

impl BattleSim {
    /// Marks the surplus regiments of each side as reserves (at setup,
    /// before the deployment): the last ones by id, never the general's.
    pub(super) fn hold_reserves(&mut self) {
        let max_on_field = self.max_on_field();
        for side in SideId::BOTH {
            let mut ids: Vec<usize> = (0..self.units.len())
                .filter(|&i| self.units[i].side == side && !self.units[i].synthetic)
                .collect();
            // The general first, then id order: the tail waits.
            ids.sort_by_key(|&i| (!self.units[i].is_general, i));
            // ADR 0330: timed arrivals wait apart from the surplus rule.
            let mut counted = 0;
            for &i in &ids {
                if self.units[i].arrival_s.is_some_and(|t| t > 0.0) && !self.units[i].is_general {
                    self.units[i].reserve = true;
                } else {
                    counted += 1;
                    if counted > max_on_field {
                        self.units[i].reserve = true;
                    }
                }
            }
        }
    }

    /// Seconds until the next timed arrival of `side` (ADR 0330), `None`
    /// when none is pending. HUD: « Renforts dans MM:SS ».
    pub fn next_arrival_in(&self, side: SideId) -> Option<f64> {
        self.units
            .iter()
            .filter(|u| u.side == side && u.reserve)
            .filter_map(|u| u.arrival_s)
            .map(|t| (t - self.elapsed()).max(0.0))
            .min_by(f64::total_cmp)
    }

    /// Regiments a side may field at once (EP1: by tier of head count).
    pub fn max_on_field(&self) -> usize {
        self.scale().max_on_field
    }

    /// Regiments of `side` still waiting off the field.
    pub fn reserves(&self, side: SideId) -> usize {
        self.units
            .iter()
            .filter(|u| u.side == side && u.reserve)
            .count()
    }

    /// Brings in one waiting regiment per missing fighting regiment.
    pub(super) fn release_reserves(&mut self) {
        // ADR 0330: timed arrivals enter when their hour comes, whatever
        // the number of regiments on the field.
        let now = self.elapsed();
        for i in 0..self.units.len() {
            if self.units[i].reserve && self.units[i].arrival_s.is_some_and(|t| now >= t) {
                self.march_in(i);
            }
        }
        let max_on_field = self.max_on_field();
        for side in SideId::BOTH {
            let mut fielded = self
                .units
                .iter()
                .filter(|u| u.side == side && fighting(u))
                .count();
            while fielded < max_on_field {
                let Some(i) = self
                    .units
                    .iter()
                    .position(|u| u.side == side && u.reserve && u.arrival_s.is_none())
                else {
                    break;
                };
                self.march_in(i);
                fielded += 1;
            }
        }
    }

    fn march_in(&mut self, i: usize) {
        let side = self.units[i].side;
        // Five lanes across the edge (more on a wider field).
        let lanes = (5.0 * self.field.size.sx()).round().max(1.0) as usize;
        let lane = (i % lanes) as f64 - ((lanes - 1) / 2) as f64;
        let x = self.field.width * 0.5 + lane * 110.0;
        let depth = self.field.depth;
        let (attacker_line, defender_line) =
            (self.field.attacker_line_z(), self.field.defender_line_z());
        let (z, facing, line) = match (side, self.siege.is_some()) {
            (SideId::Defender, true) => (crate::siege::TOWN_CENTER.1, std::f64::consts::PI, None),
            (SideId::Attacker, _) => (15.0, 0.0, Some(attacker_line - 60.0)),
            (SideId::Defender, false) => (
                depth - 15.0,
                std::f64::consts::PI,
                Some(defender_line + 60.0),
            ),
        };
        let x = if line.is_none() {
            crate::siege::TOWN_CENTER.0
        } else {
            x
        };
        // ADR 0330: the edge of origin of a timed arrival.
        let own = (x, z, facing, line.map(|lz| (x, lz)));
        let (x, z, facing, destination) = match self.units[i].entry_edge {
            EntryEdge::Own => own,
            EntryEdge::Rear => {
                // Behind the enemy's line: the opposite edge, facing the field.
                let (rz, rfacing, target) = match side {
                    SideId::Attacker => (
                        depth - 15.0,
                        std::f64::consts::PI,
                        defender_line + 60.0,
                    ),
                    SideId::Defender => (15.0, 0.0, attacker_line - 60.0),
                };
                (x, rz, rfacing, Some((x, target)))
            }
            EntryEdge::West | EntryEdge::East => {
                let west = self.units[i].entry_edge == EntryEdge::West;
                let line_z = match side {
                    SideId::Attacker => attacker_line,
                    SideId::Defender => defender_line,
                } + (lane * 40.0);
                let (ex, efacing, inward) = if west {
                    (15.0, std::f64::consts::FRAC_PI_2, self.field.width * 0.3)
                } else {
                    (
                        self.field.width - 15.0,
                        -std::f64::consts::FRAC_PI_2,
                        self.field.width * 0.7,
                    )
                };
                (ex, line_z, efacing, Some((inward, line_z)))
            }
        };
        let unit = &mut self.units[i];
        unit.reserve = false;
        unit.x = x;
        unit.z = z;
        unit.facing = facing;
        unit.state = UnitState::Idle;
        unit.destination = destination;
        let label = self.unit_label(i);
        self.log(
            format!("Renforts : les {label} entrent sur le champ de bataille."),
            Some(side),
        );
        let id = self.units[i].id;
        self.alert(
            crate::alerts::AlertKind::Reinforcements,
            x,
            z,
            Some(side),
            Some(id),
        );
    }
}
