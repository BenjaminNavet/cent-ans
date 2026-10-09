//! Lot WH `turn` (ADR 0281): conditional dilemma options, faction mission
//! chains, the campaign report.
use data_model::test_support::{fac, game_data, prov};
use sim_campaign::chronicle::EventContext;
use sim_campaign::test_support::start;
use sim_campaign::{ChronicleError, Order, OrderError};

fn push_decision(state: &mut sim_campaign::CampaignState, event: &str) -> u32 {
    let id = state.chronicle.next_decision_id.max(1);
    state.chronicle.next_decision_id = id + 1;
    let event = data_model::EventId::new(event).unwrap();
    let options = (0..game_data().events[&event].options.len()).collect();
    state
        .chronicle
        .pending_decisions
        .push(sim_campaign::chronicle::Decision {
            id,
            event,
            faction: state.player_faction.clone(),
            province: None,
            options,
            expires_turn: state.turn + 5,
        });
    id
}

fn set_treasury(state: &mut sim_campaign::CampaignState, amount: i64) {
    let player = state.player_faction.clone();
    state.factions.get_mut(&player).unwrap().treasury = amount;
}

#[test]
fn an_option_with_requires_is_greyed_and_refused_until_met() {
    let data = game_data();
    let mut state = start(data, "fac_france", 1);
    let id = push_decision(&mut state, "evt_achat_dauphine");
    set_treasury(&mut state, 100);
    let player = state.player_faction.clone();
    let views = state.decision_views(data, &player);
    let option = &views[0].options[0];
    assert!(!option.allowed);
    assert!(option.reason.contains("12000"), "{}", option.reason);
    assert!(views[0].options[1].allowed, "the other option stays open");
    assert_eq!(
        state.submit_order(
            data,
            Order::ChooseEventOption {
                decision: id,
                option: 0
            }
        ),
        Err(OrderError::Chronicle(ChronicleError::OptionUnavailable(
            "Exige 12000 livres au trésor.".to_owned()
        )))
    );
    assert_eq!(state.chronicle.pending_decisions.len(), 1, "still pending");
    set_treasury(&mut state, 20_000);
    assert!(state.decision_views(data, &player)[0].options[0].allowed);
    state
        .submit_order(
            data,
            Order::ChooseEventOption {
                decision: id,
                option: 0,
            },
        )
        .expect("affordable now");
}

#[test]
fn an_unaffordable_cost_is_greyed_unless_nothing_else_is_payable() {
    let data = game_data();
    let mut state = start(data, "fac_france", 1);
    push_decision(&mut state, "evt_cocherel"); // option 0 costs 2000, no `requires`
    set_treasury(&mut state, 100);
    let player = state.player_faction.clone();
    let views = state.decision_views(data, &player);
    assert!(!views[0].options[0].allowed);
    assert!(views[0].options[0].reason.starts_with("Trésor insuffisant"));
    assert!(views[0].options[1].allowed);
    // The event used in the other direction: with every option costly the floor is lifted.
    let event = &data.events[&data_model::EventId::new("evt_nicopolis").unwrap()];
    let ctx = EventContext {
        faction: Some(fac("fac_burgundy")),
        province: None,
    };
    let mut burgundy = start(data, "fac_burgundy", 1);
    set_treasury(&mut burgundy, 0);
    let blocked = (0..event.options.len())
        .filter(|i| burgundy.option_unavailable(data, event, *i, &ctx).is_some())
        .count();
    assert!(
        blocked < event.options.len(),
        "a decision is never walled in"
    );
}

