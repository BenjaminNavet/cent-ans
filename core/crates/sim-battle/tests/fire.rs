//! Siege fires (lot S2, spec `docs/design/s2-incendies.md`).

mod common;

use common::*;
use data_model::GameData;
use sim_battle::{
    BattleSim, Command, FireRules, FireState, SideId, SiegeSetup, UnitSetup, Weather,
};

const BESIEGERS: [&str; 4] = [
    "unit_men_at_arms_foot",
    "unit_urban_militia",
    "unit_longbowmen",
    "unit_longbowmen",
];
const GARRISON: [&str; 3] = [
    "unit_urban_militia",
    "unit_crossbowmen",
    "unit_men_at_arms_foot",
];

fn siege(data: &GameData, extra: &[&str], seed: u64) -> BattleSim {
    let mut attackers: Vec<UnitSetup> = units(data, &BESIEGERS);
    attackers.extend(units(data, extra));
    let setup = setup(
        attackers,
        units(data, &GARRISON),
        Some(SiegeSetup {
            fortification: 2,
            breach: 0,
        }),
    );
    BattleSim::new(setup, seed).unwrap()
}

/// A lab siege: AIs off, given weather, every regiment parked far away in
/// a corner of the field (no shooting), fires unchanged.
fn quiet_siege(data: &GameData, seed: u64, weather: Weather) -> BattleSim {
    let mut sim = siege(data, &[], seed);
    lab(&mut sim);
    sim.set_weather(weather);
    let n = sim.units().len() as u32;
    for id in 0..n {
        place(&mut sim, id, 40.0 + 30.0 * f64::from(id), 40.0, 0.0);
    }
    hold_fire(&mut sim, SideId::Attacker);
    hold_fire(&mut sim, SideId::Defender);
    sim
}

fn fire_state(sim: &BattleSim) -> Vec<(FireState, i64)> {
    sim.siege()
        .unwrap()
        .houses
        .iter()
        .map(|h| (h.fire.state, (h.fire.intensity * 1000.0).round() as i64))
        .collect()
}

/// Houses reached by the fire (burning or burnt).
fn reached(sim: &BattleSim) -> usize {
    let works = sim.siege().unwrap();
    works.burning_houses() + works.burnt_houses()
}

/// The town house (not a suburb) closest to the town centre's east.
fn town_house(sim: &BattleSim) -> usize {
    let works = sim.siege().unwrap();
    (0..works.houses.len())
        .filter(|&i| !works.houses[i].suburb)
        .min_by(|&a, &b| {
            let d = |i: usize| {
                let h = &works.houses[i];
                (h.x - works.center.0 - 110.0).powi(2) + (h.z - works.center.1).powi(2)
            };
            d(a).total_cmp(&d(b)).then(a.cmp(&b))
        })
        .unwrap()
}

#[test]
fn the_bundled_rules_parse_and_the_town_has_suburbs() {
    let rules = FireRules::bundled();
    assert!(rules.ignition_chance("unit_trebuchet", "siege") > 0.1);
    assert!(rules.ignition_chance("unit_longbowmen", "ranged") > 0.0);
    assert_eq!(rules.ignition_chance("unit_knights", "cavalry"), 0.0);
    assert!(rules.weather(Weather::Rain).spread < rules.weather(Weather::Clear).spread);
    let data = data();
    let sim = siege(&data, &[], 1);
    let works = sim.siege().unwrap();
    let suburbs: Vec<_> = works.houses.iter().filter(|h| h.suburb).collect();
    assert_eq!(suburbs.len() as u32, rules.suburbs.count);
    assert!(suburbs.iter().all(|h| !works.inside(h.x, h.z)));
    assert!(works
        .houses
        .iter()
        .filter(|h| !h.suburb)
        .all(|h| works.inside(h.x, h.z)));
}

