//! Lot TW2-T5 (ADR 0109): the AI spends the ranks of its armies on
//! traditions according to its doctrine (England's longbows: shooting) and
//! its situation (a depleted army: stewardship), through the campaign
//! planner.

use std::path::PathBuf;

use ai::traditions::{doctrine_ranged_share, pick_tradition, plan_traditions};
use data_model::{FactionId, GameData, TraditionBranch};
use sim_campaign::traditions::grant_army_xp;
use sim_campaign::{ArmyId, CampaignState, Order};

fn real_data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn main_army(state: &CampaignState, faction: &str) -> ArmyId {
    let faction = fac(faction);
    state
        .armies
        .iter()
        .filter(|(_, a)| a.faction == faction)
        .max_by_key(|(id, a)| (a.units.len(), std::cmp::Reverse((*id).clone())))
        .map(|(id, _)| id.clone())
        .expect("faction has an army")
}

fn branch_of(data: &GameData, tradition: &str) -> TraditionBranch {
    data.army_tradition_rules
        .tradition(tradition)
        .expect("known tradition")
        .branch
}

#[test]
fn england_trains_its_archers_and_a_depleted_army_its_stewards() {
    let data = real_data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 3).unwrap();
    assert!(
        doctrine_ranged_share(&state, &data, &fac("fac_england"))
            > doctrine_ranged_share(&state, &data, &fac("fac_france"))
    );
    let english = main_army(&state, "fac_england");
    // No rank: nothing to choose.
    assert_eq!(pick_tradition(&state, &data, &state.armies[&english]), None);
    let first = data.army_tradition_rules.experience.rank_thresholds[0];
    grant_army_xp(&mut state, &data, &english, first, &mut Vec::new());
    let pick = pick_tradition(&state, &data, &state.armies[&english]).unwrap();
    assert_eq!(branch_of(&data, &pick), TraditionBranch::Shooting, "{pick}");

    // A French army at a third of its strength replenishes first.
    let french = main_army(&state, "fac_france");
    for unit in &mut state.armies.get_mut(&french).unwrap().units {
        unit.strength = unit.max_strength / 3;
    }
    grant_army_xp(&mut state, &data, &french, first, &mut Vec::new());
    let pick = pick_tradition(&state, &data, &state.armies[&french]).unwrap();
    assert_eq!(
        branch_of(&data, &pick),
        TraditionBranch::Stewardship,
        "{pick}"
    );
}

#[test]
fn the_planner_spends_every_rank_and_the_orders_apply() {
    let data = real_data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 3).unwrap();
    let english = main_army(&state, "fac_england");
    let second = data.army_tradition_rules.experience.rank_thresholds[1];
    grant_army_xp(&mut state, &data, &english, second, &mut Vec::new());
    let orders = plan_traditions(&state, &data, &fac("fac_england"));
    assert_eq!(orders.len(), 2, "{orders:?}");
    for order in orders {
        assert!(matches!(order, Order::ChooseArmyTradition { .. }));
        state
            .apply_order(&data, &fac("fac_england"), order)
            .expect("the AI's choice is legal");
    }
    let chosen = &state.armies[&english].traditions.chosen;
    assert_eq!(chosen.len(), 2);
    // The second rank deepens the branch it started.
    assert_eq!(branch_of(&data, &chosen[0]), branch_of(&data, &chosen[1]));
    assert!(plan_traditions(&state, &data, &fac("fac_england")).is_empty());

    // The full campaign planner issues them too.
    let english_2 = state
        .armies
        .iter()
        .find(|(id, a)| a.faction == fac("fac_england") && **id != english)
        .map(|(id, _)| id.clone());
    if let Some(army) = english_2 {
        let first = data.army_tradition_rules.experience.rank_thresholds[0];
        grant_army_xp(&mut state, &data, &army, first, &mut Vec::new());
        let orders = ai::plan_turn(&state, &data, &fac("fac_england"));
        assert!(orders.iter().any(|o| matches!(
            o,
            Order::ChooseArmyTradition { army: a, .. } if a == &army
        )));
    }
}
