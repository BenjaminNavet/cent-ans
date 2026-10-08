//! Lot TW2-T2 (ADR 0102): seasonal replenishment of field armies and
//! recruitment pools (spec `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T2).
use data_model::test_support::{fac, game_data};
use sim_campaign::test_support::{capital_city, main_army};

use data_model::{GameData, SettlementId, SettlementKind, UnitTypeId};
use sim_campaign::replenish::{resolve_replenishment, Territory};
use sim_campaign::{
    recruit_pool::resolve_recruit_pools, ArmyId, ArmyPosition, CampaignState, Order, OrderError,
    Season, Stance,
};

/// France played by the player, its main army in Paris with every unit at
/// half strength, a full treasury, spring.
fn setup() -> (&'static GameData, CampaignState, ArmyId) {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 3).unwrap();
    state.season = Season::Spring;
    let army = main_army(&state, "fac_france");
    let paris = capital_city(&state, "fac_france");
    let entry = state.armies.get_mut(&army).unwrap();
    entry.position = ArmyPosition::Settlement(paris);
    entry.stance = Stance::Normal;
    entry.supply = 100;
    for unit in &mut entry.units {
        unit.strength = unit.max_strength / 2;
    }
    state.factions.get_mut(&fac("fac_france")).unwrap().treasury = 100_000;
    (data, state, army)
}

/// An English-held city (hostile to France at the 1337 start).
fn english_city(state: &CampaignState) -> SettlementId {
    state
        .settlements
        .iter()
        .find(|(_, s)| s.controller == fac("fac_england") && s.kind == SettlementKind::City)
        .map(|(id, _)| id.clone())
        .expect("an English city")
}

#[test]
fn army_in_own_city_regains_men_and_pays() {
    let (data, mut state, army) = setup();
    assert_eq!(state.army_territory(data, &army), Territory::Own);
    let preview = state.army_replenishment(data, &army).unwrap();
    assert!(preview.blocked.is_none(), "{:?}", preview.blocked);
    assert!(preview.men > 0 && preview.cost > 0);
    assert!(preview.percent() >= 20, "own lands, in a city: {preview:?}");
    assert!(preview.percent() <= data.replenishment_rules.replenishment.max_percent);
    let before = state.armies[&army].total_strength();
    let treasury = state.factions[&fac("fac_france")].treasury;
    let mut events = Vec::new();
    resolve_replenishment(&mut state, data, &mut events);
    assert_eq!(state.armies[&army].total_strength(), before + preview.men);
    assert!(
        state.factions[&fac("fac_france")].treasury <= treasury - i64::from(preview.cost),
        "other French armies may also pay"
    );
    assert!(events
        .iter()
        .any(|e| e.text_fr.starts_with("Reconstitution")));
}

#[test]
fn hostile_lands_forced_march_and_battle_block_replenishment() {
    let (data, mut state, army) = setup();
    // Hostile lands: nothing.
    let english = english_city(&state);
    let point = data.settlement_point(&english).unwrap();
    state.armies.get_mut(&army).unwrap().position = ArmyPosition::field(point);
    assert_eq!(state.army_territory(data, &army), Territory::Hostile);
    let preview = state.army_replenishment(data, &army).unwrap();
    assert_eq!(preview.men, 0);
    assert_eq!(preview.blocked.as_deref(), Some("en terre ennemie"));

    // Forced march at home: nothing either.
    let (data, mut state, army) = setup();
    state.armies.get_mut(&army).unwrap().stance = Stance::ForcedMarch;
    let preview = state.army_replenishment(data, &army).unwrap();
    assert_eq!(preview.men, 0);

    // A battle this season: nothing.
    let (data, mut state, army) = setup();
    sim_campaign::replenish::mark_fought(&mut state, &army);
    let before = state.armies[&army].total_strength();
    resolve_replenishment(&mut state, data, &mut Vec::new());
    assert_eq!(state.armies[&army].total_strength(), before);
    assert_eq!(
        state
            .army_replenishment(data, &army)
            .unwrap()
            .blocked
            .as_deref(),
        Some("a combattu cette saison")
    );
}

