//! Q5 player playtest: the assault odds of the army bar match the assault
//! screen, besiegers stand down once the town is theirs, and messages name
//! armies instead of showing their raw ids.

use std::path::PathBuf;

use data_model::{FactionId, GameData, ProvinceId, SettlementId};
use sim_campaign::{ArmyId, CampaignState, Order, Stance};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn city(state: &CampaignState, province: &str) -> SettlementId {
    state
        .province_city_id(&ProvinceId::new(province).unwrap())
        .unwrap()
        .clone()
}

fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

/// France's first army before the city of English Guyenne, in siege stance
/// (the English field armies are sent to Kent).
fn besiege_guyenne(data: &GameData, seed: u64) -> (CampaignState, ArmyId) {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), seed).unwrap();
    let army = state
        .armies
        .iter()
        .find(|(_, a)| a.faction == fac("fac_france"))
        .map(|(id, _)| id.clone())
        .unwrap();
    let guyenne = city(&state, "prov_guyenne");
    let kent = city(&state, "prov_kent");
    let english: Vec<ArmyId> = state
        .armies
        .iter()
        .filter(|(_, a)| {
            a.faction != fac("fac_france")
                && a.settlement().and_then(|s| state.settlement_province(s))
                    == Some(&ProvinceId::new("prov_guyenne").unwrap())
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
    (state, army)
}

#[test]
fn army_bar_odds_match_the_assault_screen() {
    let data = data();
    let (mut state, army) = besiege_guyenne(&data, 5);
    state.end_turn_with(&data, idle);
    let bar = state
        .assault_win_chance(&data, &army)
        .expect("the army besieges Guyenne");
    state
        .submit_order(&data, Order::Assault { army: army.clone() })
        .unwrap();
    let index = state
        .pending_battles
        .iter()
        .position(|b| b.siege && b.attacker == army)
        .expect("the assault waits for the player");
    let screen = state.battle_forecast(&data, index).unwrap();
    assert!(
        (screen.attacker_win_chance - bar).abs() < 1e-9,
        "bar {bar} vs screen {}",
        screen.attacker_win_chance
    );
}

#[test]
fn besiegers_stand_down_when_the_town_falls() {
    let data = data();
    let (mut state, army) = besiege_guyenne(&data, 1);
    let guyenne = city(&state, "prov_guyenne");
    let mut taken = false;
    for _ in 0..14 {
        state.end_turn_with(&data, idle);
        if state.settlement_state(&guyenne).unwrap().controller == fac("fac_france") {
            taken = true;
            break;
        }
    }
    assert!(taken, "Guyenne starves and falls");
    let stance = state.armies.get(&army).map(|a| a.stance);
    assert_eq!(stance, Some(Stance::Normal), "no siège label left behind");
}

#[test]
fn messages_name_armies_not_ids() {
    let data = data();
    let state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    for id in state.armies.keys() {
        let name = state.army_name(&data, id);
        assert!(name.starts_with("l'ost "), "{name}");
        assert!(!name.contains("army_"), "{name}");
    }
}
