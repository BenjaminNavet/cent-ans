//! F1 « règles inertes » integration tests: building, technology and trait
//! effects that used to be displayed without effect, allied armies joining
//! battles, and the new chronicle effects (capture, ransom, delayed events).
//! See `docs/design/v2-finalisation.md` (lot F1).

use std::path::PathBuf;

use data_model::{BuildingId, FactionId, GameData, ProvinceId, TechnologyId, UnitTypeId};
use sim_campaign::{ArmyId, CampaignState, Order, Unit};

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

fn bld(id: &str) -> BuildingId {
    BuildingId::new(id).unwrap()
}

fn unit(id: &str) -> UnitTypeId {
    UnitTypeId::new(id).unwrap()
}

fn tech(id: &str) -> TechnologyId {
    TechnologyId::new(id).unwrap()
}

fn grant_tech(state: &mut CampaignState, faction: &str, id: &str) {
    state
        .factions
        .get_mut(&fac(faction))
        .unwrap()
        .technologies
        .insert(tech(id));
}

fn france(data: &GameData, seed: u64) -> CampaignState {
    CampaignState::new_1337(data, fac("fac_france"), seed).expect("1337 start")
}

/// France at spring 1337 without chronicle events (no random noise).
fn quiet_france(data: &GameData, seed: u64) -> CampaignState {
    let mut state = france(data, seed);
    state.chronicle.disabled = true;
    state
}

/// A planner that does nothing: only the orders the test submits apply.
fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

fn first_army_of(state: &CampaignState, faction: &str) -> ArmyId {
    state
        .armies
        .iter()
        .find(|(_, a)| a.faction == fac(faction))
        .map(|(id, _)| id.clone())
        .expect("an army")
}

// =========================================================================
// 1. Buildings: Garrison, RecruitCost, Supply, class targeting
// =========================================================================

#[test]
fn class_targeted_building_effects_reach_only_their_class() {
    let data = data();
    let province = prov("prov_ile_de_france");
    let mut with = quiet_france(&data, 7);
    let mut without = with.clone();
    with.provinces
        .get_mut(&province)
        .unwrap()
        .buildings
        .push(bld("bld_guild_hall"));
    without
        .provinces
        .get_mut(&province)
        .unwrap()
        .buildings
        .retain(|b| b != &bld("bld_guild_hall"));
    for _ in 0..4 {
        with.end_turn_with(&data, idle);
        without.end_turn_with(&data, idle);
    }
    let a = &with.provinces[&province].population;
    let b = &without.provinces[&province].population;
    assert!(
        a.burghers.wealth > b.burghers.wealth,
        "guild hall: burghers richer ({} vs {})",
        a.burghers.wealth,
        b.burghers.wealth
    );
    assert_eq!(a.peasants.wealth, b.peasants.wealth, "peasants untouched");
    assert_eq!(a.nobility.wealth, b.nobility.wealth, "nobility untouched");
}

#[test]
fn garrison_effect_makes_garrison_units_free() {
    let data = data();
    let mut state = quiet_france(&data, 3);
    let france_id = fac("fac_france");
    let province = prov("prov_ile_de_france");
    let walls = ["bld_palisade", "bld_stone_walls", "bld_castle"].map(bld);
    let p = state.provinces.get_mut(&province).unwrap();
    assert!(!p.garrison.is_empty());
    p.buildings.retain(|b| !walls.contains(b));
    assert_eq!(state.province_effects(&data, &province).garrison.flat, 0.0);
    let before = state.faction_upkeep(&data, &france_id);
    state
        .provinces
        .get_mut(&province)
        .unwrap()
        .buildings
        .push(bld("bld_palisade"));
    let after = state.faction_upkeep(&data, &france_id);
    assert!(
        after < before,
        "palisade: one free unit ({before} -> {after})"
    );
}

#[test]
fn garrison_effect_reinforces_a_depleted_garrison() {
    let data = data();
    let mut state = quiet_france(&data, 4);
    let province = prov("prov_ile_de_france");
    let p = state.provinces.get_mut(&province).unwrap();
    assert!(!p.garrison.is_empty());
    p.buildings.push(bld("bld_castle"));
    for unit in &mut p.garrison {
        unit.strength = unit.max_strength / 2;
    }
    let before: u32 = p.garrison.iter().map(|u| u.strength).sum();
    state.end_turn_with(&data, idle);
    let after: u32 = state.provinces[&province]
        .garrison
        .iter()
        .map(|u| u.strength)
        .sum();
    assert!(after > before, "castle levies: {before} -> {after}");
}

