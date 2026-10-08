//! G4 « Bourgogne et Brabant » tests: blood feud (Montereau), relative
//! dominance, money fiefs and the Low Countries web, tuning read from
//! `data/ai/alignment.json`.
use data_model::test_support::fac;
use sim_campaign::plan_cache::PlanCache;

use data_model::GameData;
use sim_campaign::diplomacy::{OpinionModifier, AGGRESSION_REASON, GIFT_REASON};
use sim_campaign::{CampaignState, Order};

use crate::common::data;

/// `holder` holds a running grudge of `value` against `with`.
fn grudge(state: &mut CampaignState, holder: &str, with: &str, value: i32, reason: &str) {
    let expires_turn = state.turn + 20;
    state
        .factions
        .get_mut(&fac(holder))
        .unwrap()
        .modifiers
        .push(OpinionModifier {
            with: fac(with),
            value,
            reason_fr: reason.to_owned(),
            expires_turn,
        });
}

/// Orders `faction` plans for side changes over the next four turns (each
/// faction plans them one turn in four).
fn side_change_orders(state: &mut CampaignState, data: &GameData, faction: &str) -> Vec<Order> {
    let start = state.turn;
    let mut orders = Vec::new();
    for turn in start..start + 4 {
        state.turn = turn;
        orders.extend(ai::alignment::plan_side_change(
            &PlanCache::new(state),
            data,
            &fac(faction),
        ));
    }
    state.turn = start;
    orders
}

#[test]
fn the_tuning_comes_from_data() {
    let data = data();
    let rules = data.ai_alignment.as_ref().expect("data/ai/alignment.json");
    assert!(rules.grievance.grudge < 0);
    assert!(rules.defection.dominance_realm_share > 0.0);
    assert!(rules.money_fief.amount > 0);
}

#[test]
fn without_tuning_the_ai_never_changes_sides() {
    let mut data = data().clone();
    let mut state = CampaignState::new_1337(&data, fac("fac_papacy"), 1).unwrap();
    grudge(
        &mut state,
        "fac_burgundy",
        "fac_france",
        -60,
        "Meurtre de Montereau",
    );
    assert!(
        ai::alignment::side_change(&PlanCache::new(&state), &data, &fac("fac_burgundy")).is_some()
    );
    data.ai_alignment = None;
    assert_eq!(
        ai::alignment::side_change(&PlanCache::new(&state), &data, &fac("fac_burgundy")),
        None
    );
}

#[test]
fn a_blood_feud_turns_burgundy_to_the_english_pretender() {
    let data = data();
    let (france, england, burgundy) = (fac("fac_france"), fac("fac_england"), fac("fac_burgundy"));
    let mut state = CampaignState::new_1337(data, fac("fac_papacy"), 1).unwrap();
    // No grudge, no feud: the duke follows his king.
    assert_eq!(
        ai::alignment::grievance_change(&PlanCache::new(&state), data, &burgundy),
        None
    );
    // The reputation of an aggressor is no personal grudge.
    grudge(
        &mut state,
        "fac_burgundy",
        "fac_france",
        -60,
        AGGRESSION_REASON,
    );
    assert_eq!(ai::alignment::grievance(&state, &burgundy, &france), 0);
    assert_eq!(
        ai::alignment::grievance_change(&PlanCache::new(&state), data, &burgundy),
        None
    );
    // Montereau: the murdered duke's son sides with Henry V.
    grudge(
        &mut state,
        "fac_burgundy",
        "fac_france",
        -40,
        "Meurtre de Montereau",
    );
    assert_eq!(ai::alignment::grievance(&state, &burgundy, &france), -40);
    assert_eq!(
        ai::alignment::grievance_change(&PlanCache::new(&state), data, &burgundy),
        Some((france.clone(), england.clone()))
    );
    // The grudge fades after its term.
    state.turn += 21;
    assert_eq!(
        ai::alignment::grievance_change(&PlanCache::new(&state), data, &burgundy),
        None
    );
}

