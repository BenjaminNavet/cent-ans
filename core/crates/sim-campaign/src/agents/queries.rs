//! Read-only queries: reach, recruit options, available actions.

use super::*;

impl CampaignState {
    pub fn agent(&self, id: &AgentId) -> Option<&Agent> {
        self.agents.agents.get(id)
    }

    /// Agents of `faction`, in id order.
    pub fn agents_of(&self, faction: &FactionId) -> Vec<(&AgentId, &Agent)> {
        self.agents
            .agents
            .iter()
            .filter(|(_, a)| &a.faction == faction)
            .collect()
    }

    pub(super) fn agents_in_province<'a>(
        &'a self,
        province: &'a ProvinceId,
    ) -> impl Iterator<Item = (&'a AgentId, &'a Agent)> + 'a {
        self.agents
            .agents
            .iter()
            .filter(move |(_, a)| self.settlement_province(&a.location) == Some(province))
    }

    /// Movement points an agent receives at the start of a season.
    pub fn agent_movement_allowance(&self, data: &GameData, kind: AgentKind) -> u32 {
        let steps = rules(data).types.get(&kind).map_or(3, |t| t.movement_steps);
        let steps = if self.season == crate::state::Season::Winter {
            steps.saturating_sub(1).max(1)
        } else {
            steps
        };
        (f64::from(steps) * movement::points_per_step(data)).round() as u32
    }

    /// Settlements `agent` can reach this season with its remaining points.
    pub fn agent_reachable(&self, data: &GameData, agent: &AgentId) -> BTreeMap<SettlementId, u32> {
        let Some(a) = self.agent(agent) else {
            return BTreeMap::new();
        };
        let cap = self.agent_movement_allowance(data, a.kind);
        agent_dijkstra(data, &a.location, Some(a.movement_points), cap)
            .into_iter()
            .filter(|(id, _)| id != &a.location)
            .map(|(id, reach)| (id, reach.cost))
            .collect()
    }

    /// Cheapest path of `agent` to `target` (may span several seasons).
    pub fn agent_find_path(
        &self,
        data: &GameData,
        agent: &AgentId,
        target: &SettlementId,
    ) -> Option<Vec<SettlementId>> {
        let a = self.agent(agent)?;
        if &a.location == target {
            return Some(Vec::new());
        }
        let cap = self.agent_movement_allowance(data, a.kind);
        AgentTable::search(data, &a.location, None, cap, Some(target)).path_to(target)
    }

    /// Recruitment options of `settlement` for `faction`.
    pub fn agent_recruit_options(
        &self,
        data: &GameData,
        faction: &FactionId,
        settlement: &SettlementId,
    ) -> Vec<AgentRecruitOption> {
        let rules = rules(data);
        AgentKind::ALL
            .into_iter()
            .filter_map(|kind| {
                let t = rules.types.get(&kind)?;
                let check = self.check_recruit(data, faction, settlement, kind);
                Some(AgentRecruitOption {
                    kind,
                    name: t.name.clone(),
                    cost: crate::coinage::priced(self, faction, i64::from(t.cost)),
                    upkeep: crate::coinage::priced(self, faction, i64::from(t.upkeep)),
                    count: self.count_agents(faction, kind),
                    max: t.max_per_faction,
                    available: check.is_ok(),
                    reason: check.err().map(|e| e.to_string()),
                })
            })
            .collect()
    }

    pub(super) fn count_agents(&self, faction: &FactionId, kind: AgentKind) -> u32 {
        self.agents
            .agents
            .values()
            .filter(|a| &a.faction == faction && a.kind == kind)
            .count() as u32
    }

    pub(super) fn check_recruit(
        &self,
        data: &GameData,
        faction: &FactionId,
        settlement: &SettlementId,
        kind: AgentKind,
    ) -> Result<i64, AgentError> {
        let rules = rules(data);
        let t = rules
            .types
            .get(&kind)
            .ok_or_else(|| AgentError::CannotRecruit("type d'agent inconnu".to_owned()))?;
        let s = self
            .settlements
            .get(settlement)
            .ok_or_else(|| AgentError::UnknownSettlement(settlement.clone()))?;
        if &s.controller != faction {
            return Err(AgentError::NotYourSettlement);
        }
        if s.siege.is_some() {
            return Err(AgentError::CannotRecruit(
                "colonie assiégée : personne n'en sort".to_owned(),
            ));
        }
        if !t.settlement_kinds.contains(&s.kind) {
            let needed = if t.settlement_kinds == [SettlementKind::City] {
                "une cité"
            } else {
                "une colonie fortifiée (pas un village)"
            };
            return Err(AgentError::CannotRecruit(format!(
                "{} se recrute dans {needed}",
                t.name
            )));
        }
        if t.requires_religious_building
            && s.kind != SettlementKind::Abbey
            && !s.buildings.iter().any(|b| {
                data.buildings
                    .get(b)
                    .is_some_and(|d| d.category == BuildingCategory::Religious)
            })
        {
            return Err(AgentError::CannotRecruit(format!(
                "{} se recrute dans une abbaye ou auprès d'une église (bâtiment religieux)",
                t.name
            )));
        }
        if self.count_agents(faction, kind) >= t.max_per_faction {
            return Err(AgentError::Cap {
                name: t.name.to_lowercase(),
                max: t.max_per_faction,
            });
        }
        let cost = crate::coinage::priced(self, faction, i64::from(t.cost));
        let available = self.factions.get(faction).map_or(0, |f| f.treasury);
        if available < cost {
            return Err(AgentError::InsufficientFunds {
                needed: cost,
                available,
            });
        }
        Ok(cost)
    }

    /// The action bar of `agent`: every action of its type, with the best
    /// target in range (its own settlement first, then the neighbours).
    pub fn agent_actions(&self, data: &GameData, agent: &AgentId) -> Vec<AgentActionOption> {
        let Some(a) = self.agent(agent) else {
            return Vec::new();
        };
        let rules = rules(data);
        let mut targets = vec![a.location.clone()];
        targets.extend(
            movement::edges(data, &a.location)
                .into_iter()
                .map(|(id, _)| id),
        );
        AgentActionKind::ALL
            .into_iter()
            .filter(|action| action.agent() == a.kind)
            .map(|action| {
                let r = rules.actions.get(&action);
                let mut first_error = None;
                let mut best: Option<ActionPlan> = None;
                for target in &targets {
                    match self.check_action(data, agent, action, Some(target), None) {
                        Ok(plan) => {
                            if best.as_ref().is_none_or(|b| plan.chance > b.chance) {
                                best = Some(plan);
                            }
                        }
                        Err(e) => {
                            if first_error.is_none() {
                                first_error = Some(e);
                            }
                        }
                    }
                }
                let reason = if best.is_some() {
                    None
                } else {
                    first_error.map(|e| e.to_string())
                };
                AgentActionOption {
                    action,
                    name: r.map_or_else(|| action.key().to_owned(), |r| r.name.clone()),
                    target: best.as_ref().map(|p| p.target.clone()),
                    character: best.as_ref().and_then(|p| p.character.clone()),
                    available: best.is_some(),
                    reason,
                    chance: best.as_ref().map_or(0, |p| p.chance),
                    cost: best.as_ref().map_or(0, |p| p.cost),
                    death_risk: r.map_or(0, |r| r.death_risk),
                    description: r.and_then(|r| r.description.clone()).unwrap_or_default(),
                }
            })
            .collect::<Vec<_>>()
            .into_iter()
            .fold(Vec::<AgentActionOption>::new(), |mut bar, option| {
                // Actions of one group share a button: the first available one.
                let group = |o: &AgentActionOption| {
                    rules.actions.get(&o.action).and_then(|s| s.group.as_ref())
                };
                match bar
                    .iter()
                    .position(|o| group(o).is_some() && group(o) == group(&option))
                {
                    Some(at) if !bar[at].available && option.available => bar[at] = option,
                    Some(_) => {}
                    None => bar.push(option),
                }
                bar
            })
    }

    /// Success chance (percent) and cost of `action` by `agent` on `target`
    /// (default: the agent's own settlement).
    pub fn agent_action_odds(
        &self,
        data: &GameData,
        agent: &AgentId,
        action: AgentActionKind,
        target: Option<&SettlementId>,
        character: Option<&CharacterId>,
    ) -> Result<(u32, i64), AgentError> {
        self.check_action(data, agent, action, target, character)
            .map(|plan| (plan.chance, plan.cost))
    }

    /// Validates an action against its [`ActionSpec`] and computes its odds
    /// (see the design § 6 and ADR 0209).
    pub(super) fn check_action(
        &self,
        data: &GameData,
        agent_id: &AgentId,
        action: AgentActionKind,
        target: Option<&SettlementId>,
        character: Option<&CharacterId>,
    ) -> Result<ActionPlan, AgentError> {
        let rules = rules(data);
        let agent = self
            .agent(agent_id)
            .ok_or_else(|| AgentError::UnknownAgent(agent_id.clone()))?;
        let spec = rules
            .actions
            .get(&action)
            .ok_or_else(|| AgentError::InvalidTarget("action inconnue".to_owned()))?;
        if action.agent() != agent.kind {
            return Err(AgentError::WrongAgent);
        }
        if agent.acted {
            return Err(AgentError::AlreadyActed);
        }
        let faction = &agent.faction;
        let target = target.unwrap_or(&agent.location).clone();
        if target != agent.location && movement::edge_cost(data, &agent.location, &target).is_none()
        {
            return Err(AgentError::OutOfRange);
        }
        let settlement = self
            .settlements
            .get(&target)
            .ok_or_else(|| AgentError::UnknownSettlement(target.clone()))?;
        let province = settlement.province.clone();
        let controller = settlement.controller.clone();
        let aim = match spec.aim {
            ActionAim::SettlementMaster => Some(controller.clone()),
            ActionAim::ProvinceMaster => self.province_controller(&province).cloned(),
        };
        let at_war = self.is_at_war(faction, &controller);
        let mut captive = None;
        for condition in &spec.conditions {
            let met = match condition.check {
                ActionCheck::ForeignMaster => &controller != faction,
                ActionCheck::NotRebels => controller.as_str() != REBELS_FACTION,
                ActionCheck::AtWar => at_war,
                ActionCheck::NotAtWar => !at_war,
                ActionCheck::AlliedMaster => self.is_allied(faction, &controller),
                ActionCheck::NoEmbassy => !self.factions.get(&controller).is_some_and(|f| {
                    f.modifiers.iter().any(|m| {
                        &m.with == faction
                            && m.reason_fr == PARLEY_REASON
                            && m.expires_turn > self.turn
                    })
                }),
                ActionCheck::NoTruce => !self.has_truce(faction, &controller),
                ActionCheck::NoHostileArmy => self.hostile_armies_at(faction, &target).is_empty(),
                ActionCheck::CaptiveHeld => {
                    captive = character
                        .cloned()
                        .or_else(|| self.first_captive_held_by(faction, &controller))
                        .filter(|c| {
                            herald_may_ransom(self, c)
                                && self.characters.get(c).is_some_and(|c| {
                                    c.alive
                                        && c.captive
                                        && &c.faction == faction
                                        && c.captor.as_ref() == Some(&controller)
                                })
                        });
                    captive.is_some()
                }
                ActionCheck::FriendlyProvince => self.is_friendly_territory(faction, &province),
                ActionCheck::DenounceableMaster => aim
                    .as_ref()
                    .is_some_and(|m| m != faction && self.denounceable(data, faction, m)),
                ActionCheck::PapalCity => controller.as_str() == PAPACY_FACTION,
                ActionCheck::CatholicFaction => crate::religion::is_catholic(self, data, faction),
                ActionCheck::HostileCharacter => {
                    captive = self.strike_target(
                        data,
                        agent,
                        spec.target_ruler_min_level,
                        &target,
                        character,
                    );
                    captive.is_some()
                }
                ActionCheck::FriendlyArmyNear => !self
                    .armies_in_reach(data, &target, |a| self.is_allied(faction, &a.faction))
                    .is_empty(),
                ActionCheck::HostileArmyNear => !self
                    .armies_in_reach(data, &target, |a| self.is_at_war(faction, &a.faction))
                    .is_empty(),
            };
            if !met {
                return Err(AgentError::InvalidTarget(condition.reason_fr.clone()));
            }
        }
        let target_faction = aim.unwrap_or_else(|| controller.clone());
        let men = settlement.garrison_strength();
        let mut cost = 0i64;
        if spec.cost.ransom {
            let captive = captive
                .as_ref()
                .ok_or_else(|| AgentError::InvalidTarget("aucun captif désigné".to_owned()))?;
            cost += ransom_price(self, data, captive, agent.level);
        }
        cost += crate::coinage::priced(
            self,
            faction,
            i64::from(spec.cost.flat) + i64::from(men) * i64::from(spec.cost.per_man_percent) / 100,
        );
        let mut malus = 0i32;
        malus += men
            .checked_div(spec.garrison_men_per_malus)
            .map_or(0, |per| per as i32);
        if spec.hostile {
            malus += rules.fortification_malus * i32::from(settlement.fortification_level);
            let spies: i32 = self
                .agents_in_province(&province)
                .filter(|(_, a)| a.kind == AgentKind::Spy && a.faction == target_faction)
                .map(|(_, a)| {
                    rules.counter_spy_malus + rules.counter_spy_malus_per_level * i32::from(a.level)
                })
                .sum();
            malus += spies.min(rules.counter_spy_malus_cap);
        }
        let available = self.factions.get(faction).map_or(0, |f| f.treasury);
        if cost > 0 && available < cost {
            return Err(AgentError::InsufficientFunds {
                needed: cost,
                available,
            });
        }
        let chance = (spec.base_chance + spec.per_level * (i32::from(agent.level) - 1) - malus)
            .clamp(rules.min_chance, rules.max_chance);
        Ok(ActionPlan {
            target,
            province,
            target_faction,
            character: captive,
            chance: chance.max(0) as u32,
            cost,
        })
    }

    /// Armies (in the field or in a place) within `army_reach_km` of
    /// `target` that satisfy `keep`.
    pub(super) fn armies_in_reach(
        &self,
        data: &GameData,
        target: &SettlementId,
        keep: impl Fn(&crate::state::Army) -> bool,
    ) -> Vec<crate::state::ArmyId> {
        let Some(center) = data.settlement_point(target) else {
            return Vec::new();
        };
        let reach = rules(data).army_reach_km as f32 * crate::march::px_per_km(data);
        self.armies
            .iter()
            .filter(|(_, a)| keep(a))
            .filter(|(_, a)| {
                let [x, y] = self.army_point(data, a);
                (x - center[0]).hypot(y - center[1]) <= reach
            })
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// The general or governor of the master of `target` that a spy may
    /// strike: `wanted` if it qualifies, else the first one by id. The
    /// sovereign qualifies only from seal `ruler_min_level`.
    pub(super) fn strike_target(
        &self,
        data: &GameData,
        agent: &Agent,
        ruler_min_level: Option<u8>,
        target: &SettlementId,
        wanted: Option<&CharacterId>,
    ) -> Option<CharacterId> {
        let settlement = self.settlements.get(target)?;
        let master = &settlement.controller;
        let ruler = self.factions.get(master).and_then(|f| f.ruler.clone());
        let here = self.armies_in_reach(data, target, |a| &a.faction == master);
        let qualifies = |id: &CharacterId, c: &crate::state::CharacterState| {
            c.alive
                && !c.captive
                && &c.faction == master
                && (c.governor_of.as_ref() == Some(&settlement.province)
                    || c.army.as_ref().is_some_and(|a| here.contains(a)))
                && (ruler.as_ref() != Some(id)
                    || ruler_min_level.is_some_and(|min| agent.level >= min))
        };
        wanted
            .filter(|id| self.characters.get(*id).is_some_and(|c| qualifies(id, c)))
            .cloned()
            .or_else(|| {
                self.characters
                    .iter()
                    .find(|(id, c)| qualifies(id, c))
                    .map(|(id, _)| id.clone())
            })
    }

    pub(super) fn first_captive_held_by(
        &self,
        faction: &FactionId,
        captor: &FactionId,
    ) -> Option<CharacterId> {
        self.characters
            .iter()
            .find(|(id, c)| {
                c.alive
                    && c.captive
                    && &c.faction == faction
                    && c.captor.as_ref() == Some(captor)
                    && herald_may_ransom(self, id)
            })
            .map(|(id, _)| id.clone())
    }

    /// `true` when a preacher of `faction` may denounce `master`: an enemy,
    /// an excommunicated prince, or (Schism) a prince of the rival obedience.
    pub(super) fn denounceable(
        &self,
        data: &GameData,
        faction: &FactionId,
        master: &FactionId,
    ) -> bool {
        if master.as_str() == REBELS_FACTION || master.as_str() == PAPACY_FACTION {
            return false;
        }
        if self.is_at_war(faction, master) || crate::religion::is_excommunicated(self, master) {
            return true;
        }
        self.schism
            && crate::religion::is_catholic(self, data, faction)
            && crate::religion::is_catholic(self, data, master)
            && crate::religion::faction_religion(self, data, faction)
                != crate::religion::faction_religion(self, data, master)
    }

    /// Provinces kept in sight by `faction`'s agents and intelligence:
    /// `(province, land steps of sight)`.
    pub fn agent_sight(
        &self,
        data: &GameData,
        lends_sight: &dyn Fn(&FactionId) -> bool,
        faction: &FactionId,
    ) -> Vec<(ProvinceId, u32)> {
        let rules = rules(data);
        let mut out = Vec::new();
        for agent in self.agents.agents.values() {
            if !lends_sight(&agent.faction) {
                continue;
            }
            if let Some(province) = self.settlement_province(&agent.location) {
                let range = rules.types.get(&agent.kind).map_or(0, |t| t.vision_range);
                out.push((province.clone(), range));
            }
        }
        for intel in &self.agents.intel {
            if &intel.faction == faction && intel.until_turn > self.turn {
                out.push((intel.province.clone(), 0));
            }
        }
        out
    }
}

/// Price paid by a herald of seal `level` for `captive`.
/// A herald buys back only a captive held for money: not one his captor
/// keeps (`Hold`, treaty hostages included) or frees against a province.
pub(super) fn herald_may_ransom(state: &CampaignState, captive: &CharacterId) -> bool {
    state.characters.get(captive).is_some_and(|c| {
        matches!(
            c.ransom_terms.clone().unwrap_or_default(),
            crate::ransom::RansomTerms::Money
        )
    })
}

pub(super) fn ransom_price(
    state: &CampaignState,
    data: &GameData,
    captive: &CharacterId,
    level: u8,
) -> i64 {
    let effects = rules(data);
    let percent = (effects.ransom_price_percent
        - effects.ransom_price_per_level * i64::from(level.saturating_sub(1)))
    .max(effects.ransom_price_floor);
    let fair = crate::ransom::ransom_amount(state, data, captive);
    (fair * percent / 100 / 50 * 50).max(50)
}

/// Seal reached with `experience` points.
pub fn level_for(data: &GameData, experience: u32) -> u8 {
    let passed = rules(data)
        .experience_thresholds
        .iter()
        .filter(|t| experience >= **t)
        .count() as u8;
    (1 + passed).min(MAX_LEVEL)
}
