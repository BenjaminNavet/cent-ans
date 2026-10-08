//! LR-11: a war against a realm down to its last bastions (F4: never
//! besieged without a claim) has nothing left to fight for once the
//! campaign seasons are over: both sides lean towards a white peace
//! ("Guerre sans enjeu"), instead of armies idling before the bastions until
//! the long-war pressure (EQ6) builds up years later.

use data_model::FactionId;
use sim_campaign::negotiation::{bastion_stalemate, evaluate_treaty, Article, LAST_BASTIONS};
use sim_campaign::CampaignState;

const REASON: &str = "Guerre sans enjeu : derniers bastions";

use data_model::test_support::{fac, game_data};

fn held(state: &CampaignState, faction: &FactionId) -> usize {
    state
        .provinces
        .keys()
        .filter(|p| state.holds_province(faction, p))
        .count()
}

#[test]
fn a_stalemate_before_last_bastions_leans_towards_peace() {
    let data = game_data();
    let rules = &data.ai_diplomacy.negotiation;
    assert!(rules.bastion_war_points > 0, "the rule is on in the data");
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 1).unwrap();
    // The Papacy and Urbino are at war from 1337; Urbino holds one province.
    let (papacy, urbino) = (fac("fac_papacy"), fac("fac_urbino"));
    assert!(state.is_at_war(&papacy, &urbino));
    assert!(held(&state, &urbino) <= LAST_BASTIONS);
    assert!(bastion_stalemate(&state, data, &papacy, &urbino));
    assert!(bastion_stalemate(&state, data, &urbino, &papacy));
    let white = [Article::Peace];
    let has_reason = |state: &CampaignState, proposer: &FactionId, recipient: &FactionId| {
        evaluate_treaty(state, data, proposer, recipient, &white)
            .context
            .iter()
            .any(|(label, value)| label == REASON && *value == rules.bastion_war_points)
    };
    // A campaign season does not end a war.
    let started = state.factions[&urbino].war_started[&papacy];
    state.turn = started + rules.min_war_turns - 1;
    assert!(!has_reason(&state, &papacy, &urbino));
    // Past it, both sides weigh the stalemate.
    let before = evaluate_treaty(&state, data, &papacy, &urbino, &white).score;
    state.turn = started + rules.min_war_turns;
    assert!(has_reason(&state, &papacy, &urbino));
    assert!(has_reason(&state, &urbino, &papacy));
    let after = evaluate_treaty(&state, data, &papacy, &urbino, &white).score;
    assert!(after >= before + rules.bastion_war_points - 2);
}

#[test]
fn no_stalemate_between_two_large_realms_or_under_a_claim() {
    let data = game_data();
    let state = CampaignState::new_1337(data, fac("fac_france"), 1).unwrap();
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    assert!(state.is_at_war(&france, &england));
    assert!(!bastion_stalemate(&state, data, &france, &england));
}
