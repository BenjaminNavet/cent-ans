//! Lot JR1 (ADR 0165): the crusader faction and its fervour on the real
//! data of 1337 (`data/rules/crusade.json`, the faction of lot JR2).

use data_model::{CrusadeRules, FactionId, GameData};
use sim_campaign::crusade::{self, crusade_view};
use sim_campaign::{ArmyPosition, CampaignState, EventKind, Order};

use data_model::test_support::{fac, game_data};

fn rules(data: &GameData) -> &CrusadeRules {
    data.crusade_rules
        .as_ref()
        .expect("data/rules/crusade.json is loaded")
}

/// A campaign played by the crusader faction of the rules.
fn crusade(data: &GameData, seed: u64) -> (CampaignState, FactionId) {
    let faction = rules(data).faction.clone();
    let state = CampaignState::new_1337(data, faction.clone(), seed).expect("1337 start");
    (state, faction)
}

#[test]
fn the_rules_name_existing_things() {
    let data = game_data();
    let rules = rules(data);
    assert!(data.factions.contains_key(&rules.faction));
    assert!(data
        .settlements
        .get(&rules.base_settlement)
        .is_some_and(|s| s.port));
    assert!(data.provinces.contains_key(&rules.target_province));
    for province in rules.holy_land.iter().chain(&rules.coastal_holy_land) {
        assert!(data.provinces.contains_key(province), "{province}");
    }
    for unit in rules
        .starting_army
        .iter()
        .chain(rules.passage.unit_table.iter().map(|e| &e.unit))
    {
        assert!(data.unit_types.contains_key(unit), "{unit}");
    }
}

#[test]
fn the_starting_army_stands_in_the_base_settlement() {
    let data = game_data();
    let (state, faction) = crusade(data, 1);
    let rules = rules(data);
    let base = &state.settlements[&rules.base_settlement];
    assert_eq!(base.owner, faction);
    assert_eq!(base.controller, faction);
    let armies: Vec<_> = state
        .armies
        .values()
        .filter(|a| a.faction == faction)
        .collect();
    assert_eq!(armies.len(), 1);
    let army = armies[0];
    assert!(
        army.is_at(&rules.base_settlement),
        "not in the city of the capital: {:?}",
        army.position
    );
    let types: Vec<_> = army.units.iter().map(|u| u.unit_type.clone()).collect();
    assert_eq!(types, rules.starting_army);
    assert_eq!(
        army.general, state.factions[&faction].ruler,
        "led by its ruler"
    );
    // The crusade is open, at the starting fervour; the city of the capital
    // province is not the faction's.
    let crusade = state.crusade.as_ref().expect("crusade opened");
    assert_eq!(crusade.fervor, rules.fervor.start);
    let capital = &state.factions[&faction].capital;
    assert_ne!(state.province_controller(capital), Some(&faction));
    // Only the crusaders see the panel.
    assert!(crusade_view(&state, data, &faction).is_some());
    assert!(crusade_view(&state, data, &fac("fac_france")).is_none());
}

