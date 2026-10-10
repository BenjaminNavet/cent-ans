//! Lot WH armyb (ADR 0279): recruit into an army, forced sortie, raid supplies,
//! surrender demand.
use data_model::test_support::{fac, game_data};
use data_model::GameData;
use sim_campaign::test_support::{capital_city, main_army};
use sim_campaign::{ArmyId, ArmyPosition, CampaignState, Order, OrderError, Season, Stance};

fn start() -> (&'static GameData, CampaignState) {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 5).unwrap();
    state.chronicle.disabled = true;
    state.season = Season::Summer;
    state.factions.get_mut(&fac("fac_france")).unwrap().treasury = 100_000;
    (data, state)
}

fn recruitable_unit(
    state: &CampaignState,
    data: &GameData,
    place: &data_model::SettlementId,
) -> data_model::UnitTypeId {
    state
        .recruitable(data, place)
        .into_iter()
        .find(|o| o.available)
        .expect("a recruitable unit")
        .unit_type
}

/// The main French army parked in Paris.
fn army_in_paris(state: &mut CampaignState) -> (ArmyId, data_model::SettlementId) {
    let army = main_army(state, "fac_france");
    let paris = capital_city(state, "fac_france");
    let entry = state.armies.get_mut(&army).unwrap();
    entry.position = ArmyPosition::Settlement(paris.clone());
    entry.stance = Stance::Normal;
    (army, paris)
}

#[test]
fn recruit_into_army_joins_the_army_not_the_garrison() {
    let (data, mut state) = start();
    let (army, paris) = army_in_paris(&mut state);
    state.armies.get_mut(&army).unwrap().units.truncate(3);
    let unit = recruitable_unit(&state, data, &paris);
    let garrison = state.settlements[&paris].garrison.len();
    state
        .submit_order(
            data,
            Order::RecruitInto {
                settlement: paris.clone().into(),
                unit_type: unit,
                army: army.clone(),
            },
        )
        .unwrap();
    state.end_turn(data);
    assert_eq!(state.armies[&army].units.len(), 4);
    assert_eq!(state.settlements[&paris].garrison.len(), garrison);
}

#[test]
fn recruit_into_army_falls_back_to_garrison_when_the_army_left() {
    let (data, mut state) = start();
    let (army, paris) = army_in_paris(&mut state);
    state.armies.get_mut(&army).unwrap().units.truncate(3);
    let unit = recruitable_unit(&state, data, &paris);
    let garrison = state.settlements[&paris].garrison.len();
    state
        .submit_order(
            data,
            Order::RecruitInto {
                settlement: paris.clone().into(),
                unit_type: unit,
                army: army.clone(),
            },
        )
        .unwrap();
    state.armies.get_mut(&army).unwrap().position = ArmyPosition::Field { x: 1.0, y: 1.0 };
    state.end_turn(data);
    assert_eq!(state.armies[&army].units.len(), 3);
    assert_eq!(state.settlements[&paris].garrison.len(), garrison + 1);
}

#[test]
fn recruit_into_full_army_is_refused_and_foreign_army_too() {
    let (data, mut state) = start();
    let (army, paris) = army_in_paris(&mut state);
    let unit = recruitable_unit(&state, data, &paris);
    let cap = data.army_rules.cap();
    while state.armies[&army].units.len() < cap {
        let copy = state.armies[&army].units[0].clone();
        state.armies.get_mut(&army).unwrap().units.push(copy);
    }
    let order = |army: &ArmyId| Order::RecruitInto {
        settlement: paris.clone().into(),
        unit_type: unit.clone(),
        army: army.clone(),
    };
    assert!(matches!(
        state.submit_order(data, order(&army)),
        Err(OrderError::ArmyFull { .. })
    ));
    let english = main_army(&state, "fac_england");
    assert!(state.submit_order(data, order(&english)).is_err());
}

/// England raids (or just stands in) Boulonnais; returns (supply, loot text).
fn england_in_boulonnais(stance: Stance, devastation: u8) -> (u8, String) {
    use sim_campaign::test_support::{city, idle};
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_england"), 6).unwrap();
    state.chronicle.disabled = true;
    let english = main_army(&state, "fac_england");
    let province = data_model::test_support::prov("prov_boulonnais");
    let place = city(&state, "prov_boulonnais");
    let entry = state.armies.get_mut(&english).unwrap();
    entry.position = ArmyPosition::Settlement(place);
    entry.supply = 50;
    state.provinces.get_mut(&province).unwrap().devastation = devastation;
    state
        .submit_order(
            data,
            Order::SetStance {
                army: english.clone(),
                stance,
            },
        )
        .unwrap();
    let events = state.end_turn_with(data, idle);
    let loot = events
        .iter()
        .find(|e| e.kind == sim_campaign::EventKind::Raid)
        .map(|e| e.text_fr.clone())
        .unwrap_or_default();
    (state.armies[&english].supply, loot)
}

