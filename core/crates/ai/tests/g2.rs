//! G2 « IA : alignement historique » tests (`docs/design/m9-ai.md` § G2).

use std::path::PathBuf;

use data_model::{FactionId, GameData};
use sim_campaign::{CampaignState, Order};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn gift_to(orders: &[Order], target: &FactionId) -> i64 {
    orders
        .iter()
        .filter_map(|o| match o {
            Order::SendGift { target: t, amount } if t == target => Some(*amount),
            _ => None,
        })
        .sum()
}

#[test]
fn rich_france_pays_the_debts_of_scotland() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_papacy"), 1).unwrap();
    let (france, scotland) = (fac("fac_france"), fac("fac_scotland"));
    state.factions.get_mut(&scotland).unwrap().treasury = -3000;
    state.factions.get_mut(&france).unwrap().treasury = 200_000;
    let orders = ai::plan_turn(&state, &data, &france);
    let paid = gift_to(&orders, &scotland);
    assert!(paid >= 3000, "subsidy {paid}");
    // Solvent allies get nothing.
    state.factions.get_mut(&scotland).unwrap().treasury = 50_000;
    assert_eq!(
        gift_to(&ai::plan_turn(&state, &data, &france), &scotland),
        0
    );
}

#[test]
fn a_poor_ally_does_not_subsidise_a_rich_one() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_papacy"), 1).unwrap();
    let (france, scotland) = (fac("fac_france"), fac("fac_scotland"));
    state.factions.get_mut(&france).unwrap().treasury = -3000;
    state.factions.get_mut(&scotland).unwrap().treasury = 200_000;
    assert_eq!(
        gift_to(&ai::plan_turn(&state, &data, &scotland), &france),
        0
    );
}

/// Seeds whose campaign roll for `faction` and `salt` passes (or fails).
fn seeds_where(data: &GameData, faction: &str, salt: u64, passes: bool) -> Vec<u64> {
    (1..40)
        .filter(|seed| {
            let state = CampaignState::new_1337(data, fac("fac_papacy"), *seed).unwrap();
            let roll = ai::alignment::campaign_roll(&state, &fac(faction), salt);
            (roll < ai::alignment::HISTORY_PERMILLE) == passes
        })
        .collect()
}

#[test]
fn flanders_under_the_wool_embargo_turns_to_england_in_some_campaigns() {
    let data = data();
    let flanders = fac("fac_flanders");
    let (yes, no) = (
        seeds_where(&data, "fac_flanders", 1, true),
        seeds_where(&data, "fac_flanders", 1, false),
    );
    assert!(!yes.is_empty() && !no.is_empty(), "history hesitates");
    let state = CampaignState::new_1337(&data, fac("fac_papacy"), yes[0]).unwrap();
    assert!(state.factions[&fac("fac_england")]
        .embargoes
        .contains(&flanders));
    assert_eq!(
        ai::alignment::side_change(&state, &data, &flanders),
        Some((fac("fac_france"), fac("fac_england")))
    );
    let state = CampaignState::new_1337(&data, fac("fac_papacy"), no[0]).unwrap();
    assert_eq!(ai::alignment::side_change(&state, &data, &flanders), None);
}

/// England holds Paris and 8 provinces of the French crown.
fn english_france(data: &GameData, seed: u64) -> CampaignState {
    let mut state = CampaignState::new_1337(data, fac("fac_papacy"), seed).unwrap();
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    let mut taken: Vec<_> = data
        .provinces
        .iter()
        .filter(|(_, p)| p.owner == france)
        .map(|(id, _)| id.clone())
        .collect();
    taken.sort_by_key(|id| id.as_str() != "prov_ile_de_france");
    for id in taken.into_iter().take(8) {
        state.city_state_mut(&id).unwrap().controller = england.clone();
    }
    state
}

#[test]
fn burgundy_abandons_a_dominated_france_in_some_campaigns() {
    let data = data();
    let (france, england, burgundy) = (fac("fac_france"), fac("fac_england"), fac("fac_burgundy"));
    let state = CampaignState::new_1337(&data, fac("fac_papacy"), 1).unwrap();
    assert!(!ai::alignment::dominates_realm(
        &state, &data, &england, &france
    ));
    let yes = seeds_where(&data, "fac_burgundy", 2, true);
    let no = seeds_where(&data, "fac_burgundy", 2, false);
    let mut state = english_france(&data, yes[0]);
    assert!(ai::alignment::dominates_realm(
        &state, &data, &england, &france
    ));
    state.factions.get_mut(&burgundy).unwrap().loyalty = 80;
    assert_eq!(
        ai::alignment::side_change(&state, &data, &burgundy),
        Some((france.clone(), england.clone()))
    );
    // A loyal duke stays, and so does history in other campaigns.
    state.factions.get_mut(&burgundy).unwrap().loyalty = 95;
    assert_eq!(ai::alignment::side_change(&state, &data, &burgundy), None);
    let mut state = english_france(&data, no[0]);
    state.factions.get_mut(&burgundy).unwrap().loyalty = 80;
    assert_eq!(ai::alignment::side_change(&state, &data, &burgundy), None);
}
