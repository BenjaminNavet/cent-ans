//! M3 (cities & economy) integration tests: population dynamics, buildings,
//! goods, taxes, revolt/plague/famine events, and the version-2 save format.
//! See `docs/design/m3-cities-economy.md` § 1.6.

use std::path::PathBuf;

use data_model::{FactionId, GameData, ProvinceId};
use sim_campaign::population::weighted_unrest;
use sim_campaign::{CampaignState, EventKind, Order, Season, TaxRate};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn prov(id: &str) -> ProvinceId {
    ProvinceId::new(id).unwrap()
}

fn building(id: &str) -> data_model::BuildingId {
    data_model::BuildingId::new(id).unwrap()
}

fn france(data: &GameData, seed: u64) -> CampaignState {
    CampaignState::new_1337(data, fac("fac_france"), seed).expect("1337 start")
}

/// A planner that does nothing: only the orders the test submits apply.
fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

fn set_all_unrest(state: &mut CampaignState, province: &ProvinceId, value: u8) {
    let pop = &mut state.provinces.get_mut(province).unwrap().population;
    pop.peasants.unrest = value;
    pop.burghers.unrest = value;
    pop.clergy.unrest = value;
    pop.nobility.unrest = value;
}

fn set_all_health(state: &mut CampaignState, province: &ProvinceId, value: u8) {
    let pop = &mut state.provinces.get_mut(province).unwrap().population;
    pop.peasants.health = value;
    pop.burghers.health = value;
    pop.clergy.health = value;
    pop.nobility.health = value;
}

// ----- population dynamics (spec § 1.1) -------------------------------------

#[test]
fn growth_is_positive_when_health_is_high() {
    let data = data();
    let mut state = france(&data, 101);
    let paris = prov("prov_ile_de_france");
    set_all_health(&mut state, &paris, 90);
    let before = state
        .province_state(&paris)
        .unwrap()
        .population
        .peasants
        .count;
    state.end_turn_with(&data, idle);
    let after = state
        .province_state(&paris)
        .unwrap()
        .population
        .peasants
        .count;
    assert!(after > before, "before={before} after={after}");
}

#[test]
fn growth_is_negative_when_health_is_low() {
    let data = data();
    let mut state = france(&data, 102);
    let paris = prov("prov_ile_de_france");
    set_all_health(&mut state, &paris, 5);
    let before = state
        .province_state(&paris)
        .unwrap()
        .population
        .peasants
        .count;
    state.end_turn_with(&data, idle);
    let after = state
        .province_state(&paris)
        .unwrap()
        .population
        .peasants
        .count;
    assert!(after < before, "before={before} after={after}");
}

#[test]
fn low_health_triggers_a_plague() {
    let data = data();
    let mut state = france(&data, 103);
    let paris = prov("prov_ile_de_france");
    set_all_health(&mut state, &paris, 5);
    let before = state
        .province_state(&paris)
        .unwrap()
        .population
        .peasants
        .count;
    let events = state.end_turn_with(&data, idle);
    assert!(events.iter().any(|e| e.kind == EventKind::Plague));
    let after = state
        .province_state(&paris)
        .unwrap()
        .population
        .peasants
        .count;
    assert!(after < before, "plague must shrink the population further");
}

#[test]
fn devastated_province_starves_in_winter() {
    let data = data();
    let mut state = france(&data, 104);
    let paris = prov("prov_ile_de_france");
    for _ in 0..3 {
        state.end_turn_with(&data, idle);
    }
    assert_eq!(state.season(), Season::Winter);
    state.provinces.get_mut(&paris).unwrap().devastation = 90;
    let before = state
        .province_state(&paris)
        .unwrap()
        .population
        .peasants
        .count;
    let events = state.end_turn_with(&data, idle);
    assert!(events.iter().any(|e| e.kind == EventKind::Famine));
    let after = state
        .province_state(&paris)
        .unwrap()
        .population
        .peasants
        .count;
    assert!(after < before);
}

#[test]
fn sustained_high_unrest_triggers_a_revolt() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_england"), 105).unwrap();
    let boulogne = prov("prov_boulonnais");
    {
        let p = state.provinces.get_mut(&boulogne).unwrap();
        p.controller = fac("fac_england"); // occupied by a foreign faction
        p.devastation = 90;
    }
    set_all_unrest(&mut state, &boulogne, 99);
    let mut revolted = false;
    for _ in 0..3 {
        let events = state.end_turn_with(&data, idle);
        set_all_unrest(&mut state, &boulogne, 99); // keep it under pressure
        if events.iter().any(|e| e.kind == EventKind::Revolt) {
            revolted = true;
            break;
        }
    }
    assert!(revolted, "two seasons above the threshold must revolt");
}

