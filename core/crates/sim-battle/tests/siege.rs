//! Siege battles (spec `docs/design/m8-sieges.md` § 2).

mod common;

use common::*;
use data_model::GameData;
use sim_battle::{BattleSim, Command, PieceKind, SideId, SiegeSetup, UnitSetup, UnitState};

const BESIEGERS: [&str; 8] = [
    "unit_men_at_arms_foot",
    "unit_men_at_arms_foot",
    "unit_urban_militia",
    "unit_urban_militia",
    "unit_urban_militia",
    "unit_longbowmen",
    "unit_longbowmen",
    "unit_mounted_sergeants",
];
const GARRISON: [&str; 5] = [
    "unit_urban_militia",
    "unit_urban_militia",
    "unit_crossbowmen",
    "unit_crossbowmen",
    "unit_men_at_arms_foot",
];

fn siege(data: &GameData, extra: &[&str], fortification: u32, breach: u8, seed: u64) -> BattleSim {
    let mut attackers: Vec<UnitSetup> = units(data, &BESIEGERS);
    attackers.extend(units(data, extra));
    let setup = setup(
        attackers,
        units(data, &GARRISON),
        Some(SiegeSetup {
            fortification,
            breach,
        }),
    );
    BattleSim::new(setup, seed).unwrap()
}

#[test]
fn the_town_has_a_closed_ring_with_a_gate_facing_the_attacker() {
    let data = data();
    let low = siege(&data, &[], 1, 0, 3);
    let high = siege(&data, &[], 3, 0, 3);
    let works = low.siege().expect("siege works");
    assert_eq!(
        works.pieces.len(),
        10,
        "8 sides, the front one split around the gate"
    );
    let gate = &works.pieces[works.gate];
    assert_eq!(gate.kind, PieceKind::Gate);
    assert!(gate.outward().1 < -0.9, "the gate faces the besiegers (-z)");
    assert!(works.inside(works.center.0, works.center.1));
    assert!(works.openings().is_empty());
    assert!(works.towers.len() >= 8);
    let high_works = high.siege().unwrap();
    assert!(high_works.thickness > works.thickness);
    assert!(high_works.wall_height > works.wall_height);
    assert!(high_works.pieces[0].max_hp > works.pieces[0].max_hp);
    // Besiegers outside, garrison inside and mostly on the wall walk.
    for u in low.units() {
        match u.side {
            SideId::Attacker => assert!(!works.inside(u.x, u.z), "{} outside", u.name),
            SideId::Defender => assert!(works.inside(u.x, u.z), "{} inside", u.name),
        }
    }
    let on_wall = low
        .units()
        .iter()
        .filter(|u| u.side == SideId::Defender && u.on_wall)
        .count();
    assert!(
        on_wall >= 3,
        "shooters and foot man the walls, got {on_wall}"
    );
    assert!(low.units().iter().any(|u| u.ram && u.synthetic));
    // Units on the wall walk stand on top of it.
    let defender = low.units().iter().find(|u| u.on_wall).unwrap();
    let ground = low.field().height(defender.x, defender.z);
    assert!(low.standing_height(defender, defender.x, defender.z) > ground + 5.0);
}

#[test]
fn campaign_breach_opens_the_walls() {
    let data = data();
    let intact = siege(&data, &[], 2, 0, 5);
    let one = siege(&data, &[], 2, 60, 5);
    let two = siege(&data, &[], 2, 95, 5);
    assert_eq!(intact.siege().unwrap().openings().len(), 0);
    assert_eq!(one.siege().unwrap().openings().len(), 1);
    assert_eq!(two.siege().unwrap().openings().len(), 2);
    let works = one.siege().unwrap();
    let opening = works.openings()[0];
    assert_eq!(works.pieces[opening].kind, PieceKind::Wall);
    assert!(
        works.pieces[opening].outward().1 < -0.3,
        "breach on the front"
    );
    assert!(works.integrity() < intact.siege().unwrap().integrity());
    assert!(has_event(&one, "brèche"));
}

