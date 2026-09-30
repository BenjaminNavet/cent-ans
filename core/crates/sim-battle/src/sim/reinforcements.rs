//! Staggered reinforcements (F5d): beyond the regiments a side may field at
//! once ([`crate::scale::BattleScale::max_on_field`], EP1: by tier of head
//! count, [`MAX_ON_FIELD`] on the standard field) a side
//! keeps the surplus off the field ([`Unit::reserve`]); each time one of its
//! fighting regiments is destroyed, routs or leaves, the first waiting one
//! (id order, deterministic) marches in from the side's edge of the field
//! (a garrison's from the central square).

use super::BattleSim;
use crate::setup::SideId;
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
            for &i in ids.iter().skip(max_on_field) {
                self.units[i].reserve = true;
            }
        }
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
        let max_on_field = self.max_on_field();
        for side in SideId::BOTH {
            let mut fielded = self
                .units
                .iter()
                .filter(|u| u.side == side && fighting(u))
                .count();
            while fielded < max_on_field {
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
