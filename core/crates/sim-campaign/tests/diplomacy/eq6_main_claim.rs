//! Lot EQ6 (ADR 0085): the war of a pretender for its main crown comes
//! first (`war.main_claim_first`) and does not depend on the difficulty
//! level (`war.claim_war_ignores_difficulty`): England presses its claim on
//! France whatever other wars it declared lately and whatever goodwill the
//! "easy" level lends it towards the player. Kinship does not hold the
//! pretender back (`war.claim_war_ignores_kinship`), and a realm too weary
//! to declare a war stays out of its allies' wars (`join_war.weary_stay_out`).

use data_model::{ClaimKind, FactionId, GameData};
use sim_campaign::difficulty::Difficulty;
use sim_campaign::diplomacy::{
    answers_call_to_arms, is_cornered, main_claim, plan_diplomacy, Claim, OpinionModifier,
    MARRIAGE_REASON,
};
use sim_campaign::negotiation::plan_peace;
use sim_campaign::plan_cache::PlanCache;
use sim_campaign::{CampaignState, Order};

/// An uncapped goodwill (LR-07: herald embassies are capped when read).
const GOODWILL_REASON: &str = "Bonne volonté";

use data_model::test_support::{fac, game_data};

/// Data with both EQ6 switches set to `on`.
fn switches(data: &GameData, on: bool) -> GameData {
    let mut data = data.clone();
    data.ai_diplomacy.war.main_claim_first = on;
    data.ai_diplomacy.war.claim_war_ignores_difficulty = on;
    data
}

/// A 1337 campaign (France played) where England is at peace with
/// everyone, rich, rested from its wars and holds a claim on Verona too.
fn campaign(data: &GameData, level: Difficulty) -> CampaignState {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 11).expect("1337 start");
    state.chronicle.disabled = true;
    assert!(state.set_difficulty(level));
    let england = fac("fac_england");
    let ids: Vec<FactionId> = state.factions.keys().cloned().collect();
    for id in &ids {
        let f = state.factions.get_mut(id).unwrap();
        f.at_war_with.clear();
        f.truces.clear();
        f.war_started.clear();
        f.war_scores.clear();
    }
    // England declares on even turns (turn + slot).
    let slot = england
        .as_str()
        .bytes()
        .fold(0u32, |acc, b| acc.wrapping_add(u32::from(b)));
    state.turn = if (40 + slot).is_multiple_of(2) {
        40
    } else {
        41
    };
    let me = state.factions.get_mut(&england).unwrap();
    me.treasury = 100_000;
    me.last_budget = sim_campaign::economy::TurnBudget {
        income: 5_000,
        administration: 1_000,
        ..Default::default()
    };
    me.ledger.weariness = 0;
    me.regency = false;
    me.last_war_declared = None;
    me.claims.push(Claim {
        kind: ClaimKind::Throne,
        faction: Some(fac("fac_verona")),
        province: None,
        text_fr: "Prétention héritée par mariage".to_owned(),
        expires_turn: None,
    });
    state
}

fn declares(state: &CampaignState, data: &GameData) -> Option<FactionId> {
    plan_diplomacy(&PlanCache::new(state), data, &fac("fac_england"))
        .into_iter()
        .find_map(|order| match order {
            Order::DeclareWar { target } => Some(target),
            _ => None,
        })
}

#[test]
fn main_claim_is_the_largest_claimed_crown() {
    let data = game_data();
    let state = campaign(data, Difficulty::Normal);
    assert_eq!(
        main_claim(&state, &fac("fac_england")),
        Some(fac("fac_france"))
    );
    assert_eq!(main_claim(&state, &fac("fac_verona")), None);
}

#[test]
fn france_outranks_a_small_claimed_crown_and_ignores_the_rest() {
    let base = game_data();
    let data = switches(base, true);
    let mut state = campaign(&data, Difficulty::Normal);
    assert_eq!(declares(&state, &data), Some(fac("fac_france")));
    // A war declared elsewhere two turns ago does not hold France back...
    let turn = state.turn;
    state
        .factions
        .get_mut(&fac("fac_england"))
        .unwrap()
        .last_war_declared = Some(turn - 2);
    assert_eq!(declares(&state, &data), Some(fac("fac_france")));
    // ... whereas before EQ6 every war waited the rest.
    let old = switches(base, false);
    assert_eq!(declares(&state, &old), None);
}

#[test]
fn easy_level_goodwill_does_not_stop_the_claim_war() {
    let base = game_data();
    let data = switches(base, true);
    let old = switches(base, false);
    let mut state = campaign(&data, Difficulty::Easy);
    let england = fac("fac_england");
    let france = fac("fac_france");
    // Lift England's attitude to 25, of which +10 is the easy level.
    let now = state.attitude(&data, &england, &france).0;
    let turn = state.turn;
    state
        .factions
        .get_mut(&england)
        .unwrap()
        .modifiers
        .push(OpinionModifier {
            with: france.clone(),
            value: 25 - now,
            reason_fr: GOODWILL_REASON.to_owned(),
            expires_turn: turn + 40,
        });
    assert_eq!(state.attitude(&data, &england, &france).0, 25);
    assert_eq!(state.difficulty_attitude(&data, &england, &france), 10);
    assert_eq!(declares(&state, &data), Some(france.clone()));
    assert_ne!(declares(&state, &old), Some(france));
}

