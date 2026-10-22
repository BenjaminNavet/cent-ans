//! Short-term campaign missions (ADR 0127), mirroring
//! `data/schemas/missions.schema.json` (`data/missions.json`).

use crate::key_enum;
use serde::{Deserialize, Serialize};

/// Contents of `data/missions.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MissionRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Missions active at most at the same time (1-2).
    pub max_active: u32,
    /// First turn a mission may be offered.
    pub first_turn: u32,
    /// Turns to wait after a success or a failure before a new offer.
    pub offer_cooldown_turns: u32,
    /// Ruler prestige on failure (0 or a small negative number).
    pub failure_prestige: i32,
    /// WR turn (ADR 0304): candidate missions of one offer (the player picks
    /// one or refuses all).
    #[serde(default = "default_offer_candidates")]
    pub offer_candidates: u32,
    /// WR turn: turns an ignored offer stays open before it lapses.
    #[serde(default = "default_offer_expiry")]
    pub offer_expiry_turns: u32,
    pub templates: Vec<MissionTemplate>,
    /// Dated grudges (ADR 0720).
    pub grudge: GrudgeRules,
}

crate::bundled_rules!(MissionRules, "missions.json", default);

key_enum! {
/// Where a mission points to: the places the generator may draw from (empty:
/// the template is not offered).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissionTarget {
    /// No place; offered while the player is at war with a living faction.
    AtWar => "at_war",
    /// No place; offered while the player can raise a unit somewhere.
    CanRecruit => "can_recruit",
    /// No place; offered while another faction could sign a treaty.
    TreatyPartner => "treaty_partner",
    /// A province next to the player's, held by a faction at war with it.
    EnemyNeighbour => "enemy_neighbour",
    /// A province of the player's, next to an enemy-held one.
    ThreatenedOwn => "threatened_own",
    /// A building the player can start in one of its cities (`{lieu}`).
    Buildable => "buildable",
    /// The template's own `province` (faction missions, WH turn): offered
    /// while the player does not hold it (`control`) or holds it (`hold`).
    Fixed => "fixed",
}
}

key_enum! {
/// When a mission is fulfilled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissionGoal {
    /// The target province is controlled by the player.
    Control => "control",
    /// The target province is still the player's at the deadline (lost: failed).
    Hold => "hold",
    /// The target building stands in the target city.
    Build => "build",
    /// A peace, alliance or vassalage the player did not have at the offer.
    Treaty => "treaty",
    /// The template's `counter` reached `count`.
    Count => "count",
}
}

key_enum! {
/// What a `count` goal tallies since the mission was given.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissionCounter {
    /// Battles (assaults and sorties included) won by the player.
    BattleWon => "battle_won",
    /// Units recruited or hired by the player.
    UnitsRecruited => "units_recruited",
    /// Agent actions that succeeded (TW m2a).
    AgentActions => "agent_actions",
    /// Marriages concluded by the player's house (TW m2a).
    Marriages => "marriages",
    /// Ransoms paid or received by the player (TW m2a).
    Ransoms => "ransoms",
}
}

/// One mission template: a target generator, a goal, a counter, texts and a
/// reward; one engine serves them all (ADR 0207).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MissionTemplate {
    pub id: String,
    pub target: MissionTarget,
    pub goal: MissionGoal,
    /// Only for the `count` goal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counter: Option<MissionCounter>,
    /// French title; `{cible}`, `{lieu}` and `{n}` are filled in.
    pub title: String,
    /// French objective text; same placeholders.
    pub objective: String,
    /// Progress labels by step, for the objectives panel (`{holder}`: the
    /// target's holder). The ratio is `step / (len - 1)`. Without steps the
    /// `progress` text is used (`{done}`, `{n}`) and the ratio is done / n.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub progress: String,
    /// Turns before the deadline (3-12); for `hold`, the duration to hold.
    pub duration: u32,
    /// Number asked for (battles, units); for `hold`, the seasons.
    #[serde(default = "one")]
    pub count: u32,
    /// Relative weight of the template among the plausible ones.
    #[serde(default = "one")]
    pub weight: u32,
    pub reward: MissionReward,
    /// Faction mission: offered to this faction only (absent: everyone).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faction: Option<crate::FactionId>,
    /// Chain: offered only once the template of this id has succeeded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    /// Target province of a `fixed` mission.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub province: Option<crate::ProvinceId>,
    /// French historical note shown in the objectives panel (source).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
}