#[test]
fn faction_missions_go_to_their_faction_and_chain() {
    let data = game_data();
    let ids = |t: &str| data.mission_rules.templates.iter().any(|m| m.id == t);
    assert!(ids("fr_ponthieu") && ids("en_calais") && ids("bg_flandre"));
    let faction_missions = data
        .mission_rules
        .templates
        .iter()
        .filter(|t| t.faction.is_some())
        .count();
    assert!((6..=8).contains(&faction_missions), "{faction_missions}");
    let mut seen = std::collections::BTreeSet::new();
    let mut state = start(data, "fac_france", 4);
    for _ in 0..14 {
        state.end_turn(data);
        for m in &state.missions.active {
            seen.insert(m.template.clone());
        }
    }
    assert!(
        seen.iter()
            .all(|t| !t.starts_with("en_") && !t.starts_with("bg_")),
        "{seen:?}"
    );
    assert!(seen.contains("fr_ponthieu"), "{seen:?}");
    assert!(
        !seen.contains("fr_guyenne"),
        "chained: needs fr_ponthieu first"
    );
    // The predecessor done, the next link becomes available and the first is not offered again.
    state.missions.active.clear();
    state.missions.done.push("fr_ponthieu".to_owned());
    state.missions.last_closed_turn = None;
    let mut seen_after = std::collections::BTreeSet::new();
    for _ in 0..14 {
        state.end_turn(data);
        for m in &state.missions.active {
            seen_after.insert(m.template.clone());
        }
        state
            .missions
            .active
            .retain(|m| m.template != "take_neighbour");
    }
    assert!(seen_after.contains("fr_guyenne"), "{seen_after:?}");
    assert!(!seen_after.contains("fr_ponthieu"));
    let guyenne = state
        .missions
        .active
        .iter()
        .find(|m| m.template == "fr_guyenne");
    if let Some(m) = guyenne {
        assert_eq!(m.province, Some(prov("prov_guyenne")));
    }
}

#[test]
fn a_succeeding_faction_mission_is_remembered() {
    let data = game_data();
    let mut state = start(data, "fac_france", 4);
    state.end_turn(data);
    let player = state.player_faction.clone();
    // Hand France the Ponthieu and offer the mission directly.
    let target = prov("prov_ponthieu");
    let template = data
        .mission_rules
        .templates
        .iter()
        .find(|t| t.id == "fr_ponthieu")
        .unwrap();
    state.missions.active = vec![sim_campaign::missions::Mission {
        id: 77,
        template: template.id.clone(),
        goal: template.goal,
        counter: None,
        title: template.title.clone(),
        objective: template.objective.clone(),
        province: Some(target.clone()),
        settlement: None,
        building: None,
        count: 1,
        progress: 0,
        issued_turn: state.turn,
        deadline_turn: state.turn + 10,
        reward: template.reward.clone(),
        baseline: Vec::new(),
    }];
    let owner = state.province_controller(&target).cloned().unwrap();
    assert_ne!(owner, player);
    let city = state.province_city_id(&target).cloned().unwrap();
    let s = state.settlements.get_mut(&city).unwrap();
    s.owner = player.clone();
    s.controller = player.clone();
    state.missions.last_closed_turn = Some(state.turn);
    state.end_turn(data);
    assert!(state.missions.done.contains(&"fr_ponthieu".to_owned()));
}

#[test]
fn the_campaign_report_adds_up_and_counts_battles() {
    let data = game_data();
    let mut state = start(data, "fac_france", 2);
    state.end_turn(data);
    state.end_turn(data);
    let report = state.campaign_report(data);
    assert_eq!(report.provinces_start, report.provinces_end);
    assert!(report.provinces_peak >= report.provinces_end);
    assert_eq!(
        report.score_provinces
            + report.score_objectives
            + report.score_prestige
            + report.score_treasury
            + report.score_other,
        report.score
    );
    assert_eq!(report.turns, state.turn);
    assert_eq!(report.battles_won + report.battles_lost, 0);
    // Counters survive a save.
    state.stats.battles_won = 3;
    let json = serde_json::to_string(&state.stats).unwrap();
    let back: sim_campaign::campaign_stats::CampaignStats = serde_json::from_str(&json).unwrap();
    assert_eq!(back.battles_won, 3);
    // An older save has no `stats`.
    let empty: sim_campaign::campaign_stats::CampaignStats = serde_json::from_str("{}").unwrap();
    assert_eq!(empty.battles_won, 0);
}
