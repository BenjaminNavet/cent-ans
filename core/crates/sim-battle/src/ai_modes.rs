//! CB2: unit modes chosen by the field-battle AI (plan
//! `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md`, CB2).
//!
//! On the defensive, light shooters (armour at most
//! `ai.light_shooter_max_armor` of `data/rules/unit_modes.json`, neither
//! stakes nor pavises: horse archers, jinetes) skirmish, and the line holds
//! on guard while it waits (a regiment sent at an enemy drops it: the AI
//! keeps its own pursuit); advancing, the line drops the guard. The longbows
//! behind their stakes and the crossbows behind their pavises keep their
//! ground: their own falling back is the AI's (`plan_shooter`).

use data_model::{Ability, UnitCategory};

use super::{Roles, View};
use crate::command::Command;
use crate::modes::{UnitMode, UnitModeRules};
use crate::unit::{Unit, UnitState};

/// Light shooter: skirmishes on the defensive.
fn light_shooter(unit: &Unit) -> bool {
    unit.can_shoot()
        && unit.category != UnitCategory::Siege
        && !unit.has(Ability::Stakes)
        && !unit.has(Ability::Pavise)
        && u32::from(unit.stats.armor) <= UnitModeRules::bundled().ai.light_shooter_max_armor
}

/// Emits a `set_mode` for regiment `i` when `mode` is not already `on`.
fn want(view: &mut View, i: usize, mode: UnitMode, on: bool) {
    let unit = &view.units[i];
    if unit.mode(mode) == on || !view.sim.mode_available(unit, mode) {
        return;
    }
    view.commands.push(Command::SetMode {
        units: vec![unit.id],
        mode,
        enabled: on,
    });
}

/// Modes of the side's regiments for this decision step.
pub(super) fn plan_modes(view: &mut View, roles: &Roles, defensive: bool) {
    // The line waits on guard; a regiment the AI sends at an enemy (counter-
    // charge, rescue) or caught in a melee drops it, so that the AI keeps
    // its own pursuit (with the guard kept in the fight, Poitiers fell from
    // 14 to 5 English victories out of 20).
    let line = roles.line.clone();
    for i in line {
        let u = &view.units[i];
        let waiting = u.target.is_none() && u.state != UnitState::Melee;
        want(view, i, UnitMode::Guard, defensive && waiting);
    }
    if defensive {
        let shooters: Vec<usize> = roles
            .shooters
            .iter()
            .chain(roles.horse.iter())
            .copied()
            .filter(|&i| light_shooter(&view.units[i]))
            .collect();
        for i in shooters {
            want(view, i, UnitMode::Skirmish, true);
        }
    }
}
