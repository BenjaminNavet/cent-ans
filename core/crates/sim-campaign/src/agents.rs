//! Campaign agents (lot C6): spies, heralds and preachers.
//!
//! See `docs/design/2026-09-24-agents.md` and ADR 0009. Agents are light
//! entities (not dynasty characters) kept in [`CampaignState::agents`]; they
//! walk the settlement graph of the armies ([`movement::edges`]) without
//! being stopped by enemy places or armies, and attempt one action per
//! season. Every roll uses a generator derived from (game seed, turn, agent,
//! action serial): the main [`CampaignState::rng`] stream is never consumed,
//! so a game without agents keeps exactly the same draws.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap};
use std::fmt;
use std::sync::OnceLock;

use data_model::{
    AgentActionKind, AgentKind, AgentRules, BuildingCategory, CharacterId, FactionId, GameData,
    ProvinceId, SettlementId, SettlementKind,
};
use serde::{Deserialize, Serialize};

use crate::diplomacy::{faction_name, Proposal, PAPACY_FACTION, REBELS_FACTION};
use crate::events::{EventKind, GameEvent};
use crate::movement;
use crate::orders::Order;
use crate::rng::CampaignRng;
use crate::siege::settlement_name;
use crate::state::CampaignState;

/// Opinion reason of an embassy (herald `parley`, `truce`).
pub const PARLEY_REASON: &str = "Ambassade d'un héraut";
/// Opinion reason of a failed bribe.
pub const BRIBE_REASON: &str = "Tentative de corruption";
/// Treasury an AI faction keeps before recruiting an agent.
pub const AI_RECRUIT_RESERVE: i64 = 1_500;
/// An AI herald only courts factions whose attitude is below this.
pub const AI_PARLEY_ATTITUDE: i32 = 10;
/// Highest seal.
pub const MAX_LEVEL: u8 = 5;

/// Rules in use: `data/rules/agents.json`, or the defaults of the design.
pub fn rules(data: &GameData) -> &AgentRules {
    static DEFAULT: OnceLock<AgentRules> = OnceLock::new();
    data.agent_rules
        .as_ref()
        .unwrap_or_else(|| DEFAULT.get_or_init(AgentRules::default))
}

// ----- identifiers and state ------------------------------------------------

/// Identifier of an agent, `"agent_0001"`, ... allocated by the state.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentId(String);

impl AgentId {
    pub const PREFIX: &'static str = "agent_";

    pub fn from_index(index: u32) -> Self {
        AgentId(format!("{}{index:04}", Self::PREFIX))
    }

    /// Parses an existing id such as `"agent_0007"`.
    pub fn parse(raw: &str) -> Option<Self> {
        let digits = raw.strip_prefix(Self::PREFIX)?;
        if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
            Some(AgentId(raw.to_owned()))
        } else {
            None
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Numeric part of the id (0 when malformed).
    pub fn index(&self) -> u32 {
        self.0
            .strip_prefix(Self::PREFIX)
            .and_then(|d| d.parse().ok())
            .unwrap_or(0)
    }
}

impl fmt::Debug for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AgentId({:?})", self.0)
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for AgentId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Outcome of the last action of an agent (or of the last action resolved).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentReport {
    pub turn: u32,
    pub agent: AgentId,
    pub faction: FactionId,
    pub action: AgentActionKind,
    pub success: bool,
    /// Success chance of the attempt (percent).
    pub chance: u32,
    /// The agent was killed or jailed.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub lost: bool,
    pub text_fr: String,
}

/// A mobile agent on the campaign map.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Agent {
    pub faction: FactionId,
    pub kind: AgentKind,
    pub name: String,
    /// Settlement (node of the movement graph) the agent stands on.
    pub location: SettlementId,
    /// In kilometres of plain, like the armies'.
    pub movement_points: u32,
    pub experience: u32,
    /// Seal 1-5.
    pub level: u8,
    /// Has already attempted an action this season.
    #[serde(default)]
    pub acted: bool,
    /// Where the agent keeps walking at the start of each season.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub destination: Option<SettlementId>,
    pub recruited_turn: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_report: Option<AgentReport>,
}

/// A province kept in sight by a successful scouting.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Intel {
    pub faction: FactionId,
    pub province: ProvinceId,
    /// Turn at which the intelligence goes stale.
    pub until_turn: u32,
}

/// Every agent of the campaign (`CampaignState::agents`, `serde(default)`:
/// saves written before lot C6 load with none).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AgentsState {
    #[serde(default)]
    pub agents: BTreeMap<AgentId, Agent>,
    #[serde(default)]
    pub next_index: u32,
    /// Serial of the next action (salt of the derived generator).
    #[serde(default)]
    pub action_serial: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub intel: Vec<Intel>,
    /// Upkeep paid by each faction during the last resolved turn.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub upkeep_last_turn: BTreeMap<FactionId, i64>,
    /// Last action resolved (any faction), for the UI's immediate feedback.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_report: Option<AgentReport>,
}

/// Why an agent order was refused (French, for the UI).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AgentError {
    #[error("agent inconnu : {0}")]
    UnknownAgent(AgentId),
    #[error("cet agent n'appartient pas à votre faction")]
    NotYourAgent,
    #[error("colonie inconnue : {0}")]
    UnknownSettlement(SettlementId),
    #[error("cette colonie ne vous appartient pas")]
    NotYourSettlement,
    #[error("{0}")]
    CannotRecruit(String),
    #[error("plafond atteint : {max} {name} au plus")]
    Cap { name: String, max: u32 },
    #[error("trésor insuffisant : {needed} livres nécessaires, {available} disponibles")]
    InsufficientFunds { needed: i64, available: i64 },
    #[error("destination inaccessible")]
    NoPath,
    #[error("cet agent a déjà agi cette saison")]
    AlreadyActed,
    #[error("cette action n'est pas du ressort de cet agent")]
    WrongAgent,
    #[error("cible hors de portée : la colonie de l'agent ou une voisine")]
    OutOfRange,
    #[error("{0}")]
    InvalidTarget(String),
}

