//! Lot FE5: the feudal AI (spec FE § 5, ADR 0110).

use data_model::GameData;
use sim_campaign::feudal::{self, FelonyReason, PROTECTION_REFUSED_REASON};
use sim_campaign::{CampaignState, Order};

use data_model::test_support::{fac, game_data};

fn start(data: &GameData) -> CampaignState {
    ai::feudal::install();
    CampaignState::new_1337(data, fac("fac_papacy"), 7).expect("1337 start")
}

fn set_loyalty(state: &mut CampaignState, faction: &str, loyalty: u8) {
    state.factions.get_mut(&fac(faction)).unwrap().loyalty = loyalty;
}

fn has_modifier(state: &CampaignState, holder: &str, with: &str, reason: &str) -> bool {
    state.factions[&fac(holder)]
        .modifiers
        .iter()
        .any(|m| m.with == fac(with) && m.reason_fr == reason)
}

#[test]
fn a_strong_and_loyal_suzerain_protects() {
    let data = game_data();
    let mut state = start(data);
    let (france, blois, foix) = (fac("fac_france"), fac("fac_blois"), fac("fac_foix_bearn"));
    assert_eq!(feudal::liege_of(&state, data, &blois), Some(france.clone()));
    set_loyalty(&mut state, "fac_blois", 90);
    let (score, reason) = ai::feudal::protection_score(&state, data, &france, &blois, &foix);
    assert!(score >= 0, "France protects Blois: {score} ({reason})");
    state.declare_war(data, &foix, &blois).unwrap();
    assert!(state.is_at_war(&france, &foix), "the king answers the call");
}

#[test]
fn a_weak_suzerain_shirks() {
    let data = game_data();
    let mut state = start(data);
    let (brittany, penthievre, england) = (
        fac("fac_brittany"),
        fac("fac_penthievre"),
        fac("fac_england"),
    );
    assert_eq!(
        feudal::liege_of(&state, data, &penthievre),
        Some(brittany.clone())
    );
    let (score, reason) =
        ai::feudal::protection_score(&state, data, &brittany, &penthievre, &england);
    assert!(
        score < 0,
        "Brittany will not face England: {score} ({reason})"
    );
    state.declare_war(data, &england, &penthievre).unwrap();
    assert!(!state.is_at_war(&brittany, &england), "the duke shirks");
    assert!(has_modifier(
        &state,
        "fac_penthievre",
        "fac_brittany",
        PROTECTION_REFUSED_REASON
    ));
}

#[test]
fn a_county_declares_no_hopeless_war() {
    let data = game_data();
    let state = start(data);
    let foix = fac("fac_foix_bearn");
    assert_eq!(
        feudal::primary_rank(&state, data, &foix),
        Some(data_model::TitleRank::County)
    );
    let mut orders = vec![
        Order::DeclareWar {
            target: fac("fac_france"),
        },
        Order::Research {
            technology: data_model::TechnologyId::new("tech_aqua_vitae").unwrap(),
        },
    ];
    ai::feudal::filter_suicidal_wars(&state, data, &foix, &mut orders);
    assert!(
        !orders.iter().any(|o| matches!(o, Order::DeclareWar { .. })),
        "{orders:?}"
    );
    assert_eq!(orders.len(), 1, "other orders untouched");
    // A crown follows no such doctrine.
    let mut orders = vec![Order::DeclareWar {
        target: fac("fac_foix_bearn"),
    }];
    ai::feudal::filter_suicidal_wars(&state, data, &fac("fac_castile"), &mut orders);
    assert_eq!(orders.len(), 1);
}

