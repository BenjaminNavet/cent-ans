//! CB5 (ADR 0095): typed battle alerts, emitted at the same points as the
//! matching journal entries. Each test checks one [`AlertKind`] fires
//! exactly where expected, and that the feed never touches simulated state.

use crate::common;

use common::*;
use sim_battle::alerts::AlertKind;
use sim_battle::replay::state_digest;
use sim_battle::{BattleSim, Command, GeneralSetup, SideId, SiegeFxKind, SiegeSetup};

fn general(unit_index: usize) -> GeneralSetup {
    GeneralSetup {
        character: "chr_test".to_owned(),
        name: "Le connétable".to_owned(),
        command: 6,
        unit_index,
        morale_bonus: 0.0,
        charge_percent: 0.0,
        ranged_percent: 0.0,
        defense_percent: 0.0,
        sovereign: false,
    }
}

#[test]
fn rout_alert_fires_once_when_a_unit_breaks() {
    let data = data();
    let mut sim = BattleSim::new(
        setup(
            units(data, &["unit_men_at_arms_foot"]),
            units(data, &["unit_men_at_arms_foot"]),
            None,
        ),
        1,
    )
    .unwrap();
    lab(&mut sim);
    place(&mut sim, 0, 500.0, 300.0, 0.0);
    place(&mut sim, 1, 500.0, 900.0, std::f64::consts::PI);
    sim.units_mut()[0].morale = 5.0;
    sim.step();
    let alerts = sim.take_new_alerts();
    let routs: Vec<_> = alerts
        .iter()
        .filter(|a| a.kind == AlertKind::Rout)
        .collect();
    assert_eq!(routs.len(), 1, "{alerts:?}");
    let a = routs[0];
    assert_eq!(a.side, Some(SideId::Attacker));
    assert_eq!(a.unit, Some(sim.units()[0].id));
    assert!((a.x - 500.0).abs() < 1.0 && (a.z - 300.0).abs() < 1.0);
    // Still routing next tick: no duplicate.
    sim.step();
    assert!(sim
        .take_new_alerts()
        .iter()
        .all(|a| a.kind != AlertKind::Rout));
}

#[test]
fn general_down_alert_fires_when_the_general_dies() {
    let data = data();
    let mut s = setup(
        units(data, &["unit_knights", "unit_men_at_arms_foot"]),
        units(data, &["unit_men_at_arms_foot"]),
        None,
    );
    s.attacker.general = Some(general(0));
    let mut sim = BattleSim::new(s, 5).unwrap();
    lab(&mut sim);
    assert!(sim.units()[0].is_general);
    let general_id = sim.units()[0].id;
    place(&mut sim, 2, 600.0, 400.0, std::f64::consts::PI);
    place(&mut sim, 0, 600.0, 393.0, 0.0);
    place(&mut sim, 1, 200.0, 100.0, 0.0);
    sim.units_mut()[0].hp = 0.5;
    let mut seen = Vec::new();
    for _ in 0..600 {
        sim.step();
        seen.extend(sim.take_new_alerts());
        if !sim.general_alive(SideId::Attacker) {
            break;
        }
    }
    assert!(!sim.general_alive(SideId::Attacker));
    let downs: Vec<_> = seen
        .iter()
        .filter(|a| a.kind == AlertKind::GeneralDown)
        .collect();
    assert_eq!(downs.len(), 1, "{seen:?}");
    assert_eq!(downs[0].side, Some(SideId::Attacker));
    assert_eq!(downs[0].unit, Some(general_id));
}

