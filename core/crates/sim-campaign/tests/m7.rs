//! M7 integration: player battles wait for the 3D battle (spec
//! `docs/design/m7-battles.md` § 2).

use std::path::PathBuf;

use data_model::{FactionId, GameData, ProvinceId, SettlementId};
use sim_battle::{BattleSim, SideId};
use sim_campaign::{ArmyId, BattleRequestError, CampaignState, EventKind, Order};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn prov(id: &str) -> ProvinceId {
    ProvinceId::new(id).unwrap()
}

/// Lot C4: the battlefield, Saint-Denis next to Paris (the French start).
fn battlefield() -> SettlementId {
    SettlementId::new("set_saint_denis").unwrap()
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

fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

/// France marches on an English army teleported to Saint-Denis: returns the
/// state after the turn, and both army ids.
fn pending_battle(data: &GameData, seed: u64) -> (CampaignState, ArmyId, ArmyId) {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), seed).unwrap();
    let french = main_army(&state, "fac_france");
    let english = main_army(&state, "fac_england");
    state.armies.get_mut(&english).unwrap().location = battlefield();
    state
        .submit_order(data, Order::move_along(french.clone(), vec![battlefield()]))
        .unwrap();
    state.end_turn_with(data, idle);
    (state, french, english)
}

#[test]
fn player_battle_waits_in_pending_battles() {
    let data = data();
    let mut reference = CampaignState::new_1337(&data, fac("fac_france"), 3).unwrap();
    assert!(reference.interactive_battles, "on by default");
    let (state, french, english) = pending_battle(&data, 3);
    assert_eq!(state.pending_battles.len(), 1);
    let request = &state.pending_battles[0];
    assert_eq!(request.attacker, french);
    assert_eq!(request.defender, english);
    assert_eq!(request.province, prov("prov_ile_de_france"));
    assert_eq!(request.location, battlefield());
    // Nobody fought yet: both armies stand at Saint-Denis at full strength.
    let french_army = state.army(&french).unwrap();
    assert_eq!(french_army.location, battlefield());
    assert!(french_army.path.is_empty());
    reference.end_turn_with(&data, idle);
    assert_eq!(
        french_army.total_strength(),
        reference.army(&french).unwrap().total_strength(),
        "no casualties before the battle"
    );
    assert!(state
        .events()
        .iter()
        .any(|e| e.kind == EventKind::Battle && e.text_fr.contains("Bataille en vue")));
    let views = state.pending_battle_views(&data);
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].player_side, Some(SideId::Attacker));
    assert_eq!(views[0].attacker_name, "France");
    assert_eq!(views[0].defender_name, "Angleterre");
}

#[test]
fn battle_setup_describes_both_armies() {
    let data = data();
    let (state, french, english) = pending_battle(&data, 3);
    let setup = state.battle_setup(&data, 0).unwrap();
    assert_eq!(setup.player_side, Some(SideId::Attacker));
    assert_eq!(setup.province, "prov_ile_de_france");
    assert_eq!(
        setup.attacker.units.len(),
        state.army(&french).unwrap().units.len()
    );
    assert_eq!(
        setup.defender.units.len(),
        state.army(&english).unwrap().units.len()
    );
    let first = &setup.attacker.units[0];
    let campaign = &state.army(&french).unwrap().units[0];
    assert_eq!(first.soldiers, campaign.strength);
    assert_eq!(first.unit_type, campaign.unit_type.to_string());
    if state.army(&french).unwrap().general.is_some() {
        let general = setup.attacker.general.as_ref().expect("general carried");
        assert!(general.unit_index < setup.attacker.units.len());
    }
    assert!(matches!(
        state.battle_setup(&data, 5),
        Err(BattleRequestError::UnknownBattle(5))
    ));
    // The setup round-trips through JSON (the GDExtension format).
    let json = serde_json::to_string(&setup).unwrap();
    assert_eq!(
        serde_json::from_str::<sim_battle::BattleSetup>(&json).unwrap(),
        setup
    );
}

