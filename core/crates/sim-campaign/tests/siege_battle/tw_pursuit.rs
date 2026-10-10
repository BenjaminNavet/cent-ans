//! TW pursuit (ADR 0321): standards and baggage pay gold and prestige,
//! regiments earn experience, the chronicle tells the pursuit.

use sim_battle::{BattleEnd, BattleOutcome, SideId, SideResult, StandardTrophy};
use sim_campaign::test_support::first_army;
use sim_campaign::{ArmyId, CampaignState};

use data_model::test_support::{fac, game_data};

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
    let count = state.armies[army].units.len();
    SideResult {
        losses: vec![0; count],
        morale_delta: if won { 5 } else { -20 },
        routed: !won,
        ..Default::default()
    }
}

fn milli(state: &CampaignState, army: &ArmyId) -> u32 {
    state.armies[army]
        .units
        .iter()
        .map(|u| u32::from(u.experience) * 1000 + u32::from(u.experience_residue))
        .sum()
}

fn prestige(state: &CampaignState, faction: &str) -> i32 {
    let ruler = state.factions[&fac(faction)].ruler.clone().unwrap();
    state.characters[&ruler].prestige
}

fn trophy(general: bool) -> StandardTrophy {
    StandardTrophy {
        unit_type: "unit_longbowmen".to_owned(),
        unit_name: "Archers anglais".to_owned(),
        faction: "fac_england".to_owned(),
        general,
        captor: None,
        time: 100.0,
        taken_by: SideId::Attacker,
    }
}

#[test]
fn standards_and_baggage_pay_gold_and_prestige() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 3).unwrap();
    state.chronicle.disabled = true;
    let (attacker, defender, index) = stage(&mut state);
    state
        .factions
        .get_mut(&fac("fac_england"))
        .unwrap()
        .treasury = 10_000;
    let (gold_a, gold_d) = (
        state.factions[&fac("fac_france")].treasury,
        state.factions[&fac("fac_england")].treasury,
    );
    let prestige_before = prestige(&state, "fac_france");
    let mut won = side(&state, &attacker, true);
    won.standards_taken = vec![trophy(false), trophy(true)];
    let mut lost = side(&state, &defender, false);
    lost.standards_lost = 2;
    lost.baggage_lost = true;
    let outcome = BattleOutcome {
        winner: SideId::Attacker,
        attacker: won,
        defender: lost,
        duration: 300.0,
        end: BattleEnd::Rout,
    };
    let spoils = &data.battle_outcome_rules.spoils;
    state.resolve_pending_battle(data, index, &outcome).unwrap();
    let gold = 2 * spoils.standard_gold + spoils.baggage_gold;
    assert!(gold > 0);
    assert_eq!(state.factions[&fac("fac_france")].treasury, gold_a + gold);
    assert_eq!(state.factions[&fac("fac_england")].treasury, gold_d - gold);
    // The class prestige of the victory comes on top of the trophies'.
    let trophies = 2 * spoils.standard_prestige + spoils.general_standard_prestige;
    assert!(prestige(&state, "fac_france") >= prestige_before + trophies);
}

#[test]
fn pursuit_is_told_and_regiments_earn_experience() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 3).unwrap();
    state.chronicle.disabled = true;
    let (attacker, defender, index) = stage(&mut state);
    let before = milli(&state, &defender);
    let won = side(&state, &attacker, true);
    let mut lost = side(&state, &defender, false);
    lost.unit_xp_milli = vec![200; lost.losses.len()];
    lost.captured = 4;
    lost.pursuit_losses = vec![0; lost.losses.len()];
    lost.pursuit_losses[0] = 3;
    lost.losses[0] = 7;
    let outcome = BattleOutcome {
        winner: SideId::Attacker,
        attacker: won,
        defender: lost,
        duration: 300.0,
        end: BattleEnd::Rout,
    };
    let units = state.armies[&defender].units.len() as u32;
    let events = state.resolve_pending_battle(data, index, &outcome).unwrap();
    let line = events
        .iter()
        .find(|e| e.text_fr.contains("Poursuite"))
        .expect("a pursuit line");
    assert!(
        line.text_fr.contains("3 tués, 4 prisonniers"),
        "{}",
        line.text_fr
    );
    // The beaten side gets no flat victory level: only its own 0.2 level
    // per regiment (units already at level 10 gain nothing). Losses of 7
    // men may drop a regiment, hence the upper bound only.
    let gained = milli(&state, &defender).saturating_sub(before);
    assert!(gained > 0 && gained <= 200 * units, "{gained}");
}

#[test]
fn a_refused_battle_pays_nothing() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 3).unwrap();
    state.chronicle.disabled = true;
    let (attacker, defender, index) = stage(&mut state);
    let before = milli(&state, &attacker);
    let gold = state.factions[&fac("fac_england")].treasury;
    let outcome = BattleOutcome {
        winner: SideId::Defender,
        attacker: side(&state, &attacker, false),
        defender: side(&state, &defender, true),
        duration: 300.0,
        end: BattleEnd::Refused,
    };
    let events = state.resolve_pending_battle(data, index, &outcome).unwrap();
    assert!(events.iter().all(|e| !e.text_fr.contains("Poursuite")));
    assert_eq!(milli(&state, &attacker), before);
    assert_eq!(state.factions[&fac("fac_england")].treasury, gold);
}