fn loot_of(text: &str) -> i64 {
    text.split(" livres de butin")
        .next()
        .and_then(|head| head.rsplit(' ').next())
        .and_then(|n| n.parse().ok())
        .expect("a loot figure")
}

#[test]
fn raiding_feeds_the_army_and_loot_diminishes_in_a_ravaged_province() {
    let (raid_supply, fresh_loot) = england_in_boulonnais(Stance::Raid, 0);
    let (idle_supply, _) = england_in_boulonnais(Stance::Normal, 0);
    assert!(raid_supply > idle_supply, "{raid_supply} vs {idle_supply}");
    let (_, ravaged_loot) = england_in_boulonnais(Stance::Raid, 80);
    let (fresh, ravaged) = (loot_of(&fresh_loot), loot_of(&ravaged_loot));
    assert!(
        fresh > 0 && ravaged * 2 <= fresh + 1,
        "{fresh} vs {ravaged}"
    );
}

/// England besieges French Boulogne; the siege has begun. `english_units` /
/// `garrison_units` cap each side so the outcome of a sortie is known.
fn besieged_boulogne(
    english_units: usize,
    garrison_units: usize,
) -> (CampaignState, data_model::SettlementId, ArmyId) {
    use sim_campaign::test_support::idle;
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 4).unwrap();
    state.chronicle.disabled = true;
    let english = main_army(&state, "fac_england");
    let boulogne = data_model::SettlementId::new("set_boulogne").unwrap();
    state
        .armies
        .get_mut(&english)
        .unwrap()
        .units
        .truncate(english_units);
    let garrison = &mut state.settlements.get_mut(&boulogne).unwrap().garrison;
    while garrison.len() < garrison_units {
        let copy = garrison[0].clone();
        garrison.push(copy);
    }
    garrison.truncate(garrison_units);
    state.armies.get_mut(&english).unwrap().position = ArmyPosition::Settlement(boulogne.clone());
    state.armies.get_mut(&english).unwrap().stance = Stance::Siege;
    state.end_turn_with(data, idle);
    assert!(state.settlements[&boulogne].siege.is_some(), "siege begun");
    (state, boulogne, english)
}

#[test]
fn sortie_order_breaks_the_siege_when_won() {
    let data = game_data();
    // Even siege first (no automatic sortie), then the besiegers dwindle.
    let (mut state, boulogne, english) = besieged_boulogne(8, 8);
    state.interactive_battles = false;
    state.armies.get_mut(&english).unwrap().units.truncate(1);
    for unit in &mut state
        .armies
        .get_mut(&main_army(&state, "fac_england"))
        .unwrap()
        .units
    {
        unit.strength = unit.max_strength.clamp(1, 20);
    }
    state
        .submit_order(
            data,
            Order::Sortie {
                settlement: boulogne.clone().into(),
            },
        )
        .unwrap();
    assert!(state.settlements[&boulogne].siege.is_none());
}

#[test]
fn sortie_order_lost_costs_the_garrison_and_the_siege_goes_on() {
    let data = game_data();
    let (mut state, boulogne, english) = besieged_boulogne(8, 1);
    state.interactive_battles = false;
    let before = state.settlements[&boulogne].garrison_strength();
    state
        .submit_order(
            data,
            Order::Sortie {
                settlement: boulogne.clone().into(),
            },
        )
        .unwrap();
    assert!(state.settlements[&boulogne].garrison_strength() < before);
    assert!(state.settlements[&boulogne].siege.is_some());
    assert_eq!(state.armies[&english].stance, Stance::Siege);
}

#[test]
fn sortie_order_needs_a_siege_and_your_own_place() {
    let (data, mut state) = start();
    let paris = capital_city(&state, "fac_france");
    assert!(matches!(
        state.submit_order(
            data,
            Order::Sortie {
                settlement: paris.into()
            }
        ),
        Err(OrderError::SortieUnavailable(_))
    ));
    let london = capital_city(&state, "fac_england");
    assert!(state
        .submit_order(
            data,
            Order::Sortie {
                settlement: london.into()
            }
        )
        .is_err());
}

fn demand(state: &mut CampaignState, place: &data_model::SettlementId) -> Result<(), OrderError> {
    state.apply_order(
        game_data(),
        &fac("fac_england"),
        Order::DemandSurrender {
            settlement: place.clone().into(),
        },
    )
}