fn default_offer_candidates() -> u32 {
    3
}

fn default_offer_expiry() -> u32 {
    2
}

fn one() -> u32 {
    1
}

/// Dated grudges (ADR 0720): missions anchored in an affront suffered.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrudgeRules {
    /// Grudges kept at most (the oldest are forgotten first).
    pub max_grudges: u32,
    /// Age tiers, ascending: the last tier whose `min_age` (turns = seasons)
    /// is reached gives the reward multiplier.
    pub age_tiers: Vec<GrudgeAgeTier>,
    /// Emblematic places: losing one weighs more in prestige.
    pub emblematic_settlements: Vec<crate::SettlementId>,
    /// Prestige multiplier when the place lost is emblematic.
    pub emblematic_prestige_multiplier: f64,
    /// Failure penalty = `failure_prestige` x tier multiplier, capped at this.
    pub failure_multiplier_cap: f64,
    /// Lost settlement.
    pub lost_settlement: GrudgeKindRules,
    /// Army of ours defeated.
    pub defeated_army: GrudgeKindRules,
    /// Province of ours raided.
    pub raided_province: GrudgeKindRules,
}

/// One age tier of a grudge.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrudgeAgeTier {
    pub min_age: u32,
    pub multiplier: f64,
}

/// How one kind of grudge becomes a mission.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrudgeKindRules {
    /// Weight in the offer's draw (like a template's).
    pub weight: u32,
    pub goal: MissionGoal,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counter: Option<MissionCounter>,
    pub duration: u32,
    /// `{cible}` = place or province, `{auteur}` = faction at fault.
    pub title: String,
    pub objective: String,
    /// Progress label of a `count` goal ({done}, {n}).
    #[serde(default)]
    pub progress: String,
    /// Reward at age multiplier 1.
    pub reward: MissionReward,
}

impl GrudgeRules {
    /// Reward multiplier for a grudge `age` turns old.
    pub fn age_multiplier(&self, age: u32) -> f64 {
        self.age_tiers
            .iter()
            .filter(|t| t.min_age <= age)
            .next_back()
            .map_or(1.0, |t| t.multiplier)
    }
}

/// What a success brings.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MissionReward {
    /// Livres paid into the treasury.
    #[serde(default)]
    pub gold: i64,
    /// Ruler prestige.
    #[serde(default)]
    pub prestige: i32,
    /// Unrest removed from the target province (or the capital).
    #[serde(default)]
    pub public_order: u8,
    /// A free company joins the capital's garrison.
    #[serde(default)]
    pub free_unit: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_file_parses_and_is_consistent() {
        let rules = MissionRules::default();
        assert!((1..=2).contains(&rules.max_active));
        assert!(rules.templates.len() >= 6);
        let g = &rules.grudge;
        assert!(g.age_tiers.windows(2).all(|w| w[0].min_age < w[1].min_age));
        assert!((g.age_multiplier(0) - 1.0).abs() < 1e-9);
        assert!(g.age_multiplier(40) >= g.age_multiplier(10));
        let mut ids = std::collections::BTreeSet::new();
        for t in &rules.templates {
            assert!(ids.insert(&t.id), "{}", t.id);
            assert!((3..=12).contains(&t.duration), "{}", t.id);
            assert_eq!(
                t.goal == MissionGoal::Count,
                t.counter.is_some(),
                "{}: counter only for count",
                t.id
            );
            assert!(!t.steps.is_empty() || !t.progress.is_empty(), "{}", t.id);
        }
    }
}

#[cfg(test)]
mod key_enum_tests {
    use super::*;
    use crate::key_enum::assert_keys_match_serde;

    #[test]
    fn keys_match_serde_names() {
        assert_keys_match_serde::<MissionTarget>();
        assert_keys_match_serde::<MissionGoal>();
        assert_keys_match_serde::<MissionCounter>();
    }
}
