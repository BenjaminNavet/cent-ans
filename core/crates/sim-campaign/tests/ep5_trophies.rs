//! EP5 (ADR 0034): standards taken in battle reach the campaign chronicle,
//! and the ruler leading his army is flagged for the royal banner.

use sim_battle::{BattleOutcome, SideId, SideResult, StandardTrophy};
use sim_campaign::{ArmyId, CampaignState};

use data_model::test_support::{fac, game_data};

fn first_army(state: &CampaignState, faction: &str) -> ArmyId {
    state
        .armies
        .iter()
        .find(|(_, a)| a.faction == fac(faction))
        .map(|(id, _)| id.clone())
        .unwrap()
}

fn stage(state: &mut CampaignState) -> (ArmyId, ArmyId, usize) {
    for (a, b) in [("fac_france", "fac_england"), ("fac_england", "fac_france")] {
        state
            .factions
            .get_mut(&fac(a))
            .unwrap()
            .at_war_with
            .insert(fac(b));
    }
    let attacker = first_army(state, "fac_france");
    let defender = first_army(state, "fac_england");
    let index = state.debug_stage_battle(&attacker, &defender).unwrap();
    (attacker, defender, index)
}

fn side(state: &CampaignState, army: &ArmyId, won: bool) -> SideResult {
    let losses: Vec<u32> = state.armies[army]
        .units
        .iter()
        .map(|u| u.strength / 4)
        .collect();
    SideResult {
        total_losses: losses.iter().sum(),
        losses,
        morale_delta: if won { 5 } else { -20 },
        routed: !won,
        general_killed: false,
        general_captured: false,
        no_quarter: false,
        withdrew: false,
        standards_taken: Vec::new(),
        standards_lost: 0,
        baggage_lost: false,
    }
}

#[test]
fn trophies_are_told_in_the_chronicle() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 3).unwrap();
    state.chronicle.disabled = true;
    let (attacker, defender, index) = stage(&mut state);
    let mut won = side(&state, &attacker, true);
    won.standards_taken = vec![
        StandardTrophy {
            unit_type: "unit_longbowmen".to_owned(),
            unit_name: "Archers anglais".to_owned(),
            faction: "fac_england".to_owned(),
            general: false,
            captor: Some("Chevaliers".to_owned()),
            time: 120.0,
            taken_by: SideId::Attacker,
        };
        2
    ];
    let mut lost = side(&state, &defender, false);
    lost.standards_lost = 2;
    let outcome = BattleOutcome {
        winner: SideId::Attacker,
        attacker: won,
        defender: lost,
        duration: 300.0,
        end: Default::default(),
    };
    let events = state.resolve_pending_battle(data, index, &outcome).unwrap();
    let line = events
        .iter()
        .find(|e| e.text_fr.contains("étendards"))
        .expect("a chronicle line for the trophies");
    assert!(line.text_fr.contains("2 étendards"), "{}", line.text_fr);
    assert!(line.text_fr.contains("de France"), "{}", line.text_fr);
}

#[test]
fn no_trophy_no_line() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 3).unwrap();
    state.chronicle.disabled = true;
    let (attacker, defender, index) = stage(&mut state);
    let outcome = BattleOutcome {
        winner: SideId::Attacker,
        attacker: side(&state, &attacker, true),
        defender: side(&state, &defender, false),
        duration: 300.0,
        end: Default::default(),
    };
    let events = state.resolve_pending_battle(data, index, &outcome).unwrap();
    assert!(events.iter().all(|e| !e.text_fr.contains("étendard")));
}
