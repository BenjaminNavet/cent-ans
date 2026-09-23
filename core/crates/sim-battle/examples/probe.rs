//! Debug probe: prints a few lab scenarios tick by tick.
//! `cargo run -p sim-battle --example probe -- <front|flank|stakes|rally>`

use std::path::PathBuf;

use data_model::{GameData, Terrain, UnitTypeId};
use sim_battle::*;

fn unit(data: &GameData, id: &str) -> UnitSetup {
    let t = &data.unit_types[&UnitTypeId::new(id).unwrap()];
    UnitSetup::from_unit_type(t, t.soldiers, t.stats.morale, 0)
}

fn side(units: Vec<UnitSetup>) -> SideSetup {
    SideSetup {
        faction: "f".into(),
        faction_name: "France".into(),
        army: String::new(),
        units,
        general: None,
    }
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let data = GameData::load(&root).unwrap().0;
    let scen = std::env::args().nth(1).unwrap_or_default();
    if scen == "ai" {
        let army = |n: usize| -> Vec<UnitSetup> {
            let kinds = [
                "unit_men_at_arms_foot",
                "unit_longbowmen",
                "unit_knights",
                "unit_urban_militia",
                "unit_crossbowmen",
            ];
            (0..n)
                .map(|i| unit(&data, kinds[i % kinds.len()]))
                .collect()
        };
        for seed in 0..5 {
            let setup = BattleSetup {
                province: String::new(),
                province_name: String::new(),
                terrain: Terrain::Plains,
                river: seed % 2 == 0,
                season: BattleSeason::Summer,
                attacker: side(army(20)),
                defender: side(army(20)),
                player_side: None,
            };
            let mut sim = BattleSim::new(setup, seed).unwrap();
            let start = std::time::Instant::now();
            let mut steps = 0;
            while !sim.is_finished() && steps < 36_000 {
                sim.step();
                steps += 1;
            }
            let outcome = sim.outcome().unwrap();
            println!(
                "seed {seed}: {steps} steps ({:.2} ms/step), winner {:?}, losses {} / {}, weather {:?}",
                start.elapsed().as_secs_f64() * 1000.0 / steps as f64,
                outcome.winner,
                outcome.attacker.total_losses,
                outcome.defender.total_losses,
                sim.weather()
            );
        }
        return;
    }
    let (a, d) = match scen.as_str() {
        "rally" => ("unit_men_at_arms_foot", "unit_urban_militia"),
        "stakes" => ("unit_knights", "unit_longbowmen"),
        _ => ("unit_knights", "unit_urban_militia"),
    };
    let setup = BattleSetup {
        province: String::new(),
        province_name: String::new(),
        terrain: Terrain::Plains,
        river: false,
        season: BattleSeason::Summer,
        attacker: side(vec![unit(&data, a)]),
        defender: side(vec![unit(&data, d)]),
        player_side: None,
    };
    let mut sim = BattleSim::new(setup, 7).unwrap();
    sim.set_ai(SideId::Attacker, false);
    sim.set_ai(SideId::Defender, false);
    sim.set_weather(Weather::Clear);
    {
        let u = sim.units_mut();
        u[1].x = 600.0;
        u[1].z = 400.0;
        u[1].facing = std::f64::consts::PI;
        if scen == "flank" {
            u[0].x = 450.0;
            u[0].z = 400.0;
            u[0].facing = std::f64::consts::FRAC_PI_2;
        } else {
            u[0].x = 600.0;
            u[0].z = 250.0;
            u[0].facing = 0.0;
        }
        if scen == "rally" {
            u[0].z = 400.0;
            u[1].z = 700.0;
            u[0].morale = 10.0;
        }
    }
    if scen != "rally" {
        if scen == "stakes" {
            sim.units_mut()[1].ammo = 0;
            for _ in 0..160 {
                sim.step();
            }
        }
        sim.issue_command(Command::Attack {
            units: vec![0],
            target: 1,
            run: true,
        })
        .unwrap();
    }
    for t in 0..600 {
        sim.step();
        if t % 10 == 0 {
            let u = sim.units();
            println!(
                "t={:.1} a: {:?} ({:.0},{:.0}) hp {:.1} m {:.1} ct {:.1} | d: {:?} ({:.0},{:.0}) f {:.2} hp {:.1} m {:.1} stakes {} left {}",
                sim.elapsed(),
                u[0].state,
                u[0].x,
                u[0].z,
                u[0].hp,
                u[0].morale,
                u[0].charge_timer,
                u[1].state,
                u[1].x,
                u[1].z,
                u[1].facing,
                u[1].hp,
                u[1].morale,
                u[1].stakes_planted,
                u[0].left_field
            );
        }
    }
    for e in sim.events() {
        println!("{:.1} {}", e.time, e.text_fr);
    }
}
