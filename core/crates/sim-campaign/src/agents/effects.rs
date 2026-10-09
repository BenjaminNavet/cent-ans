//! Effects of an agent action, in table order.

use super::*;

/// Text variables shared by the effects of one action.
struct EffectText {
    agent: String,
    place: String,
    province: String,
    target: String,
    cost: String,
    captive: String,
}

impl EffectText {
    /// Fills `template` with the action variables plus `extra`.
    fn say(&self, template: &str, extra: &[(&str, &str)]) -> String {
        let mut vars = vec![
            ("agent", self.agent.as_str()),
            ("place", self.place.as_str()),
            ("province", self.province.as_str()),
            ("target", self.target.as_str()),
            ("cost", self.cost.as_str()),
            ("captive", self.captive.as_str()),
        ];
        vars.extend_from_slice(extra);
        fill(template, &vars)
    }
}

impl CampaignState {
    /// Applies the effects of an action in table order; returns the French
    /// report (the non-empty texts of the effects, joined).
    pub(super) fn apply_effects(
        &mut self,
        data: &GameData,
        agent: &Agent,
        effects: &[ActionEffect],
        plan: &ActionPlan,
        events: &mut Vec<GameEvent>,
    ) -> String {
        let province_label = data.province_name(&plan.province);
        let say = EffectText {
            agent: agent.name.clone(),
            place: data.settlement_name(&plan.target),
            province: crate::events::de(&province_label),
            target: data.faction_name(&plan.target_faction),
            cost: plan.cost.to_string(),
            captive: plan
                .character
                .as_ref()
                .map_or_else(String::new, |c| self.character_name(data, c)),
        };
        let mut texts: Vec<String> = Vec::new();
        for effect in effects {
            let text = self.apply_effect(data, agent, plan, effect, &say, events);
            if !text.is_empty() {
                texts.push(text);
            }
        }
        texts.join(" ")
    }

    /// Applies one effect and returns its report text (possibly empty).
    fn apply_effect(
        &mut self,
        data: &GameData,
        agent: &Agent,
        plan: &ActionPlan,
        effect: &ActionEffect,
        say: &EffectText,
        events: &mut Vec<GameEvent>,
    ) -> String {
        let faction = &agent.faction;
        let seal = i32::from(agent.level) - 1;
        match effect {
            ActionEffect::Scout {
                intel_turns,
                text_fr,
            } => self.effect_scout(data, faction, plan, *intel_turns, text_fr, say),
            ActionEffect::Sabotage {
                breach,
                supplies,
                delay_turns,
                morale,
                siege_fr,
                works_fr,
                troubled_fr,
            } => {
                let s = self.settlements.get_mut(&plan.target).expect("validated");
                let (turns, value) = (delay_turns.to_string(), morale.to_string());
                let extra = [("turns", turns.as_str()), ("value", value.as_str())];
                if let Some(siege) = s.siege.as_mut() {
                    siege.breach = siege.breach.saturating_add(*breach).min(100);
                    siege.supplies = siege.supplies.saturating_sub(*supplies);
                    say.say(siege_fr, &extra)
                } else {
                    for unit in &mut s.garrison {
                        unit.morale = unit.morale.saturating_sub(*morale);
                    }
                    if let Some(c) = s.construction.as_mut() {
                        c.turns_left += delay_turns;
                        say.say(works_fr, &extra)
                    } else {
                        say.say(troubled_fr, &extra)
                    }
                }
            }
            ActionEffect::Unrest {
                classes,
                delta,
                per_level,
                text_fr,
            } => {
                let delta = delta + per_level * seal;
                if let Some(p) = self.provinces.get_mut(&plan.province) {
                    for class in classes {
                        let gauge = &mut p.population.get_mut(*class).unrest;
                        *gauge = (i32::from(*gauge) + delta).clamp(0, 100) as u8;
                    }
                }
                say.say(text_fr, &[("value", &delta.to_string())])
            }
            ActionEffect::Heresy {
                drop,
                per_level,
                receded_fr,
                uprooted_fr,
            } => {
                let drop = drop.saturating_add(per_level.saturating_mul(agent.level - 1));
                match self.provinces.get_mut(&plan.province) {
                    Some(p) if p.heresy > 0 => {
                        p.heresy = p.heresy.saturating_sub(drop);
                        if p.heresy == 0 {
                            p.heresy_religion = None;
                            say.say(uprooted_fr, &[])
                        } else {
                            say.say(receded_fr, &[("value", &drop.to_string())])
                        }
                    }
                    _ => String::new(),
                }
            }
            ActionEffect::Opinion {
                reason_fr,
                value,
                per_level,
                divisor,
                turns,
                text_fr,
            } => {
                let value = (value + per_level * seal) / (*divisor).max(1);
                self.add_capped_modifier(
                    data,
                    &plan.target_faction,
                    faction,
                    value,
                    reason_fr,
                    *turns,
                );
                say.say(
                    text_fr,
                    &[("value", &value.to_string()), ("turns", &turns.to_string())],
                )
            }
            ActionEffect::Mediation {
                turns,
                proposed_fr,
                agreed_fr,
                refused_fr,
            } => self.effect_mediation(
                data,
                faction,
                plan,
                *turns,
                [proposed_fr, agreed_fr, refused_fr],
                say,
            ),
            ActionEffect::Favor {
                on,
                amount,
                per_level,
                text_fr,
            } => {
                let amount = amount + per_level * seal;
                let who = match on {
                    EffectSubject::Own => faction,
                    EffectSubject::Target => &plan.target_faction,
                };
                crate::religion::change_favor(self, &who.clone(), amount);
                say.say(text_fr, &[("value", &amount.to_string())])
            }
            ActionEffect::Capture { text_fr } => {
                crate::siege::capture(self, data, &plan.target, faction, events);
                say.say(text_fr, &[])
            }
            ActionEffect::Release { text_fr } => {
                let captive = plan.character.clone().expect("validated");
                crate::chronicle::release_character(self, data, &captive, plan.cost, events);
                say.say(text_fr, &[])
            }
            ActionEffect::Unmask {
                found_fr,
                none_fr,
                executed_fr,
            } => {
                self.counter_espionage(data, agent, plan, events, [found_fr, none_fr, executed_fr])
            }
        }
    }

