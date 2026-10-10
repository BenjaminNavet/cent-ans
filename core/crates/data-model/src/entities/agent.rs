//! Campaign agents (spies, heralds, preachers), mirroring
//! `data/schemas/agent_rules.schema.json` (`data/rules/agents.json`).
//! See `docs/design/2026-09-24-agents.md`.

use crate::key_enum;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::entities::settlement::SettlementKind;

key_enum! {
/// Type of agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    /// Espion: sees, reports, sabotages, stirs up revolts.
    Spy => "spy",
    /// Héraut: embassies, truces, bribes, ransoms.
    Emissary => "emissary",
    /// Prédicateur: preaching against heresy, denouncing schismatics, the Curia.
    Preacher => "preacher",
    /// Marchand: settles in a trading place and earns a seasonal income (ADR 0332).
    Merchant => "merchant",
}
}

key_enum! {
/// An action an agent can attempt (one per season).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentActionKind {
    /// Spy: detailed report on a settlement and sight of its province.
    Scout => "scout",
    /// Spy: opens a breach in a besieged town, or delays works.
    Sabotage => "sabotage",
    /// Spy: stirs up the province's unrest.
    Incite => "incite",
    /// Spy: unmasks foreign agents in the province.
    Counter => "counter",
    /// Herald: improves the opinion of the settlement's master.
    Parley => "parley",
    /// Herald: carries a truce proposal to an enemy.
    Truce => "truce",
    /// Herald: buys the garrison of an enemy settlement.
    Bribe => "bribe",
    /// Herald: buys back one of our captives at a discount.
    Ransom => "ransom",
    /// Preacher: fights heresy and unrest in a friendly province.
    Preach => "preach",
    /// Preacher: preaches against an enemy, excommunicated or schismatic prince.
    Denounce => "denounce",
    /// Preacher: pleads at the papal court (papal favour).
    Curia => "curia",
    /// Spy: strikes a general or governor of the place (death or wound).
    Assassinate => "assassinate",
    /// Spy: poisons a general or governor of the place, in secret.
    Poison => "poison",
    /// Herald: guides a friendly army near the place (longer march, sight).
    GuideArmy => "guide_army",
    /// Spy: ambushes a hostile army near the place (morale, delay).
    Ambush => "ambush",
    /// Merchant: sets up a trading post in the place where he stands.
    TradePost => "trade_post",
    /// Merchant: buys out or drives away a rival merchant's trading post.
    Outbid => "outbid",
}
}

impl AgentActionKind {
    /// The agent type able to attempt this action.
    pub fn agent(self) -> AgentKind {
        match self {
            AgentActionKind::Scout
            | AgentActionKind::Sabotage
            | AgentActionKind::Incite
            | AgentActionKind::Counter
            | AgentActionKind::Assassinate
            | AgentActionKind::Poison
            | AgentActionKind::Ambush => AgentKind::Spy,
            AgentActionKind::Parley
            | AgentActionKind::Truce
            | AgentActionKind::Bribe
            | AgentActionKind::Ransom
            | AgentActionKind::GuideArmy => AgentKind::Emissary,
            AgentActionKind::Preach | AgentActionKind::Denounce | AgentActionKind::Curia => {
                AgentKind::Preacher
            }
            AgentActionKind::TradePost | AgentActionKind::Outbid => AgentKind::Merchant,
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
    /// Report when an action fails and the agent is lost (`{agent}`).
    pub lost_fr: String,
}

/// A condition on the target of an action, evaluated in table order: the
/// first one that fails refuses the action with its `reason_fr`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionCheck {
    /// The settlement's master is another faction than the agent's.
    ForeignMaster,
    /// The master is not the rebels.
    NotRebels,
    AtWar,
    NotAtWar,
    /// The master is the agent's faction or an ally of it.
    AlliedMaster,
    /// No embassy of this faction is still running at the master's court.
    NoEmbassy,
    NoTruce,
    /// No hostile army stands on the settlement.
    NoHostileArmy,
    /// The faction has a captive the master holds for money (selects it).
    CaptiveHeld,
    /// The province is friendly territory.
    FriendlyProvince,
    /// The province's master is an enemy, an excommunicated or a rival-obedience prince.
    DenounceableMaster,
    /// The settlement belongs to the Papacy.
    PapalCity,
    CatholicFaction,
    /// A general standing at the place, or the governor of its province,
    /// belongs to the master and may be struck (selects it; the sovereign
    /// only from `target_ruler_min_level`).
    HostileCharacter,
    /// An army of the agent's faction or an ally is within `army_reach_km`.
    FriendlyArmyNear,
    /// An army at war with the agent's faction is within `army_reach_km`.
    HostileArmyNear,
    /// The target is the agent's own place, of a kind that takes a trading
    /// post, and the agent does not already hold one there.
    PostFree,
    /// A merchant of a faction that is neither ours nor an ally holds a post
    /// in the target (the agent's own place).
    RivalPost,
}

/// One condition of an action and its French refusal text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionCondition {
    pub check: ActionCheck,
    pub reason_fr: String,
}

