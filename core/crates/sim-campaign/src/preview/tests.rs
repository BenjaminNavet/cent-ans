//! IB5: previews checked against the real application of the effect (the
//! construction phase, the research insertion) on the 1337 campaign.

use data_model::{BuildingId, FactionId, GameData, SettlementId, TechnologyId, UnitTypeId};

use super::*;
use crate::state::{CampaignState, Construction};

use data_model::test_support::game_data;

fn france() -> FactionId {
    FactionId::new("fac_france").unwrap()
}

fn campaign(data: &GameData) -> CampaignState {
    CampaignState::new_1337(data, france(), 7).expect("campaign")
}

/// A settlement held by France without `building` (nor what it upgrades
/// into), where it is allowed.
fn place_without(state: &CampaignState, data: &GameData, building: &BuildingId) -> SettlementId {
    let definition = &data.buildings[building];
    state
        .settlements
        .iter()
        .find(|(id, s)| {
            s.controller == france()
                && s.owner == france()
                && state.province_city_id(&s.province) == Some(*id)
                && definition.allowed_in(s.kind)
                && !data.has_building(&s.buildings, building)
                && definition
                    .upgrades_from
                    .as_ref()
                    .is_none_or(|from| s.buildings.contains(from))
        })
        .map(|(id, _)| id.clone())
        .expect("a French city without the building")
}

#[test]
fn effect_keys_carry_the_target() {
    let data = game_data();
    let fair = &data.buildings[&BuildingId::new("bld_fair").unwrap()];
    let keys: Vec<String> = fair.effects.iter().map(effect_key).collect();
    assert!(keys.contains(&"trade_income".to_owned()));
    assert!(keys.contains(&"wealth:burghers".to_owned()));
    assert!(keys.contains(&"unrest:burghers".to_owned()));
}

#[test]
fn building_preview_matches_the_completed_construction() {
    let data = game_data();
    let state = campaign(data);
    // Tin blowing house: tax income +5 %; Hôtel-Dieu: health +8.
    let tin = BuildingId::new("bld_tin_blowing_house").unwrap();
    let place = place_without(&state, data, &tin);
    let income = state.building_before_after(data, &place, &tin)["tax_income"];
    assert!(income[1] > income[0], "{income:?}");
    let built = completed(&state, data, &place, &tin);
    let province = state.settlements[&place].province.clone();
    let read = |s: &CampaignState| {
        let tax_rate = s.factions[&france()].tax_rate;
        let tech = crate::research::faction_province_tech_effects(s, data, &france());
        s.province_gross_income(data, &province, &france(), tax_rate, &tech) as f64
    };
    assert_eq!(income, [read(&state), read(&built)]);

    let hospital = BuildingId::new("bld_hotel_dieu").unwrap();
    let place = place_without(&state, data, &hospital);
    let preview = state.building_before_after(data, &place, &hospital);
    let health = preview["health"];
    assert!(health[1] > health[0], "{health:?}");
    assert!(preview.contains_key("unrest:peasants"), "{preview:?}");
    let built = completed(&state, data, &place, &hospital);
    let province = state.settlements[&place].province.clone();
    let health_of = |s: &CampaignState| {
        let classes = crate::population::equilibrium(s, data, &province).unwrap();
        let total: f64 = classes.iter().map(|(_, n, _)| *n as f64).sum();
        let sum: f64 = classes.iter().map(|(_, n, t)| *n as f64 * t.health).sum();
        (sum / total * 10.0).round() / 10.0
    };
    assert_eq!(health, [health_of(&state), health_of(&built)]);
    // Nothing is left behind in the real state.
    assert!(!state.settlements[&place].buildings.contains(&hospital));
}

/// `state` after the construction phase completes `building` in `place`.
fn completed(
    state: &CampaignState,
    data: &GameData,
    place: &SettlementId,
    building: &BuildingId,
) -> CampaignState {
    let mut built = state.clone();
    built.settlements.get_mut(place).unwrap().construction = Some(Construction {
        building: building.clone(),
        turns_left: 1,
        paid: 0,
        drawn: Default::default(),
    });
    crate::buildings::resolve_construction(&mut built, data, &mut Vec::new());
    assert!(built.settlements[place].buildings.contains(building));
    built
}

#[test]
fn equilibrium_is_the_fixed_point_of_the_seasonal_update() {
    let data = game_data();
    let mut state = campaign(data);
    let capital = state.factions[&france()].capital.clone();
    let targets = crate::population::equilibrium(&state, data, &capital).unwrap();
    {
        let population = &mut state.provinces.get_mut(&capital).unwrap().population;
        for (class, _, t) in &targets {
            let entry = match class {
                SocialClass::Peasants => &mut population.peasants,
                SocialClass::Burghers => &mut population.burghers,
                SocialClass::Clergy => &mut population.clergy,
                SocialClass::Nobility => &mut population.nobility,
            };
            entry.health = t.health.round() as u8;
            entry.wealth = t.wealth.round() as u8;
            entry.goods_satisfaction = t.goods_satisfaction.round() as u8;
            entry.unrest = t.unrest.round() as u8;
        }
    }
    let before = state.provinces[&capital].population.clone();
    crate::population::resolve_population(&mut state, data, &mut Vec::new());
    let after = &state.provinces[&capital].population;
    for class in SocialClass::ALL {
        let (b, a) = (before.get(class), after.get(class));
        for (x, y) in [
            (b.health, a.health),
            (b.wealth, a.wealth),
            (b.goods_satisfaction, a.goods_satisfaction),
            (b.unrest, a.unrest),
        ] {
            assert!(x.abs_diff(y) <= 1, "{class:?}: {b:?} → {a:?}");
        }
    }
}

