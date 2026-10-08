//! Lot C4 (core on settlements): income split, derived control, the
//! full-province bonus, village capture, castle sieges, pathfinding on the
//! fallback and on a fixture graph, determinism.
//! See `docs/design/2026-09-24-echelle-colonies.md` § 4 and § 7 (the v4
//! save refusal is tested in `campaign.rs`).

use data_model::{FactionId, GameData, ProvinceId, SettlementEdge, SettlementId, SettlementKind};
use sim_campaign::{ArmyId, CampaignState, Order, Stance};

use data_model::test_support::{fac, game_data, prov};

/// Cheapest path of `army` to `target` on the settlement graph (the
/// skeleton of the AI's planning since lot M2).
fn graph_path(
    state: &CampaignState,
    data: &GameData,
    army: &ArmyId,
    target: &SettlementId,
) -> Option<Vec<SettlementId>> {
    let entry = &state.armies[army];
    let from = entry.settlement()?;
    let table = sim_campaign::movement::dijkstra(state, data, &entry.faction, from, None, None);
    sim_campaign::movement::path_to(&table, target)
}

fn start(data: &GameData, player: &str) -> CampaignState {
    CampaignState::new_1337(data, fac(player), 7).expect("1337 start")
}

/// First non-city settlement of `province` of the given kind.
fn settlement_of_kind(
    state: &CampaignState,
    province: &ProvinceId,
    kind: SettlementKind,
) -> Option<SettlementId> {
    state.provinces[province]
        .settlements
        .iter()
        .find(|id| state.settlements[*id].kind == kind)
        .cloned()
}

/// First army of `faction`, by id.
fn first_army(state: &CampaignState, faction: &FactionId) -> ArmyId {
    state
        .armies
        .iter()
        .find(|(_, a)| &a.faction == faction)
        .map(|(id, _)| id.clone())
        .expect("the faction has an army")
}

/// Ends the turn without any AI order.
fn quiet_turn(state: &mut CampaignState, data: &GameData) {
    state.end_turn_with(data, |_, _, _| Vec::new());
}

/// A French settlement of `kind` whose node has a neighbour free of armies,
/// with that neighbour: where to stage an English attack.
fn staging_target(
    state: &CampaignState,
    data: &GameData,
    kind: SettlementKind,
    need_garrison: bool,
) -> (SettlementId, SettlementId) {
    let france = fac("fac_france");
    for (id, settlement) in &state.settlements {
        if settlement.kind != kind
            || settlement.controller != france
            || settlement.garrison.is_empty() == need_garrison
            || !state.armies_at(id).is_empty()
        {
            continue;
        }
        let staging = sim_campaign::movement::edges(data, id)
            .into_iter()
            .map(|(to, _)| to)
            .find(|to| {
                state.armies_at(to).is_empty() && !data.movement_graph.edge(id, to).unwrap().sea
            });
        if let Some(staging) = staging {
            return (id.clone(), staging);
        }
    }
    panic!("no French {kind:?} to stage an attack on");
}

#[test]
fn weights_split_the_province_income_between_controllers() {
    let data = game_data();
    let mut state = start(data, "fac_france");
    let province = prov("prov_ile_de_france");
    // The weight shares of a province add up to one.
    let total: f64 = state.provinces[&province]
        .settlements
        .iter()
        .map(|s| sim_campaign::settlements::weight_share(data, s))
        .sum();
    assert!((total - 1.0).abs() < 1e-9, "shares sum to {total}");

    let town = settlement_of_kind(&state, &province, SettlementKind::Town)
        .or_else(|| settlement_of_kind(&state, &province, SettlementKind::Village))
        .expect("Île-de-France has a secondary settlement");
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    let france_before = state.faction_income(data, &france);
    let england_before = state.faction_income(data, &england);
    state.settlements.get_mut(&town).unwrap().controller = england.clone();
    let france_after = state.faction_income(data, &france);
    let england_after = state.faction_income(data, &england);

    let tax_rate = state.factions[&england].tax_rate;
    let tech = sim_campaign::research::faction_province_tech_effects(&state, data, &england);
    // Embargoes (M5) cut the whole income, the town's share included.
    let share =
        state.settlement_tax(data, &town, tax_rate, &tech) * state.embargo_income_factor(&england);
    assert!(share > 0.0);
    let gain = england_after - england_before;
    assert!(
        (gain as f64 - share).abs() <= 2.0,
        "England gains the town's share: {gain} vs {share}"
    );
    assert!(
        france_after < france_before,
        "France loses the town's share"
    );
}

#[test]
fn control_of_the_province_follows_its_city() {
    let data = game_data();
    let mut state = start(data, "fac_france");
    let province = prov("prov_normandie");
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    assert_eq!(state.province_controller(&province), Some(&france));
    // A secondary settlement changes hands: the province stays French.
    let other = state.provinces[&province]
        .settlements
        .iter()
        .find(|s| **s != state.provinces[&province].city)
        .cloned()
        .expect("Normandy has several settlements");
    state.settlements.get_mut(&other).unwrap().controller = england.clone();
    assert_eq!(state.province_controller(&province), Some(&france));
    assert!(state.controls_province(&france, &province));
    // The city falls: the province is English.
    let city = state.provinces[&province].city.clone();
    state.settlements.get_mut(&city).unwrap().controller = england.clone();
    assert_eq!(state.province_controller(&province), Some(&england));
    assert!(state.controls_province(&england, &province));
    assert_eq!(
        state.province_owner(&province),
        Some(&france),
        "ownership is not changed by occupation"
    );
}