#[test]
fn an_aggrieved_vassal_makes_peace_then_breaks_with_its_lord() {
    let data = data();
    let (france, england, burgundy) = (fac("fac_france"), fac("fac_england"), fac("fac_burgundy"));
    let mut state = CampaignState::new_1337(data, fac("fac_papacy"), 1).unwrap();
    state.factions.get_mut(&burgundy).unwrap().treasury = 50_000;
    grudge(
        &mut state,
        "fac_burgundy",
        "fac_france",
        -40,
        "Meurtre de Montereau",
    );
    // At war with England beside its lord: a white peace first (if England
    // would sign it), never a war on the king while the English fight us.
    assert!(state.is_at_war(&burgundy, &england));
    assert_eq!(
        ai::alignment::side_change(&PlanCache::new(&state), data, &burgundy),
        Some((france.clone(), england.clone()))
    );
    let orders = side_change_orders(&mut state, data, "fac_burgundy");
    assert!(
        orders
            .iter()
            .all(|o| matches!(o, Order::ProposePeace { target, .. } if *target == england)),
        "{orders:?}"
    );
    // At peace with England: independence war and alliance offer, after
    // dropping the allies England counts as rivals.
    for (a, b) in [(&burgundy, &england), (&england, &burgundy)] {
        state.factions.get_mut(a).unwrap().at_war_with.remove(b);
    }
    state
        .factions
        .get_mut(&burgundy)
        .unwrap()
        .allies
        .insert(fac("fac_scotland"));
    state
        .factions
        .get_mut(&fac("fac_scotland"))
        .unwrap()
        .allies
        .insert(burgundy.clone());
    let orders = side_change_orders(&mut state, data, "fac_burgundy");
    let war = orders
        .iter()
        .position(|o| matches!(o, Order::DeclareWar { target } if *target == france));
    let scots = orders.iter().position(
        |o| matches!(o, Order::BreakAlliance { target } if target.as_str() == "fac_scotland"),
    );
    let offer = orders
        .iter()
        .position(|o| matches!(o, Order::ProposeAlliance { target } if *target == england));
    assert!(
        war.is_some() && scots.is_some() && offer.is_some(),
        "{orders:?}"
    );
    assert!(scots < offer, "the Scots go before the offer");
}

/// England holds `count` provinces of the French crown's own lands, the
/// capital excluded.
fn english_provinces(data: &GameData, seed: u64, count: usize) -> CampaignState {
    let mut state = CampaignState::new_1337(data, fac("fac_papacy"), seed).unwrap();
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    let capital = data.factions[&france].capital.clone();
    let taken: Vec<_> = data
        .provinces
        .iter()
        .filter(|(id, p)| p.owner == france && **id != capital)
        .map(|(id, _)| id.clone())
        .take(count)
        .collect();
    for id in taken {
        // C4: a province's controller is its city's.
        state.city_state_mut(&id).unwrap().controller = england.clone();
    }
    state
}

#[test]
fn dominance_is_a_share_of_the_realm() {
    let data = data();
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    let rules = data.ai_alignment.as_ref().unwrap();
    let realm = ai::alignment::realm_provinces(data, &france).len();
    let needed = (rules.defection.dominance_realm_share * realm as f64).ceil() as usize;
    // Seven provinces (a quarter of France's own lands) and no Paris: the
    // absolute rule of G2 (eight) said no.
    let count = needed.max(7);
    assert!(count < 8, "share {needed} of {realm}");
    let state = english_provinces(data, 1, count);
    assert!(ai::alignment::dominates_realm(
        &state, data, &england, &france
    ));
    let state = english_provinces(data, 1, 2);
    assert!(!ai::alignment::dominates_realm(
        &state, data, &england, &france
    ));
}

/// Seeds whose dynastic roll lets history court `prince`.
fn courted_seed(data: &GameData, prince: &str) -> u64 {
    (1..60)
        .find(|seed| {
            let state = CampaignState::new_1337(data, fac("fac_papacy"), *seed).unwrap();
            ai::alignment::campaign_roll(&state, &fac(prince), 7)
                < data.ai_alignment.as_ref().unwrap().history_permille
        })
        .expect("some campaign courts the prince")
}

