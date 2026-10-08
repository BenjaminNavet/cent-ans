//! NT5 (ADR 0128): N6 army unit cap (`data/rules/armies.json`) and N7
//! siege engines built on the spot (`data/rules/siege_engines.json`).

use data_model::{BuiltEngineKind, GameData, SettlementId, UnitTypeId};
use sim_battle::{BattleSim, SideId};
use sim_campaign::test_support::idle;
use sim_campaign::{
    siege::AssaultError, siege_engines, ArmyId, CampaignState, Order, OrderError, Stance, Unit,
};

use data_model::test_support::{fac, game_data, prov};

fn units(data: &GameData, unit_type: &str, n: usize) -> Vec<Unit> {
    let t = &data.unit_types[&UnitTypeId::new(unit_type).unwrap()];
    (0..n).map(|_| Unit::fresh(t)).collect()
}

fn guyenne(state: &CampaignState) -> SettlementId {
    state
        .province_city_id(&prov("prov_guyenne"))
        .unwrap()
        .clone()
}

fn french_army(state: &CampaignState) -> ArmyId {
    state
        .armies
        .iter()
        .find(|(_, a)| a.faction == fac("fac_france"))
        .map(|(id, _)| id.clone())
        .unwrap()
}

/// France's main army besieging English Guyenne; the siege has just begun
/// (no turn of progress yet).
fn besiege_guyenne(data: &GameData, seed: u64) -> (CampaignState, ArmyId) {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), seed).unwrap();
    let army = french_army(&state);
    let city = guyenne(&state);
    let kent = state.province_city_id(&prov("prov_kent")).unwrap().clone();
    let english: Vec<ArmyId> = state
        .armies
        .iter()
        .filter(|(_, a)| {
            a.settlement().and_then(|s| state.settlement_province(s)) == Some(&prov("prov_guyenne"))
                && a.faction != fac("fac_france")
        })
        .map(|(id, _)| id.clone())
        .collect();
    for id in english {
        state.armies.get_mut(&id).unwrap().position =
            sim_campaign::ArmyPosition::Settlement(kent.clone());
    }
    let a = state.armies.get_mut(&army).unwrap();
    a.position = sim_campaign::ArmyPosition::Settlement(city.clone());
    a.stance = Stance::Siege;
    a.clear_plan();
    state.end_turn_with(data, idle);
    assert!(state.settlement_state(&city).unwrap().siege.is_some());
    (state, army)
}

// ----- N6 -------------------------------------------------------------------

#[test]
fn the_cap_is_data() {
    let data = game_data();
    assert_eq!(data.army_rules.max_units, 40);
}

#[test]
fn a_41st_unit_cannot_form_an_army() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 3).unwrap();
    let paris = state
        .province_city_id(&prov("prov_ile_de_france"))
        .unwrap()
        .clone();
    state.settlements.get_mut(&paris).unwrap().garrison = units(data, "unit_crossbowmen", 45);
    let refused = state.submit_order(
        data,
        Order::CreateArmy {
            settlement: paris.clone().into(),
            units_from_garrison: (0..41).collect(),
            general: None,
        },
    );
    assert_eq!(refused, Err(OrderError::ArmyFull { cap: 40 }));
    assert!(refused
        .unwrap_err()
        .to_string()
        .contains("40 unités au plus"));
    let before = state.armies.len();
    state
        .submit_order(
            data,
            Order::CreateArmy {
                settlement: paris.into(),
                units_from_garrison: (0..40).collect(),
                general: None,
            },
        )
        .unwrap();
    assert_eq!(state.armies.len(), before + 1);
}

#[test]
fn a_merge_stays_within_the_cap() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 3).unwrap();
    let target = french_army(&state);
    let place = state.armies[&target].settlement().unwrap().clone();
    state.armies.get_mut(&target).unwrap().units = units(data, "unit_crossbowmen", 35);
    let mut source_army = state.armies[&target].clone();
    source_army.general = None;
    source_army.units = units(data, "unit_crossbowmen", 6);
    let source = ArmyId::from_index(9_999);
    state.armies.insert(source.clone(), source_army);
    let refused = state.submit_order(
        data,
        Order::MergeArmies {
            source: source.clone(),
            target: target.clone(),
        },
    );
    assert_eq!(refused, Err(OrderError::ArmyFull { cap: 40 }));
    assert_eq!(state.armies[&target].units.len(), 35);
    state.armies.get_mut(&source).unwrap().units.truncate(5);
    state
        .submit_order(
            data,
            Order::MergeArmies {
                source: source.clone(),
                target: target.clone(),
            },
        )
        .unwrap();
    assert_eq!(state.armies[&target].units.len(), 40);
    assert!(!state.armies.contains_key(&source));
    assert!(state.armies[&target].is_at(&place));
}