#[test]
fn winter_and_field_slow_replenishment() {
    let (data, mut state, army) = setup();
    let in_city = state.army_replenishment(data, &army).unwrap().rate_bp;
    // In the field of the same province, normal stance: slower.
    let paris = capital_city(&state, "fac_france");
    let point = data.settlement_point(&paris).unwrap();
    state.armies.get_mut(&army).unwrap().position = ArmyPosition::field(point);
    let field = state.army_replenishment(data, &army).unwrap().rate_bp;
    assert!(field < in_city, "field {field} < city {in_city}");
    // An entrenched camp does better than marching.
    state.armies.get_mut(&army).unwrap().stance = Stance::Entrenched;
    let camp = state.army_replenishment(data, &army).unwrap().rate_bp;
    assert!(camp > field);
    state.armies.get_mut(&army).unwrap().stance = Stance::Normal;
    state.season = Season::Winter;
    let winter = state.army_replenishment(data, &army).unwrap();
    assert!(winter.rate_bp < field);
    assert!(winter.factors.iter().any(|f| f.label == "Hiver"));
}

#[test]
fn stewardship_of_the_general_raises_the_rate() {
    let (data, mut state, army) = setup();
    let general = state.armies[&army].general.clone().expect("French general");
    state
        .characters
        .get_mut(&general)
        .unwrap()
        .skills_learned
        .clear();
    state
        .characters
        .get_mut(&general)
        .unwrap()
        .skills
        .governance = 0;
    state.characters.get_mut(&general).unwrap().traits.clear();
    let plain = state.army_replenishment(data, &army).unwrap().rate_bp;
    let skill = data_model::SkillId::new("skill_logistique_militaire").unwrap();
    state
        .characters
        .get_mut(&general)
        .unwrap()
        .skills_learned
        .insert(skill);
    let steward = state.army_replenishment(data, &army).unwrap();
    assert!(steward.rate_bp > plain || steward.rate_bp == 100 * 40);
    assert!(steward
        .factors
        .iter()
        .any(|f| f.label == "Intendance du chef" && f.percent > 0));
}

#[test]
fn treasury_caps_the_men_regained_and_dead_units_stay_dead() {
    let (data, mut state, army) = setup();
    // Empty treasury: nothing.
    state.factions.get_mut(&fac("fac_france")).unwrap().treasury = 0;
    let preview = state.army_replenishment(data, &army).unwrap();
    assert_eq!(preview.men, 0);
    assert_eq!(preview.blocked.as_deref(), Some("trésor vide"));
    // Short treasury: part of it, never below zero.
    let full = {
        state.factions.get_mut(&fac("fac_france")).unwrap().treasury = 100_000;
        state.army_replenishment(data, &army).unwrap()
    };
    let short = i64::from(full.cost / 3).max(1);
    state.factions.get_mut(&fac("fac_france")).unwrap().treasury = short;
    let partial = state.army_replenishment(data, &army).unwrap();
    assert!(partial.men < full.men);
    assert!(i64::from(partial.cost) <= short);
    // A unit at zero is not brought back.
    state.factions.get_mut(&fac("fac_france")).unwrap().treasury = 100_000;
    state.armies.get_mut(&army).unwrap().units[0].strength = 0;
    let preview = state.army_replenishment(data, &army).unwrap();
    assert_eq!(preview.per_unit[0], 0);
}

fn first_recruitable(state: &CampaignState, data: &GameData, city: &SettlementId) -> UnitTypeId {
    state
        .recruitable(data, city)
        .into_iter()
        .find(|o| o.available)
        .map(|o| o.unit_type)
        .expect("something recruitable")
}

