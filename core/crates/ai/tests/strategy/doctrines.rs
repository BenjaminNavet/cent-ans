//! IA-DOCTRINES (ADR 0782-0783): the strategic doctrines of the AI, their assignment by faction,
//! sovereign and defeats, their effect on the planner, the named reason of a war, and the
//! behaviour tiers of the difficulty levels. Fixed seed, no clock: deterministic.

use data_model::test_support::fac;
use data_model::{DifficultyModifiers, StrategicDoctrines};
use sim_campaign::causes::CauseKind;
use sim_campaign::difficulty::Difficulty;
use sim_campaign::doctrines;
use sim_campaign::{CampaignState, Order};

use crate::common::data;

const SEED: u64 = 1337;

fn start(player: &str) -> CampaignState {
    CampaignState::new_1337(data(), fac(player), SEED).expect("1337 campaign")
}

#[test]
fn the_catalogue_names_every_historical_doctrine_and_the_data_resolve() {
    let catalogue = StrategicDoctrines::bundled();
    for id in [
        "chevauchee_pillage",
        "attrition_fortresses",
        "chivalric_battle",
        "feudal_marriage",
        "merchant_credit",
        "crusade_frontier",
        "passive_defence",
        "balanced",
    ] {
        let doctrine = catalogue.doctrines.get(id).expect(id);
        assert!(!doctrine.label.is_empty() && !doctrine.description.is_empty());
    }
    // Every faction an assignment names exists.
    for a in &catalogue.assignments {
        if let Some(f) = &a.condition.faction {
            assert!(data().factions.contains_key(f), "unknown faction {f}");
        }
        if let Some(r) = &a.condition.ruler {
            assert!(data().characters.contains_key(r), "unknown ruler {r}");
        }
    }
}

#[test]
fn each_faction_follows_its_historical_doctrine_in_1337() {
    let state = start("fac_france");
    for (faction, expected) in [
        ("fac_england", "chevauchee_pillage"),
        ("fac_france", "chivalric_battle"),
        ("fac_burgundy", "feudal_marriage"),
        ("fac_austria", "feudal_marriage"),
        ("fac_flanders", "merchant_credit"),
        ("fac_genoa", "merchant_credit"),
        ("fac_teutonic", "crusade_frontier"),
        ("fac_hungary", "crusade_frontier"),
        ("fac_swiss", "passive_defence"),
        ("fac_castile", "balanced"),
    ] {
        assert_eq!(
            doctrines::active(&state, &fac(faction)).id,
            expected,
            "{faction}"
        );
    }
}

#[test]
fn a_new_sovereign_or_a_run_of_defeats_changes_the_doctrine() {
    let mut state = start("fac_england");
    let france = fac("fac_france");
    let england = fac("fac_england");
    assert_eq!(doctrines::active(&state, &france).id, "chivalric_battle");
    // Charles V succeeds (the ruler of the assignment).
    state.factions.get_mut(&france).unwrap().ruler =
        Some(data_model::CharacterId::new("chr_charles_v").unwrap());
    assert_eq!(
        doctrines::active(&state, &france).id,
        "attrition_fortresses"
    );
    state.factions.get_mut(&france).unwrap().ruler =
        Some(data_model::CharacterId::new("chr_philippe_vi").unwrap());
    assert_eq!(doctrines::active(&state, &france).id, "chivalric_battle");
    // Two defeats: not yet; three: the realm turns to attrition.
    for _ in 0..2 {
        state.chronicle.record_victory(&england, &france);
    }
    assert_eq!(doctrines::active(&state, &france).id, "chivalric_battle");
    state.chronicle.record_victory(&england, &france);
    assert_eq!(
        doctrines::active(&state, &france).id,
        "attrition_fortresses"
    );
    // The English victories do not change England's doctrine.
    assert_eq!(doctrines::active(&state, &england).id, "chevauchee_pillage");
}