/// France (AI, England played) at war with England since this very
/// season, with France's war score set to `score`.
fn fresh_war(data: &GameData, score: i32) -> CampaignState {
    let mut state = CampaignState::new_1337(data, fac("fac_england"), 11).expect("1337 start");
    state.chronicle.disabled = true;
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    state.turn = 40;
    let ids: Vec<FactionId> = state.factions.keys().cloned().collect();
    for id in &ids {
        state.factions.get_mut(id).unwrap().at_war_with.clear();
    }
    for (a, b) in [(&france, &england), (&england, &france)] {
        let f = state.factions.get_mut(a).unwrap();
        f.at_war_with.insert(b.clone());
        f.truces.remove(b);
        f.war_started.insert(b.clone(), 40);
        f.war_scores.insert(b.clone(), 0);
    }
    let base = state.war_score(data, &france, &england);
    let me = state.factions.get_mut(&france).unwrap();
    me.war_scores.insert(england.clone(), score - base);
    // Weary and well disposed enough to sign a white peace.
    me.ledger.weariness = 100;
    me.modifiers.push(OpinionModifier {
        with: england.clone(),
        value: 40,
        reason_fr: GOODWILL_REASON.to_owned(),
        expires_turn: 80,
    });
    assert_eq!(state.war_score(data, &france, &england), score);
    state
}

#[test]
fn a_cornered_crown_fights_before_it_sues() {
    let mut data = game_data().clone();
    // Every crown counts as cornered.
    data.ai_diplomacy.peace.cornered_provinces = 1000;
    data.ai_diplomacy.peace.cornered_waits_for_defeat = true;
    let france = fac("fac_france");
    let fresh = fresh_war(&data, 0);
    assert!(is_cornered(&fresh, &data, &france));
    assert_eq!(plan_peace(&PlanCache::new(&fresh), &data, &france), None);
    let beaten = fresh_war(&data, -60);
    let beaten_offer = plan_peace(&PlanCache::new(&beaten), &data, &france);
    // Before EQ6 it sued the very season war was declared.
    data.ai_diplomacy.peace.cornered_waits_for_defeat = false;
    assert!(plan_peace(&PlanCache::new(&fresh), &data, &france).is_some());
    // Once beaten, it treats as before.
    assert_eq!(
        plan_peace(&PlanCache::new(&beaten), &data, &france),
        beaten_offer
    );
}

#[test]
fn marriages_do_not_stop_the_claim_war() {
    let base = game_data();
    let mut data = switches(base, true);
    data.ai_diplomacy.war.claim_war_ignores_kinship = true;
    // Marriages uncapped (LR-07 reads them at most at the cap): a large goodwill.
    data.diplomacy_rules.opinion_caps.clear();
    let mut old = data.clone();
    old.ai_diplomacy.war.claim_war_ignores_kinship = false;
    let mut state = campaign(&data, Difficulty::Normal);
    let england = fac("fac_england");
    let france = fac("fac_france");
    // Three marriages between the two houses: +45.
    let turn = state.turn;
    for _ in 0..3 {
        state
            .factions
            .get_mut(&england)
            .unwrap()
            .modifiers
            .push(OpinionModifier {
                with: france.clone(),
                value: 15,
                reason_fr: MARRIAGE_REASON.to_owned(),
                expires_turn: turn + 80,
            });
    }
    assert!(state.attitude(&data, &england, &france).0 >= 20);
    assert!(state.kinship_attitude(&data, &england, &france) >= 45);
    assert_eq!(declares(&state, &data), Some(france.clone()));
    assert_ne!(declares(&state, &old), Some(france));
}

#[test]
fn an_exhausted_realm_stays_out_of_its_allys_war() {
    let mut data = game_data().clone();
    data.ai_diplomacy.join_war.weary_stay_out = true;
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 11).expect("1337 start");
    state.chronicle.disabled = true;
    let (england, portugal, castile) =
        (fac("fac_england"), fac("fac_portugal"), fac("fac_castile"));
    let turn = state.turn;
    let me = state.factions.get_mut(&england).unwrap();
    me.treasury = 100_000;
    me.ledger.weariness = 0;
    me.modifiers.push(OpinionModifier {
        with: portugal.clone(),
        value: 60,
        reason_fr: GOODWILL_REASON.to_owned(),
        expires_turn: turn + 40,
    });
    assert!(answers_call_to_arms(
        &state, &data, &england, &portugal, &castile
    ));
    state.factions.get_mut(&england).unwrap().ledger.weariness = 100;
    assert!(!answers_call_to_arms(
        &state, &data, &england, &portugal, &castile
    ));
    // Before EQ6 weariness did not matter.
    data.ai_diplomacy.join_war.weary_stay_out = false;
    assert!(answers_call_to_arms(
        &state, &data, &england, &portugal, &castile
    ));
}
