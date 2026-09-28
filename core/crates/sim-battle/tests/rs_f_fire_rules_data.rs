//! RS-F (ADR 0099): the siege fire rules are read from `data/` at load time
//! (`FireRules::load` + `install`), the compiled-in copy staying the default.
//!
//! One test only in this binary: `install` changes the rules of every
//! battle built afterwards in the process.

mod common;

use std::path::PathBuf;

use common::*;
use sim_battle::{BattleSim, FireRules, SiegeSetup};

fn suburbs(data: &data_model::GameData) -> usize {
    let setup = setup(
        units(data, &["unit_men_at_arms_foot", "unit_longbowmen"]),
        units(data, &["unit_urban_militia"]),
        Some(SiegeSetup {
            fortification: 2,
            breach: 0,
        }),
    );
    let sim = BattleSim::new(setup, 11).unwrap();
    sim.siege()
        .unwrap()
        .houses
        .iter()
        .filter(|h| h.suburb)
        .count()
}

#[test]
fn fire_rules_come_from_the_data_folder_once_installed() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let loaded = FireRules::load(&root).expect("data/rules/siege_fire.json loads");
    assert_eq!(
        &loaded,
        FireRules::bundled(),
        "the bundled copy is the data file"
    );
    assert!(FireRules::load(&root.join("nowhere")).is_err());
    assert!(FireRules::from_json("{}").is_err());

    let data = data();
    let bundled = FireRules::bundled().suburbs.count as usize;
    assert_eq!(suburbs(&data), bundled, "bundled rules before any install");

    // A tuned file changes the next battles, without recompiling.
    let mut tuned = loaded.clone();
    tuned.suburbs.count = 2;
    FireRules::install(Some(tuned.clone()));
    assert_eq!(*FireRules::current(), tuned);
    assert_eq!(suburbs(&data), 2);

    FireRules::install(None);
    assert_eq!(*FireRules::current(), *FireRules::bundled());
    assert_eq!(suburbs(&data), bundled);
}
