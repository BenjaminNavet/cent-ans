//! Player feedback 10-10: too many military alliances offered to the player.
//! Over 40 turns the player, who answers nothing, receives few of them.

use std::collections::BTreeSet;

use sim_campaign::negotiation::Article;
use sim_campaign::CampaignState;

use data_model::test_support::{fac, game_data};

fn alliance_offers(player: &str, turns: u32) -> usize {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac(player), 1).unwrap();
    let mut seen = BTreeSet::new();
    for _ in 0..turns {
        state.end_turn(data);
        for offer in &state.factions[&fac(player)].offers {
            if offer
                .proposal
                .articles
                .iter()
                .any(|a| matches!(a, Article::Alliance | Article::DefensiveAlliance))
            {
                seen.insert(offer.id);
            }
        }
    }
    eprintln!("{player}: {} alliance offers in {turns} turns", seen.len());
    seen.len()
}

#[test]
fn few_alliance_offers_reach_the_player() {
    for player in ["fac_france", "fac_england"] {
        let count = alliance_offers(player, 40);
        assert!(count <= 6, "{player}: {count} alliance offers in 40 turns");
    }
}