#[test]
fn an_assignment_reads_years_and_the_first_matching_line_wins() {
    let state = start("fac_france");
    let catalogue: StrategicDoctrines = serde_json::from_str(
        r#"{
          "default": "balanced",
          "tuning": { "revenge_min_net_defeats": 1, "revenge_war_discount_percent": 30, "revenge_target_bonus": 30.0 },
          "doctrines": {},
          "assignments": []
        }"#,
    )
    .expect("parse");
    // Borrow the bundled doctrines for a catalogue with era lines.
    let mut catalogue = StrategicDoctrines {
        doctrines: StrategicDoctrines::bundled().doctrines.clone(),
        ..catalogue
    };
    catalogue.assignments = serde_json::from_str(
        r#"[
          { "if": { "faction": "fac_france", "from_year": 1400 }, "doctrine": "passive_defence" },
          { "if": { "faction": "fac_france", "to_year": 1340 }, "doctrine": "chivalric_battle" },
          { "if": { "faction": "fac_france" }, "doctrine": "attrition_fortresses" }
        ]"#,
    )
    .expect("assignments");
    let (id, _) = doctrines::active_in(&catalogue, &state, &fac("fac_france"));
    assert_eq!(state.year, 1337);
    assert_eq!(id, "chivalric_battle");
    let (other, _) = doctrines::active_in(&catalogue, &state, &fac("fac_castile"));
    assert_eq!(other, "balanced");
}

fn strategy_of(state: &CampaignState, faction: &str) -> ai::strategy::Strategy {
    ai::strategy::Strategy::of(state, data(), &fac(faction))
}

#[test]
fn the_doctrines_order_the_risk_taken_in_battle() {
    let state = start("fac_england");
    let engage = |f: &str| strategy_of(&state, f).engage_factor();
    let retreat = |f: &str| strategy_of(&state, f).retreat_factor();
    // Chivalry seeks battle, the merchant and the passive realm avoid it.
    assert!(engage("fac_france") < engage("fac_england"));
    assert!(engage("fac_england") < engage("fac_castile"));
    assert!(
        (engage("fac_castile") - 1.0).abs() < 1e-9,
        "balanced is neutral"
    );
    assert!(engage("fac_castile") < engage("fac_flanders"));
    assert!(engage("fac_flanders") < engage("fac_swiss"));
    // The chivalric host falls back late; the passive one early.
    assert!(retreat("fac_france") < retreat("fac_castile"));
    assert!(retreat("fac_castile") < retreat("fac_swiss"));
    // The crusader order is bold, the attrition doctrine siege-minded.
    assert!(engage("fac_teutonic") < 1.0);
    let mut attrition = start("fac_england");
    let france = fac("fac_france");
    for _ in 0..3 {
        attrition
            .chronicle
            .record_victory(&fac("fac_england"), &france);
    }
    let strategy = strategy_of(&attrition, "fac_france");
    assert_eq!(strategy.doctrine.id, "attrition_fortresses");
    assert!(strategy.engage_factor() > 1.2, "avoids battle");
    assert!(strategy.siege_factor() < 1.0, "takes places by siege");
    assert!(strategy.revenge() > 1.5, "avenges the defeats");
}

#[test]
fn the_objectives_weigh_the_values_of_the_targets() {
    let state = start("fac_england");
    assert!(
        strategy_of(&state, "fac_genoa").treasury() > strategy_of(&state, "fac_castile").treasury()
    );
    assert!(
        strategy_of(&state, "fac_swiss").security() > strategy_of(&state, "fac_england").security()
    );
    assert!(
        strategy_of(&state, "fac_england").conquest()
            > strategy_of(&state, "fac_flanders").conquest()
    );
    assert!(
        strategy_of(&state, "fac_france").prestige() > strategy_of(&state, "fac_swiss").prestige()
    );
}

