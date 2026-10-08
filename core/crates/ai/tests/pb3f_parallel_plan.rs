//! PB3f (ADR 0091): the planner's pool changes no bit of the campaign.
//! Two campaigns from the same seed, one planned on the calling thread
//! (`plan_turn_sequential`), the other on the pool (`plan_turn`), must keep
//! the same state and the same events turn after turn; and on every state
//! reached, both planners give each faction the same orders.
use data_model::test_support::game_data;

use data_model::{FactionId, GameData};
use sim_campaign::CampaignState;

fn data() -> &'static GameData {
    ai::feudal::install();
    game_data()
}

/// Turns played per seed: long enough for wars, sieges and debts.
const TURNS: u32 = 12;

fn campaigns_agree(seed: u64) {
    let data = data();
    let player = FactionId::new("fac_france").unwrap();
    let start = CampaignState::new_1337(data, player.clone(), seed).expect("1337 start");
    let mut sequential = start.clone();
    let mut parallel = start;
    sequential.interactive_battles = false;
    parallel.interactive_battles = false;
    for turn in 0..TURNS {
        // Same orders for every faction on the state reached.
        let factions: Vec<FactionId> = parallel
            .factions
            .iter()
            .filter(|(_, f)| f.alive)
            .map(|(id, _)| id.clone())
            .collect();
        for faction in &factions {
            assert_eq!(
                ai::plan_turn(&parallel, data, faction),
                ai::plan_turn_sequential(&parallel, data, faction),
                "seed {seed}, turn {turn}: orders of {faction} differ"
            );
        }
        // The player's side is played by the same AI.
        for order in ai::plan_turn_sequential(&sequential, data, &player) {
            let _ = sequential.submit_order(data, order);
        }
        for order in ai::plan_turn(&parallel, data, &player) {
            let _ = parallel.submit_order(data, order);
        }
        let sequential_events = sequential.end_turn_with(data, ai::plan_turn_sequential);
        let parallel_events = parallel.end_turn_with(data, ai::plan_turn);
        assert_eq!(
            parallel_events, sequential_events,
            "seed {seed}: events differ at turn {turn}"
        );
        assert_eq!(
            parallel.save_json(),
            sequential.save_json(),
            "seed {seed}: state differs at turn {turn}"
        );
    }
}

#[test]
fn parallel_planner_matches_sequential_seed_1() {
    campaigns_agree(1);
}

#[test]
fn parallel_planner_matches_sequential_seed_7() {
    campaigns_agree(7);
}

#[test]
fn parallel_planner_matches_sequential_seed_1337() {
    campaigns_agree(1337);
}
