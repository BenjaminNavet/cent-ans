//! CB6: group formations (attack / defence presets, proposed placement).
//!
//! `data/rules/group_formations.json` holds six presets; the pure
//! `BattleSim::formation_slots` turns one into a place per regiment. The
//! initial deployment and the AI deployment go through « Ligne de
//! bataille » with the very placement they had before CB6 (golden file
//! `fixtures/cb6_deploy_golden.json`, written by the ignored
//! `write_deploy_golden` test on the code before CB6).

mod common;

use std::path::PathBuf;

use common::*;
use sim_battle::{BattleSetup, BattleSim, HistoricalMap, SideId};

// ----- scenarios of the non-regression golden --------------------------

fn demo_setup() -> BattleSetup {
    serde_json::from_str(include_str!("fixtures/demo_battle_1337.json")).unwrap()
}

fn mixed_setup() -> BattleSetup {
    let data = data();
    let french = [
        "unit_knights",
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_crossbowmen",
        "unit_crossbowmen",
        "unit_knights",
    ];
    let english = [
        "unit_men_at_arms_foot",
        "unit_longbowmen",
        "unit_longbowmen",
        "unit_knights",
    ];
    let mut battle = setup(units(&data, &french), units(&data, &english), None);
    battle.village = Some(false);
    battle
}

/// A big army whose rows wrap (wider than the field).
fn big_setup() -> BattleSetup {
    let data = data();
    let mut french: Vec<&str> = Vec::new();
    for _ in 0..18 {
        french.push("unit_men_at_arms_foot");
    }
    for _ in 0..10 {
        french.push("unit_crossbowmen");
    }
    for _ in 0..9 {
        french.push("unit_knights");
    }
    for _ in 0..3 {
        french.push("unit_trebuchet");
    }
    let english = ["unit_longbowmen", "unit_men_at_arms_foot", "unit_knights"];
    let mut battle = setup(units(&data, &french), units(&data, &english), None);
    battle.village = Some(false);
    battle
}

/// No infantry: the shooters take the line.
fn shooters_setup() -> BattleSetup {
    let data = data();
    let a = ["unit_crossbowmen", "unit_knights", "unit_longbowmen"];
    let d = ["unit_longbowmen", "unit_longbowmen", "unit_knights", "unit_knights"];
    let mut battle = setup(units(&data, &a), units(&data, &d), None);
    battle.village = Some(false);
    battle
}

fn historical(id: &str, seed: u64) -> BattleSim {
    let data = data();
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../data/battle_maps")
        .join(format!("{id}.json"));
    let map = HistoricalMap::from_json(&std::fs::read_to_string(path).unwrap()).unwrap();
    let setup = map
        .battle_setup(
            &data.unit_types,
            data.battle_orders.values().cloned().collect(),
            Some(data.battle_standard_rules.clone()),
            None,
        )
        .unwrap();
    map.start(setup, seed).unwrap()
}

/// Every golden scenario: its name and the battle just after set-up (and,
/// for the `*_deploy` ones, after the AI deployment of the other side).
fn golden_scenarios() -> Vec<(String, BattleSim)> {
    let mut out = Vec::new();
    for (name, setup) in [
        ("demo", demo_setup()),
        ("mixed", mixed_setup()),
        ("big", big_setup()),
        ("shooters", shooters_setup()),
    ] {
        out.push((name.to_owned(), BattleSim::new(setup.clone(), 7).unwrap()));
        for player in SideId::BOTH {
            let mut s = setup.clone();
            s.player_side = Some(player);
            let mut sim = BattleSim::new(s, 7).unwrap();
            sim.set_ai(player, false);
            sim.set_ai(player.other(), true);
            sim.begin_deployment();
            out.push((format!("{name}_deploy_{player:?}"), sim));
        }
    }
    for id in ["crecy", "azincourt", "poitiers"] {
        out.push((id.to_owned(), historical(id, 3)));
    }
    out
}

/// Exact positions (bit patterns) of every regiment.
fn positions(sim: &BattleSim) -> Vec<[u64; 3]> {
    sim.units()
        .iter()
        .map(|u| [u.x.to_bits(), u.z.to_bits(), u.facing.to_bits()])
        .collect()
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cb6_deploy_golden.json")
}

/// Writes the golden file (run once on the code before CB6).
#[test]
#[ignore]
fn write_deploy_golden() {
    let map: std::collections::BTreeMap<String, Vec<[u64; 3]>> = golden_scenarios()
        .into_iter()
        .map(|(name, sim)| (name, positions(&sim)))
        .collect();
    std::fs::write(golden_path(), serde_json::to_string_pretty(&map).unwrap()).unwrap();
}

#[test]
fn battle_line_is_the_placement_before_cb6() {
    let text = std::fs::read_to_string(golden_path()).expect("golden file");
    let golden: std::collections::BTreeMap<String, Vec<[u64; 3]>> =
        serde_json::from_str(&text).unwrap();
    let scenarios = golden_scenarios();
    assert_eq!(scenarios.len(), golden.len());
    for (name, sim) in scenarios {
        let now = positions(&sim);
        let before = &golden[&name];
        assert_eq!(now.len(), before.len(), "{name}");
        for (k, (a, b)) in now.iter().zip(before).enumerate() {
            assert_eq!(
                a,
                b,
                "{name}: regiment {k} moved: now ({}, {}), before ({}, {})",
                f64::from_bits(a[0]),
                f64::from_bits(a[1]),
                f64::from_bits(b[0]),
                f64::from_bits(b[1])
            );
        }
    }
}
