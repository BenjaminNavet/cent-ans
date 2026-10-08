//! TW2 T4 (ADR 0108): capture points of a siege battle — the market square
//! (victory point) and the gate — the garrison's last stand and its fall
//! back on the square at the first breach.

mod common;

use common::*;
use data_model::GameData;
use sim_battle::alerts::AlertKind;
use sim_battle::{
    BattleEnd, BattleSim, CapturePointKind, CaptureRules, PieceKind, PointStatus, SideId,
    SiegeSetup,
};

fn siege(data: &GameData, attackers: &[&str], defenders: &[&str], seed: u64) -> BattleSim {
    let setup = setup(
        units(data, attackers),
        units(data, defenders),
        Some(SiegeSetup {
            fortification: 1,
            breach: 0,
            ..Default::default()
        }),
    );
    let mut sim = BattleSim::new(setup, seed).unwrap();
    if sim.is_deploying() {
        sim.start_battle().unwrap();
    }
    lab(&mut sim);
    sim
}

fn point(sim: &BattleSim, kind: CapturePointKind) -> sim_battle::CapturePoint {
    sim.siege()
        .unwrap()
        .capture_points()
        .into_iter()
        .find(|p| p.kind == kind)
        .unwrap()
}

fn ids(sim: &BattleSim, side: SideId) -> Vec<u32> {
    sim.units()
        .iter()
        .filter(|u| u.side == side && !u.synthetic)
        .map(|u| u.id)
        .collect()
}

/// Every defender far from the square, at the back of the town.
fn garrison_to_the_back(sim: &mut BattleSim) {
    let (cx, cz) = sim.siege().unwrap().center;
    for (k, id) in ids(sim, SideId::Defender).into_iter().enumerate() {
        place(sim, id, cx - 20.0 + k as f64 * 15.0, cz + 110.0, 0.0);
    }
}

#[test]
fn the_points_are_the_square_and_the_inside_of_the_gate() {
    let data = data();
    let sim = siege(data, &["unit_men_at_arms_foot"], &["unit_urban_militia"], 1);
    let works = sim.siege().unwrap();
    let rules = CaptureRules::bundled();
    let square = point(&sim, CapturePointKind::Square);
    assert_eq!((square.x, square.z), works.center);
    assert_eq!(square.hold_s, rules.square.hold_s);
    let gate = point(&sim, CapturePointKind::Gate);
    let (mx, mz) = works.pieces[works.gate].midpoint();
    let offset = (gate.x - mx).hypot(gate.z - mz);
    assert!((offset - rules.gate.inside_offset_m.unwrap()).abs() < 1e-6);
    assert!(works.inside(gate.x, gate.z), "gate point inside the walls");
}

#[test]
fn holding_the_square_takes_the_town_and_warns_the_garrison() {
    let data = data();
    let mut sim = siege(
        data,
        &["unit_men_at_arms_foot", "unit_men_at_arms_foot"],
        &["unit_urban_militia"],
        2,
    );
    sim.set_end_conditions(true);
    garrison_to_the_back(&mut sim);
    let (cx, cz) = sim.siege().unwrap().center;
    for (k, id) in ids(&sim, SideId::Attacker).into_iter().enumerate() {
        place(&mut sim, id, cx - 6.0 + k as f64 * 12.0, cz, 0.0);
    }
    let hold = CaptureRules::bundled().square.hold_s;
    run(&mut sim, hold * 0.5);
    let square = point(&sim, CapturePointKind::Square);
    assert_eq!(square.status, PointStatus::Capturing);
    assert!((square.share() - 0.5).abs() < 0.02, "{}", square.share());
    assert_eq!(sim.siege().unwrap().hold_time, square.progress);
    let alerts = sim.take_new_alerts();
    let threatened: Vec<_> = alerts
        .iter()
        .filter(|a| a.kind == AlertKind::SquareThreatened)
        .collect();
    assert_eq!(threatened.len(), 1, "one « La place est menacée »");
    assert_eq!(threatened[0].side, Some(SideId::Defender));
    assert!(!sim.is_finished());
    run(&mut sim, hold * 0.5 + 1.0);
    assert!(sim.is_finished());
    assert_eq!(sim.winner(), Some(SideId::Attacker));
    assert_eq!(sim.outcome().unwrap().end, BattleEnd::SquareHeld);
}

