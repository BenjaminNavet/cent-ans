//! Campaign agents (lot C6: spies, heralds, preachers), mirroring
//! `data/schemas/agent_rules.schema.json` (`data/rules/agents.json`).
//! See `docs/design/2026-09-24-agents.md`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::entities::settlement::SettlementKind;

/// Type of agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    /// Espion: sees, reports, sabotages, stirs up revolts.
    Spy,
    /// Héraut: embassies, truces, bribes, ransoms.
    Emissary,
    /// Prédicateur: preaching against heresy, denouncing schismatics, the Curia.
    Preacher,
}

impl AgentKind {
    pub const ALL: [AgentKind; 3] = [AgentKind::Spy, AgentKind::Emissary, AgentKind::Preacher];

    /// Stable snake_case key (the serde name).
    pub fn key(self) -> &'static str {
        match self {
            AgentKind::Spy => "spy",
            AgentKind::Emissary => "emissary",
            AgentKind::Preacher => "preacher",
        }
    }

    /// Parses a snake_case key.
    pub fn from_key(key: &str) -> Option<AgentKind> {
        AgentKind::ALL.into_iter().find(|k| k.key() == key)
    }
}

/// An action an agent can attempt (one per season).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentActionKind {
    /// Spy: detailed report on a settlement and sight of its province.
    Scout,
    /// Spy: opens a breach in a besieged town, or delays works.
    Sabotage,
    /// Spy: stirs up the province's unrest.
    Incite,
    /// Spy: unmasks foreign agents in the province.
    Counter,
    /// Herald: improves the opinion of the settlement's master.
    Parley,
    /// Herald: carries a truce proposal to an enemy.
    Truce,
    /// Herald: buys the garrison of an enemy settlement.
    Bribe,
    /// Herald: buys back one of our captives at a discount.
    Ransom,
    /// Preacher: fights heresy and unrest in a friendly province.
    Preach,
    /// Preacher: preaches against an enemy, excommunicated or schismatic prince.
    Denounce,
    /// Preacher: pleads at the papal court (papal favour).
    Curia,
}

impl AgentActionKind {
    pub const ALL: [AgentActionKind; 11] = [
        AgentActionKind::Scout,
        AgentActionKind::Sabotage,
        AgentActionKind::Incite,
        AgentActionKind::Counter,
        AgentActionKind::Parley,
        AgentActionKind::Truce,
        AgentActionKind::Bribe,
        AgentActionKind::Ransom,
        AgentActionKind::Preach,
        AgentActionKind::Denounce,
        AgentActionKind::Curia,
    ];

    /// Stable snake_case key (the serde name).
    pub fn key(self) -> &'static str {
        match self {
            AgentActionKind::Scout => "scout",
            AgentActionKind::Sabotage => "sabotage",
            AgentActionKind::Incite => "incite",
            AgentActionKind::Counter => "counter",
            AgentActionKind::Parley => "parley",
            AgentActionKind::Truce => "truce",
            AgentActionKind::Bribe => "bribe",
            AgentActionKind::Ransom => "ransom",
            AgentActionKind::Preach => "preach",
            AgentActionKind::Denounce => "denounce",
            AgentActionKind::Curia => "curia",
        }
    }

    /// Parses a snake_case key.
    pub fn from_key(key: &str) -> Option<AgentActionKind> {
        AgentActionKind::ALL.into_iter().find(|k| k.key() == key)
    }

    /// The agent type able to attempt this action.
    pub fn agent(self) -> AgentKind {
        match self {
            AgentActionKind::Scout
            | AgentActionKind::Sabotage
            | AgentActionKind::Incite
            | AgentActionKind::Counter => AgentKind::Spy,
            AgentActionKind::Parley
            | AgentActionKind::Truce
            | AgentActionKind::Bribe
            | AgentActionKind::Ransom => AgentKind::Emissary,
            AgentActionKind::Preach | AgentActionKind::Denounce | AgentActionKind::Curia => {
                AgentKind::Preacher
            }
        }
    }
}

