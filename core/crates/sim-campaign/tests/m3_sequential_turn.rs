//! Lot M3: the sequential turn (spec `docs/design/2026-09-24-mouvement-libre.md`
//! § 3.4) — AI factions play one after the other in id order, their
//! battles against the player are auto-resolved with a notice in the season
//! report, then the season turns and the player's marches resume.

use std::cell::RefCell;
use std::path::PathBuf;

use data_model::{FactionId, GameData, SettlementId};
use sim_campaign::{ArmyId, ArmyPosition, CampaignState, EventKind, MoveTarget, Order};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn set(id: &str) -> SettlementId {
    SettlementId::new(id).unwrap()
}

fn main_army(state: &CampaignState, faction: &str) -> ArmyId {
    let faction = fac(faction);
    state
        .armies
        .iter()
        .filter(|(_, a)| a.faction == faction)
        .max_by_key(|(id, a)| (a.units.len(), std::cmp::Reverse((*id).clone())))
        .map(|(id, _)| id.clone())
        .expect("faction has an army")
}

#[test]
fn ai_factions_play_one_after_the_other_in_id_order() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    let played = RefCell::new(Vec::new());
    state.end_turn_with(&data, |_, _, faction| {
        played.borrow_mut().push(faction.clone());
        Vec::new()
    });
    let played = played.into_inner();
    let expected: Vec<FactionId> = state
        .factions
        .iter()
        .filter(|(id, f)| f.alive && id.as_str() != "fac_france")
        .map(|(id, _)| id.clone())
        .collect();
    assert_eq!(played, expected, "every living AI faction, by id, not the player");
}

#[test]
fn an_ai_attack_on_the_player_is_auto_resolved_and_reported() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    assert!(state.interactive_battles, "3D battles are on for the player");
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
    let meaux = data.settlement_point(&set("set_meaux")).unwrap();
    let km = sim_campaign::march::px_per_km(&data);
    state.armies.get_mut(&english).unwrap().position = ArmyPosition::field(meaux);
    state.armies.get_mut(&french).unwrap().position =
        ArmyPosition::field([meaux[0] + 15.0 * km, meaux[1]]);
    let (attacker, target) = (english.clone(), french.clone());
    let events = state.end_turn_with(&data, move |_, _, faction| {
        if faction.as_str() == "fac_england" {
            vec![Order::Attack {
                army: attacker.clone(),
                target_army: target.clone(),
            }]
        } else {
            Vec::new()
        }
    });
    assert!(
        state.pending_battles.is_empty(),
        "no 3D battle during the AI's turn"
    );
    let notice = events
        .iter()
        .find(|e| e.kind == EventKind::Battle && e.text_fr.starts_with("Pendant le tour"))
        .expect("the season report tells the player");
    assert_eq!(notice.faction.as_ref(), Some(&fac("fac_france")));
    assert!(
        events
            .iter()
            .any(|e| e.kind == EventKind::Battle && e.text_fr.contains("Vainqueur")),
        "the battle was fought"
    );
}

#[test]
fn the_player_attacking_still_gets_his_battle() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
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
    let meaux = data.settlement_point(&set("set_meaux")).unwrap();
    let km = sim_campaign::march::px_per_km(&data);
    state.armies.get_mut(&french).unwrap().position = ArmyPosition::field(meaux);
    state.armies.get_mut(&english).unwrap().position =
        ArmyPosition::field([meaux[0] + 15.0 * km, meaux[1]]);
    state
        .submit_order(
            &data,
            Order::Attack {
                army: french,
                target_army: english,
            },
        )
        .unwrap();
    assert_eq!(state.pending_battles.len(), 1, "left to the player (3D)");
}

#[test]
fn the_player_march_resumes_after_the_new_season() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    let french = main_army(&state, "fac_france");
    // Far beyond one season: Paris to Bayonne.
    let outcome = state
        .submit_order_outcome(
            &data,
            Order::MoveArmy {
                army: french.clone(),
                target: sim_campaign::MoveOrderTarget::Place(sim_campaign::Place::Settlement(
                    set("set_bayonne"),
                )),
            },
        )
        .unwrap();
    let sim_campaign::OrderOutcome::Moved(report) = outcome else {
        panic!("a march");
    };
    assert!(!report.planned_path.is_empty(), "several seasons away");
    let before = state.army_point(&data, &state.armies[&french]);
    state.end_turn_with(&data, |_, _, _| Vec::new());
    let army = &state.armies[&french];
    assert_ne!(state.army_point(&data, army), before, "marched on");
    assert!(
        army.movement_left < state.army_grid_allowance(&data, army),
        "with the new season's points"
    );
    assert_eq!(
        army.destination,
        Some(MoveTarget::Settlement(set("set_bayonne")))
    );
}