#[test]
fn surrender_is_refused_with_stores_and_accepted_when_starved_and_breached() {
    let data = game_data();
    let (mut state, boulogne, _) = besieged_boulogne(8, 8);
    let set_siege = |state: &mut CampaignState, supplies: u8, breach: u8| {
        let siege = state
            .settlements
            .get_mut(&boulogne)
            .unwrap()
            .siege
            .as_mut()
            .unwrap();
        siege.supplies = supplies;
        siege.breach = breach;
    };
    set_siege(&mut state, 50, 100);
    assert_eq!(
        sim_campaign::siege::surrender_chance(&state, data, &boulogne),
        Some(0)
    );
    demand(&mut state, &boulogne).unwrap();
    assert_eq!(state.settlements[&boulogne].controller, fac("fac_france"));
    set_siege(&mut state, 20, 40);
    assert_eq!(
        sim_campaign::siege::surrender_chance(&state, data, &boulogne),
        Some(50)
    );
    set_siege(&mut state, 0, 50);
    assert_eq!(
        sim_campaign::siege::surrender_chance(&state, data, &boulogne),
        Some(100)
    );
    demand(&mut state, &boulogne).unwrap();
    let place = &state.settlements[&boulogne];
    assert_eq!(place.controller, fac("fac_england"));
    assert!(place.siege.is_none() && place.garrison.is_empty());
}

#[test]
fn only_the_besieger_may_demand_surrender() {
    let (mut state, boulogne, _) = besieged_boulogne(8, 8);
    let result = state.apply_order(
        game_data(),
        &fac("fac_france"),
        Order::DemandSurrender {
            settlement: boulogne.clone().into(),
        },
    );
    assert!(matches!(result, Err(OrderError::SurrenderUnavailable(_))));
}

// ---- WR sortie (ADR 0306): the sortie fought in 3D or auto-resolved from the dialog.

use sim_battle::{BattleOutcome, SideId, SideResult};

fn side_result(losses: Vec<u32>) -> SideResult {
    SideResult {
        total_losses: losses.iter().sum(),
        losses,
        morale_delta: 0,
        routed: false,
        general_killed: false,
        general_captured: false,
        no_quarter: false,
        withdrew: false,
        standards_taken: Vec::new(),
        standards_lost: 0,
        baggage_lost: false,
    }
}

fn order_sortie(state: &mut CampaignState, place: &data_model::SettlementId) {
    state
        .submit_order(
            game_data(),
            Order::Sortie {
                settlement: place.clone().into(),
            },
        )
        .unwrap();
}

#[test]
fn the_players_sortie_waits_for_the_pre_battle_dialog() {
    let data = game_data();
    let (mut state, boulogne, english) = besieged_boulogne(8, 4);
    let before = state.settlements[&boulogne].garrison_strength();
    order_sortie(&mut state, &boulogne);
    // Nothing fought yet.
    assert_eq!(state.settlements[&boulogne].garrison_strength(), before);
    assert!(state.settlements[&boulogne].siege.is_some());
    let views = state.pending_battle_views(data);
    assert_eq!(views.len(), 1);
    let view = &views[0];
    assert!(view.sortie && !view.siege);
    assert_eq!(view.player_side, Some(SideId::Attacker));
    assert_eq!(view.attacker_name, data.faction_name(&fac("fac_france")));
    assert_eq!(view.defender_name, data.faction_name(&fac("fac_england")));
    // Ordering twice does not queue twice.
    order_sortie(&mut state, &boulogne);
    assert_eq!(state.pending_battles.len(), 1);
    // The description: the garrison attacks the besieging army, in the open.
    let setup = state.battle_setup(data, 0).unwrap();
    assert!(setup.siege.is_none());
    assert_eq!(setup.player_side, Some(SideId::Attacker));
    assert_eq!(setup.attacker.faction, "fac_france");
    assert_eq!(setup.defender.faction, "fac_england");
    assert_eq!(
        setup.attacker.units.len(),
        state.settlements[&boulogne].garrison.len()
    );
    assert_eq!(
        setup.defender.units.len(),
        state.armies[&english].units.len()
    );
    // The forecast exists and the player may call it off.
    let forecast = state.battle_forecast(data, 0).unwrap();
    assert!(forecast.can_withdraw && forecast.lifts_siege);
}

