//! ACTES (ADR 0784, refit R1): the campaign arc in three acts at fixed dates, the renown of the
//! house, the ambitions, the reign score, the ruin of the house.

use data_model::test_support::{fac, game_data};
use data_model::GameData;
use sim_campaign::test_support::idle;
use sim_campaign::victory::OutcomeKind;
use sim_campaign::CampaignState;

fn started(data: &GameData, faction: &str) -> CampaignState {
    let mut state = CampaignState::new_1337(data, fac(faction), 7).unwrap();
    state.chronicle.disabled = true;
    state.end_turn_with(data, idle);
    assert!(state.acts.started);
    state
}

#[test]
fn the_data_describe_three_consistent_acts() {
    let data = game_data();
    let rules = &data.acts_rules;
    assert_eq!(rules.acts.len(), 3);
    for pair in rules.acts.windows(2) {
        assert_eq!(pair[0].end_year, pair[1].start_year);
    }
    assert_eq!(rules.acts[0].start_year, 1337);
    assert_eq!(rules.acts[2].end_year, 1453);
    // R1-2: every house has ambitions with effects in each act, their categories known.
    for (id, faction) in &data.factions {
        for act in &rules.acts {
            let list = rules.ambitions(
                id,
                &format!("{:?}", faction.government).to_lowercase(),
                &act.id,
            );
            assert!(list.len() >= 2, "{id} {}", act.id);
            for ambition in list {
                assert!(!ambition.effects.is_empty(), "{}", ambition.id);
                assert!(ambition
                    .objective_categories
                    .iter()
                    .all(|c| rules.categories.contains_key(c)));
                assert!(ambition
                    .renown_sources
                    .iter()
                    .all(|s| rules.renown.sources.contains_key(s)));
            }
        }
    }
    // Objective ids are unique from one act to the next (renown credits are keyed by id).
    for set in rules.objective_sets.values() {
        let mut ids: Vec<&str> = set.values().flatten().map(|o| o.id.as_str()).collect();
        let len = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), len, "duplicate objective id");
    }
    // Every faction, whatever its government, has objectives in each act.
    for (id, faction) in &data.factions {
        for act in &rules.acts {
            let list = rules.objectives(
                id,
                &format!("{:?}", faction.government).to_lowercase(),
                &act.id,
            );
            assert!(!list.is_empty(), "{id} {}", act.id);
            assert!(list
                .iter()
                .all(|o| rules.categories.contains_key(&o.category)));
        }
    }
    // The named provinces and titles exist.
    let json = serde_json::to_string(&rules.objective_sets).unwrap();
    for word in json.split('"').filter(|w| w.starts_with("prov_")) {
        assert!(
            data.provinces
                .contains_key(&data_model::ProvinceId::new(word).unwrap()),
            "unknown {word}"
        );
    }
    for word in json.split('"').filter(|w| w.starts_with("tit_")) {
        assert!(
            data.titles
                .contains_key(&data_model::TitleId::new(word).unwrap()),
            "unknown {word}"
        );
    }
}

#[test]
fn the_arc_starts_in_the_first_act_with_live_objectives() {
    let data = game_data();
    let state = started(data, "fac_france");
    assert_eq!(state.acts.current, 0);
    assert!(state.acts.reports.is_empty());
    let status = state.act_status(data).unwrap();
    assert_eq!(status.index, 0);
    assert!(status.objectives.len() >= 5);
    // What the house holds at the start is an acquis to keep, not renown (R1-1); an idle house
    // starts the act with little earned.
    assert!(status.score < 20, "{status:?}");
    let held: Vec<&str> = status
        .objectives
        .iter()
        .filter(|o| o.hold)
        .map(|o| o.objective.id.as_str())
        .collect();
    assert!(
        held.contains(&"fr1_dynasty") && held.contains(&"fr1_debt"),
        "{held:?}"
    );
    assert!(status.objectives.iter().any(|o| !o.hold));
    assert!(state.outcome.is_none());
}

/// GAMEPLAY2 (ADR 0788), judge finding C4: the arc is open from turn 0, so the first screen
/// shows the objectives of the act (not the abandoned rule of 40 seasons).
#[test]
fn the_act_is_open_before_the_first_end_of_turn() {
    let data = game_data();
    let state = CampaignState::new_1337(data, fac("fac_france"), 7).unwrap();
    assert!(state.acts.started);
    let status = state
        .act_status(data)
        .expect("the first act is live at turn 0");
    assert_eq!(status.index, 0);
    assert_eq!(state.turn, 0);
}

