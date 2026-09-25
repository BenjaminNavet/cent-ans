//! EP9 (ADR 0056): a refused battle reaches the campaign as a withdrawal of
//! the attacker, without losses, told in the chronicle.

use std::path::PathBuf;

use data_model::{FactionId, GameData};
use sim_battle::{BattleEnd, BattleOutcome, SideId, SideResult};
use sim_campaign::{ArmyId, CampaignState};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn first_army(state: &CampaignState, faction: &str) -> ArmyId {
    state
        .armies
        .iter()
        .find(|(_, a)| a.faction == fac(faction))
        .map(|(id, _)| id.clone())
        .unwrap()
}

fn untouched(state: &CampaignState, army: &ArmyId, morale_delta: i32, won: bool) -> SideResult {
    SideResult {
        losses: vec![0; state.armies[army].units.len()],
        total_losses: 0,
        morale_delta,
        routed: false,
        general_killed: false,
        general_captured: false,
        no_quarter: false,
        withdrew: !won,
        standards_taken: Vec::new(),
        standards_lost: 0,
    }
}

#[test]
fn a_refused_battle_is_told_and_costs_no_soldier() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 3).unwrap();
    state.chronicle.disabled = true;
    for (a, b) in [("fac_france", "fac_england"), ("fac_england", "fac_france")] {
        state
            .factions
            .get_mut(&fac(a))
            .unwrap()
            .at_war_with
            .insert(fac(b));
    }
    let attacker = first_army(&state, "fac_france");
    let defender = first_army(&state, "fac_england");
    let before: u32 = state.armies[&attacker].total_strength();
    let index = state.debug_stage_battle(&attacker, &defender).unwrap();
    let outcome = BattleOutcome {
        winner: SideId::Defender,
        attacker: untouched(&state, &attacker, -5, false),
        defender: untouched(&state, &defender, 2, true),
        duration: 300.0,
        end: BattleEnd::Refused,
    };
    let events = state
        .resolve_pending_battle(&data, index, &outcome)
        .unwrap();
    let line = events
        .iter()
        .find(|e| e.text_fr.contains("Bataille refusée"))
        .expect("a chronicle line for the refused battle");
    assert!(line.text_fr.contains("de France"), "{}", line.text_fr);
    let after = state
        .armies
        .get(&attacker)
        .map_or(0, |a| a.total_strength());
    assert_eq!(after, before, "a refused battle costs no soldier");
}
