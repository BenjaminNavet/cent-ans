//! Lot RG10 (ADR 0344): settlement vocations — data, order, effects,
//! construction cost, coherence tiers and AI choice.

use data_model::test_support::{fac, game_data};
use data_model::{EffectKind, SettlementId};
use sim_campaign::test_support::bld;
use sim_campaign::vocations::{adjusted_cost, vocation_delta};
use sim_campaign::{CampaignState, Order, OrderError};

const FRANCE: &str = "fac_france";

/// A city held by the player, emptied of buildings, with a rich treasury.
fn player_city(state: &mut CampaignState) -> SettlementId {
    let player = state.player_faction.clone();
    let id = state
        .settlements
        .iter()
        .find(|(_, s)| s.controller == player && s.kind == data_model::SettlementKind::City)
        .map(|(id, _)| id.clone())
        .expect("the player holds a city");
    state.settlements.get_mut(&id).unwrap().buildings.clear();
    state.factions.get_mut(&player).unwrap().treasury = 100_000;
    id
}

fn set_vocation(
    state: &mut CampaignState,
    settlement: &SettlementId,
    vocation: &str,
) -> Result<(), OrderError> {
    state.submit_order(
        game_data(),
        Order::SetSettlementVocation {
            settlement: settlement.clone().into(),
            vocation: vocation.to_owned(),
        },
    )
}

fn set_buildings(state: &mut CampaignState, settlement: &SettlementId, ids: &[&str]) {
    state.settlements.get_mut(settlement).unwrap().buildings = ids.iter().map(|b| bld(b)).collect();
}

fn new_state() -> CampaignState {
    CampaignState::new_1337(game_data(), fac(FRANCE), 1).unwrap()
}

#[test]
fn the_data_is_coherent() {
    let data = game_data();
    let rules = &data.vocation_rules;
    assert_eq!(rules.vocations.len(), 6);
    for vocation in &rules.vocations {
        for id in &vocation.buildings {
            assert!(data.buildings.contains_key(id), "{} unknown", id);
        }
        // Every vocation concerns at least one building.
        assert!(
            data.buildings.values().any(|b| vocation.concerns(b)),
            "{} concerns nothing",
            vocation.id
        );
        let mins: Vec<u32> = vocation.coherence.iter().map(|t| t.min_buildings).collect();
        assert!(mins.windows(2).all(|w| w[0] < w[1]), "tiers must ascend");
        assert!(vocation.cost_reduction_percent < 100.0);
        assert!(vocation.effect_bonus_percent <= 25.0, "moderate bonus");
    }
    for kind in [
        data_model::SettlementKind::City,
        data_model::SettlementKind::Village,
    ] {
        assert!(rules.cost_for(kind, false) > 0);
    }
}

#[test]
fn no_vocation_means_no_effect() {
    let data = game_data();
    let mut state = new_state();
    let city = player_city(&mut state);
    set_buildings(&mut state, &city, &["bld_market", "bld_guild_hall"]);
    let live = state.settlement_state(&city).unwrap().clone();
    assert!(live.vocation.is_none());
    assert_eq!(vocation_delta(data, &live), Default::default());
    let view = state.settlement_vocation(data, &city).unwrap();
    assert_eq!(view.vocation, None);
    assert!(view.active_effects.is_empty());
    let market = &data.buildings[&bld("bld_market")];
    assert_eq!(adjusted_cost(data, &live, market, 300), 300);
}

#[test]
fn old_saves_without_the_fields_load() {
    let mut state = new_state();
    let json = serde_json::to_value(&state).unwrap();
    assert!(!json.to_string().contains("vocation_set_turn"));
    let back: CampaignState = serde_json::from_value(json).unwrap();
    assert!(back.settlements.values().all(|s| s.vocation.is_none()));
    let city = player_city(&mut state);
    set_vocation(&mut state, &city, "mercantile").unwrap();
    let json = serde_json::to_string(&state).unwrap();
    let back: CampaignState = serde_json::from_str(&json).unwrap();
    assert_eq!(
        back.settlement_state(&city).unwrap().vocation.as_deref(),
        Some("mercantile")
    );
}