#[test]
fn finance_orders_follow_the_doctrine() {
    let mut state = start("fac_england");
    let orders = || {
        vec![
            Order::Borrow {
                lender: "lender".to_owned(),
                amount: 100,
            },
            Order::LevyExtraordinary {
                levy: "levy".to_owned(),
            },
            Order::Repay { amount: 5 },
        ]
    };
    for (faction, kept) in [
        ("fac_genoa", 3),
        ("fac_england", 3),
        ("fac_swiss", 1),
        ("fac_castile", 3),
    ] {
        state.factions.get_mut(&fac(faction)).unwrap().treasury = 1_000;
        let filtered = ai::strategy::filter_finance_orders(&state, &fac(faction), orders());
        assert_eq!(filtered.len(), kept, "{faction}");
    }
    // A broke passive realm borrows all the same.
    state.factions.get_mut(&fac("fac_swiss")).unwrap().treasury = -10;
    assert_eq!(
        ai::strategy::filter_finance_orders(&state, &fac("fac_swiss"), orders()).len(),
        3
    );
}

#[test]
fn revenge_is_owed_to_the_rival_that_won_more_battles() {
    let mut state = start("fac_england");
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    assert!(doctrines::revenge_on(&state, &france, &england).is_none());
    assert!((doctrines::revenge_war_factor(&state, &france, &england) - 1.0).abs() < 1e-9);
    state.chronicle.record_victory_at(
        &england,
        &france,
        Some(
            &data_model::ProvinceId::new("prov_picardy")
                .unwrap_or_else(|_| data().provinces.keys().next().expect("a province").clone()),
        ),
    );
    let revenge = doctrines::revenge_on(&state, &france, &england).expect("revenge");
    assert_eq!(revenge.net_defeats, 1);
    assert!(revenge.place.is_some());
    // The French doctrine (revenge 70) demands less than the balanced one for the war.
    assert!(doctrines::revenge_war_factor(&state, &france, &england) < 0.85);
    // A victory back cancels the debt.
    state.chronicle.record_victory(&france, &england);
    assert!(doctrines::revenge_on(&state, &france, &england).is_none());
}

/// Finds a pair of factions able to declare war and plays the declaration as the AI of `attacker`.
#[test]
fn an_ai_war_names_its_doctrine_and_the_revenge_in_the_cause_journal() {
    let data = data();
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    let mut state = start("fac_england");
    // Peace first, so that a declaration is possible; England beat France twice at one place.
    state
        .factions
        .get_mut(&france)
        .unwrap()
        .at_war_with
        .remove(&england);
    state
        .factions
        .get_mut(&england)
        .unwrap()
        .at_war_with
        .remove(&france);
    let place = data.provinces.keys().next().expect("province").clone();
    for _ in 0..2 {
        state
            .chronicle
            .record_victory_at(&england, &france, Some(&place));
    }
    let planner = |s: &CampaignState, _d: &data_model::GameData, f: &data_model::FactionId| {
        if f == &fac("fac_france") && s.turn == 0 {
            vec![Order::DeclareWar {
                target: fac("fac_england"),
            }]
        } else {
            Vec::new()
        }
    };
    state.end_turn_with(data, planner);
    let record = state
        .cause_journal()
        .records()
        .find(|r| r.cause.kind == CauseKind::WarDeclared && r.cause.actor.as_ref() == Some(&france))
        .expect("France's war is in the journal (England is the player)");
    let reason = record.cause.reason.as_ref().expect("AI reason");
    assert_eq!(reason.key, "revenge_at");
    assert_eq!(reason.doctrine.as_deref(), Some("chivalric_battle"));
    assert_eq!(record.cause.province.as_ref(), Some(&place));
}

