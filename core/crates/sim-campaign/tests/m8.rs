//! M8 campaign siege warfare: supplies, breaches, assaults, sorties.

use std::path::PathBuf;

use data_model::{FactionId, GameData, ProvinceId, SettlementId, UnitTypeId};
use sim_campaign::{ArmyId, CampaignState, Order, Stance, Unit};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn prov(id: &str) -> ProvinceId {
    ProvinceId::new(id).unwrap()
}

/// The city of a province (lot C4: sieges target settlements).
fn city(state: &CampaignState, province: &str) -> SettlementId {
    state.province_city_id(&prov(province)).unwrap().clone()
}

fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

fn unit(data: &GameData, id: &str) -> Unit {
    let t = &data.unit_types[&UnitTypeId::new(id).unwrap()];
    Unit {
        unit_type: t.id.clone(),
        strength: 100,
        max_strength: 100,
        experience: 2,
        morale: 80,
        levy_armor: 0,
        levy_ranged: 0,
    }
}

/// France's main army moved to the city of English Guyenne in siege stance.
fn besiege_guyenne(data: &GameData, seed: u64, extra: &[&str]) -> (CampaignState, ArmyId) {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), seed).unwrap();
    let army = state
        .armies
        .iter()
        .find(|(_, a)| a.faction == fac("fac_france"))
        .map(|(id, _)| id.clone())
        .unwrap();
    let guyenne = city(&state, "prov_guyenne");
    let kent = city(&state, "prov_kent");
    // Clear English field armies from Guyenne so the siege can start.
    let english: Vec<ArmyId> = state
        .armies
        .iter()
        .filter(|(_, a)| {
            a.settlement().and_then(|s| state.settlement_province(s)) == Some(&prov("prov_guyenne"))
                && a.faction != fac("fac_france")
        })
        .map(|(id, _)| id.clone())
        .collect();
    for id in english {
        state.armies.get_mut(&id).unwrap().position =
            sim_campaign::ArmyPosition::Settlement(kent.clone());
    }
    let a = state.armies.get_mut(&army).unwrap();
    a.position = sim_campaign::ArmyPosition::Settlement(guyenne);
    a.stance = Stance::Siege;
    a.clear_plan();
    for id in extra {
        a.units.push(unit(data, id));
    }
    (state, army)
}

#[test]
fn a_starved_town_capitulates() {
    let data = data();
    let (mut state, _) = besiege_guyenne(&data, 1, &[]);
    let guyenne = city(&state, "prov_guyenne");
    state.end_turn_with(&data, idle);
    let siege = state
        .settlement_state(&guyenne)
        .unwrap()
        .siege
        .clone()
        .expect("siege started");
    assert_eq!(siege.attacker, fac("fac_france"));
    assert!(siege.supplies > 0 && siege.turns_left > 0);
    let mut taken = false;
    for _ in 0..12 {
        state.end_turn_with(&data, idle);
        if state.settlement_state(&guyenne).unwrap().controller == fac("fac_france") {
            taken = true;
            break;
        }
        let s = state.settlement_state(&guyenne).unwrap().siege.clone();
        if let Some(s) = s {
            assert!(s.supplies < siege.supplies || s.turns_elapsed > 0);
        }
    }
    assert!(taken, "Guyenne starves and falls");
}

#[test]
fn siege_engines_open_a_breach() {
    let data = data();
    let (mut state, _) = besiege_guyenne(&data, 2, &["unit_trebuchet", "unit_trebuchet"]);
    let guyenne = city(&state, "prov_guyenne");
    state.end_turn_with(&data, idle);
    state.end_turn_with(&data, idle);
    let breach = state
        .settlement_state(&guyenne)
        .unwrap()
        .siege
        .as_ref()
        .map_or(100, |s| s.breach);
    assert!(breach > 0, "trebuchets damage the walls");
}

#[test]
fn a_breach_makes_assaults_easier() {
    let data = data();
    let (mut state, army) = besiege_guyenne(&data, 3, &[]);
    state.end_turn_with(&data, idle);
    let (odds_walls, walls) = state.assault_odds(&data, &army).expect("besieging");
    assert!(walls);
    let guyenne = city(&state, "prov_guyenne");
    state
        .settlements
        .get_mut(&guyenne)
        .unwrap()
        .siege
        .as_mut()
        .unwrap()
        .breach = 80;
    let (odds_breach, walls) = state.assault_odds(&data, &army).unwrap();
    assert!(!walls);
    assert!(odds_breach > odds_walls, "{odds_walls} -> {odds_breach}");
}

#[test]
fn assault_takes_the_town_or_bloodies_the_attacker() {
    let data = data();
    let (mut state, army) = besiege_guyenne(&data, 4, &[]);
    state.interactive_battles = false; // auto-resolved assault (M8 § 1)
    state.end_turn_with(&data, idle);
    let before: u32 = state.armies[&army].units.iter().map(|u| u.strength).sum();
    state
        .submit_order(&data, Order::Assault { army: army.clone() })
        .unwrap();
    let guyenne = city(&state, "prov_guyenne");
    let taken = state.settlement_state(&guyenne).unwrap().controller == fac("fac_france");
    let after: u32 = state
        .armies
        .get(&army)
        .map_or(0, |a| a.units.iter().map(|u| u.strength).sum());
    assert!(taken || after < before);
    let events = state.end_turn_with(&data, idle);
    assert!(events.iter().any(|e| e.text_fr.contains("Assaut")));
}

#[test]
fn assault_requires_a_siege() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 5).unwrap();
    let army = state
        .armies
        .iter()
        .find(|(_, a)| a.faction == fac("fac_france"))
        .map(|(id, _)| id.clone())
        .unwrap();
    assert!(state.submit_order(&data, Order::Assault { army }).is_err());
}

#[test]
fn a_strong_garrison_sallies_out() {
    let data = data();
    let (mut state, army) = besiege_guyenne(&data, 6, &[]);
    // Weak besiegers, huge garrison.
    state.armies.get_mut(&army).unwrap().units.truncate(1);
    let guyenne = city(&state, "prov_guyenne");
    let knights = unit(&data, "unit_knights");
    let garrison = &mut state.settlements.get_mut(&guyenne).unwrap().garrison;
    for _ in 0..8 {
        garrison.push(knights.clone());
    }
    let events = state.end_turn_with(&data, idle);
    assert!(
        events.iter().any(|e| e.text_fr.contains("Sortie")),
        "{events:?}"
    );
}

#[test]
fn siege_state_survives_save() {
    let data = data();
    let (mut state, _) = besiege_guyenne(&data, 7, &["unit_trebuchet"]);
    state.end_turn_with(&data, idle);
    state.end_turn_with(&data, idle);
    let loaded = CampaignState::load_json(&state.save_json()).unwrap();
    assert_eq!(loaded, state);
}
