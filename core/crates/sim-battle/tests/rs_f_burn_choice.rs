//! RS-F: the « Incendier » button of the battle bar asks the core which
//! house or gate a torch would reach (`BattleSim::burn_choice`), by the very
//! test of the `burn` command, without drawing anything.

mod common;

use common::*;
use data_model::GameData;
use sim_battle::{BattleSim, Command, CommandError, FireState, SideId, SiegeSetup, Weather};

const BESIEGERS: [&str; 3] = [
    "unit_men_at_arms_foot",
    "unit_urban_militia",
    "unit_longbowmen",
];
const GARRISON: [&str; 2] = ["unit_urban_militia", "unit_crossbowmen"];

/// A lab siege, every regiment parked in a far corner, nobody shooting.
fn quiet_siege(data: &GameData, seed: u64) -> BattleSim {
    let setup = setup(
        units(data, &BESIEGERS),
        units(data, &GARRISON),
        Some(SiegeSetup {
            fortification: 2,
            breach: 0,
        }),
    );
    let mut sim = BattleSim::new(setup, seed).unwrap();
    lab(&mut sim);
    let n = sim.units().len() as u32;
    for id in 0..n {
        place(&mut sim, id, 40.0 + 30.0 * f64::from(id), 40.0, 0.0);
    }
    hold_fire(&mut sim, SideId::Attacker);
    hold_fire(&mut sim, SideId::Defender);
    sim
}

fn first_of(sim: &BattleSim, side: SideId) -> u32 {
    sim.units().iter().find(|u| u.side == side).unwrap().id
}

#[test]
fn no_burn_order_outside_a_siege() {
    let data = data();
    let setup = setup(units(&data, &BESIEGERS), units(&data, &GARRISON), None);
    let sim = BattleSim::new(setup, 3).unwrap();
    assert_eq!(
        sim.burn_choice(SideId::Attacker, &[]),
        Err(CommandError::NotASiege)
    );
}

#[test]
fn far_besiegers_cannot_but_the_garrison_reaches_its_suburbs() {
    let data = data();
    let sim = quiet_siege(&data, 4);
    assert_eq!(
        sim.burn_choice(SideId::Attacker, &[]),
        Err(CommandError::NothingInReach)
    );
    let choice = sim.burn_choice(SideId::Defender, &[]).unwrap();
    let house = choice.house.expect("a suburb house");
    assert!(choice.suburb && sim.siege().unwrap().houses[house].suburb);
    assert_eq!(sim.units()[choice.unit as usize].side, SideId::Defender);
    // A regiment of the other side in the list is left out.
    let enemy = first_of(&sim, SideId::Attacker);
    assert_eq!(
        sim.burn_choice(SideId::Defender, &[enemy]),
        Err(CommandError::NoUnits)
    );
}

#[test]
fn a_besieger_next_to_a_house_gets_it_and_the_command_is_accepted() {
    let data = data();
    let mut sim = quiet_siege(&data, 5);
    let id = first_of(&sim, SideId::Attacker);
    let (house, hx, hz, radius) = {
        let works = sim.siege().unwrap();
        let i = (0..works.houses.len())
            .find(|&i| !works.houses[i].suburb)
            .unwrap();
        let h = &works.houses[i];
        (i, h.x, h.z, h.radius)
    };
    place(&mut sim, id, hx, hz - radius - 6.0, 0.0);
    let choice = sim.burn_choice(SideId::Attacker, &[id]).unwrap();
    assert_eq!(choice.unit, id);
    assert!(choice.house.is_some() || choice.gate);
    if choice.house == Some(house) {
        assert!(choice.distance_m <= sim.fire_rules().unwrap().torch.reach_m);
    }
    sim.issue_command(Command::Burn {
        units: vec![choice.unit],
        house: choice.house,
        gate: choice.gate,
    })
    .expect("the proposed order is valid");
}

#[test]
fn asking_draws_nothing() {
    let data = data();
    let state = |ask: bool| {
        let mut sim = quiet_siege(&data, 6);
        let id = first_of(&sim, SideId::Defender);
        let suburb = (0..sim.siege().unwrap().houses.len())
            .find(|&i| sim.siege().unwrap().houses[i].suburb)
            .unwrap();
        for _ in 0..20 {
            if ask {
                let _ = sim.burn_choice(SideId::Defender, &[]);
                let _ = sim.burn_choice(SideId::Attacker, &[]);
            }
        }
        let _ = sim.issue_command(Command::Burn {
            units: vec![id],
            house: Some(suburb),
            gate: false,
        });
        run(&mut sim, 90.0);
        sim.siege()
            .unwrap()
            .houses
            .iter()
            .map(|h| (h.fire.state, (h.fire.intensity * 1000.0).round() as i64))
            .collect::<Vec<_>>()
    };
    let asked = state(true);
    assert_eq!(asked, state(false), "the same fires with or without asking");
}

#[test]
fn nothing_left_once_everything_burns() {
    let data = data();
    let mut sim = quiet_siege(&data, 7);
    sim.set_weather(Weather::Clear);
    let n = sim.siege().unwrap().houses.len();
    for i in 0..n {
        assert!(sim.ignite_house(i));
    }
    assert!(sim.ignite_gate());
    assert_eq!(
        sim.burn_choice(SideId::Defender, &[]),
        Err(CommandError::NothingLeftToBurn)
    );
}
