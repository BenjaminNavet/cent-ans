//! Lot ECO-R0: four diplomacy defects (truce age of a short truce, funds of stacked gold and
//! tribute, suspended tribute, Auld Alliance event) and the truce label.

use data_model::test_support::{fac, game_data};
use data_model::EventId;
use sim_campaign::diplomacy::MEDIATION_TRUCE_TURNS;
use sim_campaign::effects::apply_effect;
use sim_campaign::negotiation::{self, Article, Party, TributeDue};
use sim_campaign::test_support::{idle, start};
use sim_campaign::{perjury, EventContext, EventKind};

#[test]
fn mediation_truce_broken_at_once_is_major_perjury() {
    let data = game_data();
    let mut state = start(data, "fac_france", 5);
    let (fr, en) = (fac("fac_france"), fac("fac_england"));
    assert!(state.is_at_war(&fr, &en), "France and England start at war");
    negotiation::apply_treaty(
        &mut state,
        data,
        &fr,
        &en,
        &[Article::Mediation {
            turns: MEDIATION_TRUCE_TURNS,
        }],
    )
    .unwrap();
    assert_eq!(perjury::truce_age(&state, &fr, &en), Some(0));
    assert_eq!(
        state.perjury_event_for_war(data, &fr, &en),
        Some(perjury::TRUCE_BROKEN_RECENT)
    );
}

#[test]
fn truce_age_without_signing_turn_falls_back_to_the_standard_length() {
    let data = game_data();
    let mut state = start(data, "fac_france", 5);
    let (fr, en) = (fac("fac_france"), fac("fac_england"));
    negotiation::apply_treaty(&mut state, data, &fr, &en, &[Article::Peace]).unwrap();
    // An old save: no signing turn kept.
    state.factions.get_mut(&fr).unwrap().truce_started.clear();
    assert!(perjury::truce_age(&state, &fr, &en).is_some());
}

#[test]
fn stacked_gold_cannot_exceed_treasury() {
    let data = game_data();
    let state = start(data, "fac_aragon", 9);
    let (ar, ca) = (fac("fac_aragon"), fac("fac_castile"));
    let treasury = state.factions[&ar].treasury;
    assert!(treasury > 2);
    let gold = |amount| Article::Gold {
        giver: Party::Proposer,
        amount,
    };
    let half = treasury / 2 + 1;
    assert!(negotiation::check_treaty(&state, data, &ar, &ca, &[gold(half)]).is_ok());
    assert!(negotiation::check_treaty(&state, data, &ar, &ca, &[gold(half), gold(half)]).is_err());
    // The first tribute payment counts with the gold of the same treaty.
    let tribute = Article::Tribute {
        giver: Party::Proposer,
        per_season: half,
        seasons: 4,
    };
    assert!(negotiation::check_treaty(&state, data, &ar, &ca, &[gold(half), tribute]).is_err());
}

#[test]
fn tribute_never_drives_treasury_negative() {
    let data = game_data();
    let mut state = start(data, "fac_aragon", 9);
    let (ar, ca) = (fac("fac_aragon"), fac("fac_castile"));
    // A tribute beyond the treasury cannot be agreed...
    let treasury = state.factions[&ar].treasury;
    let tribute = Article::Tribute {
        giver: Party::Proposer,
        per_season: treasury + 1,
        seasons: 4,
    };
    assert!(negotiation::check_treaty(&state, data, &ar, &ca, &[tribute]).is_err());
    // ... and one that outlives the treasury is suspended, not collected below zero.
    let due = TributeDue {
        to: ca.clone(),
        per_season: 10_000_000,
        until_turn: state.turn + 8,
    };
    state
        .factions
        .get_mut(&ar)
        .unwrap()
        .ledger
        .tributes
        .push(due);
    let before = state.factions[&ca].treasury;
    let events = state.end_turn_with(data, idle);
    assert!(events
        .iter()
        .any(|e| e.kind == EventKind::Diplomacy && e.text_fr.contains("tribut")));
    assert!(state.factions[&ca].treasury < before + 10_000_000);
    assert!(state.factions[&ar].treasury > -10_000_000);
    assert!(state.factions[&ca]
        .modifiers
        .iter()
        .any(|m| m.with == ar && m.reason_fr == "Tribut impayé"));
}

#[test]
fn auld_alliance_event_creates_alliance() {
    let data = game_data();
    let mut state = start(data, "fac_france", 5);
    let (sc, fr) = (fac("fac_scotland"), fac("fac_france"));
    for (a, b) in [(&sc, &fr), (&fr, &sc)] {
        state.factions.get_mut(a).unwrap().allies.remove(b);
        state.factions.get_mut(a).unwrap().at_war_with.remove(b);
    }
    let event = &data.events[&EventId::new("evt_auld_alliance").unwrap()];
    let ctx = EventContext {
        faction: Some(sc.clone()),
        province: None,
    };
    let mut events = Vec::new();
    for effect in &event.options[0].effects {
        apply_effect(&mut state, data, effect, &ctx, &mut events);
    }
    assert!(state.is_allied(&sc, &fr));
}

#[test]
fn truce_label_follows_the_duration() {
    assert_eq!(negotiation::truce_label(20), "Trêve de 5 ans");
    assert_eq!(negotiation::truce_label(8), "Trêve de 2 ans");
    assert_eq!(negotiation::truce_label(4), "Trêve de 1 an");
    assert_eq!(
        game_data().ai_diplomacy.negotiation.truce_turns,
        8,
        "the menu truce keeps its historical length"
    );
}