/// Whose opinion/favour an effect touches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionAim {
    /// The master of the targeted settlement (default).
    #[default]
    SettlementMaster,
    /// The master of the targeted settlement's province.
    ProvinceMaster,
}

/// Who an effect applies to (papal favour).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectSubject {
    /// The agent's own faction.
    #[default]
    Own,
    /// The action's target faction.
    Target,
}

/// One effect of an action. Texts are French templates with `{agent}`,
/// `{place}`, `{province}` (as "de X"), `{target}`, `{value}`, `{turns}`,
/// `{cost}`, `{captive}`, `{report}`, `{names}`; an empty text says nothing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActionEffect {
    /// Detailed report on the settlement, province kept in sight.
    Scout { intel_turns: u32, text_fr: String },
    /// Breach and spoiled supplies in a siege, else delayed works and morale.
    Sabotage {
        breach: u8,
        supplies: u8,
        delay_turns: u32,
        morale: u8,
        siege_fr: String,
        works_fr: String,
        troubled_fr: String,
    },
    /// Unrest of the province's classes (`delta` signed, plus `per_level` per seal above the first).
    Unrest {
        classes: Vec<crate::SocialClass>,
        delta: i32,
        #[serde(default)]
        per_level: i32,
        #[serde(default)]
        text_fr: String,
    },
    /// Pushes the province's heresy back.
    Heresy {
        drop: u8,
        per_level: u8,
        receded_fr: String,
        uprooted_fr: String,
    },
    /// ADR 0326: a sermon pushes the conversion of the province to its
    /// lord's faith (`data/rules/religion.json` `conversion.preacher_*`).
    Conversion { converted_fr: String },
    /// Opinion modifier of the target faction about the agent's, optionally
    /// negative; `value + per_level * (seal - 1)`, divided by `divisor`.
    Opinion {
        reason_fr: String,
        value: i32,
        #[serde(default)]
        per_level: i32,
        #[serde(default = "one")]
        divisor: i32,
        turns: u32,
        #[serde(default)]
        text_fr: String,
    },
    /// Proposes a mediation (truce) treaty to the target faction.
    Mediation {
        turns: u32,
        proposed_fr: String,
        agreed_fr: String,
        refused_fr: String,
    },
    /// Papal favour (negative: lost).
    Favor {
        on: EffectSubject,
        amount: i32,
        #[serde(default)]
        per_level: i32,
        #[serde(default)]
        text_fr: String,
    },
    /// The garrison opens the gates.
    Capture { text_fr: String },
    /// Buys the selected captive back.
    Release { text_fr: String },
    /// Strikes the selected character: dies with `kill_percent` (plus
    /// `kill_per_level` per seal above the first), else is wounded with
    /// `wound_percent`; a revealed killing costs the master's goodwill.
    Strike {
        wound_trait: crate::TraitId,
        kill_percent: u32,
        #[serde(default)]
        kill_per_level: u32,
        wound_percent: u32,
        /// Opinion loss of the victim's faction towards the agent's (0: none).
        #[serde(default)]
        scandal: i32,
        #[serde(default)]
        scandal_turns: u32,
        killed_fr: String,
        wounded_fr: String,
        unharmed_fr: String,
    },
    /// Friendly armies near the place march further this turn and their
    /// province is kept in sight.
    GuideArmy {
        movement_percent: u32,
        intel_turns: u32,
        text_fr: String,
    },
    /// Hostile armies near the place lose movement and morale.
    Ambush {
        movement_percent: u32,
        morale: u8,
        morale_turns: u8,
        text_fr: String,
    },
    /// ADR 0332: the merchant takes up a trading post at the place.
    TradePost { text_fr: String },
    /// ADR 0332: rival merchants' posts of the place are dropped (the rivals
    /// stay, without a post) and the agent takes the post over.
    Outbid { text_fr: String },
    /// Rolls against every foreign agent of the province.
    Unmask {
        found_fr: String,
        none_fr: String,
        executed_fr: String,
    },
}