/// Recruitment and movement of one agent type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentTypeRules {
    /// French display name.
    pub name: String,
    /// Recruitment cost in livres (times the faction's price level).
    pub cost: u32,
    /// Seasonal upkeep in livres (times the faction's price level).
    pub upkeep: u32,
    pub max_per_faction: u32,
    /// Province steps per season (one less in winter), times
    /// `MovementRules::points_per_step`.
    pub movement_steps: u32,
    /// Land steps of sight around the agent's province (0: the province only).
    #[serde(default)]
    pub vision_range: u32,
    /// Settlement kinds where the agent can be recruited.
    pub settlement_kinds: Vec<SettlementKind>,
    /// Also requires a religious building (an abbey counts as one).
    #[serde(default)]
    pub requires_religious_building: bool,
    /// Names given in turn to new agents.
    #[serde(default)]
    pub names: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Odds and risk of one action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentActionRules {
    /// French display name.
    pub name: String,
    /// Success chance (percent) of a first-seal agent.
    pub base_chance: i32,
    /// Extra percent per seal above the first.
    pub per_level: i32,
    /// Percent chance of death (or jail) when the action fails.
    #[serde(default)]
    pub death_risk: u32,
    /// Flat cost in livres (times the price level); the bribe adds its own.
    #[serde(default)]
    pub cost: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Magnitudes of the action effects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AgentEffects {
    /// Seasons a successful scouting keeps the target province in sight.
    pub intel_turns: u32,
    pub sabotage_breach: u8,
    pub sabotage_supplies: u8,
    pub sabotage_delay_turns: u32,
    pub sabotage_morale: u8,
    pub incite_unrest: u8,
    pub parley_opinion: i32,
    pub parley_opinion_per_level: i32,
    pub parley_turns: u32,
    pub truce_turns: u32,
    pub bribe_base_cost: u32,
    /// Livres per man of the garrison, in per cent (50: half a livre).
    pub bribe_cost_per_man_percent: u32,
    /// One percent of success lost per this many men of garrison.
    pub bribe_men_per_malus: u32,
    pub bribe_fail_opinion: i32,
    /// Share of the ransom paid (percent) by a first-seal herald.
    pub ransom_price_percent: i64,
    pub ransom_price_per_level: i64,
    pub ransom_price_floor: i64,
    pub preach_heresy: u8,
    pub preach_heresy_per_level: u8,
    pub preach_unrest: u8,
    pub denounce_clergy_unrest: u8,
    pub denounce_peasant_unrest: u8,
    pub denounce_favor: u8,
    pub curia_favor: u8,
    pub curia_favor_per_level: u8,
}

impl Default for AgentEffects {
    fn default() -> Self {
        AgentEffects {
            intel_turns: 4,
            sabotage_breach: 25,
            sabotage_supplies: 15,
            sabotage_delay_turns: 2,
            sabotage_morale: 15,
            incite_unrest: 12,
            parley_opinion: 10,
            parley_opinion_per_level: 2,
            parley_turns: 12,
            truce_turns: 8,
            bribe_base_cost: 150,
            bribe_cost_per_man_percent: 50,
            bribe_men_per_malus: 100,
            bribe_fail_opinion: -10,
            ransom_price_percent: 70,
            ransom_price_per_level: 5,
            ransom_price_floor: 45,
            preach_heresy: 8,
            preach_heresy_per_level: 3,
            preach_unrest: 5,
            denounce_clergy_unrest: 10,
            denounce_peasant_unrest: 5,
            denounce_favor: 3,
            curia_favor: 3,
            curia_favor_per_level: 1,
        }
    }
}

/// Contents of `data/rules/agents.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentRules {
    pub types: BTreeMap<AgentKind, AgentTypeRules>,
    pub actions: BTreeMap<AgentActionKind, AgentActionRules>,
    /// Experience needed for seals 2, 3, 4 and 5.
    pub experience_thresholds: Vec<u32>,
    #[serde(default = "default_xp_success")]
    pub xp_success: u32,
    #[serde(default = "default_xp_failure")]
    pub xp_failure: u32,
    #[serde(default = "default_min_chance")]
    pub min_chance: i32,
    #[serde(default = "default_max_chance")]
    pub max_chance: i32,
    /// Percent lost per fortification level of the target (hostile actions).
    #[serde(default = "default_fortification_malus")]
    pub fortification_malus: i32,
    /// Percent lost per enemy spy in the target province: this plus
    /// `counter_spy_malus_per_level` per seal, capped by `counter_spy_malus_cap`.
    #[serde(default = "default_counter_spy_malus")]
    pub counter_spy_malus: i32,
    #[serde(default = "default_counter_spy_malus_per_level")]
    pub counter_spy_malus_per_level: i32,
    #[serde(default = "default_counter_spy_malus_cap")]
    pub counter_spy_malus_cap: i32,
    /// Passive counter-espionage at the end of a season: base, per seal of
    /// the best local spy (minus per seal of the intruder), cap.
    #[serde(default = "default_passive_counter_base")]
    pub passive_counter_base: i32,
    #[serde(default = "default_passive_counter_per_level")]
    pub passive_counter_per_level: i32,
    #[serde(default = "default_passive_counter_cap")]
    pub passive_counter_cap: i32,
    /// Lot F8: seasonal income (livres) under which an AI faction keeps no
    /// more than a spy in wartime, like a minor power (FE made about ninety
    /// factions playable, counties of 80 livres included). 0: every playable
    /// faction keeps one agent of each kind.
    #[serde(default)]
    pub ai_network_min_income: i64,
    #[serde(default)]
    pub effects: AgentEffects,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn default_xp_success() -> u32 {
    2
}
fn default_xp_failure() -> u32 {
    1
}
fn default_min_chance() -> i32 {
    5
}
fn default_max_chance() -> i32 {
    95
}
fn default_fortification_malus() -> i32 {
    4
}
fn default_counter_spy_malus() -> i32 {
    5
}
fn default_counter_spy_malus_per_level() -> i32 {
    3
}
fn default_counter_spy_malus_cap() -> i32 {
    25
}
fn default_passive_counter_base() -> i32 {
    10
}
fn default_passive_counter_per_level() -> i32 {
    5
}
fn default_passive_counter_cap() -> i32 {
    60
}

#[allow(clippy::too_many_arguments)]
fn type_rules(
    name: &str,
    cost: u32,
    upkeep: u32,
    max_per_faction: u32,
    movement_steps: u32,
    vision_range: u32,
    settlement_kinds: &[SettlementKind],
    requires_religious_building: bool,
) -> AgentTypeRules {
    AgentTypeRules {
        name: name.to_owned(),
        cost,
        upkeep,
        max_per_faction,
        movement_steps,
        vision_range,
        settlement_kinds: settlement_kinds.to_vec(),
        requires_religious_building,
        names: Vec::new(),
        description: None,
    }
}

fn action_rules(name: &str, base_chance: i32, per_level: i32, death_risk: u32) -> AgentActionRules {
    AgentActionRules {
        name: name.to_owned(),
        base_chance,
        per_level,
        death_risk,
        cost: 0,
        description: None,
    }
}

impl Default for AgentRules {
    /// The values of `docs/design/2026-09-24-agents.md`, used when
    /// `data/rules/agents.json` is absent (tests with synthetic data).
    fn default() -> Self {
        use AgentActionKind as A;
        use SettlementKind as S;
        let fortified = [S::City, S::Town, S::Castle, S::Abbey];
        let types = BTreeMap::from([
            (
                AgentKind::Spy,
                type_rules("Espion", 250, 20, 3, 4, 1, &fortified, false),
            ),
            (
                AgentKind::Emissary,
                type_rules("Héraut", 300, 25, 2, 4, 0, &[S::City], false),
            ),
            (
                AgentKind::Preacher,
                type_rules("Prédicateur", 200, 15, 2, 3, 0, &fortified, true),
            ),
        ]);
        let actions = BTreeMap::from([
            (A::Scout, action_rules("Renseigner", 75, 5, 10)),
            (A::Sabotage, action_rules("Saboter", 45, 7, 35)),
            (A::Incite, action_rules("Soulever", 40, 7, 35)),
            (A::Counter, action_rules("Contre-espionnage", 50, 8, 0)),
            (A::Parley, action_rules("Ambassade", 70, 5, 0)),
            (A::Truce, action_rules("Trêve", 55, 6, 0)),
            (A::Bribe, action_rules("Soudoyer", 35, 7, 40)),
            (A::Ransom, action_rules("Rançon", 60, 6, 0)),
            (A::Preach, action_rules("Prêcher", 70, 5, 0)),
            (A::Denounce, action_rules("Dénoncer", 50, 6, 15)),
            (A::Curia, action_rules("Plaider à la Curie", 60, 6, 0)),
        ]);
        AgentRules {
            types,
            actions,
            experience_thresholds: vec![2, 5, 9, 14],
            xp_success: default_xp_success(),
            xp_failure: default_xp_failure(),
            min_chance: default_min_chance(),
            max_chance: default_max_chance(),
            fortification_malus: default_fortification_malus(),
            counter_spy_malus: default_counter_spy_malus(),
            counter_spy_malus_per_level: default_counter_spy_malus_per_level(),
            counter_spy_malus_cap: default_counter_spy_malus_cap(),
            passive_counter_base: default_passive_counter_base(),
            passive_counter_per_level: default_passive_counter_per_level(),
            passive_counter_cap: default_passive_counter_cap(),
            ai_network_min_income: 0,
            effects: AgentEffects::default(),
            description: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_action_belongs_to_a_type_and_has_default_rules() {
        let rules = AgentRules::default();
        for action in AgentActionKind::ALL {
            assert!(rules.actions.contains_key(&action), "{action:?}");
            assert!(rules.types.contains_key(&action.agent()));
            assert_eq!(AgentActionKind::from_key(action.key()), Some(action));
        }
        for kind in AgentKind::ALL {
            assert_eq!(AgentKind::from_key(kind.key()), Some(kind));
        }
    }

    #[test]
    fn rules_round_trip_through_json() {
        let rules = AgentRules::default();
        let json = serde_json::to_string(&rules).unwrap();
        let back: AgentRules = serde_json::from_str(&json).unwrap();
        assert_eq!(back, rules);
    }
}
