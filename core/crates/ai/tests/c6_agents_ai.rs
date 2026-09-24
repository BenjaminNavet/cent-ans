//! Lot C6: the strategic AI recruits and employs agents with valid orders.

use std::collections::BTreeMap;
use std::path::PathBuf;

use data_model::{FactionId, GameData};
use sim_campaign::{CampaignState, EventKind, Order};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

#[test]
fn ai_recruits_moves_and_uses_agents_with_few_refusals() {
    let data = data();
    let mut state =
        CampaignState::new_1337(&data, FactionId::new("fac_france").unwrap(), 11).unwrap();
    let mut issued = 0usize;
    let mut refused = 0usize;
    let mut refusals: BTreeMap<String, usize> = BTreeMap::new();
    let mut agent_events = 0usize;
    for _ in 0..16 {
        // Replay the planner's agent orders on a copy to count refusals.
        let mut probe = state.clone();
        let factions: Vec<FactionId> = probe
            .factions
            .iter()
            .filter(|(id, f)| f.alive && **id != probe.player_faction)
            .map(|(id, _)| id.clone())
            .collect();
        for faction in factions {
            for order in sim_campaign::agents::plan_agents(&probe, &data, &faction) {
                issued += 1;
                if let Err(e) = probe.apply_order(&data, &faction, order.clone()) {
                    refused += 1;
                    let key = match order {
                        Order::RecruitAgent { .. } => "recruit",
                        Order::MoveAgent { .. } => "move",
                        Order::AgentAction { .. } => "action",
                        _ => "other",
                    };
                    *refusals.entry(format!("{key}: {e}")).or_default() += 1;
                }
            }
        }
        let events = state.end_turn_with(&data, ai::plan_turn);
        agent_events += events.iter().filter(|e| e.kind == EventKind::Agent).count();
    }
    let england = FactionId::new("fac_england").unwrap();
    assert!(
        !state.agents_of(&england).is_empty(),
        "England employs agents after 16 seasons"
    );
    let acted = state
        .agents
        .agents
        .values()
        .any(|a| a.experience > 0 || a.last_report.is_some());
    assert!(acted || agent_events > 0, "some agent acted");
    assert!(issued > 0);
    assert!(
        refused * 5 <= issued,
        "refusals {refused}/{issued}: {refusals:?}"
    );
}

/// Balance probe (C6 § d):
/// `cargo test --release -p ai --test c6_agents_ai -- --ignored --nocapture`.
#[test]
#[ignore]
fn agents_balance_probe() {
    let data = data();
    let mut state =
        CampaignState::new_1337(&data, FactionId::new("fac_france").unwrap(), 5).unwrap();
    let mut per_action: BTreeMap<String, (u32, u32)> = BTreeMap::new();
    let mut lost = 0u32;
    let mut peak = 0usize;
    let mut player_lines = 0usize;
    for _ in 0..60 {
        let turn = state.turn;
        let before: Vec<_> = state.agents.agents.keys().cloned().collect();
        let events = state.end_turn_with(&data, ai::plan_turn);
        player_lines += events.iter().filter(|e| e.kind == EventKind::Agent).count();
        lost += before.iter().filter(|id| state.agent(id).is_none()).count() as u32;
        peak = peak.max(state.agents.agents.len());
        for agent in state.agents.agents.values() {
            if let Some(report) = agent.last_report.as_ref().filter(|r| r.turn == turn) {
                let entry = per_action
                    .entry(report.action.key().to_owned())
                    .or_default();
                entry.0 += 1;
                entry.1 += u32::from(report.success);
            }
        }
    }
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    for agent in state.agents.agents.values() {
        *kinds.entry(agent.kind.key().to_owned()).or_default() += 1;
    }
    println!(
        "C6 probe, 60 seasons: {} agents alive (peak {peak}), {lost} lost, {player_lines} journal lines for the player",
        state.agents.agents.len()
    );
    println!("kinds alive: {kinds:?}");
    for (action, (tried, won)) in &per_action {
        println!("  {action}: {won}/{tried} surviving attempts succeeded");
    }
}