#[test]
fn class_targeted_and_upgrade_effects() {
    let data = game_data();
    let state = campaign(data);
    // Guild hall upgrades the market: burghers' wealth and unrest move.
    let hall = BuildingId::new("bld_guild_hall").unwrap();
    let place = place_without(&state, data, &hall);
    let preview = state.building_before_after(data, &place, &hall);
    let unrest = preview.get("unrest:burghers").expect("burghers' unrest");
    assert!(unrest[1] <= unrest[0], "{unrest:?}");
    let wealth = preview.get("wealth:burghers").expect("burghers' wealth");
    assert!(wealth[1] >= wealth[0], "{wealth:?}");
    // Effects with no exact statistic here: none.
    assert!(!preview.contains_key("research_points:burghers"));
    // Already built: nothing to preview.
    let market = BuildingId::new("bld_market").unwrap();
    assert!(state
        .building_before_after(data, &place, &market)
        .is_empty());
}

#[test]
fn walls_and_recruitment_places() {
    let data = game_data();
    let state = campaign(data);
    let field = BuildingId::new("bld_muster_field").unwrap();
    let place = place_without(&state, data, &field);
    let slots = state.building_before_after(data, &place, &field)["recruit_slots"];
    assert_eq!(slots[1], slots[0] + 1.0);
}

#[test]
fn technology_preview_matches_the_acquired_technology() {
    let data = game_data();
    let state = campaign(data);
    let tech = TechnologyId::new("tech_hanseatic_trade").unwrap();
    assert!(!state.factions[&france()].technologies.contains(&tech));
    let preview = state.technology_before_after(data, &france(), &tech);
    let income = preview.get("trade_income").expect("income before/after");
    let mut learned = state.clone();
    learned
        .factions
        .get_mut(&france())
        .unwrap()
        .technologies
        .insert(tech.clone());
    assert_eq!(
        income[0],
        state.faction_income_effective(data, &france()) as f64
    );
    assert_eq!(
        income[1],
        learned.faction_income_effective(data, &france()) as f64
    );
    assert!(income[1] > income[0], "{income:?}");

    let hospital = TechnologyId::new("tech_hospital_reform").unwrap();
    if !state.factions[&france()].technologies.contains(&hospital) {
        let health = state.technology_before_after(data, &france(), &hospital)["health"];
        assert!(health[1] > health[0], "{health:?}");
    }
}

#[test]
fn requirements_state() {
    let data = game_data();
    let state = campaign(data);
    // Collegiate church: needs the parish church it upgrades.
    let collegiate = BuildingId::new("bld_collegiate_church").unwrap();
    let parish = BuildingId::new("bld_parish_church").unwrap();
    let with = state
        .settlements
        .iter()
        .find(|(_, s)| s.controller == france() && s.buildings.contains(&parish))
        .map(|(id, _)| id.clone())
        .expect("a French place with a parish church");
    let without = state
        .settlements
        .iter()
        .find(|(_, s)| s.controller == france() && !data.has_building(&s.buildings, &parish))
        .map(|(id, _)| id.clone())
        .expect("a French place without a parish church");
    let met = |place: &SettlementId| state.building_requirements(data, place, &collegiate);
    assert_eq!(
        met(&with),
        vec![Requirement::new("bld_parish_church", true)]
    );
    assert_eq!(
        met(&without),
        vec![Requirement::new("bld_parish_church", false)]
    );

    // Technology prerequisites.
    let aqua = TechnologyId::new("tech_aqua_vitae").unwrap();
    let prereqs = state.technology_requirements(data, &france(), &aqua);
    assert_eq!(prereqs.len(), 2);
    for requirement in &prereqs {
        let known = state.factions[&france()]
            .technologies
            .iter()
            .any(|t| t.as_str() == requirement.id);
        assert_eq!(requirement.met, known);
    }

    // Recruitment: a unit listed in some building's `enables_units`.
    let (unit, _) = data
        .unit_types
        .iter()
        .find(|(id, _)| {
            crate::buildings::enabling_buildings(data, id)
                .next()
                .is_some()
        })
        .expect("a unit needing a building");
    let unit: &UnitTypeId = unit;
    let rows = state.recruit_requirements(data, &with, unit);
    let enabling = rows
        .iter()
        .find(|r| r.id == "enabling_building")
        .expect("enabling building row");
    let present = crate::buildings::enabling_buildings(data, unit)
        .any(|b| data.has_building(&state.settlements[&with].buildings, &b.id));
    assert_eq!(enabling.met, present);
}
