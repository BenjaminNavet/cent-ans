//! NV2: historical names of the campaign's ships (`data/naval/ship_names.json`)
//! and the sea of a crossing to Calais (the Channel, not the North Sea).

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use data_model::{FactionId, GameData, SeaZoneId, SettlementId, ShipClassId};
use sim_campaign::naval::{crossing_sea, naval_setup, NavalRequest, ShipNamer};
use sim_campaign::{ArmyId, CampaignState};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn set(id: &str) -> SettlementId {
    SettlementId::new(id).unwrap()
}

fn sea(id: &str) -> SeaZoneId {
    SeaZoneId::new(id).unwrap()
}

fn class(id: &str) -> ShipClassId {
    ShipClassId::new(id).unwrap()
}

#[test]
fn crossings_to_calais_are_fought_in_the_channel() {
    let data = data();
    let state = CampaignState::new_1337(&data, fac("fac_england"), 1).unwrap();
    let crossing = |from: &str, to: &str| crossing_sea(&state, &data, &set(from), &set(to));
    // London lies on the North Sea, but the fight is off Calais.
    assert_eq!(
        crossing("set_londres", "set_calais"),
        Some(sea("sea_channel"))
    );
    assert_eq!(
        crossing("set_dover", "set_calais"),
        Some(sea("sea_channel"))
    );
    assert_eq!(
        crossing("set_calais", "set_dover"),
        Some(sea("sea_channel"))
    );
    assert_eq!(
        crossing("set_sandwich", "set_wissant"),
        Some(sea("sea_channel"))
    );
    // Other crossings keep the sea both coasts share.
    assert_eq!(
        crossing("set_dover", "set_sluis"),
        Some(sea("sea_north_sea"))
    );
    // Lot SL1: a sea lane carries its own sea both ways (route de l'Étape,
    // through the Strait of Dover).
    assert_eq!(
        crossing("set_calais", "set_londres"),
        Some(sea("sea_channel"))
    );
}

/// England's main army about to sail from `from` to `to`, intercepted by
/// a French squadron.
fn request(data: &GameData, from: &str, to: &str, seed: u64) -> (CampaignState, NavalRequest) {
    let mut state = CampaignState::new_1337(data, fac("fac_england"), seed).unwrap();
    state.naval.ensure(data);
    let army: ArmyId = state
        .armies()
        .iter()
        .filter(|(_, a)| a.faction == fac("fac_england"))
        .max_by_key(|(id, a)| (a.total_strength(), std::cmp::Reverse((*id).clone())))
        .map(|(id, _)| id.clone())
        .unwrap();
    state.armies.get_mut(&army).unwrap().position =
        sim_campaign::ArmyPosition::Settlement(set(from));
    let sea = crossing_sea(&state, data, &set(from), &set(to)).unwrap();
    let squadron = BTreeMap::from([(class("ship_cog"), 4), (class("ship_nef"), 2)]);
    let request = NavalRequest {
        army,
        from: set(from),
        to: set(to),
        interceptor: fac("fac_france"),
        sea,
        squadron,
        seed,
    };
    (state, request)
}

#[test]
fn campaign_ships_bear_historical_names() {
    let data = data();
    let (state, request) = request(&data, "set_rye", "set_calais", 11);
    let setup = naval_setup(&state, &data, &request);
    assert_eq!(setup.place_name, "le pas de Calais");
    assert_eq!(setup.sea_zone, "sea_channel");
    let names = &data.naval.ship_names;
    let english = names.of(&fac("fac_england")).unwrap();
    let french = names.of(&fac("fac_france")).unwrap();
    // The transports sail from Rye: its ships first.
    let rye = &english.ports[&set("set_rye")];
    let transports: Vec<&str> = setup
        .defender
        .ships
        .iter()
        .map(|s| s.name.as_str())
        .collect();
    assert!(transports.len() >= 2, "{transports:?}");
    assert_eq!(transports[0], "la Marguerite de Rye");
    for (name, expected) in transports.iter().zip(rye) {
        assert_eq!(name, expected);
    }
    // The French squadron: names of the faction (and of its Channel ports).
    let known: BTreeSet<&str> = french
        .names
        .iter()
        .chain(french.ports.values().flatten())
        .map(String::as_str)
        .collect();
    for ship in &setup.attacker.ships {
        assert!(known.contains(ship.name.as_str()), "{}", ship.name);
    }
    for side in [&setup.attacker, &setup.defender] {
        let unique: BTreeSet<&str> = side.ships.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(unique.len(), side.ships.len(), "no name twice");
        assert!(side.ships.iter().all(|s| !s.name.contains("n°")));
    }
    // Same request, same names.
    assert_eq!(naval_setup(&state, &data, &request), setup);
}

#[test]
fn names_follow_the_seed_and_run_out_gracefully() {
    let data = data();
    let naples = fac("fac_naples");
    let list = data.naval.ship_names.of(&naples).unwrap();
    let cog = class("ship_galley");
    let draw = |seed: u64, count: usize| {
        let mut namer = ShipNamer::new(&data, &naples, &[], seed);
        (0..count)
            .map(|n| namer.name(&data, &cog, n))
            .collect::<Vec<_>>()
    };
    assert_ne!(
        draw(0, 1),
        draw(1, 1),
        "the seed shifts the faction's names"
    );
    let total = list.names.len() + list.ports.values().map(Vec::len).sum::<usize>();
    let many = draw(3, total + 2);
    let unique: BTreeSet<&String> = many.iter().collect();
    assert_eq!(unique.len(), many.len());
    assert!(!many[total - 1].contains("n°"), "{many:?}");
    assert!(many[total].contains("n°"), "{many:?}");
    // A faction without names keeps « Galère n°1 ».
    let mut nobody = ShipNamer::new(&data, &fac("fac_nobody"), &[], 0);
    assert!(nobody.name(&data, &cog, 0).ends_with("n°1"));
}
