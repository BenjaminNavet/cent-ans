//! F9: historical rulers are spared natural death before their recorded
//! death year (minus a grace), so 1337's realms keep their kings.

use data_model::{CharacterId, FactionId, GameData};
use sim_campaign::{CampaignState, Order};

use data_model::test_support::game_data;

fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

#[test]
fn philippe_vi_outlives_the_1340s_in_peace() {
    let data = game_data();
    let philippe = CharacterId::new("chr_philippe_vi").unwrap();
    for seed in 1..=8 {
        let mut state =
            CampaignState::new_1337(data, FactionId::new("fac_france").unwrap(), seed).unwrap();
        // Ten years without war (idle planners): only natural death can strike.
        for _ in 0..40 {
            state.end_turn_with(data, idle);
        }
        assert!(
            state.characters[&philippe].alive,
            "seed {seed}: Philippe VI died before 1347"
        );
    }
}

#[test]
fn natural_death_follows_the_recorded_year() {
    let data = game_data();
    let state = CampaignState::new_1337(data, FactionId::new("fac_france").unwrap(), 1).unwrap();
    let philippe = CharacterId::new("chr_philippe_vi").unwrap();
    assert_eq!(
        sim_campaign::characters::natural_death_permille(&state, data, &philippe),
        0
    );
}
