//! EP7: historical battle maps (ADR 0035): Crécy, Agincourt, Poitiers.
//!
//! Each map loads, lays its site (real relief, woods, water, decor) and its
//! historical deployment; with the AI on both sides the historical result
//! is the most likely one, never a certainty.
//!
//! `cargo test --release -p sim-battle --test ep7_historical -- --ignored --nocapture survey`
//! prints the survey (winner, end, duration, losses) over more seeds.

mod common;

use std::path::PathBuf;

use common::*;
use sim_battle::{BattleSim, HistoricalMap, SideId};

/// The historical maps shipped.
const MAPS: &[&str] = &["crecy", "azincourt", "poitiers"];

fn map(id: &str) -> HistoricalMap {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../data/battle_maps")
        .join(format!("{id}.json"));
    let text = std::fs::read_to_string(&path).expect("map file");
    HistoricalMap::from_json(&text).expect("map parses")
}

/// The game data, loaded once for every test of this file.
fn game_data() -> &'static data_model::GameData {
    static DATA: std::sync::OnceLock<data_model::GameData> = std::sync::OnceLock::new();
    DATA.get_or_init(data)
}

fn start(id: &str, seed: u64) -> (HistoricalMap, BattleSim) {
    let data = game_data();
    let map = map(id);
    let setup = map
        .battle_setup(
            &data.unit_types,
            data.battle_orders.values().cloned().collect(),
            Some(data.battle_standard_rules.clone()),
            None,
        )
        .expect("setup");
    let sim = map.start(setup, seed).expect("battle starts");
    (map, sim)
}

#[test]
fn crecy_loads_and_deploys() {
    let (map, sim) = start("crecy", 1);
    assert!(sim.is_historical());
    assert_eq!(sim.field().width, map.field.width_m);
    for u in sim.units() {
        assert!(u.present(), "every regiment deployed: {}", u.name);
        assert!(sim.field().inside(u.x, u.z));
    }
}

/// Result of one AI-vs-AI run.
struct Run {
    winner: Option<SideId>,
    end: String,
    seconds: f64,
    lost: [u32; 2],
    initial: [u32; 2],
}

fn play(id: &str, seed: u64, cap: f64) -> Run {
    let (_, mut sim) = start(id, seed);
    let initial = [
        sim.strength(SideId::Attacker),
        sim.strength(SideId::Defender),
    ];
    run(&mut sim, cap);
    let end = sim
        .outcome()
        .map_or("unfinished".to_owned(), |o| format!("{:?}", o.end));
    Run {
        winner: sim.winner(),
        end,
        seconds: sim.elapsed(),
        lost: [
            initial[0].saturating_sub(sim.strength(SideId::Attacker)),
            initial[1].saturating_sub(sim.strength(SideId::Defender)),
        ],
        initial,
    }
}

/// Plays `seeds` and returns how many the historical winner took.
fn survey(id: &str, seeds: std::ops::Range<u64>) -> usize {
    let winner = map(id).historical_winner;
    let mut wins = 0;
    for seed in seeds {
        let started = std::time::Instant::now();
        let r = play(id, seed, 2400.0);
        if r.winner == Some(winner) {
            wins += 1;
        }
        println!(
            "{id} seed {seed:2}: winner {:?} ({}) at {:5.0} s, lost {}/{} vs {}/{} [{:.1} s]",
            r.winner,
            r.end,
            r.seconds,
            r.lost[0],
            r.initial[0],
            r.lost[1],
            r.initial[1],
            started.elapsed().as_secs_f64()
        );
    }
    wins
}