#[test]
fn extreme_unrest_hands_the_province_to_the_rebels() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_england"), 106).unwrap();
    let boulogne = prov("prov_boulonnais");
    {
        let p = state.provinces.get_mut(&boulogne).unwrap();
        p.controller = fac("fac_england");
        p.devastation = 95;
    }
    let rebels = fac("fac_rebels");
    let mut taken_by_rebels = false;
    for _ in 0..6 {
        set_all_unrest(&mut state, &boulogne, 100);
        state.end_turn_with(&data, idle);
        if state.province_state(&boulogne).unwrap().controller == rebels {
            taken_by_rebels = true;
            break;
        }
    }
    assert!(
        taken_by_rebels,
        "unrest > 90 must hand the province to fac_rebels; controller = {:?}",
        state.province_state(&boulogne).unwrap().controller
    );
}

// ----- goods and satisfaction (spec § 1.3) ----------------------------------

#[test]
fn more_goods_categories_raise_satisfaction() {
    let rich = data();
    let mut poor = data();
    for province in poor.provinces.values_mut() {
        province.resources.retain(|r| r.as_str() == "res_wheat");
    }
    let mut rich_state = CampaignState::new_1337(&rich, fac("fac_france"), 107).unwrap();
    let mut poor_state = CampaignState::new_1337(&poor, fac("fac_france"), 107).unwrap();
    for _ in 0..4 {
        rich_state.end_turn_with(&rich, idle);
        poor_state.end_turn_with(&poor, idle);
    }
    let paris = prov("prov_ile_de_france");
    let rich_goods = rich_state
        .province_state(&paris)
        .unwrap()
        .population
        .peasants
        .goods_satisfaction;
    let poor_goods = poor_state
        .province_state(&paris)
        .unwrap()
        .population
        .peasants
        .goods_satisfaction;
    assert!(
        rich_goods > poor_goods,
        "rich={rich_goods} poor={poor_goods}"
    );
}

// ----- buildings and construction (spec § 1.2) ------------------------------

#[test]
fn buildable_reports_the_missing_prerequisite() {
    let data = data();
    let mut state = france(&data, 108);
    let paris = prov("prov_ile_de_france");
    state.provinces.get_mut(&paris).unwrap().buildings.clear();
    let options = state.buildable(&data, &paris);
    let guild_hall = options
        .iter()
        .find(|o| o.building == building("bld_guild_hall"))
        .unwrap();
    assert!(!guild_hall.available);
    assert!(
        guild_hall.reason.as_deref().unwrap().contains("bâtiment"),
        "{:?}",
        guild_hall.reason
    );

    state
        .provinces
        .get_mut(&paris)
        .unwrap()
        .buildings
        .push(building("bld_market"));
    let options = state.buildable(&data, &paris);
    let guild_hall = options
        .iter()
        .find(|o| o.building == building("bld_guild_hall"))
        .unwrap();
    assert!(guild_hall.available, "{:?}", guild_hall.reason);
}

#[test]
fn build_order_charges_cost_and_completes_after_its_duration() {
    let data = data();
    let mut state = france(&data, 109);
    let paris = prov("prov_ile_de_france");
    let france_id = fac("fac_france");
    let armoury = building("bld_armoury"); // upgrades_from bld_muster_field, already present
    let cost = data.buildings[&armoury].cost.money;
    let turns = data.buildings[&armoury].build_time_turns;

    let treasury_before = state.faction_state(&france_id).unwrap().treasury;
    state
        .submit_order(
            &data,
            Order::Build {
                province: paris.clone(),
                building: armoury.clone(),
            },
        )
        .unwrap();
    assert_eq!(
        state.faction_state(&france_id).unwrap().treasury,
        treasury_before - i64::from(cost)
    );
    let construction = state
        .province_state(&paris)
        .unwrap()
        .construction
        .clone()
        .expect("construction started");
    assert_eq!(construction.building, armoury);
    assert_eq!(construction.turns_left, turns);

    let mut completed = false;
    let mut events = Vec::new();
    for _ in 0..turns {
        events = state.end_turn_with(&data, idle);
        if state.province_state(&paris).unwrap().construction.is_none() {
            completed = true;
            break;
        }
    }
    assert!(completed, "construction must finish after {turns} turns");
    assert!(events
        .iter()
        .any(|e| e.kind == EventKind::BuildingCompleted));
    let buildings = &state.province_state(&paris).unwrap().buildings;
    assert!(buildings.contains(&armoury));
    assert!(
        !buildings.contains(&building("bld_muster_field")),
        "the upgrade replaces the old building"
    );
}

#[test]
fn completed_building_effects_apply_to_the_province() {
    let data = data();
    let mut state = france(&data, 110);
    let province = prov("prov_agenais");
    // bld_guild_hall requires bld_market (already present) and adds +5
    // wealth / -5 unrest (both targeted at burghers, summed province-wide).
    let guild_hall = building("bld_guild_hall");
    let turns = data.buildings[&guild_hall].build_time_turns;
    let effects_before = state.province_effects(&data, &province);
    state
        .submit_order(
            &data,
            Order::Build {
                province: province.clone(),
                building: guild_hall.clone(),
            },
        )
        .unwrap();
    for _ in 0..turns {
        state.end_turn_with(&data, idle);
    }
    let buildings = &state.province_state(&province).unwrap().buildings;
    assert!(buildings.contains(&guild_hall));
    let effects_after = state.province_effects(&data, &province);
    assert!(
        effects_after.wealth.flat >= effects_before.wealth.flat + 5.0,
        "before={:?} after={:?}",
        effects_before.wealth,
        effects_after.wealth
    );
    assert!(
        effects_after.unrest.flat <= effects_before.unrest.flat - 5.0,
        "before={:?} after={:?}",
        effects_before.unrest,
        effects_after.unrest
    );
}

