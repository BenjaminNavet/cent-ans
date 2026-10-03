//! Lot RS-C: opinion caps per motive (`data/rules/diplomacy.json`) and the
//! demolition order, on the real data of 1337.

use std::path::PathBuf;

use data_model::{BuildingId, FactionId, GameData, OpinionMotive, SettlementId};

use crate::agents::PARLEY_REASON;
use crate::diplomacy::{opinion_motive, GIFT_REASON, MARRIAGE_REASON};
use crate::negotiation::TREATY_REASON;
use crate::orders::{Order, OrderError};
use crate::state::CampaignState;

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn bld(id: &str) -> BuildingId {
    BuildingId::new(id).unwrap()
}

fn running(state: &CampaignState, holder: &str, with: &str, reason: &str) -> (i32, usize) {
    let with = fac(with);
    let modifiers: Vec<_> = state.factions[&fac(holder)]
        .modifiers
        .iter()
        .filter(|m| m.with == with && m.reason_fr == reason && m.expires_turn > state.turn)
        .collect();
    (modifiers.iter().map(|m| m.value).sum(), modifiers.len())
}

fn campaign(data: &GameData) -> CampaignState {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 7).unwrap();
    state.chronicle.disabled = true;
    state
}

#[test]
fn every_capped_motive_maps_to_a_reason() {
    for (reason, motive) in [
        (MARRIAGE_REASON, OpinionMotive::Marriage),
        (PARLEY_REASON, OpinionMotive::HeraldEmbassy),
        (GIFT_REASON, OpinionMotive::Gift),
        (TREATY_REASON, OpinionMotive::Treaty),
    ] {
        assert_eq!(opinion_motive(reason), Some(motive));
    }
    assert_eq!(opinion_motive("En guerre"), None);
}

#[test]
fn marriages_stack_up_to_the_cap_then_only_renew() {
    let data = data();
    let cap = data
        .diplomacy_rules
        .opinion_cap(OpinionMotive::Marriage)
        .expect("marriage is capped");
    let mut state = campaign(&data);
    let (england, france) = (fac("fac_england"), fac("fac_france"));
    for _ in 0..10 {
        state.add_capped_modifier(&data, &england, &france, 15, MARRIAGE_REASON, 80);
    }
    let (total, count) = running(&state, "fac_england", "fac_france", MARRIAGE_REASON);
    assert_eq!(total, cap);
    assert!(count <= (cap as usize).div_ceil(15));
    // Later marriages renew the goodwill already won.
    state.turn += 70;
    state.add_capped_modifier(&data, &england, &france, 15, MARRIAGE_REASON, 80);
    let (total, _) = running(&state, "fac_england", "fac_france", MARRIAGE_REASON);
    assert_eq!(total, cap);
    state.turn += 20;
    let (total, _) = running(&state, "fac_england", "fac_france", MARRIAGE_REASON);
    assert_eq!(total, cap, "renewed up to the last marriage's expiry");
}

#[test]
fn heralds_do_not_stack_beyond_their_cap() {
    let data = data();
    let cap = data
        .diplomacy_rules
        .opinion_cap(OpinionMotive::HeraldEmbassy)
        .expect("herald embassies are capped");
    let mut state = campaign(&data);
    let (castile, france) = (fac("fac_castile"), fac("fac_france"));
    for _ in 0..6 {
        state.add_capped_modifier(&data, &castile, &france, 12, PARLEY_REASON, 12);
    }
    assert_eq!(
        running(&state, "fac_castile", "fac_france", PARLEY_REASON).0,
        cap
    );
}

#[test]
fn uncapped_motives_still_stack() {
    let mut data = data();
    data.diplomacy_rules.opinion_caps.clear();
    let mut state = campaign(&data);
    let (england, france) = (fac("fac_england"), fac("fac_france"));
    for _ in 0..10 {
        state.add_capped_modifier(&data, &england, &france, 15, MARRIAGE_REASON, 80);
    }
    assert_eq!(
        running(&state, "fac_england", "fac_france", MARRIAGE_REASON),
        (150, 10)
    );
}

#[test]
fn a_negative_motive_is_capped_from_below() {
    let mut data = data();
    data.diplomacy_rules
        .opinion_caps
        .insert(OpinionMotive::Gift, 10);
    let mut state = campaign(&data);
    let (england, france) = (fac("fac_england"), fac("fac_france"));
    for _ in 0..3 {
        state.add_capped_modifier(&data, &england, &france, -6, GIFT_REASON, 20);
    }
    assert_eq!(
        running(&state, "fac_england", "fac_france", GIFT_REASON).0,
        -10
    );
}

/// A settlement of `faction` with `buildings` and no construction nor siege.
fn place_with(state: &mut CampaignState, faction: &str, buildings: &[&str]) -> SettlementId {
    let faction = fac(faction);
    let id = state
        .settlements
        .iter()
        .find(|(_, s)| s.controller == faction && s.siege.is_none())
        .map(|(id, _)| id.clone())
        .expect("a settlement");
    let place = state.settlements.get_mut(&id).unwrap();
    place.buildings = buildings.iter().map(|b| bld(b)).collect();
    place.construction = None;
    id
}

#[test]
fn demolition_razes_the_building_and_refunds_its_share() {
    let data = data();
    let mut state = campaign(&data);
    let place = place_with(&mut state, "fac_france", &["bld_market", "bld_apothecary"]);
    let treasury = state.factions[&fac("fac_france")].treasury;
    let apothecary = &data.buildings[&bld("bld_apothecary")];
    state
        .submit_order(
            &data,
            Order::Demolish {
                settlement: place.clone().into(),
                building: bld("bld_apothecary"),
            },
        )
        .unwrap();
    assert_eq!(state.settlements[&place].buildings, vec![bld("bld_market")]);
    let refund = i64::from(apothecary.cost.money)
        * i64::from(data.economy_rules.demolition_refund_percent)
        / 100;
    assert!(refund > 0);
    assert_eq!(
        state.factions[&fac("fac_france")].treasury,
        treasury + refund
    );
}