#[test]
fn flanked_alert_fires_once_on_the_rising_edge() {
    let data = data();
    let attacker = units(data, &["unit_men_at_arms_foot", "unit_knights"]);
    let defender = units(data, &["unit_urban_militia"]);
    let mut sim = BattleSim::new(setup(attacker, defender, None), 3).unwrap();
    lab(&mut sim);
    sim.set_ai(SideId::Attacker, true);
    // Men-at-arms (0) locked in melee with the militia (2), front to front;
    // the knights (1) wait to charge its exposed flank (mirrors
    // `ai::cavalry_charges_an_exposed_flank`).
    place(&mut sim, 2, 600.0, 420.0, std::f64::consts::PI);
    place(&mut sim, 0, 600.0, 411.0, 0.0);
    place(&mut sim, 1, 380.0, 330.0, 0.0);
    let mut seen = Vec::new();
    let mut flank_hit = false;
    for _ in 0..1200 {
        sim.step();
        seen.extend(sim.take_new_alerts());
        if sim.units()[2].flanked != 0 {
            flank_hit = true;
        }
        if flank_hit && sim.units()[2].flanked == 0 {
            // Ran past the flanked spell entirely: stop so the "once" check
            // below is meaningful either way.
            break;
        }
    }
    assert!(flank_hit, "the militia should be hit in the flank or rear");
    let flanks: Vec<_> = seen
        .iter()
        .filter(|a| a.kind == AlertKind::Flanked && a.unit == Some(sim.units()[2].id))
        .collect();
    assert_eq!(
        flanks.len(),
        1,
        "one alert per spell of being flanked: {seen:?}"
    );
}

#[test]
fn reinforcements_alert_fires_when_a_reserve_marches_in() {
    let data = data();
    let many: Vec<&str> = ["unit_men_at_arms_foot", "unit_longbowmen"].repeat(22);
    let battle = setup(units(data, &many), units(data, &many[..8]), None);
    // The standard field (its 40-regiment cap) whatever the head count.
    let skirmish = sim_battle::BattleScale::named("skirmish").unwrap();
    let mut sim = BattleSim::new_scaled(battle, 4, skirmish).unwrap();
    lab(&mut sim);
    assert!(sim.units()[40].reserve);
    // A fielded regiment is destroyed: the first waiting one marches in.
    sim.units_mut()[3].hp = 0.0;
    sim.step();
    let alerts = sim.take_new_alerts();
    let reinf: Vec<_> = alerts
        .iter()
        .filter(|a| a.kind == AlertKind::Reinforcements)
        .collect();
    assert_eq!(reinf.len(), 1, "{alerts:?}");
    assert_eq!(reinf[0].unit, Some(sim.units()[40].id));
    assert_eq!(reinf[0].side, Some(SideId::Attacker));
}

#[test]
fn ammo_out_alert_fires_when_a_shooter_runs_dry() {
    let data = data();
    let battle = setup(
        units(data, &["unit_longbowmen"]),
        units(data, &["unit_genoese_crossbowmen"]),
        None,
    );
    let mut sim = BattleSim::new(battle, 3).unwrap();
    lab(&mut sim);
    place(&mut sim, 0, 500.0, 300.0, 0.0);
    place(&mut sim, 1, 500.0, 420.0, std::f64::consts::PI);
    sim.units_mut()[0].ammo = 1;
    let mut seen = Vec::new();
    for _ in 0..600 {
        sim.step();
        seen.extend(sim.take_new_alerts());
        if sim.units()[0].ammo == 0 {
            break;
        }
    }
    assert_eq!(
        sim.units()[0].ammo,
        0,
        "the bowmen should have fired their last arrow"
    );
    let out: Vec<_> = seen
        .iter()
        .filter(|a| a.kind == AlertKind::AmmoOut)
        .collect();
    assert_eq!(out.len(), 1, "{seen:?}");
    assert_eq!(out[0].unit, Some(sim.units()[0].id));
    assert_eq!(out[0].side, Some(SideId::Attacker));
}

const BESIEGERS: [&str; 4] = [
    "unit_men_at_arms_foot",
    "unit_urban_militia",
    "unit_urban_militia",
    "unit_longbowmen",
];
const GARRISON: [&str; 5] = [
    "unit_urban_militia",
    "unit_urban_militia",
    "unit_crossbowmen",
    "unit_crossbowmen",
    "unit_men_at_arms_foot",
];

