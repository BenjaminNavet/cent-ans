//! Lot EQ3: a peace also binds the allies and vassals who joined the war
//! (`negotiation.truce_binds_allies`, trêve de Leulinghem 1389).

use sim_campaign::negotiation::{apply_treaty, Article};
use sim_campaign::CampaignState;

use data_model::test_support::{fac, game_data};

/// Puts `a` and `b` at war since `turn` (both sides' records).
fn war(state: &mut CampaignState, a: &str, b: &str, turn: u32) {
    for (x, y) in [(a, b), (b, a)] {
        let f = state.factions.get_mut(&fac(x)).unwrap();
        f.allies.remove(&fac(y));
        f.truces.remove(&fac(y));
        f.at_war_with.insert(fac(y));
        f.war_started.insert(fac(y), turn);
        f.war_scores.insert(fac(y), 0);
    }
}

fn set_allied(state: &mut CampaignState, a: &str, b: &str, allied: bool) {
    for (x, y) in [(a, b), (b, a)] {
        let allies = &mut state.factions.get_mut(&fac(x)).unwrap().allies;
        if allied {
            allies.insert(fac(y));
        } else {
            allies.remove(&fac(y));
        }
    }
}

#[test]
fn peace_binds_the_allies_who_joined_the_war() {
    let data = game_data();
    assert!(data.ai_diplomacy.negotiation.truce_binds_allies);
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 1).unwrap();
    state.turn = 20;
    war(&mut state, "fac_france", "fac_england", 10);
    // Joined France's war (Auld Alliance) and England's (Hainaut).
    set_allied(&mut state, "fac_scotland", "fac_france", true);
    war(&mut state, "fac_scotland", "fac_england", 12);
    set_allied(&mut state, "fac_hainaut", "fac_england", true);
    war(&mut state, "fac_hainaut", "fac_france", 10);
    // An ally of France already at war with England before: its own war.
    set_allied(&mut state, "fac_naples", "fac_france", true);
    war(&mut state, "fac_naples", "fac_england", 5);
    // At war with England since this war, but no ally of France.
    set_allied(&mut state, "fac_castile", "fac_france", false);
    war(&mut state, "fac_castile", "fac_england", 15);

    apply_treaty(
        &mut state,
        data,
        &fac("fac_england"),
        &fac("fac_france"),
        &[Article::Peace],
    )
    .unwrap();

    let (fr, en) = (fac("fac_france"), fac("fac_england"));
    assert!(!state.is_at_war(&fr, &en));
    assert!(!state.is_at_war(&fac("fac_scotland"), &en));
    assert!(state.has_truce(&fac("fac_scotland"), &en));
    assert!(!state.is_at_war(&fac("fac_hainaut"), &fr));
    assert!(state.has_truce(&fac("fac_hainaut"), &fr));
    assert!(state.is_at_war(&fac("fac_naples"), &en));
    assert!(state.is_at_war(&fac("fac_castile"), &en));
}

#[test]
fn a_great_crown_is_not_bound_by_a_lesser_ally() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 1).unwrap();
    state.turn = 20;
    // England falls on Scotland; France answers the Auld Alliance.
    set_allied(&mut state, "fac_scotland", "fac_france", true);
    war(&mut state, "fac_scotland", "fac_england", 10);
    war(&mut state, "fac_france", "fac_england", 10);
    apply_treaty(
        &mut state,
        data,
        &fac("fac_england"),
        &fac("fac_scotland"),
        &[Article::Peace],
    )
    .unwrap();
    assert!(!state.is_at_war(&fac("fac_scotland"), &fac("fac_england")));
    assert!(state.is_at_war(&fac("fac_france"), &fac("fac_england")));
}