#[test]
fn fire_is_deterministic() {
    let data = data();
    let run_one = |seed: u64| {
        let mut sim = siege(&data, &["unit_trebuchet", "unit_mangonel"], seed);
        run(&mut sim, 240.0);
        (fire_state(&sim), sim.siege().unwrap().gate_fire.state)
    };
    let a = run_one(5);
    assert_eq!(a, run_one(5), "same seed, same fires");
    assert!(
        a.0.iter().any(|(state, _)| *state != FireState::Intact),
        "incendiary volleys and pots set some houses on fire"
    );
}

#[test]
fn rain_slows_the_spread() {
    let data = data();
    let spread = |weather: Weather| -> usize {
        (0..6)
            .map(|seed| {
                let mut sim = quiet_siege(&data, seed, weather);
                let start = town_house(&sim);
                assert!(sim.ignite_house(start));
                run(&mut sim, 300.0);
                reached(&sim)
            })
            .sum()
    };
    let clear = spread(Weather::Clear);
    let rain = spread(Weather::Rain);
    println!("spread over 6 seeds: clear {clear}, rain {rain}");
    assert!(
        clear >= rain * 2 && clear > 6,
        "clear {clear} houses reached, rain {rain}"
    );
}

#[test]
fn a_burning_house_burns_out_into_a_ruin() {
    let data = data();
    let mut sim = quiet_siege(&data, 2, Weather::Clear);
    let mut rules = FireRules::bundled().clone();
    rules.spread.chance_per_period = 0.0;
    sim.set_fire_rules(Some(rules.clone()));
    let house = town_house(&sim);
    assert!(sim.ignite_house(house));
    assert!(!sim.ignite_house(house), "already burning");
    run(&mut sim, 30.0);
    let blaze = sim.siege().unwrap().houses[house].fire;
    assert_eq!(blaze.state, FireState::Burning);
    assert!(blaze.intensity > rules.house.initial_intensity);
    run(&mut sim, rules.house.burn_duration_s * 2.0);
    let blaze = sim.siege().unwrap().houses[house].fire;
    assert_eq!(blaze.state, FireState::Burnt);
    assert_eq!(blaze.intensity, 0.0);
    assert!(has_event(&sim, "Le feu prend dans la ville"));
}

#[test]
fn a_burnt_house_can_be_crossed() {
    let data = data();
    let cross = |burnt: bool| -> f64 {
        let mut sim = quiet_siege(&data, 3, Weather::Clear);
        let house = town_house(&sim);
        let (hx, hz, radius) = {
            let h = &sim.siege().unwrap().houses[house];
            (h.x, h.z, h.radius)
        };
        if burnt {
            let works = sim.siege_mut().unwrap();
            works.houses[house].fire.state = FireState::Burnt;
        }
        // A defender (inside the walls) walks straight across the house.
        let id = sim
            .units()
            .iter()
            .find(|u| {
                u.side == SideId::Defender && u.category == data_model::UnitCategory::Infantry
            })
            .unwrap()
            .id;
        place(&mut sim, id, hx, hz - radius - 12.0, 0.0);
        sim.issue_command(Command::Move {
            units: vec![id],
            x: hx,
            z: hz + radius + 12.0,
            run: false,
            facing: None,
        })
        .unwrap();
        let mut closest = f64::INFINITY;
        for _ in 0..600 {
            sim.step();
            let u = &sim.units()[id as usize];
            closest = closest.min(((u.x - hx).powi(2) + (u.z - hz).powi(2)).sqrt());
        }
        closest
    };
    let standing = cross(false);
    let ruin = cross(true);
    assert!(
        ruin < 3.0,
        "the ruin is crossed ({ruin:.1} m from its centre)"
    );
    assert!(
        standing > 7.0,
        "a standing house is walked around ({standing:.1} m)"
    );
}