#[test]
fn forfeiture_is_declared_on_felony_when_the_odds_allow() {
    let data = game_data();
    let mut state = start(data);
    let (france, blois) = (fac("fac_france"), fac("fac_blois"));
    // F8: the case of 1337 against England is left aside.
    state.feudal.felonies.clear();
    assert_eq!(ai::feudal::plan_commise(&state, data, &france), None);
    feudal::open_felony(&mut state, data, &blois, FelonyReason::RefusedHost).expect("case");
    let order = ai::feudal::plan_commise(&state, data, &france);
    assert_eq!(
        order,
        Some(Order::DeclareCommise {
            vassal: blois.clone()
        })
    );
    state.apply_order(data, &france, order.unwrap()).unwrap();
    assert!(feudal::has_forfeiture(&state, &france, &blois));
    assert!(state.is_at_war(&france, &blois));
}

#[test]
fn a_mighty_felon_is_spared() {
    let data = game_data();
    let mut state = start(data);
    let (brittany, penthievre) = (fac("fac_brittany"), fac("fac_penthievre"));
    feudal::open_felony(&mut state, data, &penthievre, FelonyReason::RefusedHost).expect("case");
    // Penthièvre raises a host far above its duke's.
    let army = state
        .armies
        .values()
        .find(|a| a.faction == brittany)
        .expect("a Breton army")
        .clone();
    for n in 0..6 {
        let mut copy = army.clone();
        copy.faction = penthievre.clone();
        let id = sim_campaign::state::ArmyId::from_index(9000 + n);
        state.armies.insert(id, copy);
    }
    assert_eq!(ai::feudal::plan_commise(&state, data, &brittany), None);
}

#[test]
fn a_disloyal_vassal_refuses_the_host() {
    let data = game_data();
    let mut state = start(data);
    let (france, blois, alencon, navarre) = (
        fac("fac_france"),
        fac("fac_blois"),
        fac("fac_alencon"),
        fac("fac_navarre"),
    );
    set_loyalty(&mut state, "fac_blois", 5);
    set_loyalty(&mut state, "fac_alencon", 90);
    assert!(ai::feudal::host_score(&state, data, &blois, &france, &navarre) < 0);
    assert!(!feudal::answers_host(
        &state, data, &blois, &france, &navarre
    ));
    assert!(feudal::answers_host(
        &state, data, &alencon, &france, &navarre
    ));
    feudal::summon_host(&mut state, data, &france, &navarre);
    assert!(
        !state.is_at_war(&blois, &navarre),
        "the disloyal count stays home"
    );
    assert!(
        state.is_at_war(&alencon, &navarre),
        "the loyal count marches"
    );
    assert!(
        state.feudal.felonies.iter().any(|c| c.vassal == blois
            && c.liege == france
            && c.reason == FelonyReason::RefusedHost),
        "the refusal is a felony"
    );
}

#[test]
fn a_disloyal_vassal_strong_enough_revolts() {
    let data = game_data();
    let mut state = start(data);
    let (brittany, penthievre) = (fac("fac_brittany"), fac("fac_penthievre"));
    set_loyalty(&mut state, "fac_penthievre", 5);
    // Odds: Penthièvre as strong as its duke; the chance is rolled per turn.
    let army = state
        .armies
        .values()
        .find(|a| a.faction == brittany)
        .expect("a Breton army")
        .clone();
    for n in 0..4 {
        let mut copy = army.clone();
        copy.faction = penthievre.clone();
        let id = sim_campaign::state::ArmyId::from_index(9100 + n);
        state.armies.insert(id, copy);
    }
    let turn = (0..40)
        .find(|t| {
            state.turn = *t;
            ai::feudal::plan_revolt(&state, data, &penthievre).is_some()
        })
        .expect("revolts within 40 turns");
    state.turn = turn;
    state.apply_order(data, &penthievre, Order::Revolt).unwrap();
    assert_eq!(feudal::liege_of(&state, data, &penthievre), None);
    assert!(state.is_at_war(&penthievre, &brittany));
    assert!(state
        .feudal
        .felonies
        .iter()
        .any(|c| c.vassal == penthievre && c.reason == FelonyReason::Revolt));
    // A loyal one never does.
    set_loyalty(&mut state, "fac_blois", 90);
    assert!((0..40).all(|t| {
        state.turn = t;
        ai::feudal::plan_revolt(&state, data, &fac("fac_blois")).is_none()
    }));
}