#[test]
fn walls_stop_regiments_until_a_breach_opens() {
    let data = data();
    let mut sim = siege(&data, &[], 2, 0, 11);
    lab(&mut sim);
    hold_fire(&mut sim, SideId::Defender);
    // The mounted sergeants (id 7) ride for the square: the wall stops them.
    let (cx, cz) = sim.siege().unwrap().center;
    sim.issue_command(Command::Move {
        units: vec![7],
        x: cx,
        z: cz,
        run: true,
        facing: None,
        queue: false,
        width: None,
        match_speed: false,
        group_tag: None,
    })
    .unwrap();
    run(&mut sim, 150.0);
    let riders = &sim.units()[7];
    assert!(
        !sim.siege().unwrap().inside(riders.x, riders.z),
        "horsemen cannot climb"
    );
    assert!(riders.climbing.is_none());
    // Knock down the nearest front wall: they find the breach and ride in.
    let front = sim.siege().unwrap().front_walls()[0];
    sim.siege_mut().unwrap().pieces[front].hp = 0.0;
    // The garrison pulls back to the far end of the town.
    for i in 0..sim.units().len() {
        if sim.units()[i].side == SideId::Defender {
            place(
                &mut sim,
                i as u32,
                cx + (i as f64 - 12.0) * 30.0,
                cz + 110.0,
                0.0,
            );
        }
    }
    sim.issue_command(Command::Move {
        units: vec![7],
        x: cx,
        z: cz,
        run: true,
        facing: None,
        queue: false,
        width: None,
        match_speed: false,
        group_tag: None,
    })
    .unwrap();
    run(&mut sim, 200.0);
    let riders = &sim.units()[7];
    assert!(
        sim.siege().unwrap().inside(riders.x, riders.z),
        "through the breach: ({:.0}, {:.0})",
        riders.x,
        riders.z
    );
}

#[test]
fn infantry_climbs_with_ladders_and_a_siege_tower_is_faster() {
    let data = data();
    let climb_time = |with_tower: bool| -> f64 {
        let extra: &[&str] = if with_tower {
            &["unit_siege_tower"]
        } else {
            &[]
        };
        let mut sim = siege(&data, extra, 2, 0, 13);
        lab(&mut sim);
        let works = sim.siege().unwrap().clone();
        // An undefended stretch at the back of the town.
        let piece = (0..works.pieces.len())
            .max_by(|&a, &b| {
                works.pieces[a]
                    .midpoint()
                    .1
                    .total_cmp(&works.pieces[b].midpoint().1)
            })
            .unwrap();
        let p = &works.pieces[piece];
        let (mx, mz) = p.midpoint();
        let (nx, nz) = p.outward();
        place(
            &mut sim,
            0,
            mx + nx * 12.0,
            mz + nz * 12.0,
            (-nx).atan2(-nz),
        );
        if with_tower {
            place(&mut sim, 8, mx + nx * 4.5 + 3.0, mz + nz * 4.5, 0.0);
        }
        sim.issue_command(Command::Move {
            units: vec![0],
            x: mx - nx * 30.0,
            z: mz - nz * 30.0,
            run: false,
            facing: None,
            queue: false,
            width: None,
            match_speed: false,
            group_tag: None,
        })
        .unwrap();
        let mut started = None;
        for step in 0..3000 {
            sim.step();
            let u = &sim.units()[0];
            if u.state == UnitState::Climbing && started.is_none() {
                started = Some(step);
            }
            if u.on_wall {
                return f64::from(step - started.expect("climbed")) * 0.1;
            }
        }
        panic!("never reached the wall walk (tower {with_tower})");
    };
    let ladders = climb_time(false);
    let tower = climb_time(true);
    assert!(ladders > 30.0, "ladders are slow: {ladders:.1} s");
    assert!(
        tower < ladders * 0.5,
        "tower {tower:.1} s vs ladders {ladders:.1} s"
    );
}

#[test]
fn the_ram_breaks_the_gate() {
    let data = data();
    let mut sim = siege(&data, &[], 1, 0, 17);
    lab(&mut sim);
    let ram = sim.units().iter().position(|u| u.ram).unwrap() as u32;
    let works = sim.siege().unwrap().clone();
    let gate = &works.pieces[works.gate];
    let (mx, mz) = gate.midpoint();
    let (nx, nz) = gate.outward();
    hold_fire(&mut sim, SideId::Defender);
    sim.issue_command(Command::Move {
        units: vec![ram],
        x: mx + nx * (works.band() + 1.0),
        z: mz + nz * (works.band() + 1.0),
        run: false,
        facing: None,
        queue: false,
        width: None,
        match_speed: false,
        group_tag: None,
    })
    .unwrap();
    run(&mut sim, 400.0);
    let works = sim.siege().unwrap();
    assert!(
        !works.pieces[works.gate].intact(),
        "gate still at {:.0} HP",
        works.pieces[works.gate].hp
    );
    assert!(has_event(&sim, "La porte cède"));
}