#[test]
fn preaching_lands_a_contingent_on_time() {
    let data = game_data();
    let (mut state, faction) = crusade(data, 2);
    let rules = rules(data);
    let view = crusade_view(&state, data, &faction).expect("view");
    assert!(view.passage_available, "{}", view.passage_blocker);
    let expected_units = view.passage_units;
    let treasury = state.factions[&faction].treasury;
    state
        .submit_order(data, Order::PreachPassage)
        .expect("preached");
    assert_eq!(
        state.factions[&faction].treasury,
        treasury - view.passage_cost
    );
    let after = crusade_view(&state, data, &faction).expect("view");
    assert_eq!(
        i32::from(after.fervor),
        i32::from(view.fervor) + rules.fervor.preach
    );
    assert!(!after.passage_available);
    assert!(!after.passage_blocker.is_empty());
    assert_eq!(after.pending.len(), 1);
    assert_eq!(after.pending[0].units, expected_units);
    assert_eq!(after.pending[0].turns_left, rules.passage.delay_turns);
    assert_eq!(after.pending[0].port, rules.base_settlement);
    // A second call is refused, in French.
    let refused = state
        .submit_order(data, Order::PreachPassage)
        .expect_err("on cooldown");
    assert!(refused.to_string().contains("Passage"), "{refused}");

    let garrison = |state: &CampaignState| state.settlements[&rules.base_settlement].garrison.len();
    let before = garrison(&state);
    let mut landed_after = None;
    let mut events = Vec::new();
    for turn in 1..=rules.passage.delay_turns {
        events = state.end_turn(data);
        if garrison(&state) > before && landed_after.is_none() {
            landed_after = Some(turn);
        }
    }
    assert_eq!(landed_after, Some(rules.passage.delay_turns));
    assert_eq!(garrison(&state), before + expected_units as usize);
    assert!(events
        .iter()
        .any(|e| e.kind == EventKind::Crusade && e.faction.as_ref() == Some(&faction)));
    let view = crusade_view(&state, data, &faction).expect("view");
    assert!(view.pending.is_empty());
    // The alms are part of the season's income.
    assert!(view.alms_last_turn > 0);
    assert!(state.factions[&faction].last_budget.income >= view.alms_last_turn);
}

#[test]
fn an_older_save_without_crusade_loads() {
    let data = game_data();
    let (state, faction) = crusade(data, 3);
    let mut json: serde_json::Value = serde_json::from_str(&state.save_json()).unwrap();
    assert!(json.as_object_mut().unwrap().remove("crusade").is_some());
    let mut old = CampaignState::load_json(&json.to_string()).expect("older save loads");
    assert!(old.crusade.is_none());
    assert_eq!(old.state_version, state.state_version);
    assert!(crusade_view(&old, data, &faction).is_none());
    assert!(old.submit_order(data, Order::PreachPassage).is_err());
    // The mechanic is inert, the game goes on.
    for _ in 0..3 {
        old.end_turn(data);
    }
    assert!(old.crusade.is_none());
    // A current save keeps the crusade.
    let reloaded = CampaignState::load_json(&state.save_json()).expect("save loads");
    assert_eq!(reloaded.crusade, state.crusade);
}

#[test]
fn forty_ai_turns_without_panic_and_the_crusaders_live() {
    let data = game_data();
    let faction = rules(data).faction.clone();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 4).expect("1337 start");
    state.interactive_battles = false;
    let mut preached = false;
    for _ in 0..40 {
        let events = state.end_turn(data);
        preached |= events.iter().any(|e| e.kind == EventKind::Crusade);
        let crusade = state.crusade.as_ref().expect("crusade kept");
        assert!(crusade.fervor <= 100);
    }
    assert!(
        state.factions[&faction].alive,
        "the crusaders are still there"
    );
    assert!(preached, "the AI preached the passage");
    assert!(crusade_view(&state, data, &faction).is_some());
}

#[test]
fn taking_the_target_province_delivers_it() {
    let data = game_data();
    let (mut state, faction) = crusade(data, 5);
    let rules = rules(data);
    let city = state
        .province_city_id(&rules.target_province)
        .expect("target city")
        .clone();
    state.crusade.as_mut().unwrap().fervor = 5;
    state.settlements.get_mut(&city).unwrap().controller = faction.clone();
    let mut events = Vec::new();
    crusade::on_settlement_taken(&mut state, data, &faction, &city, &mut events);
    let delivered: Vec<_> = events
        .iter()
        .filter(|e| e.kind == EventKind::Crusade)
        .collect();
    assert_eq!(delivered.len(), 1);
    assert_eq!(delivered[0].province.as_ref(), Some(&rules.target_province));
    assert!(
        delivered[0].text_fr.contains("délivrée"),
        "{}",
        delivered[0].text_fr
    );
    let view = crusade_view(&state, data, &faction).expect("view");
    assert!(view.target_taken);
    assert_eq!(view.floor, rules.fervor.target_floor);
    assert!(view.fervor >= rules.fervor.target_floor);
    assert_eq!(state.factions[&faction].capital, rules.target_province);
    // The floor holds through the ends of turn while the city is held.
    state.crusade.as_mut().unwrap().fervor = rules.fervor.target_floor;
    let mut journal = Vec::new();
    state.resolve_end_of_turn(data, &mut journal);
    let crusade = state.crusade.as_ref().unwrap();
    if state.province_controller(&rules.target_province) == Some(&faction) {
        assert!(crusade.fervor >= rules.fervor.target_floor);
        assert!(crusade.target_taken);
    }
}