#[test]
fn the_full_province_bonus_needs_every_settlement() {
    let data = game_data();
    let mut state = start(data, "fac_france");
    let province = prov("prov_ile_de_france");
    let france = fac("fac_france");
    let percent = data
        .settlement_rules
        .as_ref()
        .unwrap()
        .full_province_bonus
        .income_percent;
    assert!(percent > 0);
    let full = state.full_province_income_factor(data, &province, &france);
    assert!((full - (1.0 + f64::from(percent) / 100.0)).abs() < 1e-9);
    let village = settlement_of_kind(&state, &province, SettlementKind::Village)
        .or_else(|| settlement_of_kind(&state, &province, SettlementKind::Town))
        .expect("a secondary settlement");
    state.settlements.get_mut(&village).unwrap().controller = fac("fac_england");
    assert_eq!(
        state.full_province_income_factor(data, &province, &france),
        1.0
    );
}

#[test]
fn an_ungarrisoned_village_falls_on_arrival() {
    let data = game_data();
    let mut state = start(data, "fac_england");
    let england = fac("fac_england");
    let (village, staging) = staging_target(&state, data, SettlementKind::Village, false);
    let army = first_army(&state, &england);
    state.armies.get_mut(&army).unwrap().position = sim_campaign::ArmyPosition::Settlement(staging);
    state
        .submit_order(data, Order::move_along(army.clone(), vec![village.clone()]))
        .expect("move order accepted");
    quiet_turn(&mut state, data);
    assert_eq!(state.armies[&army].settlement().cloned().unwrap(), village);
    assert_eq!(
        state.settlements[&village].controller, england,
        "{village} is taken without a siege"
    );
    assert!(state.settlements[&village].siege.is_none());
}

#[test]
fn a_garrisoned_castle_is_besieged() {
    let data = game_data();
    let mut state = start(data, "fac_england");
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    let (castle, staging) = staging_target(&state, data, SettlementKind::Castle, true);
    let army = first_army(&state, &england);
    state.armies.get_mut(&army).unwrap().position = sim_campaign::ArmyPosition::Settlement(staging);
    state
        .submit_order(
            data,
            Order::SetStance {
                army: army.clone(),
                stance: Stance::Siege,
            },
        )
        .unwrap();
    state
        .submit_order(data, Order::move_along(army.clone(), vec![castle.clone()]))
        .expect("move order accepted");
    quiet_turn(&mut state, data);
    assert_eq!(state.armies[&army].settlement().cloned().unwrap(), castle);
    let settlement = &state.settlements[&castle];
    assert_eq!(
        settlement.controller, france,
        "a castle is not taken on arrival"
    );
    let siege = settlement.siege.as_ref().expect("the castle is besieged");
    assert_eq!(siege.attacker, england);
    // The siege is the castle's: the province's city is untouched.
    let city = &state.provinces[&settlement.province].city;
    if city != &castle {
        assert!(state.settlements[city].siege.is_none());
    }
}

#[test]
fn the_loaded_graph_and_the_fallback_graph_both_route_armies() {
    let mut data = game_data().clone();
    assert!(
        !data.movement_graph.fallback,
        "data/map/settlement_graph.json is used"
    );
    let state = start(&data, "fac_france");
    let france = fac("fac_france");
    let army = first_army(&state, &france);
    let target = state.provinces[&prov("prov_normandie")].city.clone();
    let real = graph_path(&state, &data, &army, &target).expect("a path on the real graph");
    assert_eq!(real.last(), Some(&target));

    data.settlement_graph.clear();
    data.build_movement_graph();
    assert!(data.movement_graph.fallback);
    let fallback = graph_path(&state, &data, &army, &target).expect("a path on the fallback graph");
    assert_eq!(fallback.last(), Some(&target));
    // Every step is an edge of the graph.
    let mut from = state.armies[&army].settlement().cloned().unwrap();
    for step in &fallback {
        assert!(data.movement_graph.edge(&from, step).is_some());
        from = step.clone();
    }
}

#[test]
fn armies_prefer_the_road_on_a_fixture_graph() {
    let mut data = game_data().clone();
    let state = start(&data, "fac_france");
    let france = fac("fac_france");
    let army = first_army(&state, &france);
    let a = state.armies[&army].settlement().cloned().unwrap();
    let province = state.settlements[&a].province.clone();
    let others: Vec<SettlementId> = state.provinces[&province]
        .settlements
        .iter()
        .filter(|s| **s != a)
        .take(2)
        .cloned()
        .collect();
    assert_eq!(others.len(), 2, "the capital province has 3 settlements");
    let (b, d) = (others[0].clone(), others[1].clone());
    let edge = |from: &SettlementId, to: &SettlementId, cost: f64, road: bool| SettlementEdge {
        from: from.clone(),
        to: to.clone(),
        cost,
        road,
        sea: false,
    };
    // The road detour A-B-D (2 x 15 baked at 0.5, rescaled by C7a to
    // `road_cost_factor`) beats the direct track A-D (60).
    data.settlement_graph = vec![
        edge(&a, &b, 15.0, true),
        edge(&b, &d, 15.0, true),
        edge(&a, &d, 60.0, false),
    ];
    data.build_movement_graph();
    assert!(!data.movement_graph.fallback);
    assert_eq!(graph_path(&state, &data, &army, &d), Some(vec![b, d]));
}

#[test]
fn settlement_campaigns_are_deterministic() {
    let data = game_data();
    let run = || {
        let mut state = CampaignState::new_1337(data, fac("fac_france"), 11).unwrap();
        for _ in 0..6 {
            state.end_turn(data);
        }
        state.save_json()
    };
    assert_eq!(run(), run());
}
