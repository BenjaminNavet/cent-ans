//! NV1 tests: naval battles, auto-resolved (ADR 0028, 0192) — determinism,
//! prizes, strength decides, the weather gauge, fireships.

use std::collections::BTreeMap;
use std::path::PathBuf;

use data_model::load::load_entities;
use data_model::{NavalData, NavalRules, UnitType, UnitTypeId};
use sim_battle::naval::{auto_resolve, CrewSetup, NavalSetup, NavalSideSetup, ShipFate, ShipSetup};
use sim_battle::{BattleSeason, SideId, UnitSetup};

fn root() -> PathBuf {
    data_model::test_support::data_dir()
}

struct Data {
    naval: NavalData,
    units: BTreeMap<UnitTypeId, UnitType>,
}

fn data() -> Data {
    let root = root();
    Data {
        naval: NavalData::load(&root).expect("naval data loads"),
        units: load_entities(&root.join("unit_types"), |u: &UnitType| &u.id).expect("unit types"),
    }
}

type ShipSpec<'a> = (&'a str, &'a [(&'a str, u32)]);

/// A fleet of `ships` (class id, [(unit type, men)]) sharing one regiment
/// per unit type.
fn fleet(data: &Data, name: &str, ships: &[ShipSpec]) -> NavalSideSetup {
    let mut units: Vec<UnitSetup> = Vec::new();
    let mut index: BTreeMap<String, usize> = BTreeMap::new();
    let mut out = Vec::new();
    for (n, (class, crew)) in ships.iter().enumerate() {
        let class = data.naval.ship_classes[*class].clone();
        let mut crew_setup = Vec::new();
        for (unit_type, men) in crew.iter() {
            let i = *index.entry((*unit_type).to_owned()).or_insert_with(|| {
                let ut = &data.units[*unit_type];
                units.push(UnitSetup::from_unit_type(ut, 0, 70, 0));
                units.len() - 1
            });
            units[i].soldiers += men;
            units[i].max_soldiers = units[i].max_soldiers.max(units[i].soldiers);
            crew_setup.push(CrewSetup { unit: i, men: *men });
        }
        out.push(ShipSetup {
            name: format!("{name} {n}"),
            class,
            crew: crew_setup,
            fireship: false,
            chain: None,
            fire_arrows: false,
        });
    }
    NavalSideSetup {
        faction: format!("fac_{}", name.to_lowercase()),
        faction_name: name.to_owned(),
        army: String::new(),
        admiral: String::new(),
        units,
        ships: out,
    }
}

fn battle(
    data: &Data,
    attacker: NavalSideSetup,
    defender: NavalSideSetup,
    gauge: Option<SideId>,
) -> NavalSetup {
    NavalSetup {
        sea_zone: "sea_channel".to_owned(),
        place_name: "Manche".to_owned(),
        season: BattleSeason::Summer,
        rain: false,
        gauge,
        attacker,
        defender,
        player_side: None,
        rules: data.naval.rules.clone(),
    }
}

const MAA: &str = "unit_men_at_arms_foot";
const BOWS: &str = "unit_longbowmen";
const MILITIA: &str = "unit_urban_militia";

#[test]
fn rules_file_matches_defaults() {
    let data = data();
    assert_eq!(*data.naval.rules, NavalRules::default());
    assert_eq!(data.naval.ship_classes.len(), 4);
    let england = data_model::FactionId::new("fac_england").unwrap();
    assert!(data.naval.fleet_of(&england).is_some());
}

/// Majority winner of the auto-resolve over three seeds.
fn majority_winner(setup: &NavalSetup) -> Option<SideId> {
    let mut count = [0, 0];
    for seed in [3, 17, 29] {
        match auto_resolve(setup, seed).winner {
            Some(SideId::Attacker) => count[0] += 1,
            Some(SideId::Defender) => count[1] += 1,
            None => {}
        }
    }
    if count[0] >= 2 {
        Some(SideId::Attacker)
    } else if count[1] >= 2 {
        Some(SideId::Defender)
    } else {
        None
    }
}

fn cog(unit: &'static str, men: u32) -> ShipSpec<'static> {
    ("ship_cog", Box::leak(Box::new([(unit, men)])))
}

#[test]
fn auto_resolve_is_deterministic_and_reports_prizes() {
    let data = data();
    let archers = vec![cog(BOWS, 60); 4];
    let militia = vec![cog(MILITIA, 70); 4];
    let setup = battle(
        &data,
        fleet(&data, "Angleterre", &archers),
        fleet(&data, "France", &militia),
        Some(SideId::Attacker),
    );
    let a = auto_resolve(&setup, 9);
    assert_eq!(a, auto_resolve(&setup, 9));
    assert_eq!(a.winner, Some(SideId::Attacker));
    let captured = a.defender.count(ShipFate::Captured);
    assert_eq!(a.attacker.prizes.len(), captured);
    assert!(a.attacker.men_lost < a.defender.men_lost);
}

#[test]
fn the_stronger_fleet_wins_whichever_side_it_is_on() {
    let data = data();
    let maa = vec![cog(MAA, 60); 3];
    let militia = vec![cog(MILITIA, 70); 3];
    for gauge in SideId::BOTH {
        let setup = battle(
            &data,
            fleet(&data, "Angleterre", &maa),
            fleet(&data, "France", &militia),
            Some(gauge),
        );
        assert_eq!(majority_winner(&setup), Some(SideId::Attacker));
        let swapped = battle(
            &data,
            fleet(&data, "France", &militia),
            fleet(&data, "Angleterre", &maa),
            Some(gauge),
        );
        assert_eq!(majority_winner(&swapped), Some(SideId::Defender));
    }
}

#[test]
fn a_fleet_without_ships_loses_by_default() {
    let data = data();
    let setup = battle(
        &data,
        fleet(&data, "Angleterre", &[]),
        fleet(&data, "France", &[cog(MAA, 60)]),
        None,
    );
    let outcome = auto_resolve(&setup, 1);
    assert_eq!(outcome.winner, Some(SideId::Defender));
    assert_eq!(outcome.duration, 0.0);
}

#[test]
fn a_fireship_hurts_the_fleet_it_drifts_onto() {
    let data = data();
    let cogs = vec![cog(MAA, 60); 2];
    let mut setup = battle(
        &data,
        fleet(&data, "Angleterre", &cogs),
        fleet(&data, "France", &cogs),
        Some(SideId::Attacker),
    );
    let calm = auto_resolve(&setup, 5);
    setup.attacker.ships[1].fireship = true;
    let burnt = auto_resolve(&setup, 5);
    assert_ne!(calm, burnt);
}
