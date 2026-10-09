//! Campaign agents: spies, heralds and preachers.
//!
//! See `docs/design/2026-09-24-agents.md` and ADR 0009. Agents are light
//! entities (not dynasty characters) kept in [`CampaignState::agents`]; they
//! walk the settlement graph of the armies ([`movement::edges`]) without
//! being stopped by enemy places or armies, and attempt one action per
//! season. Every roll uses a generator derived from (game seed, turn, agent,
//! action serial): the main [`CampaignState::rng`] stream is never consumed,
//! so a game without agents keeps exactly the same draws.

use data_model::util::splitmix64;
use std::collections::BTreeMap;

use data_model::pathfinding::{self, Labels, Visit};
use std::fmt;

use data_model::{
    ActionAim, ActionCheck, ActionEffect, ActionSpec, AgentActionKind, AgentKind, AgentRules,
    BuildingCategory, CharacterId, EffectSubject, FactionId, GameData, ProvinceId, SettlementId,
    SettlementKind,
};
use serde::{Deserialize, Serialize};

use crate::diplomacy::{PAPACY_FACTION, REBELS_FACTION};
use crate::events::{EventKind, GameEvent};
use crate::movement;
use crate::negotiation::{Article, Treaty};
use crate::orders::Order;
use crate::plan_cache::PlanCache;
use crate::rng::CampaignRng;
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
    data.agent_rules
        .as_ref()
        .unwrap_or_else(|| AgentRules::bundled())
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
/// saves written before agents existed load with none).
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

/// Generator of one roll: depends only on the game seed, the turn, the agent
/// and a salt (action serial, phase).
pub fn derived_rng(seed: u64, turn: u32, agent: u32, salt: u32) -> CampaignRng {
    let mixed = splitmix64(
        splitmix64(seed ^ 0xA6E5_7C6A_6E75_0000)
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

mod ai;
mod effects;
mod orders;
mod paths;
mod queries;
mod turn;

pub use ai::plan_agents;
#[cfg(feature = "test-support")]
pub use paths::agent_dijkstra_by_ids;
pub use paths::{agent_dijkstra, AgentTable};
pub use queries::level_for;
use queries::{herald_may_ransom, ransom_price};
use turn::{failure_text, fill, walk_agent};
pub(crate) use turn::{resolve_agents, start_season};
