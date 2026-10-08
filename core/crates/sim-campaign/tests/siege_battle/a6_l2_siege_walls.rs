//! A6-L2 (player audit M3): engines take longer against high walls and an
//! assault without breach is a poor bet (`data/rules/siege_engines.json`,
//! `data/rules/auto_resolve.json`).

use data_model::{GameData, SettlementId, UnitTypeId};
use sim_campaign::test_support::idle;
use sim_campaign::{siege_engines, ArmyId, CampaignState, Stance, Unit};

use data_model::test_support::{fac, game_data, prov};

/// France besieging English Guyenne, walls raised to `walls`, siege begun.
fn siege(data: &GameData, walls: u8) -> (CampaignState, ArmyId, SettlementId) {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 4).unwrap();
    let army = state
        .armies
        .iter()
        .find(|(_, a)| a.faction == fac("fac_france"))
        .map(|(id, _)| id.clone())
        .unwrap();
    let city = state
        .province_city_id(&prov("prov_guyenne"))
        .unwrap()
        .clone();
    let kent = state.province_city_id(&prov("prov_kent")).unwrap().clone();
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
    state
        .settlements
        .get_mut(&city)
        .unwrap()
        .fortification_level = walls;
    let a = state.armies.get_mut(&army).unwrap();
    a.position = sim_campaign::ArmyPosition::Settlement(city.clone());
    a.stance = Stance::Siege;
    a.clear_plan();
    state.end_turn_with(data, idle);
    assert!(state.settlement_state(&city).unwrap().siege.is_some());
    (state, army, city)
}

fn set_breach(state: &mut CampaignState, city: &SettlementId, breach: u8) {
    state
        .settlements
        .get_mut(city)
        .unwrap()
        .siege
        .as_mut()
        .unwrap()
        .breach = breach;
}

#[test]
fn engines_are_not_ready_on_the_first_turn_against_level_5_walls() {
    let data = game_data();
    let (state, army, city) = siege(data, 5);
    assert!(state.fortification_level(data, &city) >= 5);
    let engines = state.siege_engines(data, &city);
    assert!(
        engines.iter().all(|e| !e.ready && e.turns_left >= 1),
        "{engines:?}"
    );
    // The assault is refused: walls stand, nothing ready.
    assert!(state.assault_blocker(data, &army).is_some());
}

#[test]
fn engines_cost_more_against_higher_walls() {
    let data = game_data();
    // 2 000 men: 20 points a turn.
    let rate = 20;
    let turns =
        |walls: u32, index: usize| siege_engines::statuses(data, 0, rate, walls)[index].turns_left;
    // Ladders: a season more against level 3+, one against a palisade.
    assert!(turns(5, 0) >= 2 && turns(3, 0) >= 2);
    assert_eq!(turns(1, 0), 1);
    // Ram and tower: several seasons against level 5.
    assert!(turns(5, 1) >= 3);
    assert!(turns(5, 2) >= 8);
    assert!(turns(5, 2) > turns(3, 2) && turns(3, 2) > turns(1, 2));
}

#[test]
fn assault_chance_grows_with_the_breach() {
    let data = game_data();
    let (mut state, army, city) = siege(data, 5);
    let mut chance = |breach: u8| {
        set_breach(&mut state, &city, breach);
        state.assault_win_chance(data, &army).unwrap()
    };
    let none = chance(0);
    let some = chance(25);
    let almost = chance(45);
    let open = chance(60);
    assert!(
        none < some && some < almost && almost <= open,
        "{none} {some} {almost} {open}"
    );
}

#[test]
fn intact_level_5_walls_keep_a_3_to_1_assault_under_half() {
    let data = game_data();
    // Base level 2 plus the city's buildings: walls of level 5.
    let (mut state, army, city) = siege(data, 2);
    assert_eq!(state.fortification_level(data, &city), 5);
    let t = &data.unit_types[&UnitTypeId::new("unit_men_at_arms_foot").unwrap()];
    let guyenne = prov("prov_guyenne");
    // Power ratio of an army of `n` against the garrison, walls breached
    // (no wall modifier) or intact; the staging gives ladders, taken away.
    let forecast = |state: &mut CampaignState, n: usize, breach: u8| {
        state.armies.get_mut(&army).unwrap().units = (0..n).map(|_| Unit::fresh(t)).collect();
        state.pending_battles.clear();
        let index = state.debug_stage_siege(data, &army, &guyenne).unwrap();
        let siege = state
            .settlements
            .get_mut(&city)
            .unwrap()
            .siege
            .as_mut()
            .unwrap();
        siege.engine_work = 0;
        siege.breach = breach;
        let f = state.battle_forecast(data, index).unwrap();
        (f.attacker_power / f.defender_power, f.attacker_win_chance)
    };
    let n = (1..80)
        .find(|n| forecast(&mut state, *n, 100).0 >= 3.0)
        .expect("a 3:1 army");
    let (open, open_chance) = forecast(&mut state, n, 100);
    assert!((3.0..4.0).contains(&open), "ratio {open}");
    let (_, intact_chance) = forecast(&mut state, n, 0);
    assert!(intact_chance <= 0.5, "chance {intact_chance}");
    assert!(open_chance > intact_chance);
    // The quick estimate of the army bar agrees.
    set_breach(&mut state, &city, 0);
    assert!(state.assault_win_chance(data, &army).unwrap() <= 0.5);
}