#[test]
fn external_battle_result_is_applied() {
    let data = data();
    let (mut state, french, english) = pending_battle(&data, 3);
    let before_french = state.army(&french).unwrap().total_strength();
    let before_english = state.army(&english).unwrap().total_strength();
    let setup = state.battle_setup(&data, 0).unwrap();
    let mut battle = BattleSim::new(setup, 11).unwrap();
    battle.set_ai(SideId::Attacker, true);
    let mut steps = 0;
    while !battle.is_finished() && steps < 36_000 {
        battle.step();
        steps += 1;
    }
    let outcome = battle.outcome().expect("battle finished");

    // A malformed outcome is refused and the battle stays pending.
    let mut broken = outcome.clone();
    broken.attacker.losses.pop();
    assert!(matches!(
        state.resolve_pending_battle(&data, 0, &broken),
        Err(BattleRequestError::UnitCountMismatch { .. })
    ));
    assert_eq!(state.pending_battles.len(), 1);

    let events = state.resolve_pending_battle(&data, 0, &outcome).unwrap();
    assert!(state.pending_battles.is_empty());
    assert!(events
        .iter()
        .any(|e| e.kind == EventKind::Battle && e.text_fr.starts_with("Bataille ")));
    assert!(state
        .events()
        .iter()
        .any(|e| e.text_fr.starts_with("Bataille ")));
    let after = |id: &ArmyId| state.army(id).map_or(0, |a| a.total_strength());
    // Losses applied (units under 5 % are disbanded on top).
    assert!(outcome.attacker.total_losses + outcome.defender.total_losses > 0);
    assert!(after(&french) <= before_french - outcome.attacker.total_losses);
    assert!(after(&english) <= before_english - outcome.defender.total_losses);
    // The loser left the battlefield.
    let (loser, _) = match outcome.winner {
        SideId::Attacker => (&english, &french),
        SideId::Defender => (&french, &english),
    };
    if let Some(army) = state.army(loser) {
        // Lot C4: a beaten attacker falls back to its origin; a beaten
        // defender to a neighbouring friendly settlement, if there is one.
        let can_retreat = loser == &french
            || sim_campaign::movement::edges(&data, &battlefield())
                .iter()
                .any(|(n, _)| state.is_friendly_settlement(&army.faction, n));
        if can_retreat {
            assert_ne!(army.location, battlefield(), "the loser retreats");
        }
    }
}

#[test]
fn auto_resolution_button_resolves_now() {
    let data = data();
    let (mut state, _, _) = pending_battle(&data, 3);
    let events = state.auto_resolve_pending(&data, 0).unwrap();
    assert!(state.pending_battles.is_empty());
    assert!(events
        .iter()
        .any(|e| e.kind == EventKind::Battle && e.text_fr.contains("Vainqueur")));
    assert!(matches!(
        state.auto_resolve_pending(&data, 0),
        Err(BattleRequestError::UnknownBattle(0))
    ));
}

#[test]
fn leftover_battles_are_auto_resolved_next_turn() {
    let data = data();
    let (mut state, french, _) = pending_battle(&data, 3);
    let before = state.army(&french).unwrap().total_strength();
    let events = state.end_turn_with(&data, idle);
    assert!(events
        .iter()
        .any(|e| e.kind == EventKind::Battle && e.text_fr.contains("Vainqueur")));
    assert!(
        state.army(&french).map_or(0, |a| a.total_strength()) < before,
        "the battle was fought"
    );
}

#[test]
fn ai_battles_stay_automatic_and_saves_keep_pending_battles() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 21).unwrap();
    for _ in 0..12 {
        state.end_turn(&data);
        for view in state.pending_battle_views(&data) {
            assert!(view.player_side.is_some(), "only player battles wait");
        }
    }
    let (state, _, _) = pending_battle(&data, 3);
    let json = state.save_json();
    assert!(json.contains(&format!(
        "\"state_version\":{}",
        sim_campaign::STATE_VERSION
    )));
    let loaded = CampaignState::load_json(&json).unwrap();
    assert_eq!(loaded.pending_battles, state.pending_battles);
    assert!(loaded.interactive_battles);
}

#[test]
fn fallen_general_dies_in_the_campaign() {
    let data = data();
    let (mut state, french, _) = pending_battle(&data, 3);
    let Some(general) = state.army(&french).unwrap().general.clone() else {
        return;
    };
    let setup = state.battle_setup(&data, 0).unwrap();
    let mut battle = BattleSim::new(setup, 4).unwrap();
    while !battle.is_finished() {
        battle.tick(10.0);
    }
    let mut outcome = battle.outcome().unwrap();
    outcome.attacker.general_killed = true;
    state.resolve_pending_battle(&data, 0, &outcome).unwrap();
    assert!(!state.character(&general).unwrap().alive);
    assert!(state
        .events()
        .iter()
        .any(|e| e.text_fr.contains("tombé sur le champ de bataille")));
}

#[test]
fn interactive_battles_can_be_turned_off() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 3).unwrap();
    state.interactive_battles = false;
    let french = main_army(&state, "fac_france");
    let english = main_army(&state, "fac_england");
    state.armies.get_mut(&english).unwrap().location = battlefield();
    state
        .submit_order(&data, Order::move_along(french, vec![battlefield()]))
        .unwrap();
    let events = state.end_turn_with(&data, idle);
    assert!(state.pending_battles.is_empty());
    assert!(events.iter().any(|e| e.text_fr.contains("Vainqueur")));
}