#[test]
fn every_doctrine_plans_deterministically_and_in_parallel_like_in_sequence() {
    let data = data();
    let mut state = start("fac_england");
    // The turn after the start, with armies, treasuries and rivals as the 1337 set-up gives them.
    for faction in [
        "fac_england",
        "fac_france",
        "fac_burgundy",
        "fac_flanders",
        "fac_teutonic",
        "fac_swiss",
        "fac_castile",
    ] {
        let id = fac(faction);
        let first = ai::plan_turn(&state, data, &id);
        let second = ai::plan_turn(&state, data, &id);
        let sequential = ai::plan_turn_sequential(&state, data, &id);
        assert_eq!(first, second, "{faction}");
        assert_eq!(first, sequential, "{faction}");
    }
    // And a few turns later, the doctrines still produce the same orders from the same state.
    for _ in 0..6 {
        state.end_turn_with(data, ai::plan_turn);
    }
    let again = state.clone();
    for faction in ["fac_england", "fac_france"] {
        let id = fac(faction);
        assert_eq!(
            ai::plan_turn(&state, data, &id),
            ai::plan_turn(&again, data, &id),
            "{faction}"
        );
    }
}

#[test]
fn the_difficulty_levels_add_behaviour_tiers_not_only_numbers() {
    let data = data();
    let level = |id: &str| data.difficulty.modifiers(id);
    let (easy, normal, hard, very_hard) = (
        level("easy"),
        level("normal"),
        level("hard"),
        level("very_hard"),
    );
    // Normal is exactly neutral, so the baseline is the game as balanced.
    assert_eq!(normal, DifficultyModifiers::NEUTRAL);
    // Prudence grows with the level.
    assert!(easy.ai_prudence_percent < 100);
    assert!(hard.ai_prudence_percent > 100);
    assert!(very_hard.ai_prudence_percent > hard.ai_prudence_percent);
    // Anticipation, coordination and fog of war.
    assert_eq!(easy.ai_anticipation, 0);
    assert!(hard.ai_anticipation >= 1 && very_hard.ai_anticipation >= 1);
    assert_eq!(hard.ai_coordination, 0);
    assert!(very_hard.ai_coordination >= 1);
    assert!(easy.ai_vision_provinces > 0, "the easy AI is short-sighted");
    assert_eq!(normal.ai_vision_provinces, 0);
    assert_eq!(hard.ai_vision_provinces, 0);
}

#[test]
fn the_levels_change_the_strategy_an_ai_plays_by() {
    let data = data();
    let mut state = start("fac_england");
    let strategy = |state: &CampaignState| strategy_of(state, "fac_france");
    state.set_difficulty(Difficulty::Easy);
    let easy = strategy(&state);
    assert!(easy
        .visible_provinces(&state, data, &fac("fac_france"))
        .is_some());
    assert!(!easy.anticipates() && !easy.coordinated());
    let mut hard_state = start("fac_england");
    hard_state.set_difficulty(Difficulty::VeryHard);
    let very_hard = strategy(&hard_state);
    assert!(very_hard.anticipates() && very_hard.coordinated());
    assert!(very_hard
        .visible_provinces(&hard_state, data, &fac("fac_france"))
        .is_none());
    // The prudence of the level scales the superiority demanded before attacking.
    assert!(very_hard.engage_factor() > easy.engage_factor());
    // The player's own faction is never handicapped by the AI levels.
    let own = strategy_of(&hard_state, "fac_england");
    assert_eq!(own.difficulty, DifficultyModifiers::NEUTRAL);
}

#[test]
fn the_fog_of_war_hides_far_armies_from_the_easy_ai() {
    let data = data();
    let mut state = start("fac_england");
    state.set_difficulty(Difficulty::Easy);
    let france = fac("fac_france");
    let strategy = strategy_of(&state, "fac_france");
    let visible = strategy
        .visible_provinces(&state, data, &france)
        .expect("fog on easy");
    let own: Vec<_> = state
        .provinces
        .keys()
        .filter(|p| state.holds_province(&france, p))
        .collect();
    assert!(
        own.iter().all(|p| visible.contains(*p)),
        "its own lands are seen"
    );
    assert!(
        visible.len() < state.provinces.len(),
        "the far provinces are hidden"
    );
}