#[test]
fn regiments_near_a_fire_suffer() {
    let data = data();
    let mut sim = quiet_siege(&data, 4, Weather::Clear);
    let mut rules = FireRules::bundled().clone();
    rules.spread.chance_per_period = 0.0;
    sim.set_fire_rules(Some(rules));
    let house = town_house(&sim);
    let (hx, hz, radius) = {
        let h = &sim.siege().unwrap().houses[house];
        (h.x, h.z, h.radius)
    };
    let defenders: Vec<u32> = sim
        .units()
        .iter()
        .filter(|u| u.side == SideId::Defender)
        .map(|u| u.id)
        .collect();
    let (near, far) = (defenders[0], defenders[2]);
    // The regiment's depth stays within a few metres of the house edge.
    place(&mut sim, near, hx, hz - radius - 8.0, 0.0);
    place(&mut sim, far, hx, hz - radius - 60.0, 0.0);
    assert!(sim.ignite_house(house));
    let (hp0, morale0) = (
        sim.units()[near as usize].hp,
        sim.units()[near as usize].morale,
    );
    let (far_hp, far_morale) = (
        sim.units()[far as usize].hp,
        sim.units()[far as usize].morale,
    );
    run(&mut sim, 40.0);
    let near_unit = &sim.units()[near as usize];
    assert!(
        near_unit.hp < hp0,
        "heat losses: {} -> {}",
        hp0,
        near_unit.hp
    );
    assert!(hp0 - near_unit.hp < hp0 * 0.2, "light losses only");
    let far_unit = &sim.units()[far as usize];
    assert_eq!(far_unit.hp, far_hp);
    assert!(far_unit.morale >= far_morale);
    assert!(
        near_unit.morale < morale0.min(far_unit.morale) - 3.0,
        "morale shaken: {morale0:.1} -> {:.1} (far {:.1})",
        near_unit.morale,
        far_unit.morale
    );
}

/// BR3b: on the wall walk, raised above the street behind the parapet, a
/// regiment gets only `heat.wall_walk_factor` of the heat (the rows of
/// houses are backed against the rampart).
#[test]
fn the_wall_walk_is_sheltered_from_the_heat() {
    let data = data();
    let factor = FireRules::bundled().heat.wall_walk_factor;
    assert!(factor > 0.0 && factor < 1.0, "factor {factor}");
    let mut sim = quiet_siege(&data, 4, Weather::Clear);
    let mut rules = FireRules::bundled().clone();
    rules.spread.chance_per_period = 0.0;
    sim.set_fire_rules(Some(rules));
    let house = town_house(&sim);
    let (hx, hz, radius) = {
        let h = &sim.siege().unwrap().houses[house];
        (h.x, h.z, h.radius)
    };
    let id = sim
        .units()
        .iter()
        .find(|u| u.side == SideId::Defender && u.unit_type == "unit_crossbowmen")
        .unwrap()
        .id as usize;
    place(&mut sim, id as u32, hx, hz - radius - 8.0, 0.0);
    assert!(sim.ignite_house(house));
    run(&mut sim, 20.0);
    let street = sim.heat_intensity(&sim.units()[id]);
    assert!(street > 0.0, "within the heat");
    sim.units_mut()[id].on_wall = true;
    let wall = sim.heat_intensity(&sim.units()[id]);
    assert!(
        (wall - street * factor).abs() < 1e-9,
        "wall {wall:.3} street {street:.3}"
    );
}

#[test]
fn a_burning_gate_loses_its_hit_points_and_opens() {
    let data = data();
    let mut sim = quiet_siege(&data, 6, Weather::Clear);
    let gate = sim.siege().unwrap().gate;
    let max_hp = sim.siege().unwrap().pieces[gate].max_hp;
    assert!(sim.ignite_gate());
    assert!(has_event(&sim, "La porte de la ville prend feu"));
    run(&mut sim, 60.0);
    let hp = sim.siege().unwrap().pieces[gate].hp;
    assert!(hp < max_hp && hp > 0.0, "gate {hp} / {max_hp}");
    // Stone walls never burn.
    assert!(sim
        .siege()
        .unwrap()
        .pieces
        .iter()
        .enumerate()
        .all(|(i, p)| i == gate || p.hp == p.max_hp));
    let mut rules = FireRules::bundled().clone();
    rules.gate.damage_per_s = 50.0;
    sim.set_fire_rules(Some(rules));
    run(&mut sim, 60.0);
    let works = sim.siege().unwrap();
    assert!(!works.pieces[gate].intact(), "the gate burns open");
    assert!(works.openings().contains(&gate));
    assert!(has_event(&sim, "dévorée par les flammes"));
}

