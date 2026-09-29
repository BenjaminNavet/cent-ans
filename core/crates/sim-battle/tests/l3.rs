//! Siege battles in the landmark cities (lot L3, ADR 0026): the besieged town
//! is drawn from `data/landmarks/<id>.json` (block `siege.battle`).

mod common;

use common::*;
use data_model::GameData;
use sim_battle::{
    siege::TOWN_CENTER, BattleSim, PieceKind, SiegeLayout, SiegeSetup, SiegeWorks, UnitSetup,
};

const BESIEGERS: [&str; 6] = [
    "unit_men_at_arms_foot",
    "unit_urban_militia",
    "unit_urban_militia",
    "unit_longbowmen",
    "unit_longbowmen",
    "unit_mounted_sergeants",
];
const GARRISON: [&str; 4] = [
    "unit_urban_militia",
    "unit_crossbowmen",
    "unit_crossbowmen",
    "unit_men_at_arms_foot",
];

fn layout(data: &GameData, id: &str) -> SiegeLayout {
    SiegeLayout::from_landmark(&data.landmarks[id]).expect("siege.battle block")
}

fn landmark_siege(data: &GameData, id: &str, breach: u8, seed: u64) -> BattleSim {
    let attackers: Vec<UnitSetup> = units(data, &BESIEGERS);
    let mut setup = setup(
        attackers,
        units(data, &GARRISON),
        Some(SiegeSetup {
            fortification: 2,
            breach,
            ..Default::default()
        }),
    );
    setup.siege_layout = Some(layout(data, id));
    BattleSim::new(setup, seed).unwrap()
}

#[test]
fn every_landmark_city_gives_its_own_besieged_town() {
    let data = data();
    assert!(data.landmarks.len() >= 7, "Paris and the six L2 cities");
    for id in data.landmarks.keys() {
        let layout = layout(&data, id);
        let works =
            SiegeWorks::from_layout(&layout, 2, 0, &mut sim_battle::BattleRng::from_seed(1))
                .unwrap_or_else(|| panic!("{id}: unusable ring"));
        let gate = &works.pieces[works.gate];
        assert_eq!(gate.kind, PieceKind::Gate, "{id}");
        assert!(gate.outward().1 < -0.8, "{id}: gate faces the attacker");
        assert!(!works.front_walls().is_empty(), "{id}");
        assert!(works.inside(TOWN_CENTER.0, TOWN_CENTER.1), "{id}");
        assert!(
            works.pieces.len() >= 8,
            "{id}: {} pieces",
            works.pieces.len()
        );
        assert!(
            works.houses.len() >= 20,
            "{id}: {} houses",
            works.houses.len()
        );
        let landmark = works.landmark.as_ref().expect("landmark info");
        assert_eq!(landmark.gate_name, layout.gates[layout.gate].name, "{id}");
        assert!(!landmark.gatehouses.is_empty(), "{id}: other gates");
        assert!(landmark.plan_scale > 1.0, "{id}: drawn smaller than life");
        for house in &works.houses {
            assert!(works.inside(house.x, house.z), "{id}");
            assert!(works.house_at(TOWN_CENTER.0, TOWN_CENTER.1, 0.0).is_none());
        }
    }
}

#[test]
fn a_siege_of_paris_is_fought_in_the_left_bank_walls() {
    let data = data();
    let sim = landmark_siege(&data, "paris", 0, 5);
    let works = sim.siege().unwrap();
    let landmark = works.landmark.as_ref().expect("Paris plan");
    assert_eq!(landmark.id, "paris");
    assert_eq!(landmark.gate_name, "Porte Saint-Jacques");
    // Philippe Auguste's left-bank wall is open on the Seine: closed by a quay.
    assert!(!landmark.quay.is_empty());
    // Every regiment starts on firm ground: attackers outside, garrison inside.
    for unit in sim.units() {
        let inside = works.inside(unit.x, unit.z);
        match unit.side {
            sim_battle::SideId::Attacker => assert!(!inside, "{}", unit.name),
            sim_battle::SideId::Defender => assert!(inside || unit.on_wall, "{}", unit.name),
        }
    }
}

#[test]
fn landmark_sieges_are_deterministic_and_playable() {
    let data = data();
    for id in ["paris", "bruges"] {
        let mut a = landmark_siege(&data, id, 60, 11);
        let mut b = landmark_siege(&data, id, 60, 11);
        assert_eq!(a.siege(), b.siege(), "{id}");
        lab(&mut a);
        lab(&mut b);
        run(&mut a, 120.0);
        run(&mut b, 120.0);
        let pos = |s: &BattleSim| {
            s.units()
                .iter()
                .map(|u| (u.x, u.z, u.soldiers()))
                .collect::<Vec<_>>()
        };
        assert_eq!(pos(&a), pos(&b), "{id}");
        assert_eq!(
            a.siege().unwrap().openings().len(),
            b.siege().unwrap().openings().len()
        );
        assert!(!a.siege().unwrap().openings().is_empty(), "{id}: breach 60");
    }
}

#[test]
fn a_campaign_siege_of_a_landmark_city_carries_its_layout() {
    // The JSON round trip of the bridge (CampaignSim.get_battle_setup →
    // BattleSim.setup) keeps the layout.
    let data = data();
    let mut s = setup(
        units(&data, &BESIEGERS),
        units(&data, &GARRISON),
        Some(SiegeSetup {
            fortification: 1,
            breach: 0,
            ..Default::default()
        }),
    );
    s.siege_layout = Some(layout(&data, "rouen"));
    let json = serde_json::to_value(&s).unwrap();
    let back: sim_battle::BattleSetup = serde_json::from_value(json).unwrap();
    assert_eq!(back.siege_layout, s.siege_layout);
}