/// The largest army of `faction`.
fn main_army(state: &CampaignState, faction: &FactionId) -> sim_campaign::ArmyId {
    state
        .armies
        .iter()
        .filter(|(_, a)| &a.faction == faction)
        .max_by_key(|(id, a)| (a.units.len(), std::cmp::Reverse((*id).clone())))
        .map(|(id, _)| id.clone())
        .expect("faction has an army")
}

/// A campaign played by `player` where its main army stands in the field
/// next to the main army of `enemy`, at war with it, ready to attack.
fn facing(
    data: &GameData,
    player: &FactionId,
    enemy: &FactionId,
    seed: u64,
) -> (CampaignState, sim_campaign::ArmyId, sim_campaign::ArmyId) {
    let mut state = CampaignState::new_1337(data, player.clone(), seed).expect("1337 start");
    state.interactive_battles = false;
    let (mine, theirs) = (main_army(&state, player), main_army(&state, enemy));
    state.armies.retain(|id, _| *id == mine || *id == theirs);
    let point = [2200.0, 3580.0];
    state.armies.get_mut(&mine).unwrap().position = ArmyPosition::field(point);
    state.armies.get_mut(&theirs).unwrap().position =
        ArmyPosition::field([point[0] + 4.0, point[1]]);
    for id in [&mine, &theirs] {
        let allowance = state.army_grid_allowance(data, &state.armies[id]);
        state.armies.get_mut(id).unwrap().movement_left = allowance;
    }
    for (a, b) in [(player, enemy), (enemy, player)] {
        let f = state.factions.get_mut(a).unwrap();
        f.at_war_with.insert(b.clone());
        f.allies.remove(b);
    }
    (state, mine, theirs)
}

/// Fervour causes noted this turn.
fn causes(state: &CampaignState) -> Vec<(String, i32)> {
    state.crusade.as_ref().unwrap().last_changes.clone()
}

#[test]
fn a_real_battle_against_another_faith_moves_the_fervour() {
    // JR4: through the attack order and `apply_battle_result`, not the hook.
    let data = game_data();
    let rules = rules(data);
    let faction = rules.faction.clone();
    let holder = fac("fac_mamluks");
    let (mut state, mine, theirs) = facing(data, &faction, &holder, 3);
    let before = state.crusade.as_ref().unwrap().fervor;
    state
        .submit_order(
            data,
            Order::Attack {
                army: mine,
                target_army: theirs,
            },
        )
        .expect("the attack is fought");
    let report = state.last_battle_outcome.clone().expect("battle resolved");
    let won = report.attacker.class.is_victory();
    let expected = if won {
        rules.fervor.battle_won_other_faith
    } else {
        rules.fervor.battle_lost
    };
    let after = state.crusade.as_ref().unwrap().fervor;
    assert_eq!(
        i32::from(after),
        (i32::from(before) + expected).clamp(0, 100),
        "won: {won}, causes: {:?}",
        causes(&state)
    );
    assert!(causes(&state)
        .iter()
        .all(|(cause, _)| !cause.contains("frères de foi")));
}

