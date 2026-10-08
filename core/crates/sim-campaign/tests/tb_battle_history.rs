//! Chantier TB, lot « historique des batailles » (ADR 0157 « révision »):
//! the campaign state keeps the recent land battles for the battlefield
//! marks of the map, bounded by `data/rules/battle_history.json`, saved
//! with the game and absent from older saves.

use data_model::{BattleHistoryRules, FactionId, GameData, SettlementId, UnitTypeId};
use sim_battle::{BattleSim, SideId};
use sim_campaign::battle_history::{BattleHistory, BattleKind, BattleRecord, BattleSideRecord};
use sim_campaign::{ArmyId, ArmyPosition, CampaignState, EventKind, Order, Stance, Unit};

use data_model::test_support::{fac, game_data, prov};

fn saint_denis() -> SettlementId {
    SettlementId::new("set_saint_denis").unwrap()
}

fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

fn main_army(state: &CampaignState, faction: &str) -> ArmyId {
    let faction = fac(faction);
    state
        .armies()
        .iter()
        .filter(|(_, a)| a.faction == faction)
        .max_by_key(|(id, a)| (a.units.len(), std::cmp::Reverse((*id).clone())))
        .map(|(id, _)| id.clone())
        .expect("faction has an army")
}

/// France attacks an English army teleported to Saint-Denis: the battle
/// waits for the player (as in `tests/m7.rs`).
fn pending_battle(data: &GameData, seed: u64) -> (CampaignState, ArmyId, ArmyId) {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), seed).unwrap();
    let french = main_army(&state, "fac_france");
    let english = main_army(&state, "fac_england");
    state.armies.get_mut(&english).unwrap().position = ArmyPosition::Settlement(saint_denis());
    state
        .submit_order(
            data,
            Order::Attack {
                army: french.clone(),
                target_army: english.clone(),
            },
        )
        .unwrap();
    (state, french, english)
}

/// A campaign with one auto-resolved field battle at Saint-Denis.
fn after_a_battle(data: &GameData) -> CampaignState {
    let (mut state, _, _) = pending_battle(data, 3);
    state.auto_resolve_pending(data, 0).unwrap();
    assert_eq!(state.battle_history.len(), 1);
    state
}

fn record_at(turn: u32) -> BattleRecord {
    let side = |faction: &str| BattleSideRecord {
        faction: fac(faction),
        strength: 1000,
        losses: 100,
    };
    BattleRecord {
        turn,
        province: prov("prov_ile_de_france"),
        position: [10.0, 20.0],
        kind: BattleKind::Field,
        attacker: side("fac_france"),
        defender: side("fac_england"),
        attacker_won: true,
    }
}

fn rules(max_age_turns: u32, max_records: u32) -> BattleHistoryRules {
    BattleHistoryRules {
        max_age_turns,
        max_records,
        description: None,
    }
}

#[test]
fn rules_are_read_from_data_and_match_their_default() {
    let mut from_file = game_data().battle_history_rules.clone();
    assert!(
        from_file.description.is_some(),
        "battle_history.json not read"
    );
    from_file.description = None;
    assert_eq!(from_file, BattleHistoryRules::default());
    assert!(from_file.max_age_turns > 0 && from_file.max_records > 0);
}

#[test]
fn an_auto_resolved_field_battle_is_recorded() {
    let data = game_data();
    let (mut state, french, english) = pending_battle(data, 3);
    assert!(state.battle_history.is_empty(), "nothing fought yet");
    let strength = |state: &CampaignState, id: &ArmyId| state.army(id).unwrap().total_strength();
    let (french_before, english_before) = (strength(&state, &french), strength(&state, &english));
    let events = state.auto_resolve_pending(data, 0).unwrap();

    let records = state.battle_history.records();
    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(record.turn, state.turn());
    assert_eq!(record.age(state.turn()), 0);
    assert_eq!(record.kind, BattleKind::Field);
    assert_eq!(record.province, prov("prov_ile_de_france"));
    // The battlefield is the defender's place, in map pixels.
    assert_eq!(
        record.position,
        data.settlement_point(&saint_denis()).unwrap()
    );
    assert_eq!(record.attacker.faction, fac("fac_france"));
    assert_eq!(record.defender.faction, fac("fac_england"));
    assert_eq!(record.attacker.strength, french_before);
    assert_eq!(record.defender.strength, english_before);
    assert!(record.attacker.losses <= record.attacker.strength);
    assert!(record.defender.losses <= record.defender.strength);
    assert!(record.attacker.losses + record.defender.losses > 0);
    // Same winner and losses as the chronicle line.
    let line = events
        .iter()
        .find(|e| e.kind == EventKind::Battle && e.text_fr.contains("Vainqueur"))
        .expect("battle told");
    assert_eq!(line.faction.as_ref(), Some(record.winner()));
    assert!(
        line.text_fr.contains(&format!(
            "Pertes : {} contre {}.",
            record.attacker.losses, record.defender.losses
        )),
        "{}",
        line.text_fr
    );
}

