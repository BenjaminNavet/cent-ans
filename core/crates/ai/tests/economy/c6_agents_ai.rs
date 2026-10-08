//! Lot C6: the strategic AI recruits and employs agents with valid orders.
use sim_campaign::plan_cache::PlanCache;

use std::collections::BTreeMap;

use data_model::FactionId;
use sim_campaign::{CampaignState, EventKind, Order};

use crate::common::data;

#[test]
fn ai_recruits_moves_and_uses_agents_with_few_refusals() {
    let data = data();
    let mut state =
        CampaignState::new_1337(data, FactionId::new("fac_france").unwrap(), 11).unwrap();
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
            for order in sim_campaign::agents::plan_agents(&PlanCache::new(&probe), data, &faction)
            {
                issued += 1;
                if let Err(e) = probe.apply_order(data, &faction, order.clone()) {
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
        let events = state.end_turn_with(data, ai::plan_turn);
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
