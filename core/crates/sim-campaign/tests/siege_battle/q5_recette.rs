//! Q5 player playtest: the assault odds of the army bar match the assault
//! screen, besiegers stand down once the town is theirs, and messages name
//! armies instead of showing their raw ids.

use data_model::{GameData, ProvinceId, SettlementId};
use sim_campaign::test_support::{city, idle};
use sim_campaign::{ArmyId, CampaignState, Order, Stance};

use data_model::test_support::{fac, game_data};

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

/// NT5 (N7): the besiegers' ladders are built (an assault behind standing
/// walls needs one ready engine).
fn ladders_ready(state: &mut CampaignState, data: &GameData, place: &SettlementId) {
    // A6-L2: the ladders cost more against high walls.
    let ladders = data.siege_engine_rules.engines[0].cost(
        data.siege_engine_rules.scaling_min_wall_level,
        state.fortification_level(data, place),
    );
    if let Some(siege) = state
        .settlements
        .get_mut(place)
        .and_then(|s| s.siege.as_mut())
    {
        siege.engine_work = siege.engine_work.max(ladders);
    }
}

#[test]
fn army_bar_odds_match_the_assault_screen() {
    let data = game_data();
    let (mut state, army) = besiege_guyenne(data, 5);
    state.end_turn_with(data, idle);
    let place = state.armies[&army].settlement().unwrap().clone();
    ladders_ready(&mut state, data, &place);
    let bar = state
        .assault_win_chance(data, &army)
        .expect("the army besieges Guyenne");
    state
        .submit_order(data, Order::Assault { army: army.clone() })
        .unwrap();
    let index = state
        .pending_battles
        .iter()
        .position(|b| b.siege && b.attacker == army)
        .expect("the assault waits for the player");
    let screen = state.battle_forecast(data, index).unwrap();
    assert!(
        (screen.attacker_win_chance - bar).abs() < 1e-9,
        "bar {bar} vs screen {}",
        screen.attacker_win_chance
    );
}

#[test]
fn besiegers_stand_down_when_the_town_falls() {
    let data = game_data();
    let (mut state, army) = besiege_guyenne(data, 1);
    let guyenne = city(&state, "prov_guyenne");
    let mut taken = false;
    for _ in 0..14 {
        state.end_turn_with(data, idle);
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
    let data = game_data();
    let state = CampaignState::new_1337(data, fac("fac_france"), 1).unwrap();
    for id in state.armies.keys() {
        let name = state.army_name(data, id);
        assert!(name.starts_with("l'ost "), "{name}");
        assert!(!name.contains("army_"), "{name}");
    }
}