#[test]
fn a_full_army_hires_no_company() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 3).unwrap();
    let army = french_army(&state);
    state.armies.get_mut(&army).unwrap().units = units(data, "unit_crossbowmen", 40);
    let market = state.mercenary_market(data, &army).unwrap();
    let blocked = market.blocked.expect("a full army hires nobody");
    assert!(blocked.contains("armée complète"), "{blocked}");
    assert!(market.options.iter().all(|o| !o.available));
    assert_eq!(sim_campaign::orders::army_room(&state, data, &army), 0);
}

// ----- N7 -------------------------------------------------------------------

#[test]
fn engines_take_turns_by_army_size() {
    let data = game_data();
    // 2 000 men: 20 points a turn; 400 men: the floor.
    let big = siege_engines::work_per_turn(data, 2_000, 0.0);
    let small = siege_engines::work_per_turn(data, 400, 0.0);
    assert_eq!(big, 20);
    assert_eq!(small, data.siege_engine_rules.min_work_per_turn);
    // A siege-speed general hastens the work.
    assert!(siege_engines::work_per_turn(data, 2_000, 50.0) > big);
    let tower_turns = |rate: u32| {
        siege_engines::statuses(data, 0, rate, 0)
            .into_iter()
            .find(|s| s.kind == BuiltEngineKind::Tower)
            .unwrap()
            .turns_left
    };
    assert!(tower_turns(big) >= 2, "a tower takes several turns");
    assert!(tower_turns(small) > tower_turns(big));
    // In list order: ladders first.
    let ladders = data.siege_engine_rules.engines[0].work;
    let first = siege_engines::statuses(data, ladders, 1, 0);
    assert!(first[0].ready && !first[1].ready && !first[2].ready);
    assert_eq!(
        siege_engines::ready_kinds(data, 1_000, 0),
        vec![
            BuiltEngineKind::Ladders,
            BuiltEngineKind::Ram,
            BuiltEngineKind::Tower
        ]
    );
}

#[test]
fn engines_are_built_during_the_siege() {
    let data = game_data();
    let (mut state, army) = besiege_guyenne(data, 4);
    let city = guyenne(&state);
    let engines = state.siege_engines(data, &city);
    assert_eq!(engines.len(), data.siege_engine_rules.engines.len());
    assert!(engines.iter().all(|e| !e.ready && e.turns_left >= 1));
    let rate = state.engine_rate(data, &city);
    assert!(rate >= data.siege_engine_rules.min_work_per_turn);
    state.end_turn_with(data, idle);
    let siege = state
        .settlement_state(&city)
        .unwrap()
        .siege
        .clone()
        .unwrap();
    assert_eq!(siege.engine_work, rate);
    let engines = state.siege_engines(data, &city);
    // A6-L2: against walls of level 5 the ladders take several turns.
    assert_eq!(
        engines[0].ready,
        rate >= data.siege_engine_rules.engines[0].cost(
            data.siege_engine_rules.scaling_min_wall_level,
            state.fortification_level(data, &city)
        ),
        "ladders after one turn: {engines:?}"
    );
    assert!(engines[0].ready || engines[0].turns_left >= 1);
    // The army is still there.
    assert!(state.armies.contains_key(&army));
}

#[test]
fn an_assault_waits_for_an_engine() {
    let data = game_data();
    let (mut state, army) = besiege_guyenne(data, 4);
    let city = guyenne(&state);
    assert!(state.fortification_level(data, &city) > 0);
    let blocker = state.assault_blocker(data, &army).expect("no engine yet");
    assert!(blocker.contains("aucun engin prêt"), "{blocker}");
    let refused = state.submit_order(data, Order::Assault { army: army.clone() });
    assert!(matches!(
        refused,
        Err(OrderError::Assault(AssaultError::NoEngine(_)))
    ));
    assert!(state.pending_battles.is_empty());
    // A breach opens the way without engines.
    state
        .settlements
        .get_mut(&city)
        .unwrap()
        .siege
        .as_mut()
        .unwrap()
        .breach = 60;
    assert_eq!(state.assault_blocker(data, &army), None);
    state
        .settlements
        .get_mut(&city)
        .unwrap()
        .siege
        .as_mut()
        .unwrap()
        .breach = 0;
    // A6-L2: one turn of work is not enough for ladders against these
    // walls; with the work done, the assault is given.
    state.end_turn_with(data, idle);
    assert!(state.assault_blocker(data, &army).is_some());
    let ladders = data.siege_engine_rules.engines[0].cost(
        data.siege_engine_rules.scaling_min_wall_level,
        state.fortification_level(data, &city),
    );
    state
        .settlements
        .get_mut(&city)
        .unwrap()
        .siege
        .as_mut()
        .unwrap()
        .engine_work = ladders;
    assert_eq!(state.assault_blocker(data, &army), None);
    state
        .submit_order(data, Order::Assault { army: army.clone() })
        .unwrap();
    assert_eq!(state.pending_battles.len(), 1);
}