#[test]
fn the_order_is_validated_and_charged() {
    let data = game_data();
    let mut state = new_state();
    let city = player_city(&mut state);
    let player = state.player_faction.clone();
    let kind = state.settlement_state(&city).unwrap().kind;
    let first = data.vocation_rules.cost_for(kind, true);
    let full = data.vocation_rules.cost_for(kind, false);
    assert!(first < full);

    // Unknown vocation, foreign settlement.
    assert!(matches!(
        set_vocation(&mut state, &city, "voc_nope"),
        Err(OrderError::VocationRefused(_))
    ));
    let foreign = state
        .settlements
        .iter()
        .find(|(_, s)| s.controller != player)
        .map(|(id, _)| id.clone())
        .unwrap();
    assert!(matches!(
        set_vocation(&mut state, &foreign, "mercantile"),
        Err(OrderError::NotYourSettlement(_))
    ));
    // Not enough gold.
    state.factions.get_mut(&player).unwrap().treasury = i64::from(first) - 1;
    assert!(matches!(
        set_vocation(&mut state, &city, "mercantile"),
        Err(OrderError::InsufficientFunds { .. })
    ));
    state.factions.get_mut(&player).unwrap().treasury = 10_000;

    // The first vocation is cheaper.
    set_vocation(&mut state, &city, "mercantile").unwrap();
    assert_eq!(state.factions[&player].treasury, 10_000 - i64::from(first));
    // Same vocation refused, and the lock holds.
    assert!(matches!(
        set_vocation(&mut state, &city, "mercantile"),
        Err(OrderError::VocationRefused(_))
    ));
    let err = set_vocation(&mut state, &city, "military").unwrap_err();
    assert!(err.to_string().contains("ne peut plus changer"), "{err}");
    let view = state.settlement_vocation(data, &city).unwrap();
    assert!(!view.can_change);
    assert_eq!(
        view.locked_until,
        Some(state.turn + data.vocation_rules.change_lock_turns)
    );

    // After the lock, a change costs the full price.
    state.turn += data.vocation_rules.change_lock_turns;
    let before = state.factions[&player].treasury;
    set_vocation(&mut state, &city, "military").unwrap();
    assert_eq!(state.factions[&player].treasury, before - i64::from(full));
    assert_eq!(
        state.settlement_state(&city).unwrap().vocation.as_deref(),
        Some("military")
    );
}

#[test]
fn bonus_and_coherence_reach_the_effects() {
    let data = game_data();
    let mut state = new_state();
    let city = player_city(&mut state);
    set_buildings(&mut state, &city, &["bld_market", "bld_counting_house"]);
    let trade_before = state.settlement_effects(data, &city)[EffectKind::TradeIncome].percent;
    set_vocation(&mut state, &city, "mercantile").unwrap();
    let vocation = data.vocation_rules.get("mercantile").unwrap();
    let trade_after = state.settlement_effects(data, &city)[EffectKind::TradeIncome].percent;
    // 15 % of (5 + 20) plus the tier-2 coherence (+5).
    let expected = (5.0 + 20.0) * vocation.effect_bonus_percent / 100.0
        + vocation.coherence[0].effects[0].value;
    assert!(
        (trade_after - trade_before - expected).abs() < 1e-9,
        "{trade_before} -> {trade_after}, expected +{expected}"
    );
    // The province sees the delta too (weighed by the settlement kind).
    let province = state.settlement_state(&city).unwrap().province.clone();
    let province_trade =
        state.province_building_effects(data, &province)[EffectKind::TradeIncome].percent;
    assert!(province_trade > 0.0);
}

#[test]
fn coherence_tiers_are_cumulative_and_shown() {
    let data = game_data();
    let mut state = new_state();
    let city = player_city(&mut state);
    set_vocation(&mut state, &city, "mercantile").unwrap();

    set_buildings(&mut state, &city, &["bld_market"]);
    let view = state.settlement_vocation(data, &city).unwrap();
    assert_eq!((view.count, view.reached_tiers), (1, 0));
    assert_eq!(view.next_tier.as_ref().unwrap().min_buildings, 2);

    set_buildings(&mut state, &city, &["bld_market", "bld_guild_hall"]);
    let view = state.settlement_vocation(data, &city).unwrap();
    assert_eq!((view.count, view.reached_tiers), (2, 1));
    let first_tier_only = view.active_effects.clone();

    set_buildings(
        &mut state,
        &city,
        &["bld_market", "bld_guild_hall", "bld_fair", "bld_port"],
    );
    let view = state.settlement_vocation(data, &city).unwrap();
    assert_eq!((view.count, view.reached_tiers), (4, 2));
    assert!(view.next_tier.is_none());
    // The coherence of tier 1 is still there under tier 2 (cumulative).
    let tax = view
        .active_effects
        .iter()
        .find(|e| e.effect == EffectKind::TaxIncome)
        .expect("tier 2 grants tax income");
    assert!(tax.value > 0.0);
    assert!(first_tier_only
        .iter()
        .all(|e| e.effect != EffectKind::TaxIncome));
}