#[test]
fn only_the_battles_the_crusade_seeks_against_its_faith_are_a_scandal() {
    let data = game_data();
    let rules = rules(data);
    let faction = rules.faction.clone();
    let brothers = fac("fac_hospitallers");
    // The crusade attacks brothers in faith: the scandal.
    let (mut state, mine, theirs) = facing(data, &faction, &brothers, 3);
    state
        .submit_order(
            data,
            Order::Attack {
                army: mine,
                target_army: theirs,
            },
        )
        .expect("the attack is fought");
    assert!(
        causes(&state)
            .iter()
            .any(|(cause, delta)| cause.contains("frères de foi")
                && *delta == rules.fervor.battle_same_faith),
        "{:?}",
        causes(&state)
    );
    // Attacked by them, it only defends itself: no malus but a defeat's.
    let (mut state, theirs, mine) = facing(data, &brothers, &faction, 3);
    let before = state.crusade.as_ref().unwrap().fervor;
    state
        .submit_order(
            data,
            Order::Attack {
                army: theirs,
                target_army: mine,
            },
        )
        .expect("the attack is fought");
    let report = state.last_battle_outcome.clone().expect("battle resolved");
    let expected = if report.attacker.class.is_victory() {
        rules.fervor.battle_lost
    } else {
        0
    };
    assert_eq!(
        i32::from(state.crusade.as_ref().unwrap().fervor),
        (i32::from(before) + expected).clamp(0, 100),
        "{:?}",
        causes(&state)
    );
    assert!(causes(&state)
        .iter()
        .all(|(cause, _)| !cause.contains("frères de foi")));
}

#[test]
fn the_deliverance_and_the_loss_of_the_target_are_news_for_every_faction() {
    // JR5: a game played by another faction reads both in its turn journal:
    // the events are `public` (the bridge hands the flag over; the
    // interface shows public news to every player); the loss is a `loss`.
    // The capital moves to the target and back.
    let data = game_data();
    let rules = rules(data);
    let faction = rules.faction.clone();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 5).expect("1337 start");
    state.interactive_battles = false;
    let seat = state.factions[&faction].capital.clone();
    let city = state
        .province_city_id(&rules.target_province)
        .expect("target city")
        .clone();
    let holder = state.settlements[&city].controller.clone();
    let public = |events: &[sim_campaign::GameEvent]| -> Vec<sim_campaign::GameEvent> {
        events
            .iter()
            .filter(|e| e.kind == EventKind::Crusade && e.public)
            .cloned()
            .collect()
    };
    state.settlements.get_mut(&city).unwrap().controller = faction.clone();
    let events = state.end_turn_with(data, |_, _, _| Vec::new());
    let news = public(&events);
    assert_eq!(news.len(), 1, "{news:?}");
    assert_eq!(news[0].province.as_ref(), Some(&rules.target_province));
    assert_eq!(news[0].faction.as_ref(), Some(&faction));
    assert!(!news[0].loss);
    assert!(state.crusade.as_ref().unwrap().target_taken);
    assert_eq!(state.factions[&faction].capital, rules.target_province);
    // Lost again: news for all too, a loss, and the former seat back.
    state.settlements.get_mut(&city).unwrap().controller = holder;
    let events = state.end_turn_with(data, |_, _, _| Vec::new());
    let news = public(&events);
    assert_eq!(news.len(), 1, "{news:?}");
    assert!(news[0].loss, "{news:?}");
    assert!(!state.crusade.as_ref().unwrap().target_taken);
    assert_eq!(state.factions[&faction].capital, seat);
    // An old save without the flags still loads them as false.
    let old: sim_campaign::GameEvent =
        serde_json::from_str(r#"{"kind":"crusade","text_fr":"x"}"#).expect("old event");
    assert!(!old.public && !old.loss);
}

#[test]
fn the_vow_binds_the_ai_to_no_peace_with_the_master_of_its_goal() {
    // JR4: led by the AI, the crusade neither sues for peace with the
    // holder of the target nor signs one (it bought a truce with its whole
    // treasury on the first turn and its fervour bled away).
    use sim_campaign::negotiation::{evaluate_treaty, plan_peace, Article};
    let data = game_data();
    let rules = rules(data);
    let faction = rules.faction.clone();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 5).expect("1337 start");
    let holder = state
        .province_controller(&rules.target_province)
        .expect("target held")
        .clone();
    assert!(state.is_at_war(&faction, &holder));
    // Desperate: no money, beaten.
    state.factions.get_mut(&faction).unwrap().treasury = -5000;
    assert!(crusade::ai_vow_forbids_peace(
        &state, data, &faction, &holder
    ));
    let verdict = evaluate_treaty(&state, data, &holder, &faction, &[Article::Peace]);
    assert!(verdict.blocked.is_some(), "{verdict:?}");
    assert_eq!(verdict.chance, 0);
    for turn in 0..4 {
        state.turn += turn;
        let offer = plan_peace(&state, data, &faction);
        assert!(
            !matches!(&offer, Some(Order::ProposeTreaty { target, .. }) if *target == holder),
            "{offer:?}"
        );
    }
    // Others, and a crusade the player leads, keep their freedom.
    let cyprus = fac("fac_cyprus");
    assert!(!crusade::ai_vow_forbids_peace(
        &state, data, &cyprus, &holder
    ));
    let played = CampaignState::new_1337(data, faction.clone(), 5).expect("1337 start");
    assert!(!crusade::ai_vow_forbids_peace(
        &played, data, &faction, &holder
    ));
}

