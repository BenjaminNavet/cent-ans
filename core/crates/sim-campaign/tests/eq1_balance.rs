//! Lot EQ1 (balance) integration tests: revolt cooldown, disorder feeding
//! unrest, event costs scaled to income, rebels outside the economy, the
//! English opening economy.

use std::path::PathBuf;

use data_model::{FactionId, GameData, ProvinceId};
use sim_campaign::{chronicle, population, CampaignState, EventKind};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn set_unrest(state: &mut CampaignState, province: &ProvinceId, value: u8) {
    let p = state.provinces.get_mut(province).unwrap();
    for class in [
        &mut p.population.peasants,
        &mut p.population.burghers,
        &mut p.population.clergy,
        &mut p.population.nobility,
    ] {
        class.unrest = value;
    }
}

/// A province kept above the threshold revolts once per
/// `revolt_seasons`, not every season.
#[test]
fn a_revolt_needs_several_seasons_again_after_each_one() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_papacy"), 1).unwrap();
    state.chronicle.disabled = true;
    let province = ProvinceId::new("prov_kent").unwrap();
    let needed = data.population_rules.revolt_seasons;
    assert!(needed >= 2);
    let mut revolts = 0;
    let turns = needed * 3;
    for _ in 0..turns {
        // Unrest settles near 85 after the season (above the threshold,
        // below the hand-over to the rebels).
        set_unrest(&mut state, &province, 100);
        let events = state.end_turn(&data);
        revolts += events
            .iter()
            .filter(|e| e.kind == EventKind::Revolt && e.province.as_ref() == Some(&province))
            .count() as u32;
    }
    assert!(revolts >= 1, "a lasting unrest ends in revolt");
    assert!(
        revolts <= turns / needed + 1,
        "{revolts} revolts in {turns} seasons (one every {needed} at most)"
    );
}

/// The province's disorder gauge (captures, raids) raises class unrest.
#[test]
fn recent_disorder_raises_unrest() {
    let data = data();
    let province = ProvinceId::new("prov_kent").unwrap();
    let run = |disorder: u8| {
        let mut state = CampaignState::new_1337(&data, fac("fac_papacy"), 1).unwrap();
        state.chronicle.disabled = true;
        state.provinces.get_mut(&province).unwrap().unrest = disorder;
        state.end_turn(&data);
        population::weighted_unrest(&state.provinces[&province].population)
    };
    assert!(run(60) > run(0) + 3.0);
}

/// A small realm pays a chronicle cost in proportion to its income; a large
/// one pays it in full.
#[test]
fn event_costs_scale_with_income() {
    let data = data();
    let state = CampaignState::new_1337(&data, fac("fac_papacy"), 1).unwrap();
    let france = fac("fac_france");
    let swiss = fac("fac_swiss");
    assert_eq!(
        chronicle::event_treasury_amount(&state, &data, Some(&france), -2000),
        -2000
    );
    let small = chronicle::event_treasury_amount(&state, &data, Some(&swiss), -2000);
    let floor = (-2000.0 * data.economy_rules.event_treasury_min_scale).round() as i64;
    assert!(small > -2000 && small <= floor, "{small}");
}

/// The rebels hold no treasury: they never go bankrupt.
#[test]
fn rebels_are_outside_the_economy() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_papacy"), 1).unwrap();
    state.chronicle.disabled = true;
    let rebels = fac("fac_rebels");
    let province = ProvinceId::new("prov_kent").unwrap();
    state.city_state_mut(&province).unwrap().controller = rebels.clone();
    state.factions.get_mut(&rebels).unwrap().treasury = -500;
    let events = state.end_turn(&data);
    assert!(!events
        .iter()
        .any(|e| e.kind == EventKind::Bankruptcy && e.faction.as_ref() == Some(&rebels)));
}

/// Q1 point 10: an English player who gives no order keeps a positive
/// seasonal balance at the start and a full treasury for five turns.
#[test]
fn england_opens_without_deficit() {
    let data = data();
    let england = fac("fac_england");
    let mut state = CampaignState::new_1337(&data, england.clone(), 1).unwrap();
    state.interactive_battles = false;
    let e = state.faction_economy(&data, &england).unwrap();
    let net = e.projected_income + e.trade_income
        - e.army_upkeep
        - e.building_upkeep
        - e.administration_upkeep
        - e.table_upkeep;
    assert!(net >= 0, "England's opening balance is {net}");
    let start = state.factions[&england].treasury;
    for _ in 0..5 {
        let events = state.end_turn(&data);
        assert!(!events
            .iter()
            .any(|e| e.kind == EventKind::Bankruptcy && e.faction.as_ref() == Some(&england)));
    }
    assert!(state.factions[&england].treasury >= start * 9 / 10);
}
