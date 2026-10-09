//! RX (ADR 0245): no passive feudal victory right after the 1337 start.

use data_model::test_support::{fac, game_data};
use sim_campaign::feudal::{self, VictoryStreaks};
use sim_campaign::test_support::idle;
use sim_campaign::victory::OutcomeKind;
use sim_campaign::CampaignState;

/// Even with every streak already full, no faction wins before the
/// configured year.
#[test]
fn no_generic_victory_before_min_year() {
    let data = game_data();
    let min_year = data.feudal_rules.generic_victory_min_year;
    assert!(min_year >= 1360);
    let mut s = CampaignState::new_1337(data, fac("fac_france"), 1).expect("1337 start");
    feudal::sync_suzerains(&mut s, data);
    let full = VictoryStreaks {
        independent: 1000,
        first_vassal: 1000,
    };
    let starters: Vec<_> = s.feudal.start_crowns.keys().cloned().collect();
    assert!(starters.len() > 100);
    for f in &starters {
        s.feudal.streaks.insert(f.clone(), full);
    }
    s.year = min_year - 1;
    for f in &starters {
        assert!(feudal::generic_victory(&s, data, f).is_none(), "{f}");
    }
    // The paths stay open once the year is reached.
    s.year = min_year;
    assert!(starters
        .iter()
        .any(|f| feudal::generic_victory(&s, data, f).is_some()));
}

#[test]
fn first_vassal_needs_a_long_streak() {
    let data = game_data();
    assert!(data.feudal_rules.ascension_turns >= 60);
}

/// Previously winners at turn 20 without orders: idle for 40 turns, no
/// outcome (and every playable faction with a victory block keeps at
/// least one objective undone at the start).
#[test]
fn idle_former_winners_do_not_win_in_40_turns() {
    let data = game_data();
    for id in ["fac_athens", "fac_moscow", "fac_naples", "fac_france"] {
        let mut s = CampaignState::new_1337(data, fac(id), 1).expect("1337 start");
        for _ in 0..40 {
            s.end_turn_with(data, idle);
        }
        let won = s
            .outcome
            .as_ref()
            .is_some_and(|o| o.kind == OutcomeKind::Victory);
        assert!(!won, "{id} won passively: {:?}", s.outcome);
    }
}

/// France used to start with two of four objectives already met.
#[test]
fn france_starts_with_no_objective_met() {
    let data = game_data();
    let s = CampaignState::new_1337(data, fac("fac_france"), 1).expect("1337 start");
    let objs = s.objectives(data, &fac("fac_france"));
    assert!(objs.iter().all(|o| !o.done), "{objs:?}");
}