#[test]
fn construction_cost_follows_the_vocation() {
    let data = game_data();
    let mut state = new_state();
    let city = player_city(&mut state);
    let price = |state: &CampaignState, building: &str| {
        state
            .buildable(data, &city)
            .into_iter()
            .find(|o| o.building == bld(building))
            .map(|o| o.cost)
            .unwrap()
    };
    let (market, forge) = (price(&state, "bld_market"), price(&state, "bld_forge"));
    set_vocation(&mut state, &city, "mercantile").unwrap();
    let vocation = data.vocation_rules.get("mercantile").unwrap();
    let cheaper = price(&state, "bld_market");
    let dearer = price(&state, "bld_forge");
    let near = |a: u32, b: f64| (f64::from(a) - b).abs() <= 2.0;
    assert!(near(
        cheaper,
        f64::from(market) * (1.0 - vocation.cost_reduction_percent / 100.0)
    ));
    assert!(near(
        dearer,
        f64::from(forge) * (1.0 + vocation.off_vocation_surcharge_percent / 100.0)
    ));
    // The order really charges the displayed price.
    let player = state.player_faction.clone();
    let before = state.factions[&player].treasury;
    state
        .submit_order(
            data,
            Order::Build {
                settlement: city.clone().into(),
                building: bld("bld_market"),
            },
        )
        .unwrap();
    assert_eq!(
        state.factions[&player].treasury,
        before - i64::from(cheaper)
    );
}

#[test]
fn structural_effects_are_not_scaled() {
    let data = game_data();
    let mut state = new_state();
    let city = player_city(&mut state);
    set_buildings(&mut state, &city, &["bld_stone_walls", "bld_armoury"]);
    let walls_before = state.settlement_effects(data, &city)[EffectKind::FortificationLevel];
    let slots_before = state.settlement_effects(data, &city)[EffectKind::RecruitSlots];
    set_vocation(&mut state, &city, "fortified").unwrap();
    let effects = state.settlement_effects(data, &city);
    assert_eq!(effects[EffectKind::FortificationLevel], walls_before);
    assert_eq!(effects[EffectKind::RecruitSlots], slots_before);
    // But the garrison and the siege resistance gain.
    assert!(effects[EffectKind::Garrison].flat > 0.0);
}

#[test]
fn options_report_cost_and_fit() {
    let data = game_data();
    let mut state = new_state();
    let city = player_city(&mut state);
    set_buildings(&mut state, &city, &["bld_parish_church", "bld_abbey"]);
    let options = state.vocation_options(data, &city);
    assert_eq!(options.len(), data.vocation_rules.vocations.len());
    let religious = options.iter().find(|o| o.id == "religious").unwrap();
    assert_eq!(religious.fit, 2);
    assert!(religious.available);
    assert!(!religious.preview_effects.is_empty());
    let first = data
        .vocation_rules
        .cost_for(data_model::SettlementKind::City, true);
    assert_eq!(religious.cost, first);
    let military = options.iter().find(|o| o.id == "military").unwrap();
    assert_eq!(military.fit, 0);
    // The current vocation cannot be re-chosen.
    set_vocation(&mut state, &city, "religious").unwrap();
    let options = state.vocation_options(data, &city);
    let current = options.iter().find(|o| o.current).unwrap();
    assert!(!current.available);
    assert!(options.iter().filter(|o| !o.current).all(|o| !o.available));
}

#[test]
fn the_ai_picks_the_best_fit_deterministically() {
    let data = game_data();
    let mut state = new_state();
    let city = player_city(&mut state);
    // Nothing built: nothing to specialise in.
    assert_eq!(state.suggest_vocation(data, &city), None);
    // One building is below `ai_min_buildings`.
    set_buildings(&mut state, &city, &["bld_market"]);
    assert_eq!(state.suggest_vocation(data, &city), None);
    set_buildings(
        &mut state,
        &city,
        &["bld_market", "bld_guild_hall", "bld_fair"],
    );
    // CC C6: `ai_min_buildings` is 3 (three commerce buildings make a mercantile place).
    let choice = state.suggest_vocation(data, &city);
    assert_eq!(choice.as_deref(), Some("mercantile"));
    assert_eq!(state.suggest_vocation(data, &city), choice);
    // Once adopted it is left alone, and locked anyway.
    set_vocation(&mut state, &city, "mercantile").unwrap();
    assert_eq!(state.suggest_vocation(data, &city), None);
    // A rare change: unlocked, and another vocation ahead by the margin.
    state.turn += data.vocation_rules.change_lock_turns;
    set_buildings(
        &mut state,
        &city,
        &[
            "bld_market",
            "bld_parish_church",
            "bld_abbey",
            "bld_scriptorium",
            "bld_hotel_dieu",
        ],
    );
    assert_eq!(
        state.suggest_vocation(data, &city).as_deref(),
        Some("religious")
    );
    // One building ahead is not enough.
    set_buildings(
        &mut state,
        &city,
        &[
            "bld_market",
            "bld_guild_hall",
            "bld_abbey",
            "bld_parish_church",
        ],
    );
    assert_eq!(state.suggest_vocation(data, &city), None);
}