#[test]
fn a_stronger_garrison_on_the_square_stops_the_capture() {
    let data = data();
    let mut sim = siege(
        data,
        &["unit_men_at_arms_foot"],
        &["unit_urban_militia", "unit_urban_militia"],
        3,
    );
    let (cx, cz) = sim.siege().unwrap().center;
    let attacker = ids(&sim, SideId::Attacker)[0];
    place(&mut sim, attacker, cx, cz - 10.0, 0.0);
    for (k, id) in ids(&sim, SideId::Defender).into_iter().enumerate() {
        place(
            &mut sim,
            id,
            cx - 10.0 + k as f64 * 20.0,
            cz + 12.0,
            std::f64::consts::PI,
        );
    }
    run(&mut sim, 5.0);
    let square = point(&sim, CapturePointKind::Square);
    assert_eq!(square.status, PointStatus::Contested);
    assert_eq!(square.progress, 0.0);
    assert!(square.defenders > square.attackers);
}

#[test]
fn taking_the_gate_opens_it() {
    let data = data();
    let mut sim = siege(
        data,
        &["unit_men_at_arms_foot", "unit_men_at_arms_foot"],
        &["unit_urban_militia"],
        4,
    );
    garrison_to_the_back(&mut sim);
    let gate = point(&sim, CapturePointKind::Gate);
    for (k, id) in ids(&sim, SideId::Attacker).into_iter().enumerate() {
        place(&mut sim, id, gate.x - 5.0 + k as f64 * 10.0, gate.z, 0.0);
    }
    let works = sim.siege().unwrap();
    assert!(works.pieces[works.gate].intact());
    run(&mut sim, CaptureRules::bundled().gate.hold_s + 1.0);
    let works = sim.siege().unwrap();
    assert_eq!(works.pieces[works.gate].kind, PieceKind::Gate);
    assert!(!works.pieces[works.gate].intact(), "the gate is opened");
    assert!(point(&sim, CapturePointKind::Gate).taken());
    assert!(has_event(&sim, "tiennent la porte"));
}

/// Same fight on the square, with and without an opening in the walls: the
/// last stand only holds once the town is open, and keeps the garrison's
/// morale higher.
#[test]
fn the_last_stand_steadies_the_garrison_once_the_town_is_open() {
    let data = data();
    let fight = |breach: bool| {
        let mut sim = siege(data, &["unit_men_at_arms_foot"], &["unit_urban_militia"], 5);
        let (cx, cz) = sim.siege().unwrap().center;
        let attacker = ids(&sim, SideId::Attacker)[0];
        let defender = ids(&sim, SideId::Defender)[0];
        place(&mut sim, defender, cx, cz + 4.0, std::f64::consts::PI);
        place(&mut sim, attacker, cx, cz - 4.0, 0.0);
        if breach {
            let works = sim.siege_mut().unwrap();
            let wall = (0..works.pieces.len())
                .find(|&p| works.pieces[p].kind == PieceKind::Wall)
                .unwrap();
            works.pieces[wall].hp = 0.0;
        }
        sim.apply_command(
            sim_battle::Command::Attack {
                units: vec![attacker],
                target: defender,
                run: false,
                queue: false,
            },
            None,
        )
        .unwrap();
        run(&mut sim, 20.0);
        sim.units()[defender as usize].morale
    };
    let open = fight(true);
    let closed = fight(false);
    assert!(open > closed, "last stand {open:.1} vs {closed:.1}");
}

#[test]
fn the_garrison_falls_back_on_the_square_at_the_first_breach() {
    let data = data();
    let mut sim = siege(
        data,
        &["unit_men_at_arms_foot"],
        &["unit_urban_militia", "unit_welsh_spearmen"],
        6,
    );
    sim.set_ai(SideId::Defender, true);
    run(&mut sim, 2.0);
    {
        let works = sim.siege_mut().unwrap();
        let wall = (0..works.pieces.len())
            .find(|&p| works.pieces[p].kind == PieceKind::Wall)
            .unwrap();
        works.pieces[wall].hp = 0.0;
    }
    run(&mut sim, 90.0);
    assert!(has_event(&sim, "la garnison se replie sur la place"));
    let works = sim.siege().unwrap();
    let on_square = sim
        .units()
        .iter()
        .filter(|u| {
            u.side == SideId::Defender && u.able() && !u.on_wall && works.in_square(u.x, u.z)
        })
        .count();
    assert!(on_square >= 1, "nobody fell back on the square");
}
