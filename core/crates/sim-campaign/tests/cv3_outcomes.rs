//! Lot CV3-1: nuanced battle outcomes (spec
//! `docs/design/2026-09-27-campagne-vivante.md` § 3): classification
//! thresholds, consequences (prestige, XP, morale modifiers that wear off)
//! and the report kept for the UI.
use data_model::test_support::{fac, game_data};
use sim_campaign::test_support::main_army;

use data_model::{BattleOutcomeClass, BattleOutcomeRules};
use sim_campaign::battle_outcome::{classify, SideTally};
use sim_campaign::movement::side_from_army;
use sim_campaign::{ArmyPosition, CampaignState, MoraleModifier, Order};

fn tally(strength: u32, losses: u32) -> SideTally {
    SideTally {
        strength,
        losses,
        general_lost: false,
    }
}

fn rules() -> BattleOutcomeRules {
    BattleOutcomeRules::default()
}

#[test]
fn heroic_victory_against_odds() {
    // 1 000 against 1 600 (1 : 1.6): heroic whatever the losses.
    assert_eq!(
        classify(&rules(), true, &tally(1000, 600), &tally(1600, 400)),
        BattleOutcomeClass::Heroic
    );
    // 1 000 against 1 400: not enough odds.
    assert_ne!(
        classify(&rules(), true, &tally(1000, 100), &tally(1400, 400)),
        BattleOutcomeClass::Heroic
    );
}

#[test]
fn decisive_victory_crushes_the_enemy_cheaply() {
    assert_eq!(
        classify(&rules(), true, &tally(1000, 200), &tally(1000, 750)),
        BattleOutcomeClass::Decisive
    );
    // 25 % own losses: no longer decisive.
    assert_eq!(
        classify(&rules(), true, &tally(1000, 250), &tally(1000, 750)),
        BattleOutcomeClass::Victory
    );
}

#[test]
fn pyrrhic_victory_costs_half_the_army() {
    assert_eq!(
        classify(&rules(), true, &tally(1000, 500), &tally(1000, 600)),
        BattleOutcomeClass::Pyrrhic
    );
}

#[test]
fn plain_victory_otherwise() {
    assert_eq!(
        classify(&rules(), true, &tally(1000, 200), &tally(1000, 400)),
        BattleOutcomeClass::Victory
    );
}

#[test]
fn honourable_defeat_inflicts_as_much_as_it_suffers() {
    assert_eq!(
        classify(&rules(), false, &tally(1000, 800), &tally(1000, 800)),
        BattleOutcomeClass::HonourableDefeat
    );
}

#[test]
fn disaster_by_losses_or_by_the_general() {
    assert_eq!(
        classify(&rules(), false, &tally(1000, 700), &tally(1000, 100)),
        BattleOutcomeClass::Disaster
    );
    let captured = SideTally {
        general_lost: true,
        ..tally(1000, 200)
    };
    assert_eq!(
        classify(&rules(), false, &captured, &tally(1000, 100)),
        BattleOutcomeClass::Disaster
    );
}

#[test]
fn plain_defeat_otherwise() {
    assert_eq!(
        classify(&rules(), false, &tally(1000, 300), &tally(1000, 100)),
        BattleOutcomeClass::Defeat
    );
}

#[test]
fn every_class_has_a_label_in_the_data() {
    let data = game_data();
    for class in BattleOutcomeClass::ALL {
        let consequence = data.battle_outcome_rules.consequence(class);
        assert!(!consequence.label.is_empty());
        assert_ne!(consequence.label, class.key(), "{class:?} has a real label");
    }
}

#[test]
fn morale_modifiers_lift_the_battle_morale_then_wear_off() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 5).unwrap();
    let french = main_army(&state, "fac_france");
    let before = side_from_army(&state, data, &state.armies[&french]).general_morale_bonus;
    state
        .armies
        .get_mut(&french)
        .unwrap()
        .morale_modifiers
        .extend([
            MoraleModifier { value: 8, turns: 2 },
            MoraleModifier {
                value: -3,
                turns: 1,
            },
        ]);
    let after = side_from_army(&state, data, &state.armies[&french]).general_morale_bonus;
    assert!((after - before - 5.0).abs() < 1e-9);
    state.end_turn_with(data, |_, _, _| Vec::new());
    assert_eq!(state.armies[&french].morale_modifier(), 8, "one worn off");
    state.end_turn_with(data, |_, _, _| Vec::new());
    assert!(
        state.armies[&french].morale_modifiers.is_empty(),
        "all gone"
    );
}

