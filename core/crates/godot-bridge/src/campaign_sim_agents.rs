//! `CampaignSim` agents API (lot C6, `docs/design/2026-09-24-agents.md`).
//!
//! Read-only views; orders go through `submit_order` (`recruit_agent`,
//! `move_agent`, `agent_action`, `dismiss_agent`).

use data_model::{FactionId, SettlementId};
use godot::prelude::*;
use sim_campaign::agents::{self, Agent, AgentId, AgentReport};
use sim_campaign::siege::settlement_name;

use crate::campaign_sim::CampaignSim;

fn report_dict(report: &AgentReport) -> VarDictionary {
    vdict! {
        "turn" => i64::from(report.turn),
        "agent" => report.agent.as_str(),
        "faction" => report.faction.as_str(),
        "action" => report.action.key(),
        "success" => report.success,
        "chance" => i64::from(report.chance),
        "lost" => report.lost,
        "text" => report.text_fr.as_str(),
    }
}

impl CampaignSim {
    fn agent_dict(&self, id: &AgentId, agent: &Agent) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let rules = agents::rules(data);
        let type_name = rules
            .types
            .get(&agent.kind)
            .map_or_else(|| agent.kind.key().to_owned(), |t| t.name.clone());
        let mut dict = vdict! {
            "id" => id.as_str(),
            "faction" => agent.faction.as_str(),
            "kind" => agent.kind.key(),
            "kind_name" => type_name.as_str(),
            "name" => agent.name.as_str(),
            "location" => agent.location.as_str(),
            "location_name" => settlement_name(data, &agent.location).as_str(),
            "province" => state.settlement_province(&agent.location).map_or("", |p| p.as_str()),
            "movement_points" => i64::from(agent.movement_points),
            "max_movement_points" => i64::from(state.agent_movement_allowance(data, agent.kind)),
            "level" => i64::from(agent.level),
            "experience" => i64::from(agent.experience),
            "acted" => agent.acted,
            "destination" => agent.destination.as_ref().map_or("", |d| d.as_str()),
            "recruited_turn" => i64::from(agent.recruited_turn),
        };
        if let Some(report) = &agent.last_report {
            dict.set("last_report", &report_dict(report).to_variant());
        }
        dict
    }
}

#[godot_api(secondary)]
impl CampaignSim {
    /// Every agent on the map, in id order: `[{id, faction, kind, kind_name,
    /// name, location, location_name, province, movement_points,
    /// max_movement_points, level, experience, acted, destination,
    /// recruited_turn, last_report?}]`. `kind` is `spy`, `emissary` or
    /// `preacher`; the UI hides foreign agents standing out of sight.
    #[func]
    fn get_agents(&self) -> VarArray {
        let Some(state) = &self.state else {
            return VarArray::new();
        };
        state
            .agents
            .agents
            .iter()
            .map(|(id, agent)| self.agent_dict(id, agent).to_variant())
            .collect()
    }

    /// One agent (see `get_agents`), empty for an unknown id.
    #[func]
    fn get_agent(&self, id: GString) -> VarDictionary {
        let Some(state) = &self.state else {
            return VarDictionary::new();
        };
        let Some(id) = AgentId::parse(&id.to_string()) else {
            return VarDictionary::new();
        };
        state
            .agent(&id)
            .map(|agent| self.agent_dict(&id, agent))
            .unwrap_or_default()
    }