#[test]
fn smoke_spoils_the_aim_through_it() {
    let data = data();
    let losses = |smoke: bool| -> f64 {
        let mut sim = quiet_siege(&data, 7, Weather::Clear);
        let mut rules = FireRules::bundled().clone();
        rules.spread.chance_per_period = 0.0;
        rules.heat.loss_per_s = 0.0;
        sim.set_fire_rules(Some(rules));
        let house = town_house(&sim);
        let (hx, hz, radius) = {
            let h = &sim.siege().unwrap().houses[house];
            (h.x, h.z, h.radius)
        };
        if smoke {
            sim.ignite_house(house);
            run(&mut sim, 20.0);
        }
        let archer = sim
            .units()
            .iter()
            .find(|u| u.side == SideId::Attacker && u.unit_type == "unit_longbowmen")
            .unwrap()
            .id;
        let target = sim
            .units()
            .iter()
            .find(|u| u.side == SideId::Defender && u.unit_type == "unit_urban_militia")
            .unwrap()
            .id;
        place(&mut sim, archer, hx, hz - radius - 50.0, 0.0);
        place(
            &mut sim,
            target,
            hx,
            hz + radius + 30.0,
            std::f64::consts::PI,
        );
        let hp0 = sim.units()[target as usize].hp;
        sim.issue_command(Command::Attack {
            units: vec![archer],
            target,
            run: false,
        })
        .unwrap();
        run(&mut sim, 30.0);
        hp0 - sim.units()[target as usize].hp
    };
    let clear = losses(false);
    let smoky = losses(true);
    assert!(clear > 0.0, "the archers hit in clear air");
    assert!(
        smoky < clear * 0.75,
        "smoke: {smoky:.1} losses vs {clear:.1}"
    );
}

#[test]
fn the_burn_command_needs_a_torch_close_by() {
    let data = data();
    let mut sim = quiet_siege(&data, 8, Weather::Clear);
    let house = town_house(&sim);
    let (hx, hz, radius) = {
        let h = &sim.siege().unwrap().houses[house];
        (h.x, h.z, h.radius)
    };
    let id = sim
        .units()
        .iter()
        .find(|u| u.side == SideId::Defender && u.unit_type == "unit_urban_militia")
        .unwrap()
        .id;
    let burn = |sim: &mut BattleSim| {
        sim.issue_command(Command::Burn {
            units: vec![id],
            house: Some(house),
            gate: false,
        })
    };
    assert!(burn(&mut sim).is_err(), "too far");
    // Suburbs: the garrison reaches them from anywhere.
    let suburb = (0..sim.siege().unwrap().houses.len())
        .find(|&i| sim.siege().unwrap().houses[i].suburb)
        .unwrap();
    sim.issue_command(Command::Burn {
        units: vec![id],
        house: Some(suburb),
        gate: false,
    })
    .unwrap();
    place(&mut sim, id, hx, hz - radius - 10.0, 0.0);
    burn(&mut sim).unwrap();
    let works = sim.siege().unwrap();
    assert_eq!(works.houses[house].fire.state, FireState::Burning);
    assert_eq!(works.houses[suburb].fire.state, FireState::Burning);
    assert!(burn(&mut sim).is_err(), "already burning");
    assert!(sim
        .issue_command(Command::Burn {
            units: vec![id],
            house: None,
            gate: false,
        })
        .is_err());
}

#[test]
fn an_ai_garrison_burns_some_of_its_suburbs() {
    let data = data();
    let burnt: usize = (0..6)
        .map(|seed| {
            let mut sim = siege(&data, &[], seed);
            sim.set_weather(Weather::Clear);
            sim.step();
            sim.siege()
                .unwrap()
                .houses
                .iter()
                .filter(|h| h.suburb && h.fire.state != FireState::Intact)
                .count()
        })
        .sum();
    let total = 6 * FireRules::bundled().suburbs.count as usize;
    assert!(
        burnt > 0 && burnt < total,
        "{burnt} / {total} suburbs burnt"
    );
}