#[test]
fn recruitment_pools_start_full_are_drawn_and_refill() {
    let (data, mut state, _) = setup();
    let paris = capital_city(&state, "fac_france");
    let unit = first_recruitable(&state, data, &paris);
    let pool = state.recruit_pool(data, &paris, &unit);
    assert!(pool.cap >= 2, "capital city: {pool:?}");
    assert_eq!(pool.available, pool.cap);
    assert!(pool.seasons_to_next.is_none());
    // One recruit draws one unit.
    state
        .submit_order(
            data,
            Order::Recruit {
                settlement: paris.clone().into(),
                unit_type: unit.clone(),
            },
        )
        .unwrap();
    let drawn = state.recruit_pool(data, &paris, &unit);
    assert_eq!(drawn.available, pool.cap - 1);
    assert!(drawn.seasons_to_next.is_some());
    let option = state
        .recruitable(data, &paris)
        .into_iter()
        .find(|o| o.unit_type == unit)
        .unwrap();
    assert_eq!(option.pool, drawn);
    // An empty reserve refuses, with the delay.
    state
        .settlements
        .get_mut(&paris)
        .unwrap()
        .recruit_pool
        .insert(unit.clone(), 0);
    let error = state
        .submit_order(
            data,
            Order::Recruit {
                settlement: paris.clone().into(),
                unit_type: unit.clone(),
            },
        )
        .unwrap_err();
    match error {
        OrderError::RecruitUnavailable(reason) => {
            assert!(reason.starts_with("réserve épuisée"), "{reason}")
        }
        other => panic!("unexpected {other:?}"),
    }
    // The season refills it by its rate.
    let rate = state.recruit_pool_rate_milli(data, &paris, &unit);
    assert!(rate > 0);
    resolve_recruit_pools(&mut state, data);
    assert_eq!(state.recruit_pool(data, &paris, &unit).milli, rate);
    // Refilled to the cap, the entry leaves the map.
    for _ in 0..40 {
        resolve_recruit_pools(&mut state, data);
    }
    assert!(!state.settlements[&paris].recruit_pool.contains_key(&unit));
}

#[test]
fn ai_factions_are_bound_by_the_pools_too() {
    let (data, mut state, _) = setup();
    let london = capital_city(&state, "fac_england");
    state
        .factions
        .get_mut(&fac("fac_england"))
        .unwrap()
        .treasury = 100_000;
    let unit = first_recruitable(&state, data, &london);
    state
        .settlements
        .get_mut(&london)
        .unwrap()
        .recruit_pool
        .insert(unit.clone(), 999);
    let result = state.apply_order(
        data,
        &fac("fac_england"),
        Order::Recruit {
            settlement: london.clone().into(),
            unit_type: unit.clone(),
        },
    );
    assert!(matches!(result, Err(OrderError::RecruitUnavailable(_))));
    // The strategic planner never asks for it.
    let orders = ai_orders_without(&state, data, &unit, &london);
    assert!(orders, "the AI recruited from an empty reserve");
}

/// `true` when the built-in planner of `faction` does not recruit `unit` in
/// `site` (the minimal planner filters on `available`).
fn ai_orders_without(
    state: &CampaignState,
    data: &GameData,
    unit: &UnitTypeId,
    site: &SettlementId,
) -> bool {
    sim_campaign::ai_minimal::plan_turn(state, data, &fac("fac_england"))
        .into_iter()
        .all(|order| match order {
            Order::Recruit {
                settlement,
                unit_type,
            } => {
                !(&unit_type == unit && settlement == sim_campaign::Place::Settlement(site.clone()))
            }
            _ => true,
        })
}

#[test]
fn pools_and_battle_marks_survive_a_save() {
    let (data, mut state, army) = setup();
    let paris = capital_city(&state, "fac_france");
    let unit = first_recruitable(&state, data, &paris);
    state
        .settlements
        .get_mut(&paris)
        .unwrap()
        .recruit_pool
        .insert(unit.clone(), 1250);
    sim_campaign::replenish::mark_fought(&mut state, &army);
    let loaded = CampaignState::load_json(&state.save_json()).unwrap();
    assert_eq!(
        loaded.settlements[&paris].recruit_pool.get(&unit),
        Some(&1250)
    );
    assert_eq!(loaded.armies[&army].fought_turn, Some(state.turn));
}