#[test]
fn demolition_is_refused_when_another_building_depends_on_it() {
    let data = data();
    let mut state = campaign(&data);
    let place = place_with(&mut state, "fac_france", &["bld_market", "bld_apothecary"]);
    let refused = state.submit_order(
        &data,
        Order::Demolish {
            settlement: place.clone().into(),
            building: bld("bld_market"),
        },
    );
    assert!(matches!(refused, Err(OrderError::DemolitionRefused(_))));
    assert_eq!(state.settlements[&place].buildings.len(), 2);
    // A building that is not there cannot be razed.
    let absent = state.submit_order(
        &data,
        Order::Demolish {
            settlement: place.clone().into(),
            building: bld("bld_fair"),
        },
    );
    assert!(matches!(absent, Err(OrderError::DemolitionRefused(_))));
}

#[test]
fn demolition_needs_ones_own_settlement() {
    let data = data();
    let mut state = campaign(&data);
    let place = place_with(&mut state, "fac_england", &["bld_market"]);
    let refused = state.submit_order(
        &data,
        Order::Demolish {
            settlement: place.into(),
            building: bld("bld_market"),
        },
    );
    assert!(matches!(refused, Err(OrderError::NotYourSettlement(_))));
}

#[test]
fn the_order_reads_from_the_bridge_json() {
    let order: Order = serde_json::from_str(
        r#"{"type": "demolish", "settlement": "set_paris", "building": "bld_market"}"#,
    )
    .expect("demolish order parses");
    assert!(matches!(order, Order::Demolish { .. }));
}

#[test]
fn deficit_seasons_count_the_seasons_in_the_red() {
    let data = data();
    let mut state = campaign(&data);
    let france = fac("fac_france");
    // An absurd upkeep: every building of the realm is an apothecary.
    let ids: Vec<SettlementId> = state
        .settlements
        .iter()
        .filter(|(_, s)| s.controller == france)
        .map(|(id, _)| id.clone())
        .collect();
    for id in &ids {
        state.settlements.get_mut(id).unwrap().buildings = vec![bld("bld_apothecary"); 40];
    }
    let mut events = Vec::new();
    crate::economy::resolve_economy(&mut state, &data, &mut events);
    crate::economy::resolve_economy(&mut state, &data, &mut events);
    assert_eq!(state.factions[&france].deficit_seasons, 2);
    // Nothing left to pay: no building, no troops.
    for id in &ids {
        let place = state.settlements.get_mut(id).unwrap();
        place.buildings.clear();
        place.garrison.clear();
    }
    state.armies.retain(|_, a| a.faction != france);
    crate::economy::resolve_economy(&mut state, &data, &mut events);
    assert_eq!(state.factions[&france].deficit_seasons, 0);
}

/// LR-07: modifiers written past the cap (a save from before RS-C, an event,
/// a direct push) weigh at most the cap in the attitude, on one line.
#[test]
fn the_attitude_reads_a_capped_motive_at_most_at_its_cap() {
    let data = data();
    let cap = data
        .diplomacy_rules
        .opinion_cap(OpinionMotive::Marriage)
        .expect("marriage is capped");
    let mut state = campaign(&data);
    let (england, france) = (fac("fac_england"), fac("fac_france"));
    let turn = state.turn;
    for _ in 0..10 {
        state.factions.get_mut(&england).unwrap().modifiers.push(
            crate::diplomacy::OpinionModifier {
                with: france.clone(),
                value: 15,
                reason_fr: MARRIAGE_REASON.to_owned(),
                expires_turn: turn + 80,
            },
        );
    }
    let (_, reasons) = state.attitude(&data, &england, &france);
    let marriage: Vec<i32> = reasons
        .iter()
        .filter(|(reason, _)| reason == MARRIAGE_REASON)
        .map(|(_, value)| *value)
        .collect();
    assert_eq!(marriage, vec![cap]);
    // Uncapped, the ten marriages would still stack.
    let mut uncapped = data.clone();
    uncapped.diplomacy_rules.opinion_caps.clear();
    let (_, reasons) = state.attitude(&uncapped, &england, &france);
    let total: i32 = reasons
        .iter()
        .filter(|(reason, _)| reason == MARRIAGE_REASON)
        .map(|(_, value)| *value)
        .sum();
    assert_eq!(total, 150);
}

/// LR-07: an event's opinion effect with a capped motive is capped too.
#[test]
fn an_event_opinion_of_a_capped_motive_is_capped() {
    let data = data();
    let cap = data
        .diplomacy_rules
        .opinion_cap(OpinionMotive::Marriage)
        .expect("marriage is capped");
    let mut state = campaign(&data);
    let (england, france) = (fac("fac_england"), fac("fac_france"));
    let effect = data_model::EventEffect::Opinion {
        faction: england.clone(),
        towards: Some(france.clone()),
        amount: 25,
        reason: MARRIAGE_REASON.to_owned(),
        duration: Some(40),
    };
    let ctx = crate::chronicle::EventContext::default();
    let mut events = Vec::new();
    for _ in 0..4 {
        crate::chronicle::apply_effect(&mut state, &data, &effect, &ctx, &mut events);
    }
    assert_eq!(
        running(&state, "fac_england", "fac_france", MARRIAGE_REASON).0,
        cap
    );
}