#[test]
fn the_year_closes_the_acts_one_after_the_other() {
    let data = game_data();
    let mut state = started(data, "fac_france");
    state.year = 1361;
    state.end_turn_with(data, idle);
    assert_eq!(state.acts.current, 1);
    assert_eq!(state.acts.reports.len(), 1);
    assert_eq!(state.acts.reports[0].act_id, "act_1");
    assert_eq!(state.acts.reports[0].ended_by, "year");
    assert!(state.act_transition_pending());
    assert!(state.acts.ambitions.is_empty());
    state.choose_act_ambition(data, "fr2_amb_guesclin").unwrap();
    assert!(state.choose_act_ambition(data, "fr1_amb_aides").is_err());
    state.mark_act_transition_seen();
    assert!(!state.act_transition_pending());
    state.year = 1416;
    state.end_turn_with(data, idle);
    assert_eq!(state.acts.current, 2);
    assert_eq!(state.acts.reports[1].ambitions, vec!["fr2_amb_guesclin"]);
    assert!(state.outcome.is_none(), "the third act is still to play");
}

/// R1-1: an act closes with its year and only then: neither a lasting peace nor a collapse of
/// the domain ends it (the ruin of the house stays a defeat, ADR 0784 § 8).
#[test]
fn acts_close_only_on_their_year() {
    let data = game_data();
    let mut state = started(data, "fac_france");
    let player = fac("fac_france");
    let england = fac("fac_england");
    for faction in state.factions.values_mut() {
        faction.at_war_with.clear();
    }
    let provinces: Vec<_> = state.controlled_provinces(&player).cloned().collect();
    for settlement in state.settlements.values_mut() {
        if settlement.controller == player
            && !provinces.iter().take(3).any(|p| *p == settlement.province)
        {
            settlement.controller = england.clone();
        }
    }
    for _ in 0..12 {
        state.end_turn_with(data, idle);
        if state.outcome.is_some() {
            break;
        }
    }
    assert!(state.acts.reports.is_empty(), "{:?}", state.acts.reports);
    assert_eq!(state.acts.current, 0);
    assert!(state.year < 1361);
    if state.outcome.is_none() {
        state.year = 1361;
        state.end_turn_with(data, idle);
        assert_eq!(state.acts.reports.len(), 1);
        assert_eq!(state.acts.reports[0].ended_by, "year");
    }
}

/// R1-1: an objective earns its renown the season it is met, not at the close of the act.
#[test]
fn early_success_is_rewarded() {
    let data = game_data();
    let mut state = started(data, "fac_france");
    let france = fac("fac_france");
    let before = state.act_status(data).unwrap();
    for settlement in state.settlements.values_mut() {
        if matches!(settlement.province.as_str(), "prov_flandre" | "prov_artois") {
            settlement.controller = france.clone();
        }
    }
    state.end_turn_with(data, idle);
    let after = state.act_status(data).unwrap();
    let flanders = after
        .objectives
        .iter()
        .find(|o| o.objective.id == "fr1_flanders")
        .unwrap();
    assert!(flanders.met && !flanders.hold, "{flanders:?}");
    assert!(flanders.renown >= 25, "{flanders:?}");
    assert!(after.renown >= before.renown + 25, "{after:?}");
    assert!(after.score > before.score);
    assert!(state.reign_score(data) > 0);
    assert!(state.year < 1360);
    assert!(state
        .acts
        .renown
        .journal
        .iter()
        .any(|f| f.source == "objective_met" && f.points >= 25));
    // Lost again, the objective gives its renown back.
    for settlement in state.settlements.values_mut() {
        if settlement.province.as_str() == "prov_flandre" {
            settlement.controller = fac("fac_flanders");
        }
    }
    state.end_turn_with(data, idle);
    let lost = state.act_status(data).unwrap();
    let flanders = lost
        .objectives
        .iter()
        .find(|o| o.objective.id == "fr1_flanders")
        .unwrap();
    assert_eq!(flanders.renown, 0);
}

/// R1-1: an act whose every objective is already held is no free 100 (the old `base = 100`).
#[test]
fn nothing_to_earn_is_not_a_free_hundred() {
    let mut data = game_data().clone();
    let set = data
        .acts_rules
        .objective_sets
        .get_mut("fac_france")
        .unwrap()
        .get_mut("act_1")
        .unwrap();
    set.retain(|o| matches!(o.id.as_str(), "fr1_dynasty" | "fr1_debt"));
    let data = &data;
    let mut state = started(data, "fac_france");
    let status = state.act_status(data).unwrap();
    assert!(status.objectives.iter().all(|o| o.hold && o.met), "{status:?}");
    assert!(status.score < 20, "{status:?}");
    state.year = 1361;
    state.end_turn_with(data, idle);
    assert!(
        state.acts.reports[0].score < 20,
        "{:?}",
        state.acts.reports[0]
    );
}

