//! ACTES (ADR 0784, 0785): the campaign arc in three acts, the reign score, the ruin of the house.

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
    for act in &rules.acts {
        for policy in &act.policies {
            assert!(rules.categories.contains_key(&policy.category), "{policy:?}");
        }
    }
    // Every faction, whatever its government, has objectives in each act.
    for (id, faction) in &data.factions {
        for act in &rules.acts {
            let list = rules.objectives(id, &format!("{:?}", faction.government).to_lowercase(), &act.id);
            assert!(!list.is_empty(), "{id} {}", act.id);
            assert!(list.iter().all(|o| rules.categories.contains_key(&o.category)));
        }
    }
    // The named provinces and titles exist.
    let json = serde_json::to_string(&rules.objective_sets).unwrap();
    for word in json.split('"').filter(|w| w.starts_with("prov_")) {
        assert!(
            data.provinces.contains_key(&data_model::ProvinceId::new(word).unwrap()),
            "unknown {word}"
        );
    }
    for word in json.split('"').filter(|w| w.starts_with("tit_")) {
        assert!(data.titles.contains_key(&data_model::TitleId::new(word).unwrap()), "unknown {word}");
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
    assert!(status.score > 0 && status.score < 100, "{}", status.score);
    assert!(state.outcome.is_none());
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
    assert!(state.acts.policy.is_none());
    state.choose_act_policy(data, "pol_2_rigor").unwrap();
    assert!(state.choose_act_policy(data, "pol_1_war").is_err());
    state.mark_act_transition_seen();
    assert!(!state.act_transition_pending());
    state.year = 1416;
    state.end_turn_with(data, idle);
    assert_eq!(state.acts.current, 2);
    assert_eq!(state.acts.reports[1].policy.as_deref(), Some("pol_2_rigor"));
    assert!(state.outcome.is_none(), "the third act is still to play");
}

#[test]
fn a_lasting_peace_closes_the_act_early() {
    let mut data = game_data().clone();
    data.acts_rules.acts[0].early_end.peace_seasons = 2;
    data.acts_rules.acts[0].early_end.min_seasons = 1;
    let data = &data;
    let mut state = started(data, "fac_england");
    for faction in state.factions.values_mut() {
        faction.at_war_with.clear();
    }
    for _ in 0..4 {
        state.end_turn_with(data, idle);
    }
    assert_eq!(state.acts.reports.first().map(|r| r.ended_by.as_str()), Some("peace"));
    assert_eq!(state.acts.current, 1);
    assert!(state.year < 1360);
}

#[test]
fn a_collapse_closes_the_act_early() {
    let data = game_data();
    let mut state = started(data, "fac_france");
    let player = fac("fac_france");
    let mut kept = 0;
    let england = fac("fac_england");
    let provinces: Vec<_> = state.controlled_provinces(&player).cloned().collect();
    for (id, settlement) in state.settlements.iter_mut() {
        let _ = id;
        if settlement.controller == player {
            if provinces.iter().take(3).any(|p| *p == settlement.province) {
                kept += 1;
            } else {
                settlement.controller = england.clone();
            }
        }
    }
    assert!(kept > 0);
    state.end_turn_with(data, idle);
    let report = state.acts.reports.first().expect("collapse closes the act");
    assert_eq!(report.ended_by, "collapse");
    assert!(state.outcome.is_none());
}

#[test]
fn pressures_follow_the_state_of_the_realm() {
    let data = game_data();
    let mut state = started(data, "fac_france");
    let france = fac("fac_france");
    let england = fac("fac_england");
    // In 1337 the English hold lands of the realm: the crown is contested.
    let contested = state.act_status(data).unwrap();
    assert!(contested.pressures.iter().any(|p| p.active && p.pressure.id == "pr1_crown"));
    for settlement in state.settlements.values_mut() {
        if settlement.controller == england {
            settlement.controller = france.clone();
        }
    }
    let relieved = state.act_status(data).unwrap();
    assert!(!relieved.pressures.iter().any(|p| p.active && p.pressure.id == "pr1_crown"));
    assert!(relieved.score > contested.score);
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
    assert!(outcome.text_fr.contains("banqueroute"), "{}", outcome.text_fr);
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
    for (_, settlement) in state.settlements.iter_mut() {
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
    assert_eq!(outcome.score, i64::from(state.reign_score(&rich, false)));

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
    state.choose_act_policy(data, "pol_2_regain").unwrap();
    let loaded = CampaignState::load_json(&state.save_json()).unwrap();
    assert_eq!(loaded.acts, state.acts);
    assert_eq!(loaded.reign_score(data, true), state.reign_score(data, true));
}

#[test]
fn the_policy_weighs_one_category_more() {
    let data = game_data();
    let mut state = started(data, "fac_france");
    let base = state.act_status(data).unwrap();
    state.choose_act_policy(data, "pol_1_line").unwrap();
    let with_policy = state.act_status(data).unwrap();
    let boosted = with_policy
        .objectives
        .iter()
        .filter(|o| o.factor_percent == 150)
        .count();
    assert_eq!(boosted, 1);
    // The dynasty is secure at the start: weighing it more raises the score.
    assert!(with_policy.score >= base.score);
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
        state.acts.reports.iter().map(|r| (&r.act_id, &r.ended_by, r.score)).collect::<Vec<_>>(),
        outcome.score
    );
}