// ----- derived generator ----------------------------------------------------

fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Generator of one roll: depends only on the game seed, the turn, the agent
/// and a salt (action serial, phase).
pub fn derived_rng(seed: u64, turn: u32, agent: u32, salt: u32) -> CampaignRng {
    let mixed = splitmix(
        splitmix(seed ^ 0xA6E5_7C6A_6E75_0000)
            ^ (u64::from(turn) << 40)
            ^ (u64::from(agent) << 20)
            ^ u64::from(salt),
    );
    CampaignRng::from_seed(mixed)
}

// ----- queries ----------------------------------------------------------------

/// One line of the recruitment panel of a settlement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRecruitOption {
    pub kind: AgentKind,
    pub name: String,
    pub cost: i64,
    pub upkeep: i64,
    /// Agents of this kind the faction has, and the cap.
    pub count: u32,
    pub max: u32,
    pub available: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// One button of the action bar of an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentActionOption {
    pub action: AgentActionKind,
    pub name: String,
    /// Target settlement (the agent's own or a neighbour) when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<SettlementId>,
    /// Captive concerned by a ransom.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub character: Option<CharacterId>,
    pub available: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Success chance (percent) when available.
    pub chance: u32,
    /// Livres spent by the attempt.
    pub cost: i64,
    /// Percent chance of losing the agent on failure.
    pub death_risk: u32,
    pub description: String,
}

