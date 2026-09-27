//! Debug probe: prints a few lab scenarios tick by tick.
//! `cargo run -p sim-battle --example probe -- <front|flank|stakes|rally|ai [size]|siege [breach] [engines,...]>`

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
        forced_march: false,
        entrenched: false,
        start_fatigue: 0.0,
    }
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let data = GameData::load(&root).unwrap().0;
    let scen = std::env::args().nth(1).unwrap_or_default();
    if scen == "siege" {
        let breach: u8 = std::env::args()
            .nth(2)
            .and_then(|a| a.parse().ok())
            .unwrap_or(0);
        let kinds: Vec<&str> = std::env::args()
            .nth(3)
            .map(|k| k.leak().split(',').collect())
            .unwrap_or_else(|| vec!["unit_trebuchet", "unit_trebuchet", "unit_siege_tower"]);
        let mut attackers: Vec<UnitSetup> = [
            "unit_men_at_arms_foot",
            "unit_men_at_arms_foot",
            "unit_urban_militia",
            "unit_urban_militia",
            "unit_urban_militia",
            "unit_longbowmen",
            "unit_longbowmen",
            "unit_knights",
        ]
        .iter()
        .map(|k| unit(&data, k))
        .collect();
        attackers.extend(
            kinds
                .iter()
                .filter(|k| !k.is_empty())
                .map(|k| unit(&data, k)),
        );
        let defenders: Vec<UnitSetup> = [
            "unit_urban_militia",
            "unit_urban_militia",
            "unit_crossbowmen",
            "unit_crossbowmen",
            "unit_men_at_arms_foot",
        ]
        .iter()
        .map(|k| unit(&data, k))
        .collect();
        let (mut total, mut wins, mut fires) = (0.0, 0, 0);
        let seeds: u64 = std::env::var("SEEDS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(6);
        for seed in 0..seeds {
            let setup = BattleSetup {
                province: String::new(),
                province_name: String::new(),
                terrain: Terrain::Plains,
                river: false,
                season: BattleSeason::Summer,
                coastal: false,
                village: None,
                attacker: side(attackers.clone()),
                defender: side(defenders.clone()),
                player_side: None,
                siege: Some(SiegeSetup {
                    fortification: 2,
                    breach,
                }),
                siege_layout: None,
                orders: Vec::new(),
                standards: None,
                decor_plan: None,
                opening: Default::default(),
            };
            let mut sim = BattleSim::new(setup, seed).unwrap();
            // S2: `FIRE=off` disables the siege fires, `WEATHER=clear|rain|fog|snow`
            // forces the weather.
            if std::env::var("FIRE").is_ok_and(|v| v == "off") {
                sim.set_fire_rules(None);
            }
            match std::env::var("WEATHER").as_deref() {
                Ok("clear") => sim.set_weather(Weather::Clear),
                Ok("rain") => sim.set_weather(Weather::Rain),
                Ok("fog") => sim.set_weather(Weather::Fog),
                Ok("snow") => sim.set_weather(Weather::Snow),
                _ => {}
            }
            while !sim.is_finished() {
                sim.step();
                if std::env::var("TRACE").is_ok()
                    && seed == 0
                    && (sim.ticks() < 3 || sim.ticks().is_multiple_of(300))
                {
                    for u in sim.units() {
                        println!(
                            "t={:.0} #{} {:?} {} ({:.0},{:.0}) {:?} m{:.0} hp{:.0} wall{} climb{:?}",
                            sim.elapsed(),
                            u.id,
                            u.side,
                            u.name,
                            u.x,
                            u.z,
                            u.state,
                            u.morale,
                            u.hp,
                            u.on_wall,
                            u.climbing
                        );
                    }
                }
            }
            let outcome = sim.outcome().unwrap();
            if std::env::var("EVENTS").is_ok() && seed == 0 {
                for e in sim.events() {
                    println!("  {:.0} {:?} {}", e.time, e.side, e.text_fr);
                }
            }
            total += sim.elapsed();
            if outcome.winner == SideId::Attacker {
                wins += 1;
            }
            let works = sim.siege().unwrap();
            let fire = works.burning_houses() + works.burnt_houses();
            fires += fire;
            println!(
                "seed {seed}: {:.0} s, winner {:?}, losses {} / {}, walls {:.0} %, weather {:?}, houses on fire or burnt {fire}/{}, gate fire {:?}",
                sim.elapsed(),
                outcome.winner,
                outcome.attacker.total_losses,
                outcome.defender.total_losses,
                works.integrity() * 100.0,
                sim.weather(),
                works.houses.len(),
                works.gate_fire.state,
            );
        }
        println!(
            "mean {:.0} s, attacker wins {wins}/{seeds}, houses reached by fire {:.1} per battle",
            total / seeds as f64,
            fires as f64 / seeds as f64
        );
        return;
    }
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
        let size: usize = std::env::args()
            .nth(2)
            .and_then(|a| a.parse().ok())
            .unwrap_or(20);
        let (mut total, mut att_wins) = (0.0, 0);
        for seed in 0..10 {
            let setup = BattleSetup {
                province: String::new(),
                province_name: String::new(),
                terrain: Terrain::Plains,
                river: seed % 2 == 0,
                season: BattleSeason::Summer,
                coastal: false,
                village: None,
                attacker: side(army(size)),
                defender: side(army(size)),
                player_side: None,
                siege: None,
                siege_layout: None,
                orders: Vec::new(),
                standards: None,
                decor_plan: None,
                opening: Default::default(),
            };
            let mut sim = BattleSim::new(setup, seed).unwrap();
            let start = std::time::Instant::now();
            let mut steps = 0;
            while !sim.is_finished() && steps < 36_100 {
                sim.step();
                steps += 1;
            }
            let outcome = sim.outcome().unwrap();
            if std::env::var("EVENTS").is_ok() && seed == 0 {
                for e in sim.events() {
                    println!("  {:.0} {:?} {}", e.time, e.side, e.text_fr);
                }
            }
            total += sim.elapsed();
            if outcome.winner == SideId::Attacker {
                att_wins += 1;
            }
            println!(
                "seed {seed}: {:.0} s ({:.2} ms/step), winner {:?}, losses {} / {}, weather {:?}",
                sim.elapsed(),
                start.elapsed().as_secs_f64() * 1000.0 / steps as f64,
                outcome.winner,
                outcome.attacker.total_losses,
                outcome.defender.total_losses,
                sim.weather()
            );
        }
        println!("mean {:.0} s, attacker wins {att_wins}/10", total / 10.0);
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
        coastal: false,
        village: None,
        attacker: side(vec![unit(&data, a)]),
        defender: side(vec![unit(&data, d)]),
        player_side: None,
        siege: None,
        siege_layout: None,
        orders: Vec::new(),
        standards: None,
        decor_plan: None,
        opening: Default::default(),
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
            queue: false,
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
