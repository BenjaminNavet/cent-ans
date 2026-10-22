//! Lot M6 (ADR 0721): perjury and windowed diplomatic events.

use data_model::test_support::{fac, game_data};
use sim_campaign::diplomacy::TRUCE_TURNS;
use sim_campaign::{perjury, CampaignState, Order};

fn start(seed: u64) -> CampaignState {
    let mut state = CampaignState::new_1337(game_data(), fac("fac_france"), seed).expect("start");
    state.chronicle.disabled = true;
    state
}

/// France and Aragon at peace under a truce with `left` turns to run.
fn truce(state: &mut CampaignState, left: u32) {
    let until = state.turn + left;
    for (a, b) in [("fac_france", "fac_aragon"), ("fac_aragon", "fac_france")] {
        let f = state.factions.get_mut(&fac(a)).unwrap();
        f.at_war_with.remove(&fac(b));
        f.allies.remove(&fac(b));
        f.truces.insert(fac(b), until);
    }
}

fn castile_view(state: &CampaignState, reason_prefix: &str) -> Vec<(i32, u32)> {
    state.factions[&fac("fac_castile")]
        .modifiers
        .iter()
        .filter(|m| m.with == fac("fac_france") && m.reason_fr.starts_with(reason_prefix))
        .map(|m| (m.value, m.expires_turn - state.turn))
        .collect()
}

#[test]
fn tx_m6_truce_break_inside_window_is_major() {
    let data = game_data();
    let mut state = start(7);
    truce(&mut state, TRUCE_TURNS);
    let (fr, ar) = (fac("fac_france"), fac("fac_aragon"));
    let rule = data.diplomacy_rules.event("truce_broken_recent").unwrap();
    assert_eq!(
        state.perjury_event_for_war(data, &fr, &ar),
        Some(perjury::TRUCE_BROKEN_RECENT)
    );
    let cost = state.declaration_prestige_cost(data, &fr, &ar);
    assert!(cost <= rule.prestige && cost >= rule.prestige + rule.chivalry_prestige);
    let ruler = state.factions[&fr].ruler.clone().unwrap();
    let before = state.characters[&ruler].prestige;
    state
        .submit_order(data, Order::DeclareWar { target: ar })
        .unwrap();
    assert_eq!(
        castile_view(&state, "Parjure"),
        vec![(rule.opinion, rule.duration)]
    );
    assert_eq!(state.characters[&ruler].prestige, before + cost);
}

#[test]
fn tx_m6_truce_break_outside_window_is_moderate() {
    let data = game_data();
    let mut state = start(7);
    // Signed 15 seasons ago: past the 6-season window.
    truce(&mut state, TRUCE_TURNS - 15);
    let (fr, ar) = (fac("fac_france"), fac("fac_aragon"));
    assert_eq!(
        state.perjury_event_for_war(data, &fr, &ar),
        Some(perjury::TRUCE_BROKEN)
    );
    state
        .submit_order(data, Order::DeclareWar { target: ar })
        .unwrap();
    let rule = data.diplomacy_rules.event("truce_broken").unwrap();
    assert!(
        rule.opinion
            > data
                .diplomacy_rules
                .event("truce_broken_recent")
                .unwrap()
                .opinion
    );
    assert_eq!(
        castile_view(&state, "Parjure"),
        vec![(rule.opinion, rule.duration)]
    );
}

#[test]
fn tx_m6_opinion_decays_and_expires() {
    let data = game_data();
    let mut state = start(7);
    let (fr, castile) = (fac("fac_france"), fac("fac_castile"));
    perjury::record_event(&mut state, data, perjury::ALLY_ATTACKED, &fr);
    let rule = data.diplomacy_rules.event("ally_attacked").unwrap();
    let (_, reasons) = state.attitude(data, &castile, &fr);
    assert!(reasons
        .iter()
        .any(|(t, v)| t == &rule.reason_fr && *v == rule.opinion));
    assert_eq!(
        state.reason_turns_left(&castile, &fr).get(&rule.reason_fr),
        Some(&rule.duration)
    );
    state.turn += rule.duration;
    let (_, reasons) = state.attitude(data, &castile, &fr);
    assert!(!reasons.iter().any(|(t, _)| t == &rule.reason_fr));
}

#[test]
fn tx_m6_sworn_bond_is_a_perjury_event_and_the_ai_refrains() {
    let data = game_data();
    let mut state = start(7);
    let (fr, ar) = (fac("fac_france"), fac("fac_aragon"));
    for (a, b) in [(&fr, &ar), (&ar, &fr)] {
        let f = state.factions.get_mut(a).unwrap();
        f.at_war_with.remove(b);
        f.truces.remove(b);
        f.allies.insert(b.clone());
    }
    assert_eq!(
        state.perjury_event_for_war(data, &fr, &ar),
        Some(perjury::ALLY_ATTACKED)
    );
    // Without an event table the same declaration costs nothing.
    let preview = perjury::preview(&state, data, perjury::ALLY_ATTACKED, &fr).unwrap();
    assert!(preview.opinion < 0 && preview.prestige < 0);
    assert!(perjury::preview(&state, data, "unknown", &fr).is_none());
}
