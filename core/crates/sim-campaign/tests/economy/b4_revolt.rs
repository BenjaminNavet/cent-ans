//! Lot B4 (ADR 0731): graded revolts. Tension gauge and warnings, the rebel
//! army raised by an uprising, repression and the hand-over to the rebels.

use data_model::ProvinceId;
use sim_campaign::test_support::{city, idle};
use sim_campaign::{CampaignState, EventKind};

use data_model::test_support::{fac, game_data, prov};

fn set_all_unrest(state: &mut CampaignState, province: &ProvinceId, value: u8) {
    let pop = &mut state.provinces.get_mut(province).unwrap().population;
    pop.peasants.unrest = value;
    pop.burghers.unrest = value;
    pop.clergy.unrest = value;
    pop.nobility.unrest = value;
}

fn occupied_normandy(seed: u64) -> (CampaignState, ProvinceId) {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_england"), seed).unwrap();
    state.chronicle.disabled = true;
    let province = prov("prov_normandie");
    let city = city(&state, "prov_normandie");
    state.settlements.get_mut(&city).unwrap().controller = fac("fac_england");
    state.provinces.get_mut(&province).unwrap().devastation = 90;
    (state, province)
}

/// Tension climbs season after season under pressure, warns at each level
/// and falls back (slowly) once the unrest is gone.
#[test]
fn tension_is_cumulative_warns_and_decays() {
    let data = game_data();
    let rules = &data.revolt_rules;
    let (mut state, province) = occupied_normandy(301);
    let mut warnings = Vec::new();
    let mut last = 0;
    for _ in 0..rules.min_seasons_to_revolt() + 6 {
        set_all_unrest(&mut state, &province, 100);
        let events = state.end_turn_with(data, idle);
        let live = &state.provinces[&province];
        if live.uprising.is_some() {
            break;
        }
        assert!(live.tension >= last, "tension never falls under pressure");
        last = live.tension;
        warnings.extend(
            events
                .iter()
                .filter(|e| e.kind == EventKind::Unrest && e.loss)
                .filter(|e| e.province.as_ref() == Some(&province))
                .cloned(),
        );
    }
    assert!(last > 0, "tension rose");
    assert!(
        warnings.len() >= rules.warning_tensions.len().min(1),
        "at least the first warning level was announced: {warnings:?}"
    );

    // Calm: unrest at zero, the gauge falls by `tension_decay` per season.
    let peak = state.provinces[&province].tension;
    for p in state.provinces.values_mut() {
        p.uprising = None;
    }
    for _ in 0..2 {
        set_all_unrest(&mut state, &province, 0);
        state.end_turn_with(data, idle);
    }
    let after = state.provinces[&province].tension;
    assert!(after < peak || peak == 0, "peak {peak}, after {after}");
}

/// A revolt raises a `fac_rebels` army in the province and the war with
/// the sovereign; the gauge drops to `tension_after_revolt`.
#[test]
fn an_uprising_raises_a_rebel_army() {
    let data = game_data();
    let rules = &data.revolt_rules;
    let (mut state, province) = occupied_normandy(302);
    let rebels = fac("fac_rebels");
    for _ in 0..rules.min_seasons_to_revolt() + 10 {
        set_all_unrest(&mut state, &province, 100);
        state.end_turn_with(data, idle);
        if state.provinces[&province].uprising.is_some() {
            break;
        }
    }
    let uprising = state.provinces[&province]
        .uprising
        .clone()
        .expect("an uprising under sustained unrest");
    let army = &state.armies[&uprising.army];
    assert_eq!(army.faction, rebels);
    assert!(army.total_strength() > 0);
    assert_eq!(army.total_strength(), uprising.raised_strength);
    assert!(state.is_at_war(&fac("fac_england"), &rebels));
}

/// Destroying the rebel army puts the revolt down: no hand-over, lower
/// tension, war with the rebels closed.
#[test]
fn defeating_the_rebel_army_restores_order() {
    let data = game_data();
    let rules = &data.revolt_rules;
    let (mut state, province) = occupied_normandy(303);
    let rebels = fac("fac_rebels");
    for _ in 0..rules.min_seasons_to_revolt() + 10 {
        set_all_unrest(&mut state, &province, 100);
        state.end_turn_with(data, idle);
        if state.provinces[&province].uprising.is_some() {
            break;
        }
    }
    let uprising = state.provinces[&province]
        .uprising
        .clone()
        .expect("uprising");
    // The garrison (or the player) wipes the rebel army out.
    state.armies.remove(&uprising.army);
    set_all_unrest(&mut state, &province, 100);
    let events = state.end_turn_with(data, idle);
    let live = &state.provinces[&province];
    assert!(live.uprising.is_none());
    assert!(live.tension <= rules.repression_tension.max(rules.tension_after_revolt));
    assert_ne!(state.province_controller(&province), Some(&rebels));
    assert!(!state.is_at_war(&fac("fac_england"), &rebels));
    assert!(events
        .iter()
        .any(|e| e.kind == EventKind::Unrest && !e.loss && e.province.as_ref() == Some(&province)));
}

/// An uprising left unopposed for `rebel_siege_seasons` hands the city to
/// the rebels, with the army gone.
#[test]
fn an_unopposed_uprising_takes_the_city() {
    let data = game_data();
    let rules = &data.revolt_rules;
    let (mut state, province) = occupied_normandy(304);
    let rebels = fac("fac_rebels");
    let mut army = None;
    for _ in 0..rules.min_seasons_to_revolt() + rules.rebel_siege_seasons + 12 {
        set_all_unrest(&mut state, &province, 100);
        state.end_turn_with(data, idle);
        if army.is_none() {
            army = state.provinces[&province]
                .uprising
                .as_ref()
                .map(|u| u.army.clone());
        }
        if state.province_controller(&province) == Some(&rebels) {
            break;
        }
    }
    assert_eq!(state.province_controller(&province), Some(&rebels));
    let live = &state.provinces[&province];
    assert!(live.uprising.is_none());
    assert!(!state.armies.contains_key(&army.expect("army raised")));
}

/// An old save without tension or uprising still loads (serde defaults).
#[test]
fn a_province_without_the_new_fields_deserialises() {
    let data = game_data();
    let state = CampaignState::new_1337(data, fac("fac_england"), 305).unwrap();
    let province = state.provinces.values().next().unwrap();
    let mut json = serde_json::to_value(province).unwrap();
    json.as_object_mut().unwrap().remove("tension");
    json.as_object_mut().unwrap().remove("uprising");
    let back: sim_campaign::ProvinceState = serde_json::from_value(json).unwrap();
    assert_eq!(back.tension, 0);
    assert!(back.uprising.is_none());
}