#[test]
fn cancel_build_refunds_half_the_cost() {
    let data = data();
    let mut state = france(&data, 111);
    let paris = prov("prov_ile_de_france");
    let france_id = fac("fac_france");
    let armoury = building("bld_armoury");
    let cost = data.buildings[&armoury].cost.money;

    state
        .submit_order(
            &data,
            Order::Build {
                province: paris.clone(),
                building: armoury,
            },
        )
        .unwrap();
    let treasury_after_build = state.faction_state(&france_id).unwrap().treasury;
    state
        .submit_order(
            &data,
            Order::CancelBuild {
                province: paris.clone(),
            },
        )
        .unwrap();
    let treasury_after_cancel = state.faction_state(&france_id).unwrap().treasury;
    assert_eq!(
        treasury_after_cancel,
        treasury_after_build + i64::from(cost / 2)
    );
    assert!(state.province_state(&paris).unwrap().construction.is_none());
}

// ----- taxes (spec § 1.4) ---------------------------------------------------

#[test]
fn high_tax_raises_income_and_unrest_over_normal() {
    let data = data();
    let france_id = fac("fac_france");
    let mut normal = france(&data, 112);
    let mut high = france(&data, 112);
    high.submit_order(
        &data,
        Order::SetTaxRate {
            rate: TaxRate::High,
        },
    )
    .unwrap();

    for _ in 0..3 {
        normal.end_turn_with(&data, idle);
        high.end_turn_with(&data, idle);
    }

    let income_normal = normal.faction_state(&france_id).unwrap().income_last_turn;
    let income_high = high.faction_state(&france_id).unwrap().income_last_turn;
    assert!(
        income_high as f64 > income_normal as f64 * 1.25,
        "normal={income_normal} high={income_high}"
    );

    let unrest_normal = average_unrest(&normal, &france_id);
    let unrest_high = average_unrest(&high, &france_id);
    assert!(
        unrest_high > unrest_normal,
        "normal={unrest_normal} high={unrest_high}"
    );
}

#[test]
fn low_tax_lowers_income_versus_normal() {
    let data = data();
    let france_id = fac("fac_france");
    let mut normal = france(&data, 113);
    let mut low = france(&data, 113);
    low.submit_order(&data, Order::SetTaxRate { rate: TaxRate::Low })
        .unwrap();
    normal.end_turn_with(&data, idle);
    low.end_turn_with(&data, idle);
    let income_normal = normal.faction_state(&france_id).unwrap().income_last_turn;
    let income_low = low.faction_state(&france_id).unwrap().income_last_turn;
    assert!(
        income_low < income_normal,
        "normal={income_normal} low={income_low}"
    );
}

fn average_unrest(state: &CampaignState, faction: &FactionId) -> f64 {
    let values: Vec<f64> = state
        .provinces
        .values()
        .filter(|p| &p.controller == faction)
        .map(|p| weighted_unrest(&p.population))
        .collect();
    values.iter().sum::<f64>() / values.len() as f64
}

// ----- persistence (spec § 1.5/1.6) -----------------------------------------

#[test]
fn save_json_round_trips_the_new_m3_fields() {
    let data = data();
    let mut state = france(&data, 114);
    let paris = prov("prov_ile_de_france");
    state
        .submit_order(
            &data,
            Order::Build {
                province: paris.clone(),
                building: building("bld_armoury"),
            },
        )
        .unwrap();
    state
        .submit_order(
            &data,
            Order::SetTaxRate {
                rate: TaxRate::High,
            },
        )
        .unwrap();
    state.end_turn_with(&data, idle);

    let json = state.save_json();
    let loaded = CampaignState::load_json(&json).unwrap();
    assert_eq!(loaded, state);
    assert_eq!(
        loaded.province_state(&paris).unwrap().construction,
        state.province_state(&paris).unwrap().construction
    );
    assert_eq!(
        loaded.faction_state(&fac("fac_france")).unwrap().tax_rate,
        TaxRate::High
    );
}

#[test]
fn load_json_refuses_a_version_1_save_with_a_clear_french_message() {
    let data = data();
    let state = france(&data, 115);
    let json = state
        .save_json()
        .replace("\"state_version\":2", "\"state_version\":1");
    let err = CampaignState::load_json(&json).unwrap_err();
    assert!(matches!(
        err,
        sim_campaign::CampaignError::VersionMismatch {
            found: 1,
            expected: 2
        }
    ));
    let message = err.to_string();
    assert!(
        message.contains("version") && message.contains("prise en charge"),
        "message should be a clear French explanation: {message}"
    );
}
