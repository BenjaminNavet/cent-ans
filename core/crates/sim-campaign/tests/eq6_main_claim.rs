//! Lot EQ6 (ADR 0077): the war of a pretender for its main crown comes
//! first (`war.main_claim_first`) and does not depend on the difficulty
//! level (`war.claim_war_ignores_difficulty`): England presses its claim on
//! France whatever other wars it declared lately and whatever goodwill the
//! "easy" level lends it towards the player.

use std::path::PathBuf;

use data_model::{ClaimKind, FactionId, GameData};
use sim_campaign::difficulty::Difficulty;
use sim_campaign::diplomacy::{main_claim, plan_diplomacy, Claim, OpinionModifier};
use sim_campaign::{CampaignState, Order};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

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
    me.upkeep_last_turn = 1_000;
    me.income_last_turn = 5_000;
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
    plan_diplomacy(state, data, &fac("fac_england"))
        .into_iter()
        .find_map(|order| match order {
            Order::DeclareWar { target } => Some(target),
            _ => None,
        })
}

#[test]
fn main_claim_is_the_largest_claimed_crown() {
    let data = data();
    let state = campaign(&data, Difficulty::Normal);
    assert_eq!(
        main_claim(&state, &fac("fac_england")),
        Some(fac("fac_france"))
    );
    assert_eq!(main_claim(&state, &fac("fac_verona")), None);
}

#[test]
fn france_outranks_a_small_claimed_crown_and_ignores_the_rest() {
    let base = data();
    let data = switches(&base, true);
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
    let old = switches(&base, false);
    assert_eq!(declares(&state, &old), None);
}

#[test]
fn easy_level_goodwill_does_not_stop_the_claim_war() {
    let base = data();
    let data = switches(&base, true);
    let old = switches(&base, false);
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
            reason_fr: "Ambassade d'un héraut".to_owned(),
            expires_turn: turn + 40,
        });
    assert_eq!(state.attitude(&data, &england, &france).0, 25);
    assert_eq!(state.difficulty_attitude(&data, &england, &france), 10);
    assert_eq!(declares(&state, &data), Some(france.clone()));
    assert_ne!(declares(&state, &old), Some(france));
}