    /// Scouting: reports on the target and records the intel window.
    fn effect_scout(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        plan: &ActionPlan,
        intel_turns: u32,
        text_fr: &str,
        say: &EffectText,
    ) -> String {
        let report = self.scouting_report(data, &plan.target);
        self.agents
            .intel
            .retain(|i| !(&i.faction == faction && i.province == plan.province));
        self.agents.intel.push(Intel {
            faction: faction.clone(),
            province: plan.province.clone(),
            until_turn: self.turn + intel_turns,
        });
        say.say(text_fr, &[("report", &report)])
    }

    /// Papal mediation proposed to the target; `texts` = proposed, agreed,
    /// refused.
    fn effect_mediation(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        plan: &ActionPlan,
        turns: u32,
        texts: [&str; 3],
        say: &EffectText,
    ) -> String {
        let [proposed_fr, agreed_fr, refused_fr] = texts;
        let result = self.propose(
            data,
            faction,
            &plan.target_faction.clone(),
            Treaty::single(Article::Mediation { turns }),
        );
        match result {
            Ok(()) if plan.target_faction == self.player_faction => say.say(proposed_fr, &[]),
            Ok(()) => say.say(agreed_fr, &[("turns", &turns.to_string())]),
            Err(e) => say.say(refused_fr, &[("error", &e.to_string())]),
        }
    }

    /// Detailed report on a settlement (scouting).
    pub(super) fn scouting_report(&self, data: &GameData, target: &SettlementId) -> String {
        let Some(s) = self.settlements.get(target) else {
            return String::new();
        };
        let mut parts = vec![format!("tenue par {}", data.faction_name(&s.controller))];
        let men = s.garrison_strength();
        parts.push(if men == 0 {
            "aucune garnison".to_owned()
        } else {
            format!(
                "garnison de {men} hommes en {} compagnie(s)",
                s.garrison.len()
            )
        });
        let armies: Vec<String> = self
            .armies_at(target)
            .iter()
            .filter_map(|id| self.armies.get(id))
            .map(|a| {
                format!(
                    "{} ({} hommes)",
                    data.faction_name(&a.faction),
                    a.total_strength()
                )
            })
            .collect();
        if !armies.is_empty() {
            parts.push(format!("armées : {}", armies.join(", ")));
        }
        if !s.buildings.is_empty() {
            let names: Vec<String> = s.buildings.iter().map(|b| data.building_name(b)).collect();
            parts.push(format!("bâtiments : {}", names.join(", ")));
        }
        if let Some(siege) = &s.siege {
            parts.push(format!(
                "assiégée par {} (vivres {} %, brèche {} %)",
                data.faction_name(&siege.attacker),
                siege.supplies,
                siege.breach
            ));
        }
        format!("{}.", parts.join(" ; "))
    }

    /// Counter-espionage action: rolls against every foreign, non-allied
    /// agent of the province.
    pub(super) fn counter_espionage(
        &mut self,
        data: &GameData,
        agent: &Agent,
        plan: &ActionPlan,
        events: &mut Vec<GameEvent>,
        [found_fr, none_fr, executed_fr]: [&str; 3],
    ) -> String {
        let rules = rules(data);
        let province_de = crate::events::de(&data.province_name(&plan.province));
        let intruders: Vec<(AgentId, u8)> = self
            .agents_in_province(&plan.province)
            .filter(|(_, a)| !self.is_allied(&agent.faction, &a.faction))
            .map(|(id, a)| (id.clone(), a.level))
            .collect();
        let mut caught = Vec::new();
        for (index, (intruder, level)) in intruders.iter().enumerate() {
            let chance = (plan.chance as i32
                - rules.passive_counter_per_level * (i32::from(*level) - 1))
                .clamp(0, 100) as u32;
            let mut rng = derived_rng(
                self.seed,
                self.turn,
                intruder.index(),
                0x00C0_0000 + index as u32,
            );
            if rng.below(100) < chance {
                caught.push(intruder.clone());
            }
        }
        if caught.is_empty() {
            return fill(
                none_fr,
                &[("agent", &agent.name), ("province", &province_de)],
            );
        }
        let names: Vec<String> = caught
            .iter()
            .filter_map(|id| self.agents.agents.get(id))
            .map(|a| format!("{} ({})", a.name, data.faction_name(&a.faction)))
            .collect();
        for id in &caught {
            if let Some(victim) = self.agents.agents.remove(id) {
                if victim.faction == self.player_faction && agent.faction != self.player_faction {
                    events.push(
                        GameEvent::new(
                            EventKind::Agent,
                            fill(
                                executed_fr,
                                &[("agent", &victim.name), ("province", &province_de)],
                            ),
                        )
                        .province(&plan.province)
                        .faction(&victim.faction),
                    );
                }
            }
        }
        fill(
            found_fr,
            &[("agent", &agent.name), ("names", &names.join(", "))],
        )
    }
}