#[test]
fn a_fought_sortie_has_the_effects_of_the_auto_resolved_one() {
    let data = game_data();
    // Won in 3D: the besiegers lose everything, the garrison little.
    let (mut state, boulogne, english) = besieged_boulogne(8, 4);
    order_sortie(&mut state, &boulogne);
    let setup = state.battle_setup(data, 0).unwrap();
    let garrison_units = setup.attacker.units.len();
    let siege_units = setup.defender.units.len();
    let outcome = BattleOutcome {
        winner: SideId::Attacker,
        attacker: side_result(vec![1; garrison_units]),
        defender: side_result(
            state.armies[&english]
                .units
                .iter()
                .map(|u| u.strength)
                .collect(),
        ),
        duration: 60.0,
        end: Default::default(),
    };
    assert_eq!(outcome.defender.losses.len(), siege_units);
    let garrison_before = state.settlements[&boulogne].garrison_strength();
    let events = state.resolve_pending_battle(data, 0, &outcome).unwrap();
    assert!(state.pending_battles.is_empty());
    assert!(events.iter().any(|e| e.text_fr.contains("Sortie")));
    assert!(state.settlements[&boulogne].siege.is_none(), "siege lifted");
    assert!(state.settlements[&boulogne].garrison_strength() < garrison_before);
    assert!(state
        .armies
        .get(&english)
        .is_none_or(|a| a.stance != Stance::Siege));

    // Lost in 3D: the garrison pays, the siege goes on.
    let (mut state, boulogne, english) = besieged_boulogne(8, 4);
    order_sortie(&mut state, &boulogne);
    let setup = state.battle_setup(data, 0).unwrap();
    let outcome = BattleOutcome {
        winner: SideId::Defender,
        attacker: side_result(
            state.settlements[&boulogne]
                .garrison
                .iter()
                .map(|u| u.strength)
                .collect(),
        ),
        defender: side_result(vec![0; setup.defender.units.len()]),
        duration: 60.0,
        end: Default::default(),
    };
    let garrison_before = state.settlements[&boulogne].garrison_strength();
    state.resolve_pending_battle(data, 0, &outcome).unwrap();
    assert!(state.settlements[&boulogne].garrison_strength() < garrison_before);
    assert!(state.settlements[&boulogne].siege.is_some());
    assert_eq!(state.armies[&english].stance, Stance::Siege);
}

#[test]
fn the_sortie_auto_resolution_button_matches_the_immediate_sortie() {
    let data = game_data();
    // Same state, same seed: resolving the pending sortie equals the former
    // immediate auto-resolution.
    let (mut immediate, boulogne, _) = besieged_boulogne(8, 4);
    immediate.interactive_battles = false;
    order_sortie(&mut immediate, &boulogne);
    let (mut pending, boulogne, _) = besieged_boulogne(8, 4);
    order_sortie(&mut pending, &boulogne);
    let events = pending.auto_resolve_pending(data, 0).unwrap();
    assert!(events.iter().any(|e| e.text_fr.contains("Sortie")));
    assert!(pending.pending_battles.is_empty());
    assert_eq!(
        pending.settlements[&boulogne].garrison_strength(),
        immediate.settlements[&boulogne].garrison_strength()
    );
    assert_eq!(
        pending.settlements[&boulogne].siege.is_some(),
        immediate.settlements[&boulogne].siege.is_some()
    );
}

#[test]
fn calling_off_the_sortie_keeps_the_garrison_in() {
    let data = game_data();
    let (mut state, boulogne, _) = besieged_boulogne(8, 4);
    let before = state.settlements[&boulogne].garrison_strength();
    order_sortie(&mut state, &boulogne);
    state.withdraw_pending_battle(data, 0).unwrap();
    assert!(state.pending_battles.is_empty());
    assert_eq!(state.settlements[&boulogne].garrison_strength(), before);
    assert!(state.settlements[&boulogne].siege.is_some());
}

#[test]
fn a_pending_sortie_is_auto_resolved_at_the_end_of_the_turn() {
    let data = game_data();
    let (mut state, boulogne, _) = besieged_boulogne(8, 4);
    order_sortie(&mut state, &boulogne);
    state.end_turn_with(data, sim_campaign::test_support::idle);
    assert!(state.pending_battles.iter().all(|r| !r.sortie));
}

#[test]
fn the_ai_sortie_against_the_players_army_is_a_pending_battle() {
    let data = game_data();
    // England (player) besieges French Boulogne held by the AI.
    let mut state = CampaignState::new_1337(data, fac("fac_england"), 4).unwrap();
    state.chronicle.disabled = true;
    let english = main_army(&state, "fac_england");
    let boulogne = data_model::SettlementId::new("set_boulogne").unwrap();
    state.armies.get_mut(&english).unwrap().units.truncate(1);
    let garrison = &mut state.settlements.get_mut(&boulogne).unwrap().garrison;
    while garrison.len() < 8 {
        let copy = garrison[0].clone();
        garrison.push(copy);
    }
    state.armies.get_mut(&english).unwrap().position = ArmyPosition::Settlement(boulogne.clone());
    state.armies.get_mut(&english).unwrap().stance = Stance::Siege;
    state.end_turn_with(data, sim_campaign::test_support::idle);
    let Some(view) = state
        .pending_battle_views(data)
        .into_iter()
        .find(|v| v.sortie)
    else {
        panic!("a strong garrison sallies: {:?}", state.pending_battles);
    };
    assert_eq!(view.player_side, Some(SideId::Defender));
    assert!(
        !state
            .battle_forecast(data, view.index)
            .unwrap()
            .can_withdraw
    );
}
