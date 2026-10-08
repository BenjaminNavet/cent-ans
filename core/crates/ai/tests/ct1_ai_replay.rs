//! Lot CT1: the record of the AI turn replayed on the campaign map (ADR
//! 0073) — deterministic, harmless to the game, and telling which moves
//! concern the player.
use data_model::test_support::{fac, game_data};
use sim_campaign::test_support::main_army;

use data_model::{GameData, SettlementId};
use sim_campaign::{
    AiMoveKind, AiMoveNotability, AiMoveRecord, ArmyPosition, CampaignState, Order,
};

fn data() -> &'static GameData {
    ai::feudal::install();
    game_data()
}

/// Plays `turns` turns with the strategic AI, the record on or off; returns
/// the records of every turn and the final save.
fn play(data: &GameData, seed: u64, turns: u32, record: bool) -> (Vec<Vec<AiMoveRecord>>, String) {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), seed).unwrap();
    let mut all = Vec::new();
    for _ in 0..turns {
        state.set_ai_replay_recording(record, 60.0);
        state.end_turn_with(data, ai::plan_turn);
        all.push(state.ai_turn_moves().to_vec());
    }
    (all, state.save_json())
}

#[test]
fn the_record_is_deterministic_and_does_not_change_the_game() {
    let data = data();
    let _ = play(data, 3, 1, false); // warm-up (lazy rasters)
    let started = std::time::Instant::now();
    let (off, save_off) = play(data, 3, 3, false);
    let off_time = started.elapsed();
    let started = std::time::Instant::now();
    let (first, save_on) = play(data, 3, 3, true);
    let on_time = started.elapsed();
    let (second, _) = play(data, 3, 3, true);
    eprintln!("3 turns: record on {on_time:?}, off {off_time:?}");
    assert_eq!(first, second, "same seed, same records");
    assert!(
        first.iter().all(|turn| !turn.is_empty()),
        "AI armies move every turn: {:?}",
        first.iter().map(Vec::len).collect::<Vec<_>>()
    );
    assert!(off.iter().all(Vec::is_empty), "nothing recorded when off");
    for turn in &first {
        eprintln!(
            "moves {} visible {} notable {}",
            turn.len(),
            turn.iter().filter(|r| r.visible).count(),
            turn.iter().filter(|r| r.notable.is_some()).count()
        );
    }
    assert_eq!(save_on, save_off, "recording never changes the game");
    for turn in &first {
        for (index, record) in turn.iter().enumerate() {
            assert_eq!(record.sequence as usize, index);
            assert!(record.path.len() >= 2, "start and at least one step");
            assert_ne!(record.faction, fac("fac_france"), "AI armies only");
            if record.visible {
                assert!(record.visible_from <= record.visible_to);
                assert!(record.visible_to < record.path.len());
            }
        }
    }
    // Minor factions far from France move too, unseen and not notable.
    assert!(first
        .iter()
        .flatten()
        .any(|r| !r.visible && r.notable.is_none()));
}

#[test]
fn an_ai_attack_on_the_player_is_a_visible_notable_battle() {
    let data = data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 1).unwrap();
    for (a, b) in [("fac_england", "fac_france"), ("fac_france", "fac_england")] {
        state
            .factions
            .get_mut(&fac(a))
            .unwrap()
            .at_war_with
            .insert(fac(b));
    }
    let english = main_army(&state, "fac_england");
    let french = main_army(&state, "fac_france");
    let meaux = data
        .settlement_point(&SettlementId::new("set_meaux").unwrap())
        .unwrap();
    let km = sim_campaign::march::px_per_km(data);
    state.armies.get_mut(&english).unwrap().position = ArmyPosition::field(meaux);
    state.armies.get_mut(&french).unwrap().position =
        ArmyPosition::field([meaux[0] + 15.0 * km, meaux[1]]);
    state.set_ai_replay_recording(true, 60.0);
    let (attacker, target) = (english.clone(), french.clone());
    state.end_turn_with(data, move |_, _, faction| {
        if faction.as_str() == "fac_england" {
            vec![Order::Attack {
                army: attacker.clone(),
                target_army: target.clone(),
            }]
        } else {
            Vec::new()
        }
    });
    let moves = state.ai_turn_moves();
    let battle = moves
        .iter()
        .find(|r| r.army == english)
        .expect("the English march is recorded");
    assert_eq!(battle.kind, AiMoveKind::Battle);
    assert_eq!(battle.target_army.as_ref(), Some(&french));
    assert_eq!(battle.target_faction.as_ref(), Some(&fac("fac_france")));
    assert_eq!(battle.notable, Some(AiMoveNotability::Battle));
    assert!(battle.visible, "fought next to a French army");
    assert_eq!(battle.path.first(), Some(&meaux), "starts where it stood");
}