#[test]
fn a_battle_fought_in_3d_is_recorded() {
    let data = game_data();
    let (mut state, _, _) = pending_battle(data, 3);
    let setup = state.battle_setup(data, 0).unwrap();
    let mut battle = BattleSim::new(setup, 11).unwrap();
    battle.set_ai(SideId::Attacker, true);
    let mut steps = 0;
    while !battle.is_finished() && steps < 36_000 {
        battle.step();
        steps += 1;
    }
    let outcome = battle.outcome().expect("battle finished");
    state.resolve_pending_battle(data, 0, &outcome).unwrap();

    let records = state.battle_history.records();
    assert_eq!(records.len(), 1);
    let record = &records[0];
    assert_eq!(record.kind, BattleKind::Field);
    assert_eq!(record.province, prov("prov_ile_de_france"));
    assert_eq!(record.attacker_won, outcome.winner == SideId::Attacker);
    assert_eq!(record.attacker.losses, outcome.attacker.total_losses);
    assert_eq!(record.defender.losses, outcome.defender.total_losses);
}

fn unit(data: &GameData, id: &str) -> Unit {
    let t = &data.unit_types[&UnitTypeId::new(id).unwrap()];
    Unit {
        unit_type: t.id.clone(),
        strength: 100,
        max_strength: 100,
        experience: 2,
        morale: 80,
        levy_armor: 0,
        levy_ranged: 0,
        experience_residue: 0,
    }
}

/// France's main army besieging the city of English Guyenne (as in
/// `tests/m8.rs`).
fn besiege_guyenne(data: &GameData, seed: u64) -> (CampaignState, ArmyId, SettlementId) {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), seed).unwrap();
    let army = state
        .armies
        .iter()
        .find(|(_, a)| a.faction == fac("fac_france"))
        .map(|(id, _)| id.clone())
        .unwrap();
    let guyenne = state
        .province_city_id(&prov("prov_guyenne"))
        .unwrap()
        .clone();
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
        state.armies.get_mut(&id).unwrap().position = ArmyPosition::Settlement(kent.clone());
    }
    let a = state.armies.get_mut(&army).unwrap();
    a.position = ArmyPosition::Settlement(guyenne.clone());
    a.stance = Stance::Siege;
    a.clear_plan();
    (state, army, guyenne)
}

#[test]
fn an_assault_is_recorded() {
    let data = game_data();
    let (mut state, army, guyenne) = besiege_guyenne(data, 4);
    state.interactive_battles = false;
    state.end_turn_with(data, idle);
    let ladders = data.siege_engine_rules.engines[0].cost(
        data.siege_engine_rules.scaling_min_wall_level,
        state.fortification_level(data, &guyenne),
    );
    if let Some(siege) = state
        .settlements
        .get_mut(&guyenne)
        .and_then(|s| s.siege.as_mut())
    {
        siege.engine_work = siege.engine_work.max(ladders);
    }
    let before = state.battle_history.len();
    let camp = state.army_point(data, &state.armies[&army]);
    state
        .submit_order(data, Order::Assault { army: army.clone() })
        .unwrap();
    let records = state.battle_history.records();
    assert_eq!(records.len(), before + 1);
    let record = records.last().unwrap();
    assert_eq!(record.kind, BattleKind::Assault);
    assert_eq!(record.province, prov("prov_guyenne"));
    assert_eq!(record.position, camp);
    assert_eq!(record.attacker.faction, fac("fac_france"));
    assert_eq!(record.defender.faction, fac("fac_england"));
    assert_eq!(record.turn, state.turn());
}