/// Prints the course of a battle every 30 s and its journal
/// (`EP7_MAP=crecy EP7_SEED=1`).
#[test]
#[ignore]
fn trace() {
    let id = std::env::var("EP7_MAP").unwrap_or_else(|_| "crecy".to_owned());
    let seed = std::env::var("EP7_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let (_, mut sim) = start(&id, seed);
    for _ in 0..80 {
        run(&mut sim, 30.0);
        let mut line = format!("t={:4.0}", sim.elapsed());
        for side in SideId::BOTH {
            let units: Vec<_> = sim.units().iter().filter(|u| u.side == side).collect();
            let z = units.iter().map(|u| u.z).sum::<f64>() / units.len() as f64;
            let count =
                |f: &dyn Fn(&sim_battle::Unit) -> bool| units.iter().filter(|u| f(u)).count();
            line += &format!(
                " | {} z {:5.0} mov {} melee {} rout {} str {}",
                side.key(),
                z,
                count(&|u| u.destination.is_some() || u.target.is_some()),
                count(&|u| u.state == sim_battle::UnitState::Melee),
                count(&|u| u.state == sim_battle::UnitState::Routing),
                sim.strength(side)
            );
        }
        println!("{line}");
        if sim.is_finished() {
            break;
        }
    }
    for e in sim.events() {
        println!("{:5.0} {}", e.time, e.text_fr);
    }
}

#[test]
#[ignore]
fn survey_all() {
    for &id in MAPS {
        let wins = survey(id, 1..31);
        println!("{id}: historical winner {wins}/30");
    }
}

/// English victories over seeds 1-20 of map `id`.
fn english_wins(id: &str) -> usize {
    (1..21)
        .filter(|&seed| play(id, seed, 2400.0).winner == Some(SideId::Defender))
        .count()
}

/// Crécy: with the AI on both sides the English win most of the time
/// (>= 70 % of the seeds), never always.
#[test]
fn crecy_is_most_often_an_english_victory() {
    let wins = english_wins("crecy");
    assert!((14..20).contains(&wins), "English won {wins}/20");
}

/// Agincourt: the English win most of the time (>= 70 %), never always.
#[test]
fn azincourt_is_most_often_an_english_victory() {
    let wins = english_wins("azincourt");
    assert!((14..20).contains(&wins), "English won {wins}/20");
}

/// Poitiers: a closer battle, the English still win most of the time
/// (between 55 % and 90 % of the seeds).
#[test]
fn poitiers_is_closer_but_mostly_english() {
    let wins = english_wins("poitiers");
    assert!((11..19).contains(&wins), "English won {wins}/20");
}

/// The waves of the French hold back until released: at Crécy the first
/// "battle" of knights has not moved while the Genoese open the fight, and
/// is released later; the captal de Buch waits at Poitiers.
#[test]
fn waves_are_held_then_released() {
    let (_, mut sim) = start("crecy", 3);
    let knights: Vec<usize> = (0..sim.units().len())
        .filter(|&i| {
            sim.units()[i]
                .name
                .starts_with("Chevaliers du comte d'Alençon")
        })
        .collect();
    assert!(!knights.is_empty());
    let before: Vec<(f64, f64)> = knights
        .iter()
        .map(|&i| (sim.units()[i].x, sim.units()[i].z))
        .collect();
    run(&mut sim, 60.0);
    for (k, &i) in knights.iter().enumerate() {
        assert!(sim.scenario_held(i), "held at 60 s");
        let u = &sim.units()[i];
        assert!((u.z - before[k].1).abs() < 5.0, "the held knights stay put");
    }
    run(&mut sim, 300.0);
    assert!(
        knights.iter().all(|&i| !sim.scenario_held(i)),
        "released by 360 s"
    );
    assert!(sim.scenario_waves(SideId::Attacker)[1].1);
    let (_, sim) = start("poitiers", 3);
    let captal = (0..sim.units().len())
        .find(|&i| sim.units()[i].name.contains("captal"))
        .expect("the captal de Buch");
    assert!(sim.scenario_held(captal));
}

/// The English hold their ground: after the first minutes their posted
/// regiments are still near their posts.
#[test]
fn posted_regiments_hold_their_ground() {
    let (_, mut sim) = start("azincourt", 5);
    run(&mut sim, 120.0);
    for i in 0..sim.units().len() {
        if let Some((x, z, _, leash)) = sim.scenario_post(i) {
            let u = &sim.units()[i];
            if u.able() {
                assert!(
                    (u.x - x).hypot(u.z - z) <= leash + 40.0,
                    "{} left its post",
                    u.name
                );
            }
        }
    }
}

/// A campaign battle in the province and years of a map is fought on its
/// site: the campaign armies deploy as usual on the real relief, with the
/// map's woods, water and decor, out of the woods and the water.
#[test]
fn a_campaign_battle_can_be_fought_on_the_site() {
    let data = game_data();
    let map = map("crecy");
    assert!(map.matches_campaign("prov_ponthieu", 1346));
    assert!(!map.matches_campaign("prov_ponthieu", 1415));
    assert!(!map.matches_campaign("prov_artois", 1346));
    let mut setup = common::setup(
        units(
            data,
            &["unit_knights", "unit_crossbowmen", "unit_men_at_arms_foot"],
        ),
        units(
            data,
            &[
                "unit_longbowmen",
                "unit_men_at_arms_foot",
                "unit_longbowmen",
            ],
        ),
        None,
    );
    setup.province = "prov_ponthieu".to_owned();
    setup.river = true;
    let mut sim = BattleSim::new_scaled(setup, 11, map.scale()).expect("battle");
    map.apply_site(sim.field_mut());
    let field = sim.field();
    assert!(
        field.river.is_none(),
        "the map's own water replaces the drawn river"
    );
    assert_eq!(field.streams.len(), map.streams.len());
    assert!(
        !field.decor.buildings.is_empty(),
        "Crécy, Wadicourt and the mill"
    );
    assert_eq!(field.decor.camps.len(), 2);
    assert!(
        !sim.is_historical(),
        "no scenario: the campaign armies are free"
    );
    for u in sim.units() {
        assert!(field.inside(u.x, u.z));
        assert!(
            field.water_at(u.x, u.z) != Some(true),
            "{} in deep water",
            u.name
        );
    }
    run(&mut sim, 60.0);
}

/// Every map loads, lays its site and deploys every regiment on the field,
/// out of deep water and woods for the formed regiments.
#[test]
fn every_map_deploys_on_its_site() {
    for &id in MAPS {
        let (map, sim) = start(id, 7);
        assert!(sim.is_historical(), "{id}");
        assert_eq!(sim.field().width, map.field.width_m, "{id}");
        assert!(
            sim.field().heights.iter().any(|&h| h > 1.0),
            "{id}: real relief laid"
        );
        for u in sim.units() {
            assert!(u.present(), "{id}: {} deployed", u.name);
            assert!(
                sim.field().inside(u.x, u.z),
                "{id}: {} on the field",
                u.name
            );
            assert!(
                !sim.field().in_forest(u.x, u.z),
                "{id}: {} not in a wood at {:.0},{:.0}",
                u.name,
                u.x,
                u.z
            );
        }
        assert_eq!(sim.field().decor.camps.len(), 2, "{id}: both camps");
    }
}