/// A validated action, ready to be rolled.
#[derive(Debug, Clone, PartialEq)]
struct ActionPlan {
    target: SettlementId,
    province: ProvinceId,
    /// Master of the target settlement (or of the province for preachers).
    target_faction: FactionId,
    character: Option<CharacterId>,
    chance: u32,
    cost: i64,
}

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

    /// Agents standing on `settlement`, in id order.
    pub fn agents_at(&self, settlement: &SettlementId) -> Vec<&AgentId> {
        self.agents
            .agents
            .iter()
            .filter(|(_, a)| &a.location == settlement)
            .map(|(id, _)| id)
            .collect()
    }

    fn agents_in_province<'a>(
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
        let table = agent_dijkstra(data, &a.location, None, cap);
        movement::path_to(&table, target)
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

    fn count_agents(&self, faction: &FactionId, kind: AgentKind) -> u32 {
        self.agents
            .agents
            .values()
            .filter(|a| &a.faction == faction && a.kind == kind)
            .count() as u32
    }

    fn check_recruit(
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
            .collect()
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

    /// Validates an action and computes its odds (see the design § 6).
    fn check_action(
        &self,
        data: &GameData,
        agent_id: &AgentId,
        action: AgentActionKind,
        target: Option<&SettlementId>,
        character: Option<&CharacterId>,
    ) -> Result<ActionPlan, AgentError> {
        let rules = rules(data);
        let effects = &rules.effects;
        let agent = self
            .agent(agent_id)
            .ok_or_else(|| AgentError::UnknownAgent(agent_id.clone()))?;
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
        let invalid = |text: &str| Err(AgentError::InvalidTarget(text.to_owned()));
        let at_war = self.is_at_war(faction, &controller);
        let mut cost = 0i64;
        let mut character_out = None;
        let mut hostile = false;
        let mut malus = 0i32;
        match action {
            AgentActionKind::Scout => {
                if &controller == faction {
                    return invalid("colonie déjà à vous");
                }
                hostile = true;
            }
            AgentActionKind::Sabotage | AgentActionKind::Incite => {
                if !at_war {
                    return invalid("il faut être en guerre avec le maître des lieux");
                }
                hostile = true;
            }
            AgentActionKind::Counter => {
                if !self.is_allied(faction, &controller) {
                    return invalid("le contre-espionnage se mène en terre amie");
                }
            }
            AgentActionKind::Parley => {
                if &controller == faction || controller.as_str() == REBELS_FACTION {
                    return invalid("il faut une colonie d'une autre faction");
                }
                if at_war {
                    return invalid("en guerre : le héraut ne peut porter qu'une trêve");
                }
                let already = self.factions.get(&controller).is_some_and(|f| {
                    f.modifiers.iter().any(|m| {
                        &m.with == faction
                            && m.reason_fr == PARLEY_REASON
                            && m.expires_turn > self.turn
                    })
                });
                if already {
                    return invalid("une ambassade est déjà en cours auprès de cette cour");
                }
            }
            AgentActionKind::Truce => {
                if !at_war || controller.as_str() == REBELS_FACTION {
                    return invalid("il faut être en guerre avec le maître des lieux");
                }
                if self.has_truce(faction, &controller) {
                    return invalid("une trêve court déjà");
                }
            }
            AgentActionKind::Bribe => {
                if !at_war {
                    return invalid("il faut être en guerre avec le maître des lieux");
                }
                if !self.hostile_armies_at(faction, &target).is_empty() {
                    return invalid("une armée ennemie tient la place");
                }
                let men = settlement.garrison_strength();
                cost = crate::coinage::priced(
                    self,
                    faction,
                    i64::from(effects.bribe_base_cost)
                        + i64::from(men) * i64::from(effects.bribe_cost_per_man_percent) / 100,
                );
                malus += (men / effects.bribe_men_per_malus.max(1)) as i32;
                hostile = true;
            }
            AgentActionKind::Ransom => {
                let captive = match character {
                    Some(c) => Some(c.clone()),
                    None => self.first_captive_held_by(faction, &controller),
                };
                let Some(captive) = captive else {
                    return invalid("aucun de vos captifs n'est détenu par le maître des lieux");
                };
                let held = self.characters.get(&captive).is_some_and(|c| {
                    c.alive
                        && c.captive
                        && &c.faction == faction
                        && c.captor.as_ref() == Some(&controller)
                });
                if !held {
                    return invalid("ce captif n'est pas détenu ici");
                }
                if !herald_may_ransom(self, &captive) {
                    return invalid("son geôlier refuse de le rendre contre de l'argent");
                }
                cost = ransom_price(self, data, &captive, agent.level);
                character_out = Some(captive);
            }
            AgentActionKind::Preach => {
                if !self.is_friendly_territory(faction, &province) {
                    return invalid("on prêche en terre amie");
                }
            }
            AgentActionKind::Denounce => {
                let Some(master) = self.province_controller(&province).cloned() else {
                    return invalid("province sans maître");
                };
                if &master == faction || !self.denounceable(data, faction, &master) {
                    return invalid(
                        "il faut un prince ennemi, excommunié ou de l'obédience rivale",
                    );
                }
                hostile = true;
            }
            AgentActionKind::Curia => {
                if controller.as_str() != PAPACY_FACTION {
                    return invalid("il faut se rendre dans une ville du pape (Avignon, Rome…)");
                }
                if !crate::religion::is_catholic(self, data, faction) {
                    return invalid("seul un prince catholique plaide à la Curie");
                }
                if at_war {
                    return invalid("en guerre avec le pape");
                }
            }
        }
        let target_faction = if action == AgentActionKind::Denounce {
            self.province_controller(&province)
                .cloned()
                .unwrap_or(controller.clone())
        } else {
            controller.clone()
        };
        if hostile {
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
        let r = rules
            .actions
            .get(&action)
            .ok_or_else(|| AgentError::InvalidTarget("action inconnue".to_owned()))?;
        cost += crate::coinage::priced(self, faction, i64::from(r.cost));
        let available = self.factions.get(faction).map_or(0, |f| f.treasury);
        if cost > 0 && available < cost {
            return Err(AgentError::InsufficientFunds {
                needed: cost,
                available,
            });
        }
        let chance = (r.base_chance + r.per_level * (i32::from(agent.level) - 1) - malus)
            .clamp(rules.min_chance, rules.max_chance);
        Ok(ActionPlan {
            target,
            province,
            target_faction,
            character: character_out,
            chance: chance.max(0) as u32,
            cost,
        })
    }

    fn first_captive_held_by(
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
    fn denounceable(&self, data: &GameData, faction: &FactionId, master: &FactionId) -> bool {
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
fn herald_may_ransom(state: &CampaignState, captive: &CharacterId) -> bool {
    state.characters.get(captive).is_some_and(|c| {
        matches!(
            c.ransom_terms.clone().unwrap_or_default(),
            crate::ransom::RansomTerms::Money
        )
    })
}

fn ransom_price(state: &CampaignState, data: &GameData, captive: &CharacterId, level: u8) -> i64 {
    let effects = &rules(data).effects;
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

/// Shortest paths for an agent: every edge is open (agents slip through
/// enemy places and armies); `cap` clamps each edge to a season's allowance.
pub fn agent_dijkstra(
    data: &GameData,
    start: &SettlementId,
    budget: Option<u32>,
    cap: u32,
) -> BTreeMap<SettlementId, movement::Reach> {
    let mut best: BTreeMap<SettlementId, movement::Reach> = BTreeMap::new();
    let mut heap = BinaryHeap::new();
    best.insert(
        start.clone(),
        movement::Reach {
            cost: 0,
            previous: None,
        },
    );
    heap.push(Reverse((0u32, start.clone())));
    while let Some(Reverse((cost, current))) = heap.pop() {
        if best.get(&current).is_some_and(|r| r.cost < cost) {
            continue;
        }
        for (next, edge) in movement::edges(data, &current) {
            let total = cost + edge.min(cap.max(1));
            if budget.is_some_and(|b| total > b) {
                continue;
            }
            if best.get(&next).is_none_or(|r| total < r.cost) {
                best.insert(
                    next.clone(),
                    movement::Reach {
                        cost: total,
                        previous: Some(current.clone()),
                    },
                );
                heap.push(Reverse((total, next)));
            }
        }
    }
    best
}

// ----- orders -----------------------------------------------------------------

impl CampaignState {
    fn own_agent(&self, faction: &FactionId, id: &AgentId) -> Result<&Agent, AgentError> {
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
            self.apply_success(data, &agent, action, &plan, &mut events)
        } else {
            failure_text(data, &agent, action, &plan, self)
        };
        let death_risk = rules.actions.get(&action).map_or(0, |r| r.death_risk);
        let lost = !success && death_risk > 0 && rng.below(100) < death_risk;
        if !success && action == AgentActionKind::Bribe {
            self.add_modifier(
                &plan.target_faction,
                faction,
                rules.effects.bribe_fail_opinion,
                BRIBE_REASON,
                rules.effects.parley_turns,
            );
        }
        let mut text = text;
        if lost {
            text.push(' ');
            text.push_str(&lost_text(agent.kind, &agent.name));
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
        let visible_to_target = matches!(
            action,
            AgentActionKind::Sabotage
                | AgentActionKind::Incite
                | AgentActionKind::Bribe
                | AgentActionKind::Denounce
                | AgentActionKind::Parley
        ) || !success;
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
                faction_name(data, faction)
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

    /// Applies a successful action; returns the French report.
    fn apply_success(
        &mut self,
        data: &GameData,
        agent: &Agent,
        action: AgentActionKind,
        plan: &ActionPlan,
        events: &mut Vec<GameEvent>,
    ) -> String {
        let effects = rules(data).effects.clone();
        let faction = &agent.faction;
        let place = settlement_name(data, &plan.target);
        let province_label = province_name(data, &plan.province);
        match action {
            AgentActionKind::Scout => {
                let text = self.scouting_report(data, &plan.target);
                self.agents
                    .intel
                    .retain(|i| !(&i.faction == faction && i.province == plan.province));
                self.agents.intel.push(Intel {
                    faction: faction.clone(),
                    province: plan.province.clone(),
                    until_turn: self.turn + effects.intel_turns,
                });
                format!("{} renseigne sur {place} : {text}", agent.name)
            }
            AgentActionKind::Sabotage => {
                let s = self.settlements.get_mut(&plan.target).expect("validated");
                if let Some(siege) = s.siege.as_mut() {
                    siege.breach = siege
                        .breach
                        .saturating_add(effects.sabotage_breach)
                        .min(100);
                    siege.supplies = siege.supplies.saturating_sub(effects.sabotage_supplies);
                    format!(
                        "{} ouvre une brèche et gâte les vivres de {place} assiégée.",
                        agent.name
                    )
                } else {
                    for unit in &mut s.garrison {
                        unit.morale = unit.morale.saturating_sub(effects.sabotage_morale);
                    }
                    if let Some(c) = s.construction.as_mut() {
                        c.turns_left += effects.sabotage_delay_turns;
                        format!(
                            "{} incendie le chantier de {place} (retard de {} saisons) et sème le trouble dans la garnison.",
                            agent.name, effects.sabotage_delay_turns
                        )
                    } else {
                        format!(
                            "{} sème le trouble dans la garnison de {place} (moral −{}).",
                            agent.name, effects.sabotage_morale
                        )
                    }
                }
            }
            AgentActionKind::Incite => {
                if let Some(p) = self.provinces.get_mut(&plan.province) {
                    for class in [&mut p.population.peasants, &mut p.population.burghers] {
                        class.unrest = class.unrest.saturating_add(effects.incite_unrest).min(100);
                    }
                }
                format!(
                    "{} attise la colère des paysans et des bourgeois {} (mécontentement +{}).",
                    agent.name,
                    crate::events::de(&province_label),
                    effects.incite_unrest
                )
            }
            AgentActionKind::Counter => self.counter_espionage(data, agent, plan, events),
            AgentActionKind::Parley => {
                let value = effects.parley_opinion
                    + effects.parley_opinion_per_level * (i32::from(agent.level) - 1);
                self.add_capped_modifier(
                    data,
                    &plan.target_faction,
                    faction,
                    value,
                    PARLEY_REASON,
                    effects.parley_turns,
                );
                format!(
                    "{} est reçu en ambassade par {} : opinion +{value} pour {} saisons.",
                    agent.name,
                    faction_name(data, &plan.target_faction),
                    effects.parley_turns
                )
            }
            AgentActionKind::Truce => {
                let value = (effects.parley_opinion
                    + effects.parley_opinion_per_level * (i32::from(agent.level) - 1))
                    / 2;
                self.add_capped_modifier(
                    data,
                    &plan.target_faction,
                    faction,
                    value,
                    PARLEY_REASON,
                    effects.parley_turns,
                );
                let target = plan.target_faction.clone();
                let result = self.propose(
                    data,
                    faction,
                    &target,
                    Proposal::Truce {
                        turns: effects.truce_turns,
                    },
                );
                let enemy = faction_name(data, &target);
                match result {
                    Ok(()) if target == self.player_faction => {
                        format!("{} porte une proposition de trêve à {enemy}.", agent.name)
                    }
                    Ok(()) => format!(
                        "{} obtient une trêve de {} saisons avec {enemy}.",
                        agent.name, effects.truce_turns
                    ),
                    Err(e) => format!("{} est entendu, mais {e}", agent.name),
                }
            }
            AgentActionKind::Bribe => {
                crate::siege::capture(self, data, &plan.target, faction, events);
                format!(
                    "{} achète la garnison de {place} pour {} livres : elle ouvre ses portes.",
                    agent.name, plan.cost
                )
            }
            AgentActionKind::Ransom => {
                let captive = plan.character.clone().expect("validated");
                crate::chronicle::release_character(self, data, &captive, plan.cost, events);
                format!(
                    "{} rachète {} pour {} livres.",
                    agent.name,
                    self.character_name(data, &captive),
                    plan.cost
                )
            }
            AgentActionKind::Preach => {
                let drop = effects.preach_heresy.saturating_add(
                    effects
                        .preach_heresy_per_level
                        .saturating_mul(agent.level - 1),
                );
                let mut heresy_text = String::new();
                if let Some(p) = self.provinces.get_mut(&plan.province) {
                    if p.heresy > 0 {
                        p.heresy = p.heresy.saturating_sub(drop);
                        heresy_text = format!(" L'hérésie recule (−{drop}).");
                        if p.heresy == 0 {
                            p.heresy_religion = None;
                            heresy_text = " L'hérésie est extirpée.".to_owned();
                        }
                    }
                    for class in [&mut p.population.clergy, &mut p.population.peasants] {
                        class.unrest = class.unrest.saturating_sub(effects.preach_unrest);
                    }
                }
                format!(
                    "{} prêche dans la province {} : les esprits s'apaisent.{heresy_text}",
                    agent.name,
                    crate::events::de(&province_label)
                )
            }
            AgentActionKind::Denounce => {
                if let Some(p) = self.provinces.get_mut(&plan.province) {
                    p.population.clergy.unrest = p
                        .population
                        .clergy
                        .unrest
                        .saturating_add(effects.denounce_clergy_unrest)
                        .min(100);
                    p.population.peasants.unrest = p
                        .population
                        .peasants
                        .unrest
                        .saturating_add(effects.denounce_peasant_unrest)
                        .min(100);
                }
                crate::religion::change_favor(
                    self,
                    &plan.target_faction,
                    -i32::from(effects.denounce_favor),
                );
                format!(
                    "{} tonne en chaire contre {} : le clergé {} s'agite et le pape s'en émeut.",
                    agent.name,
                    faction_name(data, &plan.target_faction),
                    crate::events::de(&province_label)
                )
            }
            AgentActionKind::Curia => {
                let gain = effects.curia_favor.saturating_add(
                    effects
                        .curia_favor_per_level
                        .saturating_mul(agent.level - 1),
                );
                crate::religion::change_favor(self, faction, i32::from(gain));
                format!(
                    "{} plaide la cause de son prince à la Curie : faveur pontificale +{gain}.",
                    agent.name
                )
            }
        }
    }

    /// Detailed report on a settlement (scouting).
    fn scouting_report(&self, data: &GameData, target: &SettlementId) -> String {
        let Some(s) = self.settlements.get(target) else {
            return String::new();
        };
        let mut parts = vec![format!("tenue par {}", faction_name(data, &s.controller))];
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
                    faction_name(data, &a.faction),
                    a.total_strength()
                )
            })
            .collect();
        if !armies.is_empty() {
            parts.push(format!("armées : {}", armies.join(", ")));
        }
        if !s.buildings.is_empty() {
            let names: Vec<String> = s
                .buildings
                .iter()
                .map(|b| {
                    data.buildings
                        .get(b)
                        .map_or_else(|| b.to_string(), |d| d.name.display.clone())
                })
                .collect();
            parts.push(format!("bâtiments : {}", names.join(", ")));
        }
        if let Some(siege) = &s.siege {
            parts.push(format!(
                "assiégée par {} (vivres {} %, brèche {} %)",
                faction_name(data, &siege.attacker),
                siege.supplies,
                siege.breach
            ));
        }
        format!("{}.", parts.join(" ; "))
    }

    /// Counter-espionage action: rolls against every foreign, non-allied
    /// agent of the province.
    fn counter_espionage(
        &mut self,
        data: &GameData,
        agent: &Agent,
        plan: &ActionPlan,
        events: &mut Vec<GameEvent>,
    ) -> String {
        let rules = rules(data);
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
            return format!(
                "{} fouille les auberges {} : aucun agent étranger démasqué.",
                agent.name,
                crate::events::de(&province_name(data, &plan.province))
            );
        }
        let names: Vec<String> = caught
            .iter()
            .filter_map(|id| self.agents.agents.get(id))
            .map(|a| format!("{} ({})", a.name, faction_name(data, &a.faction)))
            .collect();
        for id in &caught {
            if let Some(victim) = self.agents.agents.remove(id) {
                if victim.faction == self.player_faction && agent.faction != self.player_faction {
                    events.push(
                        GameEvent::new(
                            EventKind::Agent,
                            format!(
                                "{} est démasqué {} et exécuté.",
                                victim.name,
                                crate::events::de(&province_name(data, &plan.province))
                            ),
                        )
                        .province(&plan.province)
                        .faction(&victim.faction),
                    );
                }
            }
        }
        format!("{} démasque {}.", agent.name, names.join(", "))
    }
}

fn province_name(data: &GameData, id: &ProvinceId) -> String {
    data.provinces
        .get(id)
        .map_or_else(|| id.to_string(), |p| p.name.display.clone())
}

fn failure_text(
    data: &GameData,
    agent: &Agent,
    action: AgentActionKind,
    plan: &ActionPlan,
    state: &CampaignState,
) -> String {
    let place = settlement_name(data, &plan.target);
    let action_name = rules(data)
        .actions
        .get(&action)
        .map_or_else(|| action.key().to_owned(), |r| r.name.to_lowercase());
    let extra = match action {
        AgentActionKind::Bribe => format!(" ({} livres perdues)", plan.cost),
        AgentActionKind::Truce | AgentActionKind::Parley => format!(
            " : {} ne le reçoit pas",
            faction_name(data, &plan.target_faction)
        ),
        AgentActionKind::Ransom => format!(
            " : le geôlier de {} refuse",
            plan.character
                .as_ref()
                .map_or_else(String::new, |c| state.character_name(data, c))
        ),
        _ => String::new(),
    };
    format!(
        "{} échoue à {action_name} ({place}, {} % de chances){extra}.",
        agent.name, plan.chance
    )
}

fn lost_text(kind: AgentKind, name: &str) -> String {
    match kind {
        AgentKind::Spy => format!("{name} est pris et pendu."),
        AgentKind::Emissary => format!("{name}, pris à trahir son office, est jeté en geôle."),
        AgentKind::Preacher => format!("{name} est jeté en geôle."),
    }
}

/// Walks `id` along the path to its destination while its points allow.
fn walk_agent(state: &mut CampaignState, data: &GameData, id: &AgentId) {
    let Some(agent) = state.agents.agents.get(id) else {
        return;
    };
    let Some(target) = agent.destination.clone() else {
        return;
    };
    let cap = state.agent_movement_allowance(data, agent.kind);
    let Some(path) = state.agent_find_path(data, id, &target) else {
        state.agents.agents.get_mut(id).expect("exists").destination = None;
        return;
    };
    let agent = state.agents.agents.get_mut(id).expect("exists");
    for next in path {
        let Some(cost) = movement::edge_cost(data, &agent.location, &next) else {
            break;
        };
        let cost = cost.min(cap.max(1));
        if cost > agent.movement_points {
            break;
        }
        agent.movement_points -= cost;
        agent.location = next;
    }
    if agent.location == target {
        agent.destination = None;
    }
}

// ----- end of turn ------------------------------------------------------------

/// Phase of `end_turn` (after religion): upkeep, dead factions, stale
/// intelligence, passive counter-espionage.
pub(crate) fn resolve_agents(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let rules = rules(data).clone();
    // Agents of dead factions vanish.
    let factions = &state.factions;
    state
        .agents
        .agents
        .retain(|_, a| factions.get(&a.faction).is_some_and(|f| f.alive));
    let turn = state.turn;
    state.agents.intel.retain(|i| i.until_turn > turn);
    // Upkeep.
    let mut upkeep: BTreeMap<FactionId, i64> = BTreeMap::new();
    for agent in state.agents.agents.values() {
        let base = rules.types.get(&agent.kind).map_or(0, |t| t.upkeep);
        *upkeep.entry(agent.faction.clone()).or_default() +=
            crate::coinage::priced(state, &agent.faction, i64::from(base));
    }
    for (faction, amount) in &upkeep {
        if let Some(f) = state.factions.get_mut(faction) {
            f.treasury -= amount;
        }
    }
    state.agents.upkeep_last_turn = upkeep;
    // Passive counter-espionage: foreign spies where the master has a spy.
    let mut caught: Vec<(AgentId, ProvinceId, FactionId)> = Vec::new();
    for (id, agent) in &state.agents.agents {
        if agent.kind != AgentKind::Spy {
            continue;
        }
        let Some(province) = state.settlement_province(&agent.location).cloned() else {
            continue;
        };
        let Some(master) = state.province_controller(&province).cloned() else {
            continue;
        };
        if state.is_allied(&agent.faction, &master) {
            continue;
        }
        let best = state
            .agents_in_province(&province)
            .filter(|(_, a)| a.kind == AgentKind::Spy && a.faction == master)
            .map(|(_, a)| a.level)
            .max();
        let Some(best) = best else {
            continue;
        };
        let chance = (rules.passive_counter_base
            + rules.passive_counter_per_level * (i32::from(best) - i32::from(agent.level)))
        .clamp(0, rules.passive_counter_cap) as u32;
        let mut rng = derived_rng(state.seed, turn, id.index(), 0x00FF_0000);
        if rng.below(100) < chance {
            caught.push((id.clone(), province, master));
        }
    }
    let player = state.player_faction.clone();
    for (id, province, master) in caught {
        let Some(victim) = state.agents.agents.remove(&id) else {
            continue;
        };
        if victim.faction == player || master == player {
            let text = if victim.faction == player {
                format!(
                    "{} est démasqué par les espions {} et pendu.",
                    victim.name,
                    crate::events::de(&faction_name(data, &master))
                )
            } else {
                format!(
                    "Vos espions démasquent {}, agent {}, qui est pendu.",
                    victim.name,
                    crate::events::de(&faction_name(data, &victim.faction))
                )
            };
            events.push(
                GameEvent::new(EventKind::Agent, text)
                    .province(&province)
                    .faction(&victim.faction),
            );
        }
    }
}

/// Start of a season (after the date advanced): fresh movement points, the
/// action back, and the march towards the destination resumes.
pub(crate) fn start_season(state: &mut CampaignState, data: &GameData) {
    let ids: Vec<AgentId> = state.agents.agents.keys().cloned().collect();
    for id in ids {
        let kind = state.agents.agents[&id].kind;
        let points = state.agent_movement_allowance(data, kind);
        let agent = state.agents.agents.get_mut(&id).expect("exists");
        agent.movement_points = points;
        agent.acted = false;
        walk_agent(state, data, &id);
    }
}

// ----- AI -----------------------------------------------------------------------

/// Agent orders of an AI faction (called by `ai::plan_turn`): see the design § 10.
pub fn plan_agents(state: &CampaignState, data: &GameData, faction: &FactionId) -> Vec<Order> {
    let Some(f) = state.factions.get(faction) else {
        return Vec::new();
    };
    if !f.alive || faction.as_str() == REBELS_FACTION || faction.as_str() == PAPACY_FACTION {
        return Vec::new();
    }
    let playable = data.factions.get(faction).is_some_and(|d| d.playable);
    let mut orders = Vec::new();
    // Recruitment: one agent of each missing kind, one per season.
    if playable || f.treasury > 2 * AI_RECRUIT_RESERVE {
        for kind in AgentKind::ALL {
            if state.count_agents(faction, kind) > 0 {
                continue;
            }
            // Minor powers keep a single spy, and only while at war.
            if !playable && (kind != AgentKind::Spy || f.at_war_with.is_empty()) {
                continue;
            }
            if kind == AgentKind::Preacher && !heresy_or_schism(state, faction) {
                continue;
            }
            let t = &rules(data).types[&kind];
            let cost = crate::coinage::priced(state, faction, i64::from(t.cost));
            if f.treasury < AI_RECRUIT_RESERVE + cost {
                break;
            }
            let place = ai_recruit_place(state, data, faction, kind);
            if let Some(settlement) = place {
                orders.push(Order::RecruitAgent {
                    settlement: settlement.into(),
                    kind,
                });
                break;
            }
        }
    }
    for (id, agent) in state.agents_of(faction) {
        if agent.acted {
            continue;
        }
        let plan = match agent.kind {
            AgentKind::Spy => ai_spy(state, data, faction, id, agent),
            AgentKind::Emissary => ai_emissary(state, data, faction, id, agent),
            AgentKind::Preacher => ai_preacher(state, data, faction, id, agent),
        };
        orders.extend(plan);
    }
    orders
}

/// A preacher is worth his upkeep against a heresy at home or during the Schism.
fn heresy_or_schism(state: &CampaignState, faction: &FactionId) -> bool {
    state.schism
        || state
            .controlled_provinces(faction)
            .iter()
            .any(|p| state.provinces.get(p).is_some_and(|p| p.heresy > 0))
}

fn ai_recruit_place(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    kind: AgentKind,
) -> Option<SettlementId> {
    let capital = state.factions.get(faction).map(|f| f.capital.clone());
    let capital_city = capital
        .as_ref()
        .and_then(|p| state.province_city_id(p))
        .cloned();
    if let Some(city) = capital_city {
        if state.check_recruit(data, faction, &city, kind).is_ok() {
            return Some(city);
        }
    }
    state
        .settlements
        .keys()
        .find(|id| state.check_recruit(data, faction, id, kind).is_ok())
        .cloned()
}

/// Act when an action is worth it, else walk towards `goal` and try `then`
/// if the goal is within reach this season.
fn act_or_walk(
    state: &CampaignState,
    data: &GameData,
    id: &AgentId,
    agent: &Agent,
    act_now: Option<(AgentActionKind, Option<SettlementId>, Option<CharacterId>)>,
    goal: Option<(SettlementId, AgentActionKind)>,
) -> Vec<Order> {
    if let Some((action, target, character)) = act_now {
        return vec![Order::AgentAction {
            agent: id.clone(),
            action,
            target,
            character,
        }];
    }
    let Some((goal, then)) = goal else {
        return Vec::new();
    };
    if goal == agent.location
        || agent.destination.as_ref() == Some(&goal) && agent.movement_points == 0
    {
        return Vec::new();
    }
    let mut orders = vec![Order::MoveAgent {
        agent: id.clone(),
        target: goal.clone(),
    }];
    if state.agent_reachable(data, id).contains_key(&goal) {
        orders.push(Order::AgentAction {
            agent: id.clone(),
            action: then,
            target: Some(goal),
            character: None,
        });
    }
    orders
}

fn ai_spy(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    id: &AgentId,
    agent: &Agent,
) -> Vec<Order> {
    let odds = |action| state.agent_action_odds(data, id, action, None, None).ok();
    let here = state.settlements.get(&agent.location);
    let besieged = here.is_some_and(|s| s.siege.is_some());
    let mut act = None;
    if besieged && odds(AgentActionKind::Sabotage).is_some_and(|(c, _)| c >= 25) {
        act = Some((AgentActionKind::Sabotage, None, None));
    } else if let Some(province) = state.settlement_province(&agent.location) {
        let known = state.agents.intel.iter().any(|i| {
            &i.faction == faction && &i.province == province && i.until_turn > state.turn + 1
        });
        if !known && odds(AgentActionKind::Scout).is_some_and(|(c, _)| c >= 40) {
            act = Some((AgentActionKind::Scout, None, None));
        } else if odds(AgentActionKind::Incite).is_some_and(|(c, _)| c >= 35) {
            act = Some((AgentActionKind::Incite, None, None));
        } else if odds(AgentActionKind::Sabotage).is_some_and(|(c, _)| c >= 35) {
            act = Some((AgentActionKind::Sabotage, None, None));
        }
    }
    // Goal: the nearest enemy city not yet under our eyes.
    let goal = if act.is_none() {
        nearest_city(state, data, agent, |state, city, province| {
            let controller = state.settlements.get(city).map(|s| &s.controller);
            controller.is_some_and(|c| state.is_at_war(faction, c))
                && !state
                    .agents
                    .intel
                    .iter()
                    .any(|i| &i.faction == faction && &i.province == province)
                && &agent.location != city
        })
        .map(|g| (g, AgentActionKind::Scout))
    } else {
        None
    };
    act_or_walk(state, data, id, agent, act, goal)
}

fn ai_emissary(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    id: &AgentId,
    agent: &Agent,
) -> Vec<Order> {
    let odds = |action| state.agent_action_odds(data, id, action, None, None).ok();
    // 1. Buy back a captive: only one held for money that the treasury can
    // pay for (else steps 2 and 3, rather than waiting at his captor's).
    let treasury = state.factions.get(faction).map_or(0, |f| f.treasury);
    let action_cost = rules(data)
        .actions
        .get(&AgentActionKind::Ransom)
        .map_or(0, |r| i64::from(r.cost));
    let action_cost = crate::coinage::priced(state, faction, action_cost);
    let captor = state
        .characters
        .iter()
        .find(|(cid, c)| {
            c.alive
                && c.captive
                && &c.faction == faction
                && c.captor.is_some()
                && herald_may_ransom(state, cid)
                && ransom_price(state, data, cid, agent.level) + action_cost <= treasury
        })
        .and_then(|(_, c)| c.captor.clone());
    if let Some(captor) = captor {
        if odds(AgentActionKind::Ransom).is_some() {
            return act_or_walk(
                state,
                data,
                id,
                agent,
                Some((AgentActionKind::Ransom, None, None)),
                None,
            );
        }
        let goal = faction_city(state, data, agent, &captor);
        if let Some(goal) = goal {
            return act_or_walk(
                state,
                data,
                id,
                agent,
                None,
                Some((goal, AgentActionKind::Ransom)),
            );
        }
    }
    // 2. A truce with an enemy that is winning.
    let losing_to = state.factions.get(faction).and_then(|f| {
        f.at_war_with
            .iter()
            .find(|enemy| {
                enemy.as_str() != REBELS_FACTION
                    && &state.player_faction != *enemy
                    && state.war_score(data, faction, enemy) < -15
            })
            .cloned()
    });
    if let Some(enemy) = losing_to {
        let here = state
            .settlements
            .get(&agent.location)
            .map(|s| s.controller.clone());
        if here.as_ref() == Some(&enemy) && odds(AgentActionKind::Truce).is_some() {
            return act_or_walk(
                state,
                data,
                id,
                agent,
                Some((AgentActionKind::Truce, None, None)),
                None,
            );
        }
        if let Some(goal) = faction_city(state, data, agent, &enemy) {
            return act_or_walk(
                state,
                data,
                id,
                agent,
                None,
                Some((goal, AgentActionKind::Truce)),
            );
        }
    }
    // 3. An embassy to the least friendly neighbour at peace.
    if odds(AgentActionKind::Parley).is_some() {
        return act_or_walk(
            state,
            data,
            id,
            agent,
            Some((AgentActionKind::Parley, None, None)),
            None,
        );
    }
    let neighbour = state
        .factions
        .iter()
        .filter(|(other, f)| {
            *other != faction
                && f.alive
                && other.as_str() != REBELS_FACTION
                && !state.is_at_war(faction, other)
                && !state.is_allied(faction, other)
                && state.are_neighbors(data, faction, other)
                && !f.modifiers.iter().any(|m| {
                    &m.with == faction
                        && m.reason_fr == PARLEY_REASON
                        && m.expires_turn > state.turn
                })
        })
        .map(|(other, _)| (state.attitude(data, other, faction).0, other.clone()))
        // Only courts that need winning over (C6 balance: no opinion inflation).
        .filter(|(attitude, _)| *attitude < AI_PARLEY_ATTITUDE)
        .min()
        .map(|(_, other)| other);
    if let Some(other) = neighbour {
        if let Some(goal) = faction_city(state, data, agent, &other) {
            return act_or_walk(
                state,
                data,
                id,
                agent,
                None,
                Some((goal, AgentActionKind::Parley)),
            );
        }
    }
    Vec::new()
}

fn ai_preacher(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    id: &AgentId,
    agent: &Agent,
) -> Vec<Order> {
    let here = state.settlement_province(&agent.location).cloned();
    let here_heresy = here
        .as_ref()
        .and_then(|p| state.provinces.get(p))
        .map_or(0, |p| p.heresy);
    let preach_ok = state
        .agent_action_odds(data, id, AgentActionKind::Preach, None, None)
        .is_ok();
    if here_heresy > 0 && preach_ok {
        return act_or_walk(
            state,
            data,
            id,
            agent,
            Some((AgentActionKind::Preach, None, None)),
            None,
        );
    }
    // The most heretical friendly province.
    let target = state
        .controlled_provinces(faction)
        .into_iter()
        .filter_map(|p| state.provinces.get(&p).map(|s| (p.clone(), s.heresy)))
        .filter(|(_, heresy)| *heresy > 0)
        .max_by_key(|(_, heresy)| *heresy)
        .and_then(|(p, _)| state.province_city_id(&p).cloned());
    if let Some(goal) = target {
        return act_or_walk(
            state,
            data,
            id,
            agent,
            None,
            Some((goal, AgentActionKind::Preach)),
        );
    }
    // Otherwise calm the most restless province where he stands.
    let restless = here
        .as_ref()
        .and_then(|p| state.provinces.get(p))
        .is_some_and(|p| p.population.peasants.unrest >= 40 || p.population.clergy.unrest >= 40);
    if restless && preach_ok {
        return act_or_walk(
            state,
            data,
            id,
            agent,
            Some((AgentActionKind::Preach, None, None)),
            None,
        );
    }
    Vec::new()
}

/// Nearest city (by agent path cost) satisfying `wanted`.
fn nearest_city(
    state: &CampaignState,
    data: &GameData,
    agent: &Agent,
    wanted: impl Fn(&CampaignState, &SettlementId, &ProvinceId) -> bool,
) -> Option<SettlementId> {
    let cap = state.agent_movement_allowance(data, agent.kind);
    let table = agent_dijkstra(data, &agent.location, None, cap);
    state
        .provinces
        .iter()
        .filter(|(province, p)| wanted(state, &p.city, province))
        .filter_map(|(_, p)| table.get(&p.city).map(|r| (r.cost, p.city.clone())))
        .min()
        .map(|(_, city)| city)
}

/// Nearest city held by `faction`.
fn faction_city(
    state: &CampaignState,
    data: &GameData,
    agent: &Agent,
    faction: &FactionId,
) -> Option<SettlementId> {
    nearest_city(state, data, agent, |state, city, _| {
        state
            .settlements
            .get(city)
            .is_some_and(|s| &s.controller == faction)
    })
}