#[test]
fn a_garrison_sortie_is_recorded() {
    let data = game_data();
    let (mut state, army, guyenne) = besiege_guyenne(data, 6);
    state.armies.get_mut(&army).unwrap().units.truncate(1);
    let knights = unit(data, "unit_knights");
    let garrison = &mut state.settlements.get_mut(&guyenne).unwrap().garrison;
    for _ in 0..8 {
        garrison.push(knights.clone());
    }
    let events = state.end_turn_with(data, idle);
    assert!(events.iter().any(|e| e.text_fr.contains("Sortie")));
    let record = state
        .battle_history
        .records()
        .iter()
        .find(|r| r.kind == BattleKind::Sortie)
        .expect("sortie recorded");
    assert_eq!(record.province, prov("prov_guyenne"));
    // The garrison attacks, the besiegers defend.
    assert_eq!(record.attacker.faction, fac("fac_england"));
    assert_eq!(record.defender.faction, fac("fac_france"));
    assert_eq!(record.turn + 1, state.turn(), "fought during the turn");
}

#[test]
fn the_history_drops_old_battles_and_keeps_the_newest() {
    let mut history = BattleHistory::default();
    let bounds = rules(3, 4);
    for turn in [1, 2, 2, 3] {
        history.push(&bounds, record_at(turn));
    }
    assert_eq!(history.len(), 4);
    // Turn 4: the battle of turn 1 is 3 turns old.
    history.purge(&bounds, 4);
    let turns = |h: &BattleHistory| h.records().iter().map(|r| r.turn).collect::<Vec<_>>();
    assert_eq!(turns(&history), [2, 2, 3]);
    // A new battle purges as of its own turn.
    history.push(&bounds, record_at(5));
    assert_eq!(turns(&history), [3, 5]);
    // The cap drops the oldest first.
    for _ in 0..5 {
        history.push(&bounds, record_at(5));
    }
    assert_eq!(turns(&history), [5, 5, 5, 5]);
    // A zero age keeps nothing.
    history.push(&rules(0, 4), record_at(5));
    assert!(history.is_empty());
}

#[test]
fn old_battles_are_purged_as_turns_pass() {
    let mut data = game_data().clone();
    data.battle_history_rules.max_age_turns = 2;
    let mut state = after_a_battle(&data);
    let fought = state.turn();
    let ours = |state: &CampaignState| {
        state
            .battle_history
            .records()
            .iter()
            .filter(|r| r.turn == fought && r.province == prov("prov_ile_de_france"))
            .count()
    };
    state.interactive_battles = false;
    state.end_turn_with(&data, idle);
    assert_eq!(state.turn(), fought + 1);
    assert_eq!(ours(&state), 1, "one turn old: kept");
    state.end_turn_with(&data, idle);
    assert_eq!(ours(&state), 0, "two turns old: purged");
    assert!(state
        .battle_history
        .records()
        .iter()
        .all(|r| r.age(state.turn()) < 2));
}

#[test]
fn the_history_survives_a_save_round_trip() {
    let data = game_data();
    let state = after_a_battle(data);
    let json = state.save_json();
    assert!(json.contains("\"battle_history\""));
    let loaded = CampaignState::load_json(&json).unwrap();
    assert_eq!(loaded.battle_history, state.battle_history);
    assert_eq!(loaded, state);
    // The record reads as documented (save format).
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    let record = &value["battle_history"][0];
    assert_eq!(record["kind"], "field");
    assert_eq!(record["province"], "prov_ile_de_france");
    assert_eq!(record["attacker"]["faction"], "fac_france");
    assert!(record["position"].as_array().is_some_and(|p| p.len() == 2));
}

#[test]
fn a_save_without_the_field_loads_with_an_empty_history() {
    let data = game_data();
    let state = after_a_battle(data);
    // An older save: the same game, written before the history existed.
    let mut value: serde_json::Value = serde_json::from_str(&state.save_json()).unwrap();
    assert!(value
        .as_object_mut()
        .unwrap()
        .remove("battle_history")
        .is_some());
    let mut loaded = CampaignState::load_json(&value.to_string()).expect("older save loads");
    assert!(loaded.battle_history.is_empty());
    let mut expected = state.clone();
    expected.battle_history = BattleHistory::default();
    assert_eq!(loaded, expected, "nothing else changes");
    // A game without battles writes no key at all: its save is the one an
    // older build would have written.
    let fresh = CampaignState::new_1337(data, fac("fac_france"), 3).unwrap();
    assert!(!fresh.save_json().contains("battle_history"));
    // The loaded game goes on.
    loaded.interactive_battles = false;
    loaded.end_turn_with(data, idle);
    assert_eq!(loaded.turn(), state.turn() + 1);
}
