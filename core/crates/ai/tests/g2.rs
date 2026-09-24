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