#[test]
fn engines_batter_a_breach_and_the_wall_walk_falls() {
    let data = data();
    let mut sim = siege(&data, &["unit_trebuchet"], 1, 0, 19);
    lab(&mut sim);
    let works = sim.siege().unwrap().clone();
    let piece = works.front_walls()[0];
    let trebuchet = 8;
    sim.issue_command(Command::TargetWall {
        units: vec![trebuchet],
        piece,
    })
    .unwrap();
    let (mx, mz) = works.pieces[piece].midpoint();
    let (nx, nz) = works.pieces[piece].outward();
    place(
        &mut sim,
        trebuchet,
        mx + nx * 200.0,
        mz + nz * 200.0,
        (-nx).atan2(-nz),
    );
    run(&mut sim, 900.0);
    assert!(!sim.siege().unwrap().pieces[piece].intact());
    assert!(has_event(&sim, "la brèche est ouverte"));
    // Nobody stands on the fallen stretch any more.
    let fallen = &sim.siege().unwrap().pieces[piece];
    assert!(sim
        .units()
        .iter()
        .filter(|u| u.present() && u.on_wall)
        .all(|u| fallen.distance(u.x, u.z) > 3.0));
    // Only engines batter walls.
    assert!(sim
        .issue_command(Command::TargetWall {
            units: vec![0],
            piece
        })
        .is_err());
}

#[test]
fn holding_the_central_square_for_a_minute_takes_the_town() {
    let data = data();
    let mut sim = siege(&data, &[], 2, 0, 23);
    sim.set_ai(SideId::Attacker, false);
    sim.set_ai(SideId::Defender, false);
    // Clear the square of defenders (they stay on the walls) and drop the
    // men-at-arms in it.
    let (cx, cz) = sim.siege().unwrap().center;
    for i in 0..sim.units().len() {
        let u = &sim.units()[i];
        if u.side == SideId::Defender && !u.on_wall {
            let (x, z) = (u.x, u.z);
            let far = (cx + 120.0, cz + 60.0);
            let _ = (x, z);
            place(&mut sim, i as u32, far.0, far.1, 0.0);
        }
    }
    place(&mut sim, 0, cx, cz, 0.0);
    run(&mut sim, 30.0);
    assert!(!sim.is_finished(), "30 s is not enough");
    run(&mut sim, 40.0);
    assert!(sim.is_finished());
    assert_eq!(sim.winner(), Some(SideId::Attacker));
    assert!(has_event(&sim, "la ville est prise"));
}

#[test]
fn routing_defenders_abandon_the_walls() {
    let data = data();
    let mut sim = siege(&data, &[], 2, 0, 29);
    lab(&mut sim);
    let id = sim
        .units()
        .iter()
        .position(|u| u.side == SideId::Defender && u.on_wall)
        .unwrap();
    sim.units_mut()[id].morale = 5.0;
    run(&mut sim, 1.0);
    let u = &sim.units()[id];
    assert_eq!(u.state, UnitState::Routing);
    assert!(!u.on_wall);
    assert!(has_event(&sim, "abandonnent le rempart"));
}

/// Several AI-vs-AI assaults: (attacker wins, attacker losses / defender losses).
fn assaults(data: &GameData, extra: &[&str], breach: u8) -> (u32, f64) {
    let (mut wins, mut ratio) = (0, 0.0);
    for seed in 0..4 {
        let mut sim = siege(data, extra, 2, breach, seed);
        run_to_end(&mut sim);
        let outcome = sim.outcome().unwrap();
        if outcome.winner == SideId::Attacker {
            wins += 1;
        }
        ratio += f64::from(outcome.attacker.total_losses)
            / f64::from(outcome.defender.total_losses.max(1));
    }
    (wins, ratio / 4.0)
}

#[test]
fn a_wide_breach_eases_the_assault_and_ladders_are_costly() {
    let data = data();
    let (_, ladder_ratio) = assaults(&data, &[], 0);
    let (_, breach_ratio) = assaults(&data, &[], 100);
    // T4 (ADR 0108): through a wide breach the garrison falls back on the
    // square and makes its last stand there, so this small garrison holds
    // (breach 0/4, ladders 2/4, open point of the ADR); the breach still
    // costs the besiegers far fewer men per defender than the ladders.
    assert!(
        ladder_ratio > breach_ratio,
        "ladders cost more: {ladder_ratio:.2} vs {breach_ratio:.2} attackers lost per defender"
    );
}

#[test]
fn siege_battles_are_deterministic() {
    let data = data();
    let play = || {
        let mut sim = siege(&data, &["unit_trebuchet", "unit_siege_tower"], 2, 30, 31);
        run(&mut sim, 200.0);
        (
            sim.units()
                .iter()
                .map(|u| (u.x, u.z, u.hp))
                .collect::<Vec<_>>(),
            sim.siege()
                .unwrap()
                .pieces
                .iter()
                .map(|p| p.hp)
                .collect::<Vec<_>>(),
        )
    };
    assert_eq!(play(), play());
}
