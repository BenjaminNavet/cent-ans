//! CB2: unit modes and display states on the GDExtension side (plan
//! `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md`, CB2). The
//! mode entries of `get_units`; the modes are set by `issue_command`
//! (`{type: "set_mode", units: [ids], mode: "run"|"guard"|"skirmish"
//! |"melee"|"breach", enabled: bool}`).

use godot::prelude::*;
use sim_battle::Unit;

/// Adds to `dict` (one `get_units` entry): `mode_run`, `guard`, `skirmish`,
/// `melee_mode`, `breach` (modes on), `modes` (keys of the modes the
/// regiment may take), and the states computed by the core: `charging`,
/// `under_fire`, `engaged`, `wavering`.
pub(crate) fn add_mode_fields(sim: &sim_battle::BattleSim, unit: &Unit, dict: &mut VarDictionary) {
    dict.set("mode_run", unit.mode_run);
    dict.set("guard", unit.guard);
    dict.set("skirmish", unit.skirmish);
    dict.set("melee_mode", unit.melee_mode);
    dict.set("breach", unit.breach);
    let modes: PackedStringArray = sim
        .available_modes(unit)
        .iter()
        .map(|mode| GString::from(mode.key()))
        .collect();
    dict.set("modes", &modes);
    let status = sim.unit_status(unit);
    dict.set("charging", status.charging);
    dict.set("under_fire", status.under_fire);
    dict.set("engaged", status.engaged);
    dict.set("wavering", status.wavering);
}
