//! Player orders: recruit, move, dismiss, act.

use super::*;

impl CampaignState {
    pub(super) fn own_agent(
        &self,
        faction: &FactionId,
        id: &AgentId,
    ) -> Result<&Agent, AgentError> {
        let agent = self
            .agent(id)
            .ok_or_else(|| AgentError::UnknownAgent(id.clone()))?;
        if &agent.faction != faction {
            return Err(AgentError::NotYourAgent);
        }
        Ok(agent)
    }

    /// Order `recruit_agent`: pays and places a new agent on `settlement`.
    pub fn recruit_agent(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        settlement: &SettlementId,
        kind: AgentKind,
    ) -> Result<AgentId, AgentError> {
        let cost = self.check_recruit(data, faction, settlement, kind)?;
        let rules = rules(data);
        let t = &rules.types[&kind];
        self.agents.next_index += 1;
        let index = self.agents.next_index;
        let id = AgentId::from_index(index);
        let name = if t.names.is_empty() {
            format!("{} n° {index}", t.name)
        } else {
            t.names[(index as usize - 1) % t.names.len()].clone()
        };
        if let Some(f) = self.factions.get_mut(faction) {
            f.treasury -= cost;
        }
        self.agents.agents.insert(
            id.clone(),
            Agent {
                faction: faction.clone(),
                kind,
                name,
                location: settlement.clone(),
                movement_points: 0,
                experience: 0,
                level: 1,
                acted: false,
                destination: None,
                recruited_turn: self.turn,
                last_report: None,
            },
        );
        Ok(id)
    }

    /// Order `move_agent`: walks at once as far as the points allow and
    /// keeps `target` as destination for the next seasons.
    pub fn move_agent(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        id: &AgentId,
        target: &SettlementId,
    ) -> Result<(), AgentError> {
        self.own_agent(faction, id)?;
        if !self.settlements.contains_key(target) {
            return Err(AgentError::UnknownSettlement(target.clone()));
        }
        if self.agent_find_path(data, id, target).is_none() {
            return Err(AgentError::NoPath);
        }
        let agent = self.agents.agents.get_mut(id).expect("checked above");
        agent.destination = Some(target.clone());
        walk_agent(self, data, id);
        Ok(())
    }

    /// Order `dismiss_agent`.
    pub fn dismiss_agent(&mut self, faction: &FactionId, id: &AgentId) -> Result<(), AgentError> {
        self.own_agent(faction, id)?;
        self.agents.agents.remove(id);
        Ok(())
    }

    /// Order `agent_action`: validates, pays, rolls and applies the action.
    /// The outcome is stored on the agent and in `agents.last_report`.
    pub fn agent_act(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        id: &AgentId,
        action: AgentActionKind,
        target: Option<&SettlementId>,
        character: Option<&CharacterId>,
    ) -> Result<AgentReport, AgentError> {
        self.own_agent(faction, id)?;
        let plan = self.check_action(data, id, action, target, character)?;
        let rules = rules(data).clone();
        let spec = &rules.actions[&action];
        let serial = self.agents.action_serial;
        self.agents.action_serial += 1;
        let mut rng = derived_rng(self.seed, self.turn, id.index(), serial);
        let success = rng.below(100) < plan.chance;
        if plan.cost > 0 {
            if let Some(f) = self.factions.get_mut(faction) {
                f.treasury -= plan.cost;
            }
        }
        let agent = self.agents.agents[id].clone();
        let mut events = Vec::new();
        let text = if success {
            self.apply_effects(data, &agent, &spec.success, &plan, &mut events)
        } else {
            let text = failure_text(data, &agent, spec, &plan, self);
            self.apply_effects(data, &agent, &spec.failure, &plan, &mut events);
            text
        };
        let lost = !success && spec.death_risk > 0 && rng.below(100) < spec.death_risk;
        let mut text = text;
        if lost {
            text.push(' ');
            text.push_str(&fill(
                &rules.types[&agent.kind].lost_fr,
                &[("agent", &agent.name)],
            ));
        }
        let report = AgentReport {
            turn: self.turn,
            agent: id.clone(),
            faction: faction.clone(),
            action,
            success,
            chance: plan.chance,
            lost,
            text_fr: text.clone(),
        };
        if lost {
            self.agents.agents.remove(id);
        } else if let Some(a) = self.agents.agents.get_mut(id) {
            a.acted = true;
            a.movement_points = 0;
            a.experience += if success {
                rules.xp_success
            } else {
                rules.xp_failure
            };
            a.level = level_for(data, a.experience);
            a.last_report = Some(report.clone());
        }
        self.agents.last_report = Some(report.clone());
        // Journal: only what concerns the player.
        let player = self.player_faction.clone();
        let target_is_player = plan.target_faction == player && &player != faction;
        let visible_to_target = spec.reveals || !success;
        if faction == &player {
            self.push_order_event(
                GameEvent::new(EventKind::Agent, text)
                    .province(&plan.province)
                    .faction(faction),
            );
        } else if target_is_player && visible_to_target {
            let who = format!(
                "{} de {}",
                rules
                    .types
                    .get(&agent.kind)
                    .map_or("Un agent", |t| t.name.as_str()),
                data.faction_name(faction)
            );
            self.push_order_event(
                GameEvent::new(EventKind::Agent, format!("{who} : {text}"))
                    .province(&plan.province)
                    .faction(faction),
            );
        }
        for event in events {
            self.push_order_event(event);
        }
        Ok(report)
    }
}
