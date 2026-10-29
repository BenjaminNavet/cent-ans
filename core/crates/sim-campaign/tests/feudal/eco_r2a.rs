//! ECO-R2a (ADR 0798): the feudal host gives soldiers.

use data_model::{FactionId, GameData};
use sim_campaign::feudal::{self, FelonyReason};
use sim_campaign::test_support::start;
use sim_campaign::{CampaignState, Order};

use data_model::test_support::{fac, game_data};

/// A France game where a direct vassal that can be summoned against `enemy` is loyal (`loyalty`).
fn setup(data: &GameData, loyalty: u8) -> (CampaignState, FactionId, FactionId, FactionId) {
    let mut state = start(data, "fac_france", 7);
    let (france, enemy) = (fac("fac_france"), fac("fac_navarre"));
    let vassal = feudal::direct_vassals(&state, data, &france)
        .into_iter()
        .find(|v| *v != enemy && feudal::can_serve(&state, data, v, &france, &enemy))
        .expect("a vassal within muster range");
    state.factions.get_mut(&vassal).unwrap().loyalty = loyalty;
    (state, france, vassal, enemy)
}

fn at_war(state: &mut CampaignState, a: &FactionId, b: &FactionId) {
    state.factions.get_mut(a).unwrap().at_war_with.insert(b.clone());
    state.factions.get_mut(b).unwrap().at_war_with.insert(a.clone());
}

fn contingent_of(
    state: &CampaignState,
    vassal: &FactionId,
) -> Option<(sim_campaign::ArmyId, sim_campaign::Army)> {
    state
        .armies
        .iter()
        .find(|(_, a)| &a.faction == vassal && a.host_service.is_some())
        .map(|(id, a)| (id.clone(), a.clone()))
}

#[test]
fn answering_vassal_raises_a_contingent() {
    let data = game_data();
    let (mut state, france, vassal, enemy) = setup(data, 90);
    assert!(contingent_of(&state, &vassal).is_none());
    feudal::summon_host(&mut state, data, &france, &enemy);
    assert!(state.is_at_war(&vassal, &enemy));
    let (_, army) = contingent_of(&state, &vassal).expect("the vassal raised a contingent");
    let service = army.host_service.as_ref().unwrap();
    assert_eq!((&service.liege, &service.enemy), (&france, &enemy));
    let rules = &data.feudal_rules.host;
    let units = army.units.len() as u32;
    assert!((rules.min_units..=rules.max_units).contains(&units), "{units}");
    assert!(army.total_strength() > 0);
    // Proportioned to the income: a vassal twice as rich raises at least as many men.
    let poor = feudal::contingent_units(&state, data, &vassal);
    let mut rich_state = state.clone();
    rich_state.factions.get_mut(&vassal).unwrap().tax_rate = sim_campaign::TaxRate::High;
    let rich = feudal::contingent_units(&rich_state, data, &vassal);
    assert!(rich.len() >= poor.len());
}

#[test]
fn contingent_size_follows_the_income_share() {
    let data = game_data();
    let (state, _, vassal, _) = setup(data, 90);
    let men = |data: &GameData| -> u32 {
        feudal::contingent_units(&state, data, &vassal)
            .iter()
            .map(|u| u.strength)
            .sum()
    };
    let base = men(data);
    let mut bigger = data.clone();
    bigger.feudal_rules.host.contingent_percent_of_income = 200;
    bigger.feudal_rules.host.max_units = 40;
    let mut none = data.clone();
    none.feudal_rules.host.contingent_percent_of_income = 0;
    assert!(men(&bigger) > base, "{} vs {base}", men(&bigger));
    assert_eq!(men(&none), 0);
}

#[test]
fn contingent_goes_home_after_its_term() {
    let data = game_data();
    let (mut state, france, vassal, enemy) = setup(data, 90);
    // A broke suzerain cannot keep the contingent on: it goes home after the free season.
    state.factions.get_mut(&france).unwrap().treasury = 0;
    at_war(&mut state, &france, &enemy);
    feudal::summon_host(&mut state, data, &france, &enemy);
    let (id, _) = contingent_of(&state, &vassal).expect("contingent");
    let mut events = Vec::new();
    for _ in 1..data.feudal_rules.host.free_seasons {
        feudal::resolve_host_service(&mut state, data, &mut events);
        assert!(state.armies.contains_key(&id), "still within its free term");
    }
    feudal::resolve_host_service(&mut state, data, &mut events);
    assert!(!state.armies.contains_key(&id), "the contingent went home");
}

#[test]
fn contingent_stays_on_at_the_liege_pay_then_leaves() {
    let data = game_data();
    let (mut state, france, vassal, enemy) = setup(data, 90);
    state.factions.get_mut(&france).unwrap().treasury = 1_000_000;
    at_war(&mut state, &france, &enemy);
    feudal::summon_host(&mut state, data, &france, &enemy);
    let (id, _) = contingent_of(&state, &vassal).expect("contingent");
    let rules = &data.feudal_rules.host;
    let mut events = Vec::new();
    for _ in 0..rules.free_seasons {
        feudal::resolve_host_service(&mut state, data, &mut events);
    }
    assert!(state.armies[&id].host_service.as_ref().unwrap().paid);
    let (liege_before, vassal_before) = (
        state.factions[&france].treasury,
        state.factions[&vassal].treasury,
    );
    feudal::resolve_host_service(&mut state, data, &mut events);
    let paid = liege_before - state.factions[&france].treasury;
    assert!(paid > 0, "the suzerain pays the contingent");
    assert_eq!(state.factions[&vassal].treasury - vassal_before, paid);
    for _ in 1..rules.paid_seasons {
        feudal::resolve_host_service(&mut state, data, &mut events);
    }
    assert!(state.armies.contains_key(&id));
    feudal::resolve_host_service(&mut state, data, &mut events);
    assert!(!state.armies.contains_key(&id), "the paid term ended");
}

#[test]
fn refusal_still_opens_felony() {
    let data = game_data();
    let (mut state, france, vassal, enemy) = setup(data, 5);
    feudal::summon_host(&mut state, data, &france, &enemy);
    assert!(!state.is_at_war(&vassal, &enemy));
    assert!(contingent_of(&state, &vassal).is_none());
    assert!(state.feudal.felonies.iter().any(|c| c.vassal == vassal
        && c.liege == france
        && c.reason == FelonyReason::RefusedHost));
}

#[test]
fn summon_host_order_calls_the_vassals_again_after_the_delay() {
    let data = game_data();
    let (mut state, france, vassal, enemy) = setup(data, 90);
    at_war(&mut state, &france, &enemy);
    state.factions.get_mut(&france).unwrap().treasury = 0;
    let order = Order::SummonHost {
        enemy: enemy.clone(),
    };
    state.submit_order(data, order.clone()).expect("first summons");
    assert!(contingent_of(&state, &vassal).is_some());
    // The contingent goes home; the delay still holds the suzerain back.
    let mut events = Vec::new();
    feudal::resolve_host_service(&mut state, data, &mut events);
    assert!(contingent_of(&state, &vassal).is_none());
    assert!(state.submit_order(data, order.clone()).is_err(), "too soon");
    state.turn += data.feudal_rules.host.resummon_delay_seasons;
    state.submit_order(data, order).expect("second summons");
    assert!(
        contingent_of(&state, &vassal).is_some(),
        "the vassal, already at war, raises a new contingent"
    );
}

#[test]
fn summon_host_order_needs_a_war() {
    let data = game_data();
    let (mut state, _, _, enemy) = setup(data, 90);
    assert!(state
        .submit_order(data, Order::SummonHost { enemy })
        .is_err());
}