#[test]
fn built_engines_reach_the_siege_battle() {
    let data = game_data();
    let (mut state, army) = besiege_guyenne(data, 6);
    let city = guyenne(&state);
    // Ladders only.
    let ladders = data.siege_engine_rules.engines[0].cost(
        data.siege_engine_rules.scaling_min_wall_level,
        state.fortification_level(data, &city),
    );
    state
        .settlements
        .get_mut(&city)
        .unwrap()
        .siege
        .as_mut()
        .unwrap()
        .engine_work = ladders;
    state
        .submit_order(data, Order::Assault { army: army.clone() })
        .unwrap();
    let setup = state.battle_setup(data, 0).unwrap();
    let engines = setup.siege.as_ref().unwrap().engines.clone().unwrap();
    assert!(engines.ladders && !engines.ram && engines.towers.is_empty());
    let sim = BattleSim::new(setup.clone(), 1).unwrap();
    assert!(!sim.units().iter().any(|u| u.ram), "no ram built");
    // Everything built: ram and tower in the battle, walls no longer stand
    // for the auto-resolver.
    state
        .settlements
        .get_mut(&city)
        .unwrap()
        .siege
        .as_mut()
        .unwrap()
        .engine_work = 1_000;
    let setup = state.battle_setup(data, 0).unwrap();
    let engines = setup.siege.as_ref().unwrap().engines.clone().unwrap();
    assert!(engines.ladders && engines.ram && engines.towers.len() == 1);
    let sim = BattleSim::new(setup.clone(), 1).unwrap();
    let attackers: Vec<_> = sim
        .units()
        .iter()
        .filter(|u| u.side == SideId::Attacker)
        .collect();
    assert!(attackers.iter().any(|u| u.ram));
    assert!(attackers.iter().any(|u| u.siege_tower() && u.synthetic));
    assert_eq!(
        state.assault_odds(data, &army).map(|(_, walls)| walls),
        Some(false)
    );
}

/// NT9: in the auto-resolved assault a ready ram (gate broken) raises the
/// assailant's odds behind standing walls; the bonus is data.
#[test]
fn a_ready_ram_helps_the_auto_resolved_assault() {
    let data = game_data();
    let ram = data
        .siege_engine_rules
        .engines
        .iter()
        .find(|e| e.kind == BuiltEngineKind::Ram)
        .unwrap();
    assert!(ram.auto_assault_bonus_percent > 0);
    let (mut state, army) = besiege_guyenne(data, 6);
    let city = guyenne(&state);
    let ladders = data.siege_engine_rules.engines[0].cost(
        data.siege_engine_rules.scaling_min_wall_level,
        state.fortification_level(data, &city),
    );
    let set_work = |state: &mut CampaignState, work: u32| {
        state
            .settlements
            .get_mut(&city)
            .unwrap()
            .siege
            .as_mut()
            .unwrap()
            .engine_work = work;
    };
    set_work(&mut state, ladders);
    let (without, walls) = state.assault_odds(data, &army).unwrap();
    assert!(walls);
    let ram_cost = ram.cost(
        data.siege_engine_rules.scaling_min_wall_level,
        state.fortification_level(data, &city),
    );
    set_work(&mut state, ladders + ram_cost);
    let (with, walls) = state.assault_odds(data, &army).unwrap();
    assert!(walls, "the ram does not bring the walls down");
    assert!(with > without, "{with} > {without}");
}

/// Playtest 09-30: a siege begun by a march during the turn (immediate
/// marches) builds its engines at that turn's end, as the ETA promised.
#[test]
fn engines_are_built_from_the_turn_the_siege_begins() {
    let data = game_data();
    let (mut state, _army) = besiege_guyenne(data, 4);
    let city = guyenne(&state);
    let turn = state.turn;
    {
        let siege = state
            .settlements
            .get_mut(&city)
            .unwrap()
            .siege
            .as_mut()
            .unwrap();
        siege.started_turn = turn;
        siege.engine_work = 0;
    }
    let supplies = state
        .settlement_state(&city)
        .unwrap()
        .siege
        .clone()
        .unwrap()
        .supplies;
    let rate = state.engine_rate(data, &city);
    state.end_turn_with(data, idle);
    let siege = state
        .settlement_state(&city)
        .unwrap()
        .siege
        .clone()
        .unwrap();
    assert_eq!(siege.engine_work, rate);
    assert_eq!(siege.supplies, supplies, "supplies still wait a turn (M2)");
    let cost = data.siege_engine_rules.engines[0].cost(
        data.siege_engine_rules.scaling_min_wall_level,
        state.fortification_level(data, &city),
    );
    assert_eq!(state.siege_engines(data, &city)[0].ready, rate >= cost);
}