#[test]
fn the_starting_host_is_about_balanced_at_the_starting_fervour() {
    // JR4: played from the first turn, the crusade's army and garrisons cost
    // about what the alms and its few places bring at the starting fervour:
    // preaching stays a real choice, and no bankruptcy before it.
    let data = game_data();
    let (state, faction) = crusade(data, 1);
    assert_eq!(
        state.crusade.as_ref().unwrap().fervor,
        rules(data).fervor.start
    );
    let economy = state.faction_economy(data, &faction).expect("economy");
    let net = economy.net_income();
    println!(
        "receipts {} trade {} armies {} buildings {} administration {} table {} net {net}",
        economy.projected_income,
        economy.trade_income,
        economy.army_upkeep,
        economy.building_upkeep,
        economy.administration_upkeep,
        economy.table_upkeep
    );
    assert!((-300..=200).contains(&net), "net balance {net}");
}

#[test]
fn the_view_carries_the_thresholds_of_the_rules() {
    let data = game_data();
    let (state, faction) = crusade(data, 1);
    let rules = rules(data);
    let view = crusade_view(&state, data, &faction).expect("view");
    assert_eq!(view.zeal_high_threshold, rules.zeal.high_threshold);
    assert_eq!(view.zeal_low_threshold, rules.zeal.low_threshold);
    assert_eq!(view.zeal_high_morale, rules.zeal.high_morale);
    assert_eq!(view.zeal_low_morale, rules.zeal.low_morale);
    assert_eq!(view.desertion_threshold, rules.desertion.threshold);
    assert_eq!(
        view.desertion_men_percent,
        rules.desertion.men_percent_per_turn
    );
}

#[test]
fn a_besieged_crusade_sallying_against_brothers_only_defends_itself() {
    // JR5: the sortie answers the siege; the besiegers sought the fight.
    let data = game_data();
    let rules = rules(data);
    let faction = rules.faction.clone();
    let france = fac("fac_france");
    let (mut state, mine, theirs) = facing(data, &faction, &france, 6);
    let base = rules.base_settlement.clone();
    state.armies.remove(&mine);
    let besieger = state.armies.get_mut(&theirs).unwrap();
    besieger.position = ArmyPosition::Settlement(base.clone());
    besieger.stance = sim_campaign::Stance::Siege;
    besieger.clear_plan();
    let idle = |_: &CampaignState, _: &GameData, _: &FactionId| Vec::<Order>::new();
    state.end_turn_with(data, idle);
    assert!(state.settlements[&base].siege.is_some(), "the siege begins");
    let knight = sim_campaign::Unit {
        unit_type: data_model::UnitTypeId::new("unit_knights").unwrap(),
        strength: 100,
        max_strength: 100,
        experience: 2,
        morale: 80,
        levy_armor: 0,
        levy_ranged: 0,
        experience_residue: 0,
    };
    let garrison = &mut state.settlements.get_mut(&base).unwrap().garrison;
    garrison.clear();
    garrison.extend(std::iter::repeat_n(knight, 8));
    let events = state.end_turn_with(data, idle);
    assert!(
        events.iter().any(|e| e.text_fr.contains("Sortie")),
        "{events:?}"
    );
    assert!(
        causes(&state)
            .iter()
            .all(|(cause, _)| !cause.contains("frères de foi")),
        "{:?}",
        causes(&state)
    );
}
