//! Lot JR1 (ADR 0165): the crusader faction led by the campaign AI.
use data_model::test_support::game_data;

use data_model::{FactionId, GameData};
use sim_campaign::{CampaignState, EventKind, Order};

fn data() -> &'static GameData {
    ai::feudal::install();
    game_data()
}

#[test]
fn the_planner_preaches_as_soon_as_it_can() {
    let data = data();
    let faction = data.crusade_rules.as_ref().expect("rules").faction.clone();
    let state = CampaignState::new_1337(data, FactionId::new("fac_france").unwrap(), 1)
        .expect("1337 start");
    let orders = ai::plan_turn(&state, data, &faction);
    assert!(orders.contains(&Order::PreachPassage), "{orders:?}");
    // No other faction ever does.
    let cyprus = FactionId::new("fac_cyprus").unwrap();
    assert!(!ai::plan_turn(&state, data, &cyprus).contains(&Order::PreachPassage));
}

#[test]
fn forty_turns_of_the_real_ai_keep_the_crusade_alive() {
    let data = data();
    let rules = data.crusade_rules.as_ref().expect("rules");
    let faction = rules.faction.clone();
    let mut state = CampaignState::new_1337(data, FactionId::new("fac_france").unwrap(), 11)
        .expect("1337 start");
    state.interactive_battles = false;
    let mut landings = 0;
    for turn in 0..40 {
        let events = state.end_turn_with(data, ai::plan_turn);
        landings += events
            .iter()
            .filter(|e| e.kind == EventKind::Crusade && e.text_fr.contains("débarque"))
            .count();
        if turn % 10 == 9 {
            let crusade = state.crusade.as_ref().expect("crusade kept");
            let f = &state.factions[&faction];
            let units: usize = state
                .armies
                .values()
                .filter(|a| a.faction == faction)
                .map(|a| a.units.len())
                .sum();
            println!(
                "turn {}: fervour {}, treasury {}, income {}, upkeep {}, field units {units}, \
                 target taken {}",
                turn + 1,
                crusade.fervor,
                f.treasury,
                f.income_last_turn,
                f.upkeep_last_turn,
                crusade.target_taken
            );
        }
    }
    assert!(
        state.factions[&faction].alive,
        "the crusaders are still there"
    );
    assert!(landings >= 2, "contingents landed: {landings}");
    // Landed volunteers do not rot in the base's garrison: the host musters
    // them (a cityless faction raises armies from the places it holds).
    let base = &state.settlements[&rules.base_settlement];
    if base.controller == faction {
        assert!(base.garrison.len() <= 6, "garrison {}", base.garrison.len());
    }
}

#[test]
fn a_cityless_realm_never_empties_its_last_place() {
    // JR5 (ADR 0165 § Ajouts): the cityless muster rule applies to any
    // faction reduced to towns and castles, not only the crusade; it keeps
    // the starting garrison of each place's kind and marches the rest out.
    let data = data();
    let mut state = CampaignState::new_1337(data, FactionId::new("fac_france").unwrap(), 3)
        .expect("1337 start");
    let realm = FactionId::new("fac_brittany").unwrap();
    let france = FactionId::new("fac_france").unwrap();
    // Brittany loses every city; its other places stay, heavily garrisoned.
    let militia = &data.unit_types[&data_model::UnitTypeId::new("unit_urban_militia").unwrap()];
    let mut kept = Vec::new();
    let ids: Vec<_> = state.settlements.keys().cloned().collect();
    for id in ids {
        let is_city = state
            .settlement_province(&id)
            .and_then(|p| state.province_city_id(p))
            == Some(&id);
        let place = state.settlements.get_mut(&id).unwrap();
        if place.controller != realm {
            continue;
        }
        if is_city {
            place.controller = france.clone();
            place.owner = france.clone();
        } else if place.kind != data_model::SettlementKind::Village {
            while place.garrison.len() < 3 {
                place.garrison.push(sim_campaign::Unit::fresh(militia));
            }
            kept.push(id);
        }
    }
    assert!(!kept.is_empty(), "Brittany keeps towns or castles");
    state.armies.retain(|_, a| a.faction != realm);
    let orders = ai::plan_turn(&state, data, &realm);
    let musters = orders
        .iter()
        .filter(|o| matches!(o, Order::CreateArmy { .. }))
        .count();
    assert!(musters > 0, "the surplus marches out: {orders:?}");
    for order in &orders {
        if let Order::CreateArmy {
            settlement: sim_campaign::Place::Settlement(id),
            units_from_garrison,
            ..
        } = order
        {
            let garrison = state.settlements[id].garrison.len();
            assert!(
                units_from_garrison.len() < garrison,
                "{id}: {units_from_garrison:?} of {garrison}"
            );
        }
    }
}