/// R1-1: the reign score keeps moving until 1453; no victory before, even with a zero threshold.
#[test]
fn reign_score_moves_until_1453() {
    let mut data = game_data().clone();
    data.acts_rules.victory_min_score = 0;
    let data = &data;
    let mut state = started(data, "fac_england");
    state.year = 1416;
    state.end_turn_with(data, idle);
    assert_eq!(state.acts.current, 2);
    assert!(state.outcome.is_none(), "no victory before 1453");
    let before = state.reign_score(data);
    for _ in 0..3 {
        state.stats.battles_won += 2;
        state.end_turn_with(data, idle);
    }
    assert!(state.outcome.is_none());
    assert!(
        state.reign_score(data) > before,
        "the third act still counts"
    );
    state.year = 1454;
    state.end_turn_with(data, idle);
    let outcome = state.outcome.clone().expect("the arc is over");
    assert_eq!(outcome.kind, OutcomeKind::Victory);
}

/// R1-1: a source of renown brings at most its cap in a season.
#[test]
fn renown_sources_are_capped() {
    let data = game_data();
    let rules = &data.acts_rules.renown;
    let cap = rules.source("battle_won").unwrap().season_cap as i32;
    assert!(cap > 0);
    let mut state = started(data, "fac_france");
    state.stats.battles_won += 10;
    state.end_turn_with(data, idle);
    let sum = |state: &CampaignState, source: &str| -> i32 {
        state
            .acts
            .renown
            .journal
            .iter()
            .filter(|f| f.source == source)
            .map(|f| f.points)
            .sum()
    };
    assert_eq!(sum(&state, "battle_won"), cap);
    let lost_cap = rules.source("battle_lost").unwrap().season_cap as i32;
    state.stats.battles_lost += 10;
    state.end_turn_with(data, idle);
    assert_eq!(sum(&state, "battle_lost"), -lost_cap);
}

#[test]
fn pressures_follow_the_state_of_the_realm() {
    let data = game_data();
    let mut state = started(data, "fac_france");
    let france = fac("fac_france");
    let england = fac("fac_england");
    // The house has earned something (Flanders and Artois), so that the pressure shows.
    for settlement in state.settlements.values_mut() {
        if matches!(settlement.province.as_str(), "prov_flandre" | "prov_artois") {
            settlement.controller = france.clone();
        }
    }
    // In 1337 the English hold lands of the realm: the crown is contested.
    let contested = state.act_status(data).unwrap();
    assert!(contested
        .pressures
        .iter()
        .any(|p| p.active && p.pressure.id == "pr1_crown"));
    for settlement in state.settlements.values_mut() {
        if settlement.controller == england {
            settlement.controller = france.clone();
        }
    }
    let relieved = state.act_status(data).unwrap();
    assert!(!relieved
        .pressures
        .iter()
        .any(|p| p.active && p.pressure.id == "pr1_crown"));
    // R1-1: the renown of a pressure follows it season by season.
    state.end_turn_with(data, idle);
    assert!(!state.acts.renown.credits.contains_key("pressure:pr1_crown"));
}

#[test]
fn a_royal_bankruptcy_that_cannot_be_undone_ends_the_campaign() {
    let data = game_data();
    let mut state = started(data, "fac_france");
    let player = fac("fac_france");
    {
        let faction = state.factions.get_mut(&player).unwrap();
        faction.finance.defaults = 3;
        faction.treasury = -50_000_000;
    }
    state.end_turn_with(data, idle);
    let outcome = state.outcome.clone().expect("ruin is a defeat");
    assert_eq!(outcome.kind, OutcomeKind::Defeat);
    assert!(
        outcome.text_fr.contains("banqueroute"),
        "{}",
        outcome.text_fr
    );
}

#[test]
fn the_extinction_of_the_dynasty_ends_the_campaign() {
    let data = game_data();
    let mut state = started(data, "fac_france");
    state.acts.house = "maison_disparue".to_owned();
    state.end_turn_with(data, idle);
    let outcome = state.outcome.clone().expect("extinction is a defeat");
    assert_eq!(outcome.kind, OutcomeKind::Defeat);
    assert!(outcome.text_fr.contains("lignée"), "{}", outcome.text_fr);
}

#[test]
fn losing_the_capital_without_an_heir_ends_the_campaign() {
    let mut data = game_data().clone();
    data.acts_rules.ruin.capital_lost_seasons = 1;
    let data = &data;
    let mut state = started(data, "fac_france");
    let player = fac("fac_france");
    let capital = state.factions[&player].capital.clone();
    state.factions.get_mut(&player).unwrap().heir = None;
    for settlement in state.settlements.values_mut() {
        if settlement.province == capital {
            settlement.controller = fac("fac_england");
        }
    }
    state.end_turn_with(data, idle);
    // The succession may name an heir at once; the rule only fires without one.
    if state.factions[&player].heir.is_none() {
        let outcome = state.outcome.clone().expect("capital lost without heir");
        assert_eq!(outcome.kind, OutcomeKind::Defeat);
    }
}