#[test]
fn recruit_cost_effects_target_their_unit_family() {
    let data = data();
    let mut state = quiet_france(&data, 5);
    let france_id = fac("fac_france");
    let province = prov("prov_ile_de_france");
    let cost = |state: &CampaignState, id: &str| {
        state
            .recruit_option(&data, &france_id, &province, &unit(id))
            .unwrap()
            .cost
    };
    let knights = cost(&state, "unit_knights");
    let militia = cost(&state, "unit_urban_militia");
    state
        .provinces
        .get_mut(&province)
        .unwrap()
        .buildings
        .push(bld("bld_stables"));
    // −10 % of the base price (percent effects add up).
    let base = data.unit_types[&unit("unit_knights")].cost.money;
    assert_eq!(cost(&state, "unit_knights"), knights - base / 10);
    assert_eq!(cost(&state, "unit_urban_militia"), militia);
    // The treasury pays the discounted price.
    let treasury = state.factions[&france_id].treasury;
    state
        .submit_order(
            &data,
            Order::Recruit {
                province: province.clone(),
                unit_type: unit("unit_knights"),
            },
        )
        .unwrap();
    assert_eq!(
        treasury - state.factions[&france_id].treasury,
        i64::from(cost(&state, "unit_knights"))
    );
}

#[test]
fn supply_buildings_speed_up_recovery_in_the_province() {
    let data = data();
    let base = quiet_france(&data, 6);
    let army = first_army_of(&base, "fac_france");
    let location = base.armies[&army].location.clone();
    let run = |port: bool| {
        let mut state = base.clone();
        state.armies.get_mut(&army).unwrap().supply = 10;
        let p = state.provinces.get_mut(&location).unwrap();
        p.buildings.retain(|b| b != &bld("bld_port"));
        if port {
            p.buildings.push(bld("bld_port"));
        }
        state.end_turn_with(&data, idle);
        state.armies[&army].supply
    };
    assert_eq!(run(true), run(false) + 10);
}

// =========================================================================
// 2. Technologies
// =========================================================================

#[test]
fn army_upkeep_technology_lowers_the_bill() {
    let data = data();
    let mut state = quiet_france(&data, 11);
    let france_id = fac("fac_france");
    let before = state.faction_upkeep(&data, &france_id);
    grant_tech(&mut state, "fac_france", "tech_standing_companies");
    let after = state.faction_upkeep(&data, &france_id);
    let expected = before as f64 * 0.9;
    assert!(
        (after as f64 - expected).abs() <= before as f64 * 0.01 + 5.0,
        "standing companies -10 %: {before} -> {after}"
    );
}

#[test]
fn army_experience_technology_trains_recruits() {
    let data = data();
    let mut state = quiet_france(&data, 12);
    grant_tech(&mut state, "fac_france", "tech_standing_companies");
    let province = prov("prov_ile_de_france");
    state.factions.get_mut(&fac("fac_france")).unwrap().treasury = 100_000;
    let garrison_before = state.provinces[&province].garrison.len();
    state
        .submit_order(
            &data,
            Order::Recruit {
                province: province.clone(),
                unit_type: unit("unit_urban_militia"),
            },
        )
        .unwrap();
    state.end_turn_with(&data, idle);
    let recruit = &state.provinces[&province].garrison[garrison_before];
    assert_eq!(recruit.unit_type, unit("unit_urban_militia"));
    assert!(recruit.experience >= 2, "experience {}", recruit.experience);
}

#[test]
fn recruit_cost_technology_targets_its_family() {
    let data = data();
    let mut state = quiet_france(&data, 13);
    let france_id = fac("fac_france");
    let province = prov("prov_ile_de_france");
    let cost = |state: &CampaignState, id: &str| {
        state
            .recruit_option(&data, &france_id, &province, &unit(id))
            .unwrap()
            .cost
    };
    let crossbow = cost(&state, "unit_crossbowmen");
    let knights = cost(&state, "unit_knights");
    grant_tech(&mut state, "fac_france", "tech_francs_archers");
    let base = data.unit_types[&unit("unit_crossbowmen")].cost.money;
    assert_eq!(cost(&state, "unit_crossbowmen"), crossbow - base / 10);
    assert_eq!(cost(&state, "unit_knights"), knights);
}

#[test]
fn siege_trains_slow_armies_until_field_artillery() {
    let data = data();
    let mut state = quiet_france(&data, 14);
    let army_id = first_army_of(&state, "fac_france");
    let trebuchet = Unit::fresh(&data.unit_types[&unit("unit_trebuchet")]);
    let mut army = state.armies[&army_id].clone();
    army.general = None;
    let plain = state.army_movement_allowance(&data, &army);
    assert_eq!(plain, state.season().movement_points());
    army.units.push(trebuchet);
    assert_eq!(state.army_movement_allowance(&data, &army), plain - 1);
    grant_tech(&mut state, "fac_france", "tech_field_artillery");
    assert_eq!(state.army_movement_allowance(&data, &army), plain);
}

#[test]
fn production_buildings_raise_income() {
    let data = data();
    let mut state = quiet_france(&data, 15);
    let france_id = fac("fac_france");
    let province = prov("prov_ile_de_france");
    state
        .provinces
        .get_mut(&province)
        .unwrap()
        .buildings
        .retain(|b| b != &bld("bld_weaving_workshop"));
    let before = state.faction_income_effective(&data, &france_id);
    state
        .provinces
        .get_mut(&province)
        .unwrap()
        .buildings
        .push(bld("bld_weaving_workshop"));
    let after = state.faction_income_effective(&data, &france_id);
    assert!(
        after > before,
        "weaving workshop +15 % production: {before} -> {after}"
    );
}

