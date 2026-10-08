//! ADR 0182: the player's proposals are accepted iff score >= 0, never by a roll.

use sim_campaign::diplomacy::RelationKind;
use sim_campaign::negotiation::{evaluate_treaty, propose_treaty, Article, Party, ACCEPT_SCORE};
use sim_campaign::religion::faction_religion;
use sim_campaign::CampaignState;

use data_model::test_support::{fac, game_data};

#[test]
fn player_proposal_follows_the_score_sign_whatever_the_seed() {
    let data = game_data();
    let fr = fac("fac_france");
    let en = {
        let state = CampaignState::new_1337(data, fr.clone(), 1).expect("start");
        let faith = faction_religion(&state, data, &fr);
        state
            .factions
            .iter()
            .find(|(id, f)| {
                f.alive
                    && **id != fr
                    && state.relation(&fr, id) == RelationKind::Peace
                    && faction_religion(&state, data, id) == faith
            })
            .map(|(id, _)| id.clone())
            .expect("a neutral")
    };
    let mut seen = [false; 2];
    for seed in 1..=5u64 {
        for gold in [0_i64, 1_000, 10_000, 60_000] {
            let mut state = CampaignState::new_1337(data, fr.clone(), seed).expect("start");
            state.factions.get_mut(&fr).unwrap().treasury = 100_000;
            let treaty = vec![
                Article::TradeAgreement,
                Article::Gold {
                    giver: Party::Proposer,
                    amount: gold,
                },
            ];
            let verdict = evaluate_treaty(&state, data, &fr, &en, &treaty);
            let expected = verdict.blocked.is_none() && verdict.score >= ACCEPT_SCORE;
            assert_eq!(verdict.accept, expected);
            let result = propose_treaty(&mut state, data, &fr, &en, treaty);
            assert_eq!(
                result.is_ok(),
                expected,
                "seed {seed} gold {gold} score {}",
                verdict.score
            );
            seen[usize::from(expected)] = true;
        }
    }
    assert!(
        seen[0] && seen[1],
        "both outcomes must be exercised: {seen:?}"
    );
}
