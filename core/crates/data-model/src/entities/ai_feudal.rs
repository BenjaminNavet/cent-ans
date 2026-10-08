//! Weights of the feudal AI (lot FE5, spec § 5), mirroring
//! `data/schemas/ai_feudal.schema.json` (`data/ai/feudal.json`).
//!
//! Every decision of a lord or a vassal (protection, arbitration, host,
//! forfeiture, grant, revolt, change of allegiance, title demanded at the
//! peace) is a score of these weights, shifted by the faction's
//! `ai_personality`. The defaults keep a data tree without the file close
//! to the provisional rules of the core (`sim_campaign::feudal`).

use serde::{Deserialize, Serialize};

/// Contents of `data/ai/feudal.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct AiFeudal {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub protection: ProtectionWeights,
    pub arbitration: ArbitrationAi,
    pub host: HostWeights,
    pub commise: CommiseWeights,
    pub revolt: RevoltWeights,
    pub allegiance: AllegianceWeights,
    pub grant: GrantWeights,
    pub demand_title: DemandTitleWeights,
    pub foreign_alliance: ForeignAllianceRules,
    pub light_evaluation: LightEvaluation,
}

/// A suzerain called to protect a vassal (§ 4.3.4): it intervenes when
/// the score reaches `threshold`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectionWeights {
    /// Duty and prestige at stake in shirking.
    pub base: i32,
    /// Points per doubling of the power ratio (liege and its loyal direct
    /// vassals against the aggressor and its allies).
    pub power_weight: i32,
    /// Cap of |log2(ratio)|.
    pub max_power_doublings: f64,
    /// Attitude of the liege towards the vassal, divided by this.
    pub attitude_divisor: i32,
    /// Loyalty of the vassal above `loyalty_pivot`, divided by this.
    pub loyalty_pivot: u8,
    pub loyalty_divisor: i32,
    pub empty_treasury: i32,
    pub per_ongoing_war: i32,
    pub allied_with_aggressor: i32,
    /// Personality: `(aggression - 50) * weight / 50`.
    pub aggression_weight: i32,
    /// Personality: `(diplomacy - 50) * weight / 50` (keeping one's word).
    pub diplomacy_weight: i32,
    pub threshold: i32,
    /// Half-width of the « uncertain » band shown before a war.
    pub certainty_margin: i32,
}

crate::bundled_rules!(ProtectionWeights, "ai/feudal.json", at "/protection", default);

/// A lord arbitrating the private war of two direct vassals (§ 4.3.5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArbitrationAi {
    /// Attitude gap between the two parties to take a side.
    pub take_side_attitude_gap: i32,
    /// Personality: the gap shrinks by `(aggression - 50) * weight / 50`.
    pub aggression_weight: i32,
    /// Lord's power over the attacker's needed to impose peace.
    pub impose_peace_power_ratio: f64,
    /// A lord already in more wars than this lets them be.
    pub max_wars_to_impose: usize,
}

crate::bundled_rules!(ArbitrationAi, "ai/feudal.json", at "/arbitration", default);

/// A direct vassal summoned to the host (§ 4.1): it answers when the score
/// reaches `threshold`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostWeights {
    /// Loyalty points above this count for, below it against.
    pub loyalty_pivot: u8,
    pub loyalty_weight: i32,
    /// Liege this many times stronger than the vassal: refusing is felony
    /// the vassal cannot afford.
    pub fear_power_ratio: f64,
    pub fear_bonus: i32,
    /// Enemy this many times stronger than the liege: a lost cause.
    pub lost_cause_power_ratio: f64,
    pub lost_cause: i32,
    /// Attitude of the vassal towards the enemy, divided by this (subtracted).
    pub enemy_attitude_divisor: i32,
    pub allied_with_enemy: i32,
    pub aggression_weight: i32,
    pub threshold: i32,
}

crate::bundled_rules!(HostWeights, "ai/feudal.json", at "/host", default);

/// A liege with an open felony case declaring forfeiture (§ 4.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommiseWeights {
    /// Liege's power over the felon, its allies and its direct vassals.
    pub min_power_ratio: f64,
    /// Personality: the ratio needed is divided by `1 + (aggression - 50) / 50 * shift`.
    pub aggression_shift: f64,
    /// A liege in more wars than this waits.
    pub max_wars: usize,
    /// A liege with less gold waits.
    pub min_treasury: i64,
}

crate::bundled_rules!(CommiseWeights, "ai/feudal.json", at "/commise", default);

/// A disloyal AI vassal (below `feudal.json:rebellion_loyalty`) revolting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevoltWeights {
    /// Vassal and its allies against its liege.
    pub min_power_ratio: f64,
    /// Chance per turn (‰) once the odds are good enough.
    pub chance_permille: u32,
    /// Personality: ‰ added per point of aggression above 50.
    pub aggression_permille: i32,
}

crate::bundled_rules!(RevoltWeights, "ai/feudal.json", at "/revolt", default);

/// A disloyal vassal (below `feudal.json:disloyal_threshold`), or a
/// threatened sovereign county, paying homage to another lord (§ 4.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllegianceWeights {
    /// Attitude of the vassal towards its new lord.
    pub min_attitude: i32,
    /// New lord's power over the current liege's.
    pub min_power_ratio: f64,
    /// Chance per turn (‰).
    pub chance_permille: u32,
}

crate::bundled_rules!(AllegianceWeights, "ai/feudal.json", at "/allegiance", default);

/// A lord granting a spare title to a loyal direct vassal (§ 4.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantWeights {
    /// Titles a lord keeps besides its primary one.
    pub max_kept_titles: usize,
    /// Loyalty of the vassal rewarded.
    pub min_vassal_loyalty: u8,
    /// Turns between two grants of one lord.
    pub period_turns: u32,
}

crate::bundled_rules!(GrantWeights, "ai/feudal.json", at "/grant", default);

/// A winner demanding titles at the peace (§ 4.6, `Article::DemandTitle`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DemandTitleWeights {
    pub enabled: bool,
    /// Share (%) of the title's own provinces the winner controls.
    pub min_controlled_percent: u32,
}

crate::bundled_rules!(DemandTitleWeights, "ai/feudal.json", at "/demand_title", default);

/// May a vassal ally outside its liege (ADR 0110)?
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForeignAllianceRules {
    /// A vassal whose liege is neither the courter's enemy nor allied to
    /// it may be courted; allying with its liege's enemy stays a felony.
    pub allowed: bool,
}

crate::bundled_rules!(ForeignAllianceRules, "ai/feudal.json", at "/foreign_alliance", default);

/// Lighter feudal planning for the factions with no army and no war
/// (spec § 5, performance fallback).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LightEvaluation {
    pub enabled: bool,
    /// Such factions plan their feudal decisions one turn in `period`.
    pub period: u32,
}

crate::bundled_rules!(LightEvaluation, "ai/feudal.json", at "/light_evaluation", default);