#[test]
fn a_battle_is_classified_with_its_consequences() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 5).unwrap();
    state.interactive_battles = false;
    let french = main_army(&state, "fac_france");
    let english = main_army(&state, "fac_england");
    state.armies.retain(|id, _| *id == french || *id == english);
    let point = [2200.0, 3580.0];
    state.armies.get_mut(&french).unwrap().position = ArmyPosition::field(point);
    state.armies.get_mut(&english).unwrap().position =
        ArmyPosition::field([point[0] + 4.0, point[1]]);
    for id in [&french, &english] {
        let allowance = state.army_grid_allowance(data, &state.armies[id]);
        state.armies.get_mut(id).unwrap().movement_left = allowance;
    }
    let ruler_prestige = |state: &CampaignState, faction: &str| {
        let ruler = state.factions[&fac(faction)].ruler.clone().unwrap();
        state.characters[&ruler].prestige
    };
    let (france_before, england_before) = (
        ruler_prestige(&state, "fac_france"),
        ruler_prestige(&state, "fac_england"),
    );
    state
        .submit_order(
            data,
            Order::Attack {
                army: french.clone(),
                target_army: english.clone(),
            },
        )
        .unwrap();
    let report = state.last_battle_outcome.clone().expect("classified");
    assert_eq!(report.attacker_faction, fac("fac_france"));
    assert_eq!(
        report.attacker.class.is_victory(),
        !report.defender.class.is_victory()
    );
    let rules = &data.battle_outcome_rules;
    assert_eq!(
        report.attacker.label,
        rules.consequence(report.attacker.class).label
    );
    assert_eq!(
        report.class_of(&fac("fac_england")).unwrap().key,
        report.defender.class.key()
    );
    // The rulers' prestige moved by the classes' amounts (the ruler is not
    // the general here, so nothing else touched it).
    let france_general = state.armies.get(&french).and_then(|a| a.general.clone());
    let france_ruler = state.factions[&fac("fac_france")].ruler.clone();
    if france_general != france_ruler {
        assert_eq!(
            ruler_prestige(&state, "fac_france") - france_before,
            rules.consequence(report.attacker.class).prestige
        );
    }
    let english_general = state.armies.get(&english).and_then(|a| a.general.clone());
    if english_general != state.factions[&fac("fac_england")].ruler {
        assert_eq!(
            ruler_prestige(&state, "fac_england") - england_before,
            rules.consequence(report.defender.class).prestige
        );
    }
    // Surviving armies carry the morale modifier of their class.
    for (id, class) in [
        (&french, report.attacker.class),
        (&english, report.defender.class),
    ] {
        let consequence = rules.consequence(class);
        if let Some(army) = state.armies.get(id) {
            if consequence.morale != 0 && consequence.morale_turns > 0 {
                assert!(army.morale_modifiers.contains(&MoraleModifier {
                    value: consequence.morale,
                    turns: consequence.morale_turns,
                }));
            }
        }
    }
    // The report survives a save.
    let restored = CampaignState::load_json(&state.save_json()).unwrap();
    assert_eq!(restored.last_battle_outcome, Some(report));
}

#[test]
fn the_3d_battle_setup_carries_the_morale_modifiers() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 5).unwrap();
    state.interactive_battles = true;
    let french = main_army(&state, "fac_france");
    let english = main_army(&state, "fac_england");
    state.armies.retain(|id, _| *id == french || *id == english);
    let point = [2200.0, 3580.0];
    state.armies.get_mut(&french).unwrap().position = ArmyPosition::field(point);
    state.armies.get_mut(&english).unwrap().position =
        ArmyPosition::field([point[0] + 4.0, point[1]]);
    for id in [&french, &english] {
        let allowance = state.army_grid_allowance(data, &state.armies[id]);
        let army = state.armies.get_mut(id).unwrap();
        army.movement_left = allowance;
        for unit in &mut army.units {
            unit.morale = 50;
        }
    }
    state
        .submit_order(
            data,
            Order::Attack {
                army: french.clone(),
                target_army: english.clone(),
            },
        )
        .unwrap();
    let plain = state.battle_setup(data, 0).unwrap();
    state
        .armies
        .get_mut(&french)
        .unwrap()
        .morale_modifiers
        .push(MoraleModifier {
            value: 10,
            turns: 2,
        });
    let lifted = state.battle_setup(data, 0).unwrap();
    for (a, b) in plain.attacker.units.iter().zip(&lifted.attacker.units) {
        assert_eq!(b.morale, (a.morale + 10).min(100));
    }
    assert_eq!(plain.defender.units, lifted.defender.units);
}