#[test]
fn siege_resistance_and_masonry_harden_walled_towns() {
    let data = data();
    let mut state = quiet_france(&data, 16);
    let province = prov("prov_ile_de_france");
    let england = fac("fac_england");
    state
        .factions
        .get_mut(&fac("fac_france"))
        .unwrap()
        .technologies
        .remove(&tech("tech_masonry"));
    let p = state.provinces.get_mut(&province).unwrap();
    p.buildings.retain(|b| {
        !["bld_palisade", "bld_stone_walls", "bld_castle"]
            .map(bld)
            .contains(b)
    });
    let open_level = state.fortification_level(&data, &province);
    let open_resistance = state.siege_resistance(&data, &province, &england);
    state
        .provinces
        .get_mut(&province)
        .unwrap()
        .buildings
        .push(bld("bld_stone_walls"));
    let walled = state.fortification_level(&data, &province);
    let walled_resistance = state.siege_resistance(&data, &province, &england);
    assert!(walled_resistance >= open_resistance + 10.0);
    // English siege engineering eats into it.
    grant_tech(&mut state, "fac_england", "tech_siege_engineering");
    assert!(state.siege_resistance(&data, &province, &england) < walled_resistance);
    // French masonry adds a level to walled towns only.
    grant_tech(&mut state, "fac_france", "tech_masonry");
    assert_eq!(state.fortification_level(&data, &province), walled + 1);
    if open_level == 0 {
        let p = state.provinces.get_mut(&province).unwrap();
        p.buildings.retain(|b| b != &bld("bld_stone_walls"));
        assert_eq!(state.fortification_level(&data, &province), 0);
    }
}

#[test]
fn class_targeted_wealth_technology_enriches_peasants() {
    let data = data();
    let mut with = quiet_france(&data, 17);
    with.factions
        .get_mut(&fac("fac_france"))
        .unwrap()
        .technologies
        .remove(&tech("tech_three_field_rotation"));
    let mut without = with.clone();
    grant_tech(&mut with, "fac_france", "tech_three_field_rotation");
    for _ in 0..4 {
        with.end_turn_with(&data, idle);
        without.end_turn_with(&data, idle);
    }
    let province = prov("prov_ile_de_france");
    let a = &with.provinces[&province].population;
    let b = &without.provinces[&province].population;
    assert!(a.peasants.wealth > b.peasants.wealth);
    assert_eq!(a.burghers.wealth, b.burghers.wealth);
}

#[test]
fn prestige_technology_feeds_the_ruler_every_year() {
    let data = data();
    let mut state = quiet_france(&data, 18);
    let france_id = fac("fac_france");
    let before = sim_campaign::dynasty::yearly_court_prestige(&state, &data, &france_id);
    grant_tech(&mut state, "fac_france", "tech_printing_press");
    let after = sim_campaign::dynasty::yearly_court_prestige(&state, &data, &france_id);
    assert_eq!(after, before + 1, "printing press: +5 / 5 a year");
    // A full year: the ruler actually gains it (other sources aside).
    let ruler = state.factions[&france_id].ruler.clone().unwrap();
    let start = state.characters[&ruler].prestige;
    for _ in 0..4 {
        state.end_turn_with(&data, idle);
    }
    assert!(state.characters[&ruler].prestige >= start + after.min(0) + after);
}

#[test]
fn research_surplus_carries_over_to_the_next_technology() {
    let data = data();
    let mut state = quiet_france(&data, 19);
    let france_id = fac("fac_france");
    let available: Vec<TechnologyId> = data
        .technologies
        .values()
        .filter(|t| {
            sim_campaign::research::tech_status(&state, &france_id, t)
                == sim_campaign::TechStatus::Available
        })
        .map(|t| t.id.clone())
        .collect();
    assert!(available.len() >= 2);
    let (first, second) = (available[0].clone(), available[1].clone());
    state
        .submit_order(
            &data,
            Order::Research {
                technology: first.clone(),
            },
        )
        .unwrap();
    let cost = sim_campaign::research::effective_cost(&data.technologies[&first], state.year());
    state
        .factions
        .get_mut(&france_id)
        .unwrap()
        .research_progress = cost - 1;
    let points = state.research_points_per_turn(&data, &france_id);
    state.end_turn_with(&data, idle);
    let f = &state.factions[&france_id];
    assert!(f.technologies.contains(&first));
    assert!(f.research.is_none());
    assert_eq!(f.research_progress, points - 1);
    state
        .submit_order(
            &data,
            Order::Research {
                technology: second.clone(),
            },
        )
        .unwrap();
    assert_eq!(
        sim_campaign::research::tech_progress(&state, &france_id, &second),
        points - 1
    );
}
