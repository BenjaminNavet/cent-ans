//! F9: historical rulers are spared natural death before their recorded
//! death year (minus a grace), so 1337's realms keep their kings.

use data_model::{CharacterId, FactionId};
use sim_campaign::test_support::idle;
use sim_campaign::CampaignState;

use data_model::test_support::game_data;

/// Ten years without war (idle planners): only natural death can strike.
/// One test per seed so the test harness runs the eight campaigns in parallel.
fn philippe_vi_survives_the_1340s(seed: u64) {
    let data = game_data();
    let philippe = CharacterId::new("chr_philippe_vi").unwrap();
    let mut state =
        CampaignState::new_1337(data, FactionId::new("fac_france").unwrap(), seed).unwrap();
    for _ in 0..40 {
        state.end_turn_with(data, idle);
    }
    assert!(
        state.characters[&philippe].alive,
        "seed {seed}: Philippe VI died before 1347"
    );
}

macro_rules! philippe_seeds {
    ($($name:ident => $seed:expr),* $(,)?) => {
        $(#[test]
        fn $name() {
            philippe_vi_survives_the_1340s($seed);
        })*
    };
}

philippe_seeds!(
    philippe_vi_outlives_the_1340s_in_peace_seed_1 => 1,
    philippe_vi_outlives_the_1340s_in_peace_seed_2 => 2,
    philippe_vi_outlives_the_1340s_in_peace_seed_3 => 3,
    philippe_vi_outlives_the_1340s_in_peace_seed_4 => 4,
    philippe_vi_outlives_the_1340s_in_peace_seed_5 => 5,
    philippe_vi_outlives_the_1340s_in_peace_seed_6 => 6,
    philippe_vi_outlives_the_1340s_in_peace_seed_7 => 7,
    philippe_vi_outlives_the_1340s_in_peace_seed_8 => 8,
);

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