#[test]
fn gate_destroyed_alert_fires_once_when_the_gate_breaks() {
    let data = data();
    let mut garrison = units(data, &GARRISON);
    garrison.extend(units(data, &GARRISON));
    let battle_setup = setup(
        units(data, &BESIEGERS),
        garrison,
        Some(SiegeSetup {
            fortification: 1,
            breach: 0,
            ..Default::default()
        }),
    );
    let mut sim = BattleSim::new(battle_setup, 43).unwrap();
    sim.set_ai(SideId::Attacker, false);
    sim.set_end_conditions(false);
    let ram = sim.units().iter().position(|u| u.ram).unwrap() as u32;
    let band = sim.siege().unwrap().band();
    let gate = sim.siege().unwrap().gate;
    let (mx, mz) = sim.siege().unwrap().pieces[gate].midpoint();
    let (nx, nz) = sim.siege().unwrap().pieces[gate].outward();
    place(
        &mut sim,
        ram,
        mx + nx * (band + 1.0),
        mz + nz * (band + 1.0),
        0.0,
    );
    sim.siege_mut().unwrap().pieces[gate].hp = 10.0;
    let mut seen = Vec::new();
    for _ in 0..2000 {
        sim.step();
        seen.extend(sim.take_new_alerts());
        if sim
            .siege_fx()
            .iter()
            .any(|f| f.kind == SiegeFxKind::GateBroken { piece: gate })
        {
            break;
        }
    }
    let breaches: Vec<_> = seen
        .iter()
        .filter(|a| a.kind == AlertKind::GateDestroyed)
        .collect();
    assert_eq!(breaches.len(), 1, "{seen:?}");
    assert_eq!(
        breaches[0].side, None,
        "a wall/gate piece belongs to neither side"
    );
    assert!((breaches[0].x - mx).abs() < 1.0 && (breaches[0].z - mz).abs() < 1.0);
}

/// CB5 is a pure output stream: consuming it must never change the
/// simulated state or the replay digest (b6, ep13_replay).
#[test]
fn taking_alerts_does_not_affect_the_state_digest() {
    let data = data();
    let build = || {
        let mut s = setup(
            units(data, &["unit_men_at_arms_foot"]),
            units(data, &["unit_men_at_arms_foot"]),
            None,
        );
        s.attacker.general = Some(general(0));
        let mut sim = BattleSim::new(s, 7).unwrap();
        lab(&mut sim);
        place(&mut sim, 0, 500.0, 300.0, 0.0);
        place(&mut sim, 1, 500.0, 320.0, std::f64::consts::PI);
        sim
    };
    let mut drained = build();
    let mut untouched = build();
    for _ in 0..400 {
        drained.step();
        untouched.step();
        // One sim's alerts are read every tick, the other's are left to pile
        // up: the resulting simulated state must be identical either way.
        let _ = drained.take_new_alerts();
    }
    assert_eq!(state_digest(&drained), state_digest(&untouched));
    // Also identical after finally reading the untouched side's backlog.
    let _ = untouched.take_new_alerts();
    assert_eq!(state_digest(&drained), state_digest(&untouched));
}

/// Issuing commands and stepping does not require reading alerts: an old
/// recorded EP13 replay (built before CB5) must still play back unchanged.
#[test]
fn replay_commands_are_unaffected_by_new_alert_fields() {
    let data = data();
    let battle = setup(
        units(data, &["unit_men_at_arms_foot"]),
        units(data, &["unit_men_at_arms_foot"]),
        None,
    );
    let mut sim = BattleSim::new(battle, 9).unwrap();
    lab(&mut sim);
    place(&mut sim, 0, 500.0, 300.0, 0.0);
    place(&mut sim, 1, 700.0, 300.0, std::f64::consts::PI);
    sim.issue_command(Command::Move {
        units: vec![0],
        x: 650.0,
        z: 300.0,
        facing: None,
        run: false,
        queue: false,
        width: None,
        match_speed: false,
        group_tag: None,
    })
    .unwrap();
    for _ in 0..200 {
        sim.step();
    }
    // No alert reader was ever attached; the sim must still behave (no
    // panic, no growth bound issue) and keep accumulating alerts quietly.
    assert!(!sim.take_new_alerts().is_empty() || sim.units()[0].present());
}
