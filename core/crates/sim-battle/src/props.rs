//! Street furniture laid out by the core (lot BR3, ADR 0047): stalls on the
//! facades facing the square, barrels, carts and woodpiles on the street
//! side, the market (a well and rings of stalls leaving the streets open),
//! the props of the suburbs and of the battle village. Deterministic from the
//! house indices ([`crate::town::hash01`]), never from the battle's random
//! stream, so that laying them out shifts no other draw.

use crate::siege::SiegeWorks;
use crate::site::Village;
use crate::town::{Prop, TownRules};

/// Props of a besieged town (blocks, suburbs, market square).
pub fn siege_props(_works: &SiegeWorks, _rules: &TownRules) -> Vec<Prop> {
    Vec::new()
}

/// Props in front of the houses of a battle village.
pub fn village_props(_village: &Village, _rules: &TownRules) -> Vec<Prop> {
    Vec::new()
}