/// Orders `faction` plans with `plan` over the next four turns.
fn over_four_turns<T>(
    state: &mut CampaignState,
    plan: impl Fn(&CampaignState) -> Option<T>,
) -> Vec<T> {
    let start = state.turn;
    let mut out = Vec::new();
    for turn in start..start + 4 {
        state.turn = turn;
        out.extend(plan(state));
    }
    state.turn = start;
    out
}

#[test]
fn england_pensions_then_allies_brabant_beside_its_low_countries_allies() {
    let data = data();
    let (england, brabant, france) = (fac("fac_england"), fac("fac_brabant"), fac("fac_france"));
    let seed = courted_seed(data, "fac_brabant");
    let mut state = CampaignState::new_1337(data, fac("fac_papacy"), seed).unwrap();
    state.factions.get_mut(&england).unwrap().treasury = 100_000;
    // A lesser ally bordering Brabant fights France at England's side.
    let anchor = state
        .factions
        .iter()
        // FE5 (ADR 0110): the Low Countries princes are vassals of the Empire.
        .filter(|(id, f)| **id != brabant && f.alive && id.as_str() != "fac_rebels")
        // The Low Countries only: FE4b's Rhenish princes (Cologne, Trier) sort
        // first and would draw England's pension to the Rhine instead.
        .filter(|(id, _)| {
            ["fac_hainaut", "fac_guelders", "fac_flanders", "fac_julich"].contains(&id.as_str())
        })
        .map(|(id, _)| id.clone())
        .find(|id| {
            state.are_neighbors(data, &brabant, id)
                && state.faction_power(id) < 0.5 * state.faction_power(&england)
                && !state.is_at_war(id, &england)
        })
        .expect("Brabant has a lesser neighbour");
    for (a, b) in [(&england, &anchor), (&anchor, &england)] {
        state.factions.get_mut(a).unwrap().allies.insert(b.clone());
    }
    if !state.is_at_war(&anchor, &france) {
        state
            .factions
            .get_mut(&anchor)
            .unwrap()
            .at_war_with
            .insert(france.clone());
        state
            .factions
            .get_mut(&france)
            .unwrap()
            .at_war_with
            .insert(anchor.clone());
    }
    // Brabant hesitates: a pension first.
    let gifts = over_four_turns(&mut state, |s| {
        ai::alignment::plan_money_fief(&PlanCache::new(s), data, &england)
    });
    assert!(
        gifts
            .iter()
            .any(|o| matches!(o, Order::SendGift { target, .. } if *target == brabant)),
        "anchor {anchor}: {gifts:?}"
    );
    // Pensioned and won over: the alliance offer.
    let expires_turn = state.turn + 20;
    state
        .factions
        .get_mut(&brabant)
        .unwrap()
        .modifiers
        .push(OpinionModifier {
            with: england.clone(),
            value: 40,
            reason_fr: GIFT_REASON.to_owned(),
            expires_turn,
        });
    let gifts = over_four_turns(&mut state, |s| {
        ai::alignment::plan_money_fief(&PlanCache::new(s), data, &england)
    });
    assert!(
        !gifts
            .iter()
            .any(|o| matches!(o, Order::SendGift { target, .. } if *target == brabant)),
        "a running pension is not renewed"
    );
    let offers = over_four_turns(&mut state, |s| {
        ai::alignment::plan_dynastic_alliance(&PlanCache::new(s), data, &england)
    });
    assert!(
        offers
            .iter()
            .any(|o| matches!(o, Order::ProposeAlliance { target } if *target == brabant)),
        "{offers:?}"
    );
}

#[test]
fn a_vassal_courts_no_prince() {
    let data = data();
    let mut state = CampaignState::new_1337(data, fac("fac_papacy"), 1).unwrap();
    state
        .factions
        .get_mut(&fac("fac_burgundy"))
        .unwrap()
        .treasury = 1_000_000;
    let burgundy = fac("fac_burgundy");
    let gifts = over_four_turns(&mut state, |s| {
        ai::alignment::plan_money_fief(&PlanCache::new(s), data, &burgundy)
    });
    assert!(gifts.is_empty(), "{gifts:?}");
}
