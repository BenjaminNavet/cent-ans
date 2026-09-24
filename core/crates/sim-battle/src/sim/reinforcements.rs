//! Staggered reinforcements (F5d): beyond [`MAX_ON_FIELD`] regiments a side
//! keeps the surplus off the field ([`Unit::reserve`]); each time one of its
//! fighting regiments is destroyed, routs or leaves, the first waiting one
//! (id order, deterministic) marches in from the side's edge of the field
//! (a garrison's from the central square).

use super::BattleSim;
use crate::field::{ATTACKER_LINE_Z, DEFENDER_LINE_Z};
use crate::setup::SideId;
use crate::unit::{Unit, UnitState};

/// Regiments a side may field at once (the general's always among them).
pub const MAX_ON_FIELD: usize = 20;

fn fighting(unit: &Unit) -> bool {
    unit.present() && !unit.synthetic && unit.state != UnitState::Routing && !unit.withdrawing
}

impl BattleSim {
    /// Marks the surplus regiments of each side as reserves (at setup,
    /// before the deployment): the last ones by id, never the general's.
    pub(super) fn hold_reserves(&mut self) {
        for side in SideId::BOTH {
            let mut ids: Vec<usize> = (0..self.units.len())
                .filter(|&i| self.units[i].side == side && !self.units[i].synthetic)
                .collect();
            // The general first, then id order: the tail waits.
            ids.sort_by_key(|&i| (!self.units[i].is_general, i));
            for &i in ids.iter().skip(MAX_ON_FIELD) {
                self.units[i].reserve = true;
            }
        }
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
        for side in SideId::BOTH {
            let mut fielded = self
                .units
                .iter()
                .filter(|u| u.side == side && fighting(u))
                .count();
            while fielded < MAX_ON_FIELD {
                let Some(i) = self.units.iter().position(|u| u.side == side && u.reserve) else {
                    break;
                };
                self.march_in(i);
                fielded += 1;
            }
        }
    }

    fn march_in(&mut self, i: usize) {
        let side = self.units[i].side;
        // Five lanes across the edge.
        let lane = (i % 5) as f64 - 2.0;
        let x = self.field.width * 0.5 + lane * 110.0;
        let depth = self.field.depth;
        let (z, facing, line) = match (side, self.siege.is_some()) {
            (SideId::Defender, true) => (crate::siege::TOWN_CENTER.1, std::f64::consts::PI, None),
            (SideId::Attacker, _) => (15.0, 0.0, Some(ATTACKER_LINE_Z - 60.0)),
            (SideId::Defender, false) => (
                depth - 15.0,
                std::f64::consts::PI,
                Some(DEFENDER_LINE_Z + 60.0),
            ),
        };
        let x = if line.is_none() {
            crate::siege::TOWN_CENTER.0
        } else {
            x
        };
        let unit = &mut self.units[i];
        unit.reserve = false;
        unit.x = x;
        unit.z = z;
        unit.facing = facing;
        unit.state = UnitState::Idle;
        unit.destination = line.map(|lz| (x, lz));
        let label = self.unit_label(i);
        self.log(
            format!("Renforts : les {label} entrent sur le champ de bataille."),
            Some(side),
        );
    }
}
