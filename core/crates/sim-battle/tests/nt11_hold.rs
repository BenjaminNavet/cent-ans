//! NT11: "hold ground" (camp tenu), the battle option of the guided
//! prologue: the held side keeps its place (no AI move, no skirmish step
//! back, halted when turned on), while the melee, the morale and the rout
//! under pressure run as ever.

mod common;

use common::*;
use sim_battle::{BattleSim, Command, ReplayAction, SideId, UnitMode, UnitState};

const ARMY: [&str; 6] = [
    "unit_men_at_arms_foot",
    "unit_men_at_arms_foot",
    "unit_longbowmen",
    "unit_longbowmen",
    "unit_knights",
    "unit_urban_militia",
];

fn mirrored(seed: u64) -> BattleSim {
    let data = data();
    BattleSim::new(setup(units(data, &ARMY), units(data, &ARMY), None), seed).unwrap()
}

fn positions(sim: &BattleSim, side: SideId) -> Vec<(u32, f64, f64)> {
    sim.units()
        .iter()
        .filter(|u| u.side == side)
        .map(|u| (u.id, u.x, u.z))
        .collect()
}

#[test]
fn held_side_with_its_ai_on_does_not_move_before_contact() {
    let mut sim = mirrored(11);
    sim.set_ai(SideId::Attacker, true);
    sim.set_ai(SideId::Defender, true);
    sim.set_hold(SideId::Defender, true);
    assert!(sim.holds(SideId::Defender) && !sim.holds(SideId::Attacker));
    let start = positions(&sim, SideId::Defender);
    let mut steps = 0;
    while !sim.is_finished() && steps < 20_000 {
        sim.step();
        steps += 1;
        if sim.units().iter().any(|u| {
            u.side == SideId::Defender && matches!(u.state, UnitState::Melee | UnitState::Routing)
        }) {
            break;
        }
        for &(id, x, z) in &start {
            let u = sim.unit(id).unwrap();
            let moved = (u.x - x).hypot(u.z - z);
            assert!(
                moved < 0.5,
                "held regiment {id} moved {moved:.1} m before any contact"
            );
        }
    }
    assert!(steps > 100, "the attacker should have closed in first");
}

#[test]
fn without_hold_the_same_ai_moves() {
    let mut sim = mirrored(11);
    sim.set_ai(SideId::Attacker, true);
    sim.set_ai(SideId::Defender, true);
    let start = positions(&sim, SideId::Defender);
    run(&mut sim, 60.0);
    let moved = start
        .iter()
        .map(|&(id, x, z)| {
            let u = sim.unit(id).unwrap();
            (u.x - x).hypot(u.z - z)
        })
        .fold(0.0, f64::max);
    assert!(moved > 5.0, "control: the free AI moves ({moved:.1} m)");
}

#[test]
fn held_skirmishers_do_not_step_back() {
    let data = data();
    let mut sim = BattleSim::new(
        setup(
            units(data, &["unit_urban_militia"]),
            units(data, &["unit_crossbowmen"]),
            None,
        ),
        6,
    )
    .unwrap();
    lab(&mut sim);
    hold_fire(&mut sim, SideId::Defender);
    place(&mut sim, 1, 750.0, 300.0, std::f64::consts::PI);
    place(&mut sim, 0, 750.0, 230.0, 0.0);
    sim.apply_command(
        Command::SetMode {
            units: vec![1],
            mode: UnitMode::Skirmish,
            enabled: true,
        },
        None,
    )
    .unwrap();
    let mut control = sim.clone();
    sim.set_hold(SideId::Defender, true);
    let attack = Command::Attack {
        units: vec![0],
        target: 1,
        run: true,
        queue: false,
    };
    sim.apply_command(attack.clone(), None).unwrap();
    control.apply_command(attack, None).unwrap();
    run(&mut sim, 8.0);
    run(&mut control, 8.0);
    assert!(
        control.units()[1].z > 300.0 + 5.0,
        "control: the skirmishers step back (z {:.1})",
        control.units()[1].z
    );
    assert!(
        (sim.units()[1].z - 300.0).abs() < 1.0,
        "held skirmishers stand (z {:.1})",
        sim.units()[1].z
    );
}

#[test]
fn turning_hold_on_halts_a_marching_side() {
    let mut sim = mirrored(3);
    lab(&mut sim);
    let id = sim
        .units()
        .iter()
        .find(|u| u.side == SideId::Defender)
        .unwrap()
        .id;
    let (x, z) = (sim.unit(id).unwrap().x, sim.unit(id).unwrap().z);
    sim.apply_command(
        Command::Move {
            units: vec![id],
            x: x + 100.0,
            z,
            run: false,
            facing: None,
            queue: false,
            width: None,
            match_speed: false,
            group_tag: None,
        },
        None,
    )
    .unwrap();
    run(&mut sim, 2.0);
    let before = sim.unit(id).unwrap().x;
    ReplayAction::SetHold {
        side: SideId::Defender,
        enabled: true,
    }
    .apply(&mut sim);
    run(&mut sim, 5.0);
    let u = sim.unit(id).unwrap();
    assert!(u.destination.is_none() && u.state == UnitState::Idle);
    assert!(
        (u.x - before).abs() < 0.5,
        "halted, moved {:.1}",
        u.x - before
    );
}

#[test]
fn held_side_still_routs_under_pressure() {
    let mut sim = mirrored(2);
    sim.set_ai(SideId::Attacker, true);
    sim.set_ai(SideId::Defender, false);
    sim.set_hold(SideId::Defender, true);
    let mut routed = false;
    while !sim.is_finished() {
        sim.step();
        routed |= sim
            .units()
            .iter()
            .any(|u| u.side == SideId::Defender && u.state == UnitState::Routing);
    }
    assert_eq!(sim.winner(), Some(SideId::Attacker));
    assert!(routed, "the held side still breaks under pressure");
}
