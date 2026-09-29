//! NT2: custom battle — roster, budget, validation, setup, forced weather.

use std::collections::BTreeMap;
use std::path::PathBuf;

use data_model::load::load_entities;
use data_model::{Faction, FactionId, UnitType, UnitTypeId};
use sim_battle::custom::{roster, CustomBattle, CustomBattleRules, CustomData, CustomSide};
use sim_battle::{ReplayStart, SideId, Weather};

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data")
}

fn load() -> (BTreeMap<UnitTypeId, UnitType>, BTreeMap<FactionId, Faction>) {
    let dir = data_dir();
    let units = load_entities(&dir.join("unit_types"), |u: &UnitType| &u.id).unwrap();
    let factions = load_entities(&dir.join("factions"), |f: &Faction| &f.id).unwrap();
    (units, factions)
}

fn side(faction: &str, units: &[&str]) -> CustomSide {
    CustomSide {
        faction: faction.to_owned(),
        budget: 0,
        units: units.iter().map(|u| (*u).to_owned()).collect(),
    }
}

fn battle(attacker: CustomSide, defender: CustomSide) -> CustomBattle {
    CustomBattle {
        attacker,
        defender,
        terrain: "hills".to_owned(),
        season: "autumn".to_owned(),
        weather: String::new(),
        hour: String::new(),
        siege: false,
        fortification: None,
        place: None,
        player_side: "attacker".to_owned(),
    }
}

#[test]
fn rules_match_the_spec() {
    let rules = CustomBattleRules::bundled();
    assert_eq!(rules.default_budget, 6000);
    assert!(rules.min_budget <= rules.default_budget && rules.default_budget <= rules.max_budget);
}

#[test]
fn roster_follows_faction_and_culture_restrictions() {
    let (units, factions) = load();
    let france = &factions[&FactionId::new("fac_france").unwrap()];
    let ottoman = &factions[&FactionId::new("fac_ottoman").unwrap()];
    let french: Vec<&str> = roster(&units, france)
        .iter()
        .map(|u| u.id.as_str())
        .collect();
    let turkish: Vec<&str> = roster(&units, ottoman)
        .iter()
        .map(|u| u.id.as_str())
        .collect();
    assert!(french.contains(&"unit_crossbowmen"), "{french:?}");
    assert!(!french.contains(&"unit_akinci"));
    assert!(turkish.contains(&"unit_akinci"));
}

#[test]
fn budget_cap_and_roster_are_checked() {
    let (units, factions) = load();
    let data = CustomData {
        unit_types: &units,
        factions: &factions,
    };
    let rules = CustomBattleRules::bundled();
    let ok = battle(
        side("fac_france", &["unit_crossbowmen", "unit_crossbowmen"]),
        side("fac_england", &["unit_crossbowmen"]),
    );
    let report = ok.validate(&data, rules);
    assert!(report.ok, "{:?}", report.errors);
    assert_eq!(report.attacker.cost, 900);
    assert_eq!(report.attacker.budget, 6000);

    let mut over = ok.clone();
    over.attacker.budget = 1000;
    over.attacker.units = vec!["unit_crossbowmen".to_owned(); 3];
    let report = over.validate(&data, rules);
    assert!(!report.ok);
    assert!(
        report.errors.iter().any(|e| e.contains("budget")),
        "{:?}",
        report.errors
    );

    let mut foreign = ok.clone();
    foreign.attacker.units = vec!["unit_akinci".to_owned()];
    assert!(!foreign.validate(&data, rules).ok);

    let mut crowded = ok.clone();
    crowded.attacker.budget = rules.max_budget;
    crowded.attacker.units = vec!["unit_crossbowmen".to_owned(); rules.max_units_per_side + 1];
    let report = crowded.validate(&data, rules);
    assert!(
        report.errors.iter().any(|e| e.contains("au plus")),
        "{:?}",
        report.errors
    );

    let mut empty = ok.clone();
    empty.defender.units.clear();
    assert!(!empty.validate(&data, rules).ok);

    let mut bad_field = ok;
    bad_field.weather = "hail".to_owned();
    bad_field.hour = "teatime".to_owned();
    assert_eq!(bad_field.validate(&data, rules).errors.len(), 2);
}

#[test]
fn a_valid_composition_builds_and_fights() {
    let (units, factions) = load();
    let data = CustomData {
        unit_types: &units,
        factions: &factions,
    };
    let rules = CustomBattleRules::bundled();
    let mut custom = battle(
        side("fac_france", &["unit_crossbowmen", "unit_crossbowmen"]),
        side("fac_england", &["unit_crossbowmen", "unit_crossbowmen"]),
    );
    custom.weather = "fog".to_owned();
    let setup = custom
        .battle_setup(&data, rules, Vec::new(), None, Vec::new())
        .unwrap();
    assert_eq!(setup.attacker.units.len(), 2);
    assert_eq!(setup.player_side, Some(SideId::Attacker));
    assert!(setup.attacker.general.is_some());
    assert_eq!(setup.terrain, data_model::Terrain::Hills);
    let start = ReplayStart::plain(setup.clone(), 11).with_weather(Some(Weather::Fog));
    let mut sim = start.build().unwrap();
    assert_eq!(sim.weather(), Weather::Fog);
    for _ in 0..10 {
        sim.step();
    }
    // The forced weather survives the replay start's JSON round trip.
    let text = serde_json::to_string(&start).unwrap();
    let back: ReplayStart = serde_json::from_str(&text).unwrap();
    assert_eq!(back.build().unwrap().weather(), Weather::Fog);

    custom.siege = true;
    let siege = custom
        .battle_setup(&data, rules, Vec::new(), None, Vec::new())
        .unwrap();
    assert_eq!(
        siege.siege.as_ref().map(|s| s.fortification),
        Some(rules.default_fortification)
    );
    assert!(ReplayStart::plain(siege, 3)
        .build()
        .unwrap()
        .siege()
        .is_some());

    // NT1: the chosen kind of place reaches the siege setup.
    custom.place = Some(sim_battle::siege_layouts::PlaceKind::Castle);
    let castle = custom
        .battle_setup(&data, rules, Vec::new(), None, Vec::new())
        .unwrap();
    assert_eq!(
        castle.siege.as_ref().map(|s| s.place),
        Some(sim_battle::siege_layouts::PlaceKind::Castle)
    );
}
