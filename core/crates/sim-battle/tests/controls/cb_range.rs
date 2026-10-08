//! CB-M4: the shooting range drawn on the ground (`ground_range`) and the
//! drawn fire sector (`battle_hover.json`, `range_arc`).

use crate::common;

use common::*;
use sim_battle::{BattleSim, HoverRules, SideId};

fn range_lab() -> BattleSim {
    let data = data();
    let attacker = units(data, &["unit_longbowmen", "unit_knights"]);
    let defender = units(data, &["unit_crossbowmen", "unit_urban_militia"]);
    let mut sim = BattleSim::new(setup(attacker, defender, None), 7).unwrap();
    lab(&mut sim);
    sim
}

fn index_of(sim: &BattleSim, unit_type: &str) -> usize {
    sim.units()
        .iter()
        .position(|u| *u.unit_type == *unit_type)
        .unwrap_or_else(|| panic!("no {unit_type}"))
}

#[test]
fn ground_range_is_the_effective_range_of_shooters_only() {
    let mut sim = range_lab();
    let bows = index_of(&sim, "unit_longbowmen");
    let horse = index_of(&sim, "unit_knights");
    let unit = &sim.units()[bows];
    let range = sim.ground_range(unit);
    assert!(range > 0.0, "longbowmen shoot");
    assert_eq!(range, sim.effective_range(unit, unit.x, unit.z));
    assert_eq!(
        sim.ground_range(&sim.units()[horse]),
        0.0,
        "knights do not shoot"
    );
    // The comparison's range line is the same rule, aimed at the enemy.
    let foe = sim
        .units()
        .iter()
        .find(|u| u.side == SideId::Defender)
        .unwrap()
        .clone();
    let compare = sim.compare(unit, &foe);
    assert_eq!(compare.ours.range, sim.effective_range(unit, foe.x, foe.z));
    // Out of missiles: nothing to draw.
    sim.units_mut()[bows].ammo = 0;
    assert_eq!(sim.ground_range(&sim.units()[bows]), 0.0, "no arrows left");
}

#[test]
fn fire_sector_comes_from_the_rules() {
    let rules = &HoverRules::bundled().range_arc;
    assert!(rules.fire_half_angle_deg > 0.0 && rules.fire_half_angle_deg < 90.0);
    assert!((rules.fire_half_angle() - rules.fire_half_angle_deg.to_radians()).abs() < 1e-12);
}
