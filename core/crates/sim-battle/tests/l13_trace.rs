//! A6-L13 (ADR 0180): trace of one battle, every 10 s, per side: soldiers,
//! regiments routing / in melee, mean morale.
//! `L13_TRACE=substring-of-scenario-name L13_SEED=0 [L13_PACE=file.json]
//! cargo test --release -p sim-battle --test l13_trace -- --ignored --nocapture`

mod common;

use std::fmt::Write as _;
use std::path::PathBuf;

use common::*;
use data_model::{Terrain, UnitTypeId};
use serde::Deserialize;
use sim_battle::{BattleSim, SideId, UnitSetup, UnitState};

#[derive(Deserialize)]
struct Scenario {
    name: String,
    attacker: Vec<(String, usize)>,
    defender: Vec<(String, usize)>,
    terrain: Terrain,
}

#[derive(Deserialize)]
struct Fixture {
    scenarios: Vec<Scenario>,
}

#[test]
#[ignore = "trace"]
fn trace() {
    let data = data();
    let key = std::env::var("L13_TRACE").unwrap();
    let seed: u64 = std::env::var("L13_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../ai/tests/fixtures/auto_resolve_scenarios.json");
    let fixture: Fixture = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mixed = |men: u32, archer: &str| -> Vec<(String, usize)> {
        let n = (men / 85) as usize;
        vec![
            ("unit_men_at_arms_foot".into(), n / 3),
            (archer.into(), n / 3),
            ("unit_urban_militia".into(), n / 6),
            ("unit_knights".into(), n - n / 3 - n / 3 - n / 6),
        ]
    };
    let campaign = Scenario {
        name: "campagne 600 contre 550 mixte".into(),
        attacker: mixed(600, "unit_crossbowmen"),
        defender: mixed(550, "unit_longbowmen"),
        terrain: Terrain::Plains,
    };
    let s = if key == "campagne" {
        &campaign
    } else {
        fixture
            .scenarios
            .iter()
            .find(|s| s.name.contains(&key))
            .unwrap()
    };
    let comp = |c: &[(String, usize)]| {
        let mut out = Vec::new();
        for (id, n) in c {
            let t = &data.unit_types[&UnitTypeId::new(id).unwrap()];
            for _ in 0..*n {
                out.push(UnitSetup::from_unit_type(t, t.soldiers, t.stats.morale, 0));
            }
        }
        out
    };
    let mut setup = setup(comp(&s.attacker), comp(&s.defender), None);
    setup.terrain = s.terrain;
    setup.village = Some(false);
    let mut sim = BattleSim::new(setup, seed).unwrap();
    if let Ok(path) = std::env::var("L13_PACE") {
        sim.set_pace(serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap());
    }
    if let Ok(path) = std::env::var("L13_PUSH") {
        sim.set_push_rules(serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap());
    }
    if let Ok(path) = std::env::var("L13_ROUT") {
        sim.set_rout_rules(serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap());
    }
    if let Some(f) = std::env::var("L13_AMMO")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
    {
        for u in sim.units_mut() {
            u.ammo = (f64::from(u.ammo) * f).round() as u32;
        }
    }
    println!("{}", s.name);
    let mut next = 0.0;
    while !sim.is_finished() && sim.elapsed() < 3600.0 {
        sim.step();
        if sim.elapsed() >= next {
            next += 10.0;
            let mut line = format!("{:>4.0}s", sim.elapsed());
            for side in [SideId::Attacker, SideId::Defender] {
                let us: Vec<_> = sim.units().iter().filter(|u| u.side == side).collect();
                let routing = us.iter().filter(|u| u.state == UnitState::Routing).count();
                let _melee0 = 0;
                let melee = us.iter().filter(|u| u.state == UnitState::Melee).count();
                let morale: f64 = us.iter().map(|u| u.morale).sum::<f64>() / us.len() as f64;
                let _ = write!(
                    line,
                    " | men {:>4} rout {routing} melee {melee} morale {morale:>3.0}",
                    sim.strength(side)
                );
            }
            println!("{line}");
            if std::env::var("L13_UNITS").is_ok()
                && sim.elapsed()
                    > std::env::var("L13_FROM")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(255.0)
            {
                for u in sim.units().iter().filter(|u| u.side == SideId::Defender) {
                    print!(
                        " [{:.0}/{:.0} f{:.0} fl{} {:?}]",
                        u.hp, u.morale, u.fatigue, u.flanked, u.state
                    );
                }
                println!();
            }
        }
    }
    if std::env::var("L13_EVENTS").is_ok() {
        for e in sim.events() {
            println!("EV {:>4.0}s {}", e.time, e.text_fr);
        }
    }
    println!("end {:.0}s winner {:?}", sim.elapsed(), sim.winner());
}
