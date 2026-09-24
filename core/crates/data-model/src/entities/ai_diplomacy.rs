//! Tuning of the AI's wars, alliances and peaces around its borders (lot
//! G5), mirroring `data/schemas/ai_diplomacy.schema.json` (file
//! `data/ai/diplomacy.json`). Without the file the simulation keeps the F4
//! constants ([`AiDiplomacy::default`]).

use serde::{Deserialize, Serialize};

/// A much stronger neighbour is a threat: attitude malus, alliance bonus.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MenacingNeighbourRules {
    /// A bordering faction this many times more powerful menaces us.
    pub power_ratio: f64,
    /// Attitude towards a menacing neighbour that is not our ally.
    pub attitude: i32,
    /// Alliance bonus with a proposer rivalling a neighbour menacing us.
    pub counterweight: i32,
}

/// When a crown opens a new front of its own.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WarPlanningRules {
    /// No new war while the enemies pressing on us (bordering or holding
    /// our lands) weigh more than this share of our power.
    pub front_share: f64,
    /// Coalition power ratio a pretender needs with allies or a border.
    pub pretender_ratio: f64,
    /// Ratio a pretender needs with neither.
    pub pretender_ratio_alone: f64,
}

/// When a faction joins an ally's war (co-belligerence).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JoinWarRules {
    /// Coalition power ratio against the enemy.
    pub ratio: f64,
    /// Attitude towards the ally above which we follow it.
    pub min_attitude: i32,
    /// The ally's power, as a multiple of ours, at or above which we follow
    /// it: lesser princes follow a great crown into war (the Low Countries
    /// behind Edward III, Scotland behind France), not the other way round.
    pub min_ally_power_ratio: f64,
    /// A border with the enemy is reason enough only when the ally's war is
    /// a war of claims (one side claims the other's throne or provinces);
    /// otherwise we need a claim of our own.
    pub border_only_claim_wars: bool,
}

/// Peace terms the AI asks or offers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PeaceRules {
    /// A crown never cedes (nor is asked for) its capital: the treaty of
    /// Northampton leaves a Scotland, Brétigny a France.
    pub keep_capital: bool,
    /// A crown holding this many of its own provinces or fewer sues for
    /// peace every season, whatever the war score (0: never).
    pub cornered_provinces: usize,
}

/// Contents of `data/ai/diplomacy.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiDiplomacy {
    pub menacing_neighbour: MenacingNeighbourRules,
    pub war: WarPlanningRules,
    pub join_war: JoinWarRules,
    pub peace: PeaceRules,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl Default for AiDiplomacy {
    /// The F4 constants, in force before G5.
    fn default() -> Self {
        Self {
            menacing_neighbour: MenacingNeighbourRules {
                power_ratio: 1.5,
                attitude: -15,
                counterweight: 10,
            },
            war: WarPlanningRules {
                front_share: 0.5,
                pretender_ratio: 0.5,
                pretender_ratio_alone: 0.8,
            },
            join_war: JoinWarRules {
                ratio: 0.6,
                min_attitude: 10,
                min_ally_power_ratio: 0.0,
                border_only_claim_wars: false,
            },
            peace: PeaceRules {
                keep_capital: false,
                cornered_provinces: 0,
            },
            description: None,
        }
    }
}