fn one() -> i32 {
    1
}

/// Price of an action: `ransom` (price of the selected captive at the
/// agent's seal) plus `flat + garrison * per_man_percent / 100`, the latter
/// times the faction's price level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionCost {
    #[serde(default)]
    pub flat: u32,
    #[serde(default)]
    pub per_man_percent: u32,
    #[serde(default)]
    pub ransom: bool,
}

/// An action of the table `data/rules/agents.json` `actions`: price, range,
/// conditions, odds, effects and labels, run by the single engine of
/// `sim-campaign/src/agents.rs` (ADR 0209).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionSpec {
    /// French display name.
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Actions of one group share a single button of the action bar (the
    /// first available one is shown).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// Success chance (percent) of a first-seal agent.
    pub base_chance: i32,
    /// Extra percent per seal above the first.
    pub per_level: i32,
    /// Percent chance of death (or jail) when the action fails.
    #[serde(default)]
    pub death_risk: u32,
    /// Hostile: fortification and enemy spies lower the chance.
    #[serde(default)]
    pub hostile: bool,
    /// One percent of success lost per this many men of garrison (0: none).
    #[serde(default)]
    pub garrison_men_per_malus: u32,
    #[serde(default)]
    pub cost: ActionCost,
    #[serde(default)]
    pub aim: ActionAim,
    /// The target's master learns of the attempt (always on failure).
    #[serde(default)]
    pub reveals: bool,
    #[serde(default)]
    pub conditions: Vec<ActionCondition>,
    pub success: Vec<ActionEffect>,
    /// Lowest seal at which the sovereign himself may be a `HostileCharacter`
    /// (absent: never).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_ruler_min_level: Option<u8>,
    /// Applied on failure (the base text is built by the engine).
    #[serde(default)]
    pub failure: Vec<ActionEffect>,
    /// Failure suffix, with `{cost}`, `{target}`, `{captive}`.
    #[serde(default)]
    pub failure_extra_fr: String,
}

/// Contents of `data/rules/agents.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentRules {
    pub types: BTreeMap<AgentKind, AgentTypeRules>,
    pub actions: BTreeMap<AgentActionKind, ActionSpec>,
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
    /// Seasonal income (livres) under which an AI faction keeps no
    /// more than a spy in wartime, like a minor power (FE made about ninety
    /// factions playable, counties of 80 livres included). 0: every playable
    /// faction keeps one agent of each kind.
    #[serde(default)]
    pub ai_network_min_income: i64,
    /// Ransom bought by a herald: percent of the fair price at seal 1, minus
    /// `ransom_price_per_level` per further seal, never under the floor.
    #[serde(default = "default_ransom_percent")]
    pub ransom_price_percent: i64,
    #[serde(default = "default_ransom_per_level")]
    pub ransom_price_per_level: i64,
    #[serde(default = "default_ransom_floor")]
    pub ransom_price_floor: i64,
    /// Reach (km) around a settlement within which `GuideArmy` and `Ambush`
    /// find armies.
    #[serde(default = "default_army_reach_km")]
    pub army_reach_km: f64,
    /// WR ai-agents (ADR 0300): when and how often AI agents strike, guide
    /// and ambush. Zeroes disable every behaviour.
    #[serde(default)]
    pub ai_agents: AiAgentRules,
    /// Trading posts of the merchants (ADR 0332).
    #[serde(default)]
    pub merchant: MerchantRules,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Trading posts (ADR 0332): the seasonal income of a posted merchant is