    /// `{settlement_id: cost}` the agent can reach this season.
    #[func]
    fn get_agent_reachable(&self, id: GString) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let Some(id) = AgentId::parse(&id.to_string()) else {
            return VarDictionary::new();
        };
        let mut dict = VarDictionary::new();
        for (settlement, cost) in state.agent_reachable(data, &id) {
            dict.set(settlement.as_str(), i64::from(cost));
        }
        dict
    }

    /// Settlements the agent walks through to `target` (empty when
    /// unreachable or already there).
    #[func]
    fn get_agent_path(&self, id: GString, target: GString) -> PackedStringArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return PackedStringArray::new();
        };
        let (Some(id), Ok(target)) = (
            AgentId::parse(&id.to_string()),
            SettlementId::new(target.to_string()),
        ) else {
            return PackedStringArray::new();
        };
        state
            .agent_find_path(data, &id, &target)
            .map(|path| path.iter().map(|s| GString::from(s.as_str())).collect())
            .unwrap_or_default()
    }

    /// The action bar: `[{action, name, target, target_name, character,
    /// available, reason, chance, cost, death_risk, description}]`.
    #[func]
    fn get_agent_actions(&self, id: GString) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        let Some(id) = AgentId::parse(&id.to_string()) else {
            return VarArray::new();
        };
        state
            .agent_actions(data, &id)
            .iter()
            .map(|option| {
                let target_name = option
                    .target
                    .as_ref()
                    .map(|t| settlement_name(data, t))
                    .unwrap_or_default();
                vdict! {
                    "action" => option.action.key(),
                    "name" => option.name.as_str(),
                    "target" => option.target.as_ref().map_or("", |t| t.as_str()),
                    "target_name" => target_name.as_str(),
                    "character" => option.character.as_ref().map_or("", |c| c.as_str()),
                    "available" => option.available,
                    "reason" => option.reason.as_deref().unwrap_or(""),
                    "chance" => i64::from(option.chance),
                    "cost" => option.cost,
                    "death_risk" => i64::from(option.death_risk),
                    "description" => option.description.as_str(),
                }
                .to_variant()
            })
            .collect()
    }

    /// Recruitment options of a settlement for the player: `[{kind, name,
    /// cost, upkeep, count, max, available, reason}]`.
    #[func]
    fn get_agent_recruit_options(&self, settlement: GString) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        let Ok(settlement) = SettlementId::new(settlement.to_string()) else {
            return VarArray::new();
        };
        let player = state.player_faction().clone();
        state
            .agent_recruit_options(data, &player, &settlement)
            .iter()
            .map(|option| {
                vdict! {
                    "kind" => option.kind.key(),
                    "name" => option.name.as_str(),
                    "cost" => option.cost,
                    "upkeep" => option.upkeep,
                    "count" => i64::from(option.count),
                    "max" => i64::from(option.max),
                    "available" => option.available,
                    "reason" => option.reason.as_deref().unwrap_or(""),
                }
                .to_variant()
            })
            .collect()
    }

    /// Outcome of the last agent action resolved (any faction), empty if none.
    #[func]
    fn get_last_agent_report(&self) -> VarDictionary {
        self.state
            .as_ref()
            .and_then(|s| s.agents.last_report.as_ref())
            .map(report_dict)
            .unwrap_or_default()
    }

    /// Upkeep of `faction`'s agents during the last resolved turn.
    #[func]
    fn get_agent_upkeep(&self, faction: GString) -> i64 {
        let Some(state) = &self.state else {
            return 0;
        };
        let Ok(faction) = FactionId::new(faction.to_string()) else {
            return 0;
        };
        state
            .agents
            .upkeep_last_turn
            .get(&faction)
            .copied()
            .unwrap_or(0)
    }

    /// Agent types for the encyclopedia: `[{kind, name, cost, upkeep, max,
    /// description, actions: [{action, name, base_chance, per_level,
    /// death_risk, description}]}]`.
    #[func]
    fn get_agent_types(&self) -> VarArray {
        let Some(data) = &self.data else {
            return VarArray::new();
        };
        let rules = agents::rules(data);
        rules
            .types
            .iter()
            .map(|(kind, t)| {
                let actions: VarArray = rules
                    .actions
                    .iter()
                    .filter(|(action, _)| action.agent() == *kind)
                    .map(|(action, r)| {
                        vdict! {
                            "action" => action.key(),
                            "name" => r.name.as_str(),
                            "base_chance" => i64::from(r.base_chance),
                            "per_level" => i64::from(r.per_level),
                            "death_risk" => i64::from(r.death_risk),
                            "description" => r.description.as_deref().unwrap_or(""),
                        }
                        .to_variant()
                    })
                    .collect();
                let mut dict = vdict! {
                    "kind" => kind.key(),
                    "name" => t.name.as_str(),
                    "cost" => i64::from(t.cost),
                    "upkeep" => i64::from(t.upkeep),
                    "max" => i64::from(t.max_per_faction),
                    "description" => t.description.as_deref().unwrap_or(""),
                };
                dict.set("actions", &actions);
                dict.to_variant()
            })
            .collect()
    }
}