#[test]
fn the_reign_score_decides_the_final_victory() {
    let mut rich = game_data().clone();
    rich.acts_rules.victory_min_score = 0;
    let mut state = started(&rich, "fac_england");
    state.year = 1454;
    state.end_turn_with(&rich, idle);
    assert_eq!(state.acts.reports.len(), 3);
    let outcome = state.outcome.clone().expect("the arc is over");
    assert_eq!(outcome.kind, OutcomeKind::Victory);
    assert_eq!(outcome.score, i64::from(state.reign_score(&rich)));

    let mut hard = game_data().clone();
    hard.acts_rules.victory_min_score = 100;
    let mut state = started(&hard, "fac_england");
    state.year = 1454;
    state.end_turn_with(&hard, idle);
    assert_eq!(state.outcome.clone().unwrap().kind, OutcomeKind::Ended);
}

#[test]
fn the_state_with_its_reports_survives_a_save() {
    let data = game_data();
    let mut state = started(data, "fac_burgundy");
    state.year = 1361;
    state.end_turn_with(data, idle);
    let first = state.act_ambitions(data)[0].id.clone();
    state.choose_act_ambition(data, &first).unwrap();
    let loaded = CampaignState::load_json(&state.save_json()).unwrap();
    assert_eq!(loaded.acts, state.acts);
    assert_eq!(loaded.reign_score(data), state.reign_score(data));
    assert_eq!(loaded.house_renown(data), state.house_renown(data));
}

/// R1-2: two ambitions among four; a third replaces the oldest while the choice is open, then
/// the choice locks. The ambitions reach the realm's effects, the player's house only.
#[test]
fn ambitions_are_chosen_two_by_two_and_change_the_realm() {
    let data = game_data();
    let mut state = started(data, "fac_france");
    let player = fac("fac_france");
    let england = fac("fac_england");
    let ids: Vec<String> = state
        .act_ambitions(data)
        .iter()
        .map(|a| a.id.clone())
        .collect();
    assert_eq!(ids.len(), 4);
    assert_eq!(state.ambitions_pending(data), 2);
    let plain = sim_campaign::research::faction_tech_effects(&state, data, &player);
    let english = sim_campaign::research::faction_tech_effects(&state, data, &england);
    state.choose_act_ambition(data, &ids[0]).unwrap();
    state.choose_act_ambition(data, &ids[1]).unwrap();
    assert_eq!(state.ambitions_pending(data), 0);
    let with = sim_campaign::research::faction_tech_effects(&state, data, &player);
    assert_ne!(plain, with, "an ambition changes the simulation");
    assert_eq!(
        english,
        sim_campaign::research::faction_tech_effects(&state, data, &england)
    );
    state.choose_act_ambition(data, &ids[2]).unwrap();
    assert_eq!(state.acts.ambitions, vec![ids[1].clone(), ids[2].clone()]);
    assert!(state.choose_act_ambition(data, "nope").is_err());
    for _ in 0..data.acts_rules.ambition_choice_seasons {
        state.end_turn_with(data, idle);
    }
    assert!(state.choose_act_ambition(data, &ids[3]).is_err(), "locked");
}

/// A whole campaign with a passive player (the other houses play with the minimal AI) crosses
/// the three acts by the calendar, or ends in a ruin of the house.
#[test]
fn a_full_campaign_crosses_the_three_acts() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 1341).unwrap();
    let mut guard = 0;
    while state.outcome.is_none() && guard < 600 {
        state.end_turn(data);
        guard += 1;
    }
    let outcome = state.outcome.clone().expect("the campaign ends");
    if outcome.kind != OutcomeKind::Defeat {
        assert_eq!(state.acts.reports.len(), 3, "{:?}", state.acts.reports);
        assert!(state.year > 1453);
        assert!((0..=100).contains(&outcome.score));
    }
    eprintln!(
        "full campaign: {:?} year {} acts {:?} score {}",
        outcome.kind,
        state.year,
        state
            .acts
            .reports
            .iter()
            .map(|r| (&r.act_id, &r.ended_by, r.score))
            .collect::<Vec<_>>(),
        outcome.score
    );
    for report in &state.acts.reports {
        eprintln!(
            "  {} {:?}",
            report.act_id,
            report
                .objectives
                .iter()
                .map(|o| format!("{}:{}{}", o.id, o.percent, if o.met { "*" } else { "" }))
                .collect::<Vec<_>>()
        );
    }
}