/// `income[kind of the place] + resource_income * resources of the province`,
/// raised by `per_level_percent` per seal above the first, times the faction's
/// price level; a place held by another faction keeps `host_share_percent` of
/// it; a second merchant in the same place earns only `rival_share_percent`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MerchantRules {
    /// Base income (livres per season) by kind of place; kinds absent take no post.
    #[serde(default)]
    pub income: BTreeMap<SettlementKind, i64>,
    /// Extra livres per resource of the place's province.
    #[serde(default)]
    pub resource_income: i64,
    /// Percent of income gained per seal above the first.
    #[serde(default)]
    pub per_level_percent: i64,
    /// Percent of the income a merchant keeps when a rival is also posted.
    #[serde(default = "hundred")]
    pub rival_share_percent: i64,
    /// Percent of the income paid to the master of a foreign place.
    #[serde(default)]
    pub host_share_percent: i64,
}

fn hundred() -> i64 {
    100
}

fn default_army_reach_km() -> f64 {
    30.0
}
fn default_ransom_percent() -> i64 {
    70
}
fn default_ransom_per_level() -> i64 {
    5
}
fn default_ransom_floor() -> i64 {
    45
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

crate::bundled_rules!(AgentRules, "rules/agents.json", default);

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

#[cfg(test)]
mod key_enum_tests {
    use super::*;
    use crate::key_enum::assert_keys_match_serde;

    #[test]
    fn keys_match_serde_names() {
        assert_keys_match_serde::<AgentKind>();
        assert_keys_match_serde::<AgentActionKind>();
    }
}

/// Decision rules of AI agents for the WH actions (ADR 0300). Every chance
/// is a percent rolled once per agent and season, only when a valid target
/// stands within reach.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiAgentRules {
    /// Chance that a spy facing a valid target attempts `assassinate`.
    #[serde(default)]
    pub assassinate_chance: u32,
    /// Chance (rolled after the assassination) of attempting `poison`.
    #[serde(default)]
    pub poison_chance: u32,
    /// Chance that a spy facing a marching enemy army attempts `ambush`.
    #[serde(default)]
    pub ambush_chance: u32,
    /// Chance that a herald near a marching friendly army attempts `guide_army`.
    #[serde(default)]
    pub guide_chance: u32,
    /// Lowest success chance (percent) worth a strike.
    #[serde(default)]
    pub min_strike_odds: u32,
    /// Lowest success chance (percent) worth an ambush or a guide.
    #[serde(default)]
    pub min_support_odds: u32,
    /// Treasury a faction keeps after paying for an action (livres).
    #[serde(default)]
    pub reserve: i64,
    /// A general only counts as dangerous leading at least this strength.
    #[serde(default)]
    pub min_general_strength: u32,
    /// A governor is struck only when the target faction has no general in
    /// reach and this is true.
    #[serde(default)]
    pub strike_governors: bool,
    /// An enemy army must be at least this strong to be ambushed.
    #[serde(default)]
    pub min_ambush_strength: u32,
    /// A friendly army must be at least this strong to be guided.
    #[serde(default)]
    pub min_guide_strength: u32,
    /// Strikes (assassinations and poisonings) a faction may attempt per
    /// `window_turns`.
    #[serde(default)]
    pub strike_cap: u32,
    #[serde(default)]
    pub window_turns: u32,
    /// Percent added to every chance per point of the difficulty's
    /// `ai_aggression_delta` (can be negative; result floored at 0).
    #[serde(default)]
    pub aggression_percent_per_point: i32,
}
