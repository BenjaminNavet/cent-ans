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
    /// EQ6: the war of a pretender for its main crown (the largest realm
    /// whose throne it claims) comes first: it outranks lesser claims
    /// whatever their odds, and the rest after another declaration (a
    /// crusade, a small dynastic quarrel) does not hold it back. `false`:
    /// the weakest claimed crown first, every war waits the rest (pre-EQ6).
    #[serde(default)]
    pub main_claim_first: bool,
    /// EQ6: the difficulty level (ADR 0037) leaves the main claim war
    /// alone: neither the attitude nor the power ratio it adds towards the
    /// player decide whether the pretender presses its claim. Difficulty
    /// still weighs on every other war, on peace and on battles.
    #[serde(default)]
    pub claim_war_ignores_difficulty: bool,
    /// EQ6: kinship does not hold a pretender back from its main claim
    /// war: marriages between the two houses and a shared ruling house
    /// leave out of the attitude that decides it, since the claim itself
    /// comes from that kinship (Edward III, grandson of Philip IV through
    /// his mother). `false`: kinship counts as for any war (pre-EQ6).
    #[serde(default)]
    pub claim_war_ignores_kinship: bool,
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
    /// EQ6: a realm too weary to declare a war of its own
    /// (`negotiation.max_weariness_to_declare`, raised by 20 for a
    /// pretender) no more answers a call to arms, as a ruined one: the
    /// alliance breaks. `false`: weariness does not matter (pre-EQ6).
    #[serde(default)]
    pub weary_stay_out: bool,
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
    /// EQ6: a cornered crown at war with a pretender to its throne sues
    /// before `min_war_turns` only once this war has beaten it (war score
    /// at or below `SURRENDER_WAR_SCORE`, -25): it does not buy its peace
    /// on the very season war is declared. Against any other enemy it
    /// sues at once, as before. `false`: always at once (pre-EQ6).
    #[serde(default)]
    pub cornered_waits_for_defeat: bool,
}

/// Contents of `data/ai/diplomacy.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiDiplomacy {
    pub menacing_neighbour: MenacingNeighbourRules,
    pub war: WarPlanningRules,
    pub join_war: JoinWarRules,
    pub peace: PeaceRules,
    /// Lot DP1: multi-article treaties, war goals and war weariness.
    #[serde(default)]
    pub negotiation: NegotiationRules,
    /// Lot DP2: right of passage, trespass incidents, diplomatic map.
    #[serde(default)]
    pub passage: PassageRules,
    /// Weights of the reasons a recipient weighs in a treaty.
    #[serde(default)]
    pub treaty_weights: TreatyWeights,
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
                main_claim_first: false,
                claim_war_ignores_difficulty: false,
                claim_war_ignores_kinship: false,
            },
            join_war: JoinWarRules {
                ratio: 0.6,
                min_attitude: 10,
                min_ally_power_ratio: 0.0,
                border_only_claim_wars: false,
                weary_stay_out: false,
            },
            peace: PeaceRules {
                keep_capital: false,
                cornered_provinces: 0,
                cornered_waits_for_defeat: false,
            },
            negotiation: NegotiationRules::default(),
            passage: PassageRules::default(),
            treaty_weights: TreatyWeights::default(),
            description: None,
        }
    }
}

/// Lot DP1 (ADR 0025): how treaties are valued, war goals scored and war
/// weariness accumulated. `enabled: false` (the default without the data
/// file) keeps the G5 peace of `plan_diplomacy`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct NegotiationRules {
    /// War goals, war weariness and the AI's treaty peace are active.
    pub enabled: bool,
    /// Logistic scale of the acceptance chance: `100 / (1 + e^(-score / scale))`.
    pub chance_scale: f64,
    /// Livres worth one point of treaty value (gold, tribute, ransom).
    pub livres_per_point: i64,
    /// Cap of the value of one gold or tribute article.
    pub max_gold_points: i32,
    /// Provinces an attacker targets as war goals.
    pub war_goal_count: usize,
    /// Extra war score per war-goal province the side occupies.
    pub war_goal_score: i32,
    /// War score per secondary settlement (castle, town) a side occupies.
    pub settlement_score: i32,
    /// Value lost by ceding a province (the capital: `capital_cost`).
    pub province_cost: i32,
    pub capital_cost: i32,
    /// Value lost by ceding a secondary settlement.
    pub settlement_cost: i32,
    /// Share of the cost left when the province is already occupied by the
    /// party receiving it (percent).
    pub occupied_cost_percent: i32,
    /// Bonus of receiving one of our war goals.
    pub war_goal_bonus: i32,
    /// War weariness gained each season per war (non-rebel enemies).
    pub weariness_per_war: u32,
    /// Extra weariness when the war score is below -20.
    pub weariness_losing: u32,
    /// Extra weariness per own province occupied by the enemy (capped at 3).
    pub weariness_occupied: u32,
    /// Extra weariness with an empty treasury.
    pub weariness_bankrupt: u32,
    /// Weariness lost each season of peace.
    pub weariness_recovery: u32,
    /// Weariness points per point of unrest in the realm.
    pub weariness_unrest_divisor: u32,
    /// Weariness points per point of peace value.
    pub weariness_peace_divisor: u32,
    /// The AI declares no new war above this weariness.
    pub max_weariness_to_declare: u32,
    /// A pretender whose war goals are unmet resists a white peace.
    pub unmet_goals_reluctance: i32,
    /// War score from which the AI winner demands cessions.
    pub demand_score: i32,
    /// Minimum acceptance chance (percent) of a treaty the AI sends.
    pub ai_min_chance: u8,
    /// Truce (seasons) after a treaty peace (the Hundred Years' War was a
    /// string of short truces: Malestroit 1343, Bordeaux 1357).
    pub peace_truce_turns: u32,
    /// A pretender to the other's throne resists any peace that gives it
    /// no land, unless badly beaten.
    pub pretender_reluctance: i32,
    /// Most weariness gained in one season.
    pub max_weariness_gain: u32,
    /// Weariness from which the AI buys its peace (lands, gold, tribute).
    pub sue_weariness: u32,
    /// Seasons of war before the AI proposes any treaty peace (unless
    /// cornered).
    pub min_war_turns: u32,
    /// EQ3: a peace also ends the wars of the allies and vassals who joined
    /// it (they sign the same truce).
    pub truce_binds_allies: bool,
    /// EQ6: past this many years of war, both sides lean towards a treaty
    /// ending it, the winner too ("Guerre interminable"): a war nobody can
    /// win outright ends in a truce, as at Brétigny or Leulinghem. 0: off.
    #[serde(default)]
    pub long_war_years: u32,
    /// EQ6: treaty points in favour of peace per year of war beyond
    /// `long_war_years`.
    #[serde(default)]
    pub long_war_points_per_year: i32,
    /// EQ6: cap of those points.
    #[serde(default)]
    pub long_war_max_points: i32,
    /// LR-11: treaty points in favour of peace, on both sides, once a war
    /// against a realm down to its last bastions (never besieged without a
    /// claim, F4) has lasted `min_war_turns` with no clear winner: nothing is
    /// left to fight for. 0: off.
    #[serde(default)]
    pub bastion_war_points: i32,
}

impl Default for NegotiationRules {
    fn default() -> Self {
        Self {
            enabled: false,
            chance_scale: 8.0,
            livres_per_point: 250,
            max_gold_points: 60,
            war_goal_count: 2,
            war_goal_score: 12,
            settlement_score: 3,
            province_cost: 20,
            capital_cost: 60,
            settlement_cost: 6,
            occupied_cost_percent: 50,
            war_goal_bonus: 10,
            weariness_per_war: 1,
            weariness_losing: 1,
            weariness_occupied: 1,
            weariness_bankrupt: 1,
            weariness_recovery: 3,
            weariness_unrest_divisor: 5,
            weariness_peace_divisor: 3,
            max_weariness_to_declare: 40,
            unmet_goals_reluctance: 15,
            demand_score: 20,
            ai_min_chance: 60,
            peace_truce_turns: 12,
            pretender_reluctance: 20,
            max_weariness_gain: 3,
            sue_weariness: 60,
            min_war_turns: 0,
            truce_binds_allies: false,
            long_war_years: 0,
            long_war_points_per_year: 0,
            long_war_max_points: 0,
            bastion_war_points: 0,
        }
    }
}

/// Lot DP2 (ADR 0029): an army ending its season in the lands of a faction
/// at peace, without military access, creates a diplomatic incident whose
/// malus grows with its duration and gives the victim a casus belli.
/// `enabled: false` (the default without the data file) ignores trespass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PassageRules {
    /// Trespass incidents are recorded.
    pub enabled: bool,
    /// Attitude malus of the first season of trespass.
    pub base_penalty: i32,
    /// Extra malus per further consecutive season.
    pub per_season_penalty: i32,
    /// Largest malus of one incident.
    pub max_penalty: i32,
    /// Seasons the victim remembers the incident after the last trespass.
    pub memory_turns: u32,
    /// Consecutive seasons of trespass that give the victim a casus belli.
    pub casus_belli_seasons: u32,
    /// Seasons tolerated while a truce holds (armies leaving after a peace).
    pub truce_grace_seasons: u32,
    /// AI at war: aggression (0-100) from which it crosses neutral lands.
    pub ai_violate_aggression: i32,
    /// AI at war: attitude towards the owner at or below which it crosses.
    pub ai_violate_attitude: i32,
    /// AI at war: power ratio over the owner from which it crosses.
    pub ai_violate_power_ratio: f64,
    /// Diplomatic map: attitude at or below which a neutral is « tension ».
    pub tension_attitude: i32,
}

impl Default for PassageRules {
    fn default() -> Self {
        Self {
            enabled: false,
            base_penalty: 10,
            per_season_penalty: 5,
            max_penalty: 40,
            memory_turns: 12,
            casus_belli_seasons: 2,
            truce_grace_seasons: 2,
            ai_violate_aggression: 70,
            ai_violate_attitude: -40,
            ai_violate_power_ratio: 2.0,
            tension_attitude: -20,
        }
    }
}

/// Weights (in treaty points) of the reasons a recipient weighs in a treaty,
/// grouped by article kind (`negotiation::value` in `sim-campaign`). The
/// defaults are the values of `data/ai/diplomacy.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct TreatyWeights {
    pub peace: PeaceWeights,
    pub alliance: AllianceWeights,
    pub vassalage: VassalageWeights,
    pub marriage: MarriageWeights,
    pub exchange: ExchangeWeights,
    pub context: ContextWeights,
}

/// Peace, truce and papal mediation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PeaceWeights {
    /// Seasons of war per point of lassitude (until `lassitude_cap`).
    pub lassitude_seasons_per_point: u32,
    pub lassitude_cap: u32,
    /// Points when the treasury is empty.
    pub empty_treasury: i32,
    /// Devastation above which an owned province counts as ravaged.
    pub devastation_threshold: u8,
    pub per_ravaged_province: i32,
    pub ravaged_cap: i32,
    /// Aggression points (above 50) per point of reluctance.
    pub aggression_divisor: i32,
    /// Flat reluctance: the lure of victory.
    pub victory_lure: i32,
    /// A stronger enemy on another front calls for peace.
    pub other_front: i32,
    /// War score above which the claims of the pretender weigh.
    pub claim_score_floor: i32,
    pub claimed_provinces: i32,
    /// A pretender to the other's throne does not give up its claim lightly.
    pub pretender_reluctance: i32,
    /// A short truce commits to nothing.
    pub short_truce: i32,
    /// Points of a truce mediated by the pope or a herald.
    pub mediation: i32,
}

impl Default for PeaceWeights {
    fn default() -> Self {
        Self {
            lassitude_seasons_per_point: 2,
            lassitude_cap: 20,
            empty_treasury: 15,
            devastation_threshold: 30,
            per_ravaged_province: 3,
            ravaged_cap: 20,
            aggression_divisor: 5,
            victory_lure: 5,
            other_front: 15,
            claim_score_floor: -30,
            claimed_provinces: 5,
            pretender_reluctance: 10,
            short_truce: 4,
            mediation: 30,
        }
    }
}

/// Alliance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct AllianceWeights {
    pub military_commitment: i32,
    pub war_on_our_allies: i32,
    pub common_enemy: i32,
    /// Proposer power over ours above which it is a strong ally.
    pub strong_ratio: f64,
    pub strong_ally: i32,
    /// Proposer power over ours below which it is a weak ally.
    pub weak_ratio: f64,
    pub weak_ally: i32,
    pub common_rival: i32,
    pub friend_of_rival: i32,
}

impl Default for AllianceWeights {
    fn default() -> Self {
        Self {
            military_commitment: -15,
            war_on_our_allies: -60,
            common_enemy: 25,
            strong_ratio: 1.0,
            strong_ally: 10,
            weak_ratio: 0.3,
            weak_ally: -10,
            common_rival: 15,
            friend_of_rival: -40,
        }
    }
}

/// Vassalage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct VassalageWeights {
    /// Points per unit of power ratio beyond `vassalage_power_ratio`.
    pub power_margin_scale: f64,
    pub power_margin_cap: i32,
    pub independence_loss: i32,
    /// War score of the recipient below which defeat softens it.
    pub defeat_score: i32,
    pub defeat: i32,
    /// Points for the lord when the proposer offers its homage.
    pub homage_gain: i32,
}

impl Default for VassalageWeights {
    fn default() -> Self {
        Self {
            power_margin_scale: 10.0,
            power_margin_cap: 40,
            independence_loss: -40,
            defeat_score: -30,
            defeat: 30,
            homage_gain: 25,
        }
    }
}

/// Marriage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct MarriageWeights {
    pub match_bonus: i32,
    /// Prestige difference per point of value, clamped to `prestige_cap`.
    pub prestige_divisor: i32,
    pub prestige_cap: i32,
}

impl Default for MarriageWeights {
    fn default() -> Self {
        Self {
            match_bonus: 10,
            prestige_divisor: 10,
            prestige_cap: 20,
        }
    }
}

/// Access, trade, money, captives and hostages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ExchangeWeights {
    pub access_given: i32,
    pub access_rival: i32,
    pub access_allies: i32,
    pub access_received: i32,
    pub trade_base: i32,
    pub trade_per_route: i32,
    pub trade_routes_cap: i32,
    /// Partner income per point of wealth, capped at `trade_wealth_cap`.
    pub trade_wealth_divisor: i64,
    pub trade_wealth_cap: i32,
    pub trade_rival: i32,
    pub trade_embargo: i32,
    /// Share (percent) of the total tribute that counts.
    pub tribute_percent: i64,
    pub tribute_ruinous: i32,
    pub gold_empty_treasury: i32,
    /// Percent of the gold value when the treasury is empty.
    pub gold_empty_percent: i32,
    pub ransom_min: i32,
    pub ransom_max: i32,
    pub ransom_high_rank: i32,
    pub hostage_heir: i32,
    pub hostage_other: i32,
    pub pledge_heir: i32,
    pub pledge_other: i32,
    /// Share (percent) of the cost that a gain is worth.
    pub gain_percent: i32,
    pub claimed_land: i32,
    pub held_by_our_troops: i32,
}

impl Default for ExchangeWeights {
    fn default() -> Self {
        Self {
            access_given: -10,
            access_rival: -20,
            access_allies: 8,
            access_received: 6,
            trade_base: 6,
            trade_per_route: 4,
            trade_routes_cap: 12,
            trade_wealth_divisor: 1500,
            trade_wealth_cap: 10,
            trade_rival: -12,
            trade_embargo: -15,
            tribute_percent: 80,
            tribute_ruinous: -15,
            gold_empty_treasury: 5,
            gold_empty_percent: 150,
            ransom_min: 3,
            ransom_max: 40,
            ransom_high_rank: 10,
            hostage_heir: 30,
            hostage_other: 12,
            pledge_heir: 20,
            pledge_other: 10,
            gain_percent: 75,
            claimed_land: -5,
            held_by_our_troops: 5,
        }
    }
}

/// General considerations, whatever the articles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ContextWeights {
    /// Attitude points per point of value (a peace is judged on the war).
    pub attitude_divisor_peace: i32,
    pub attitude_divisor: i32,
    pub perjury: i32,
    pub hostage_held: i32,
    pub marriage_tie: i32,
    pub standing_pact: i32,
    pub trust_min: i32,
    pub trust_max: i32,
    /// Proposer power over ours above which it threatens a neighbour.
    pub threat_ratio: f64,
    pub threat_scale: f64,
    pub threat_cap: i32,
    pub weak_demander_ratio: f64,
    pub weak_demander: i32,
    pub abandoned_ally: i32,
    pub abandoned_allies_max: i32,
    pub prudence: i32,
}

impl Default for ContextWeights {
    fn default() -> Self {
        Self {
            attitude_divisor_peace: 5,
            attitude_divisor: 3,
            perjury: -15,
            hostage_held: 10,
            marriage_tie: 5,
            standing_pact: 5,
            trust_min: -30,
            trust_max: 20,
            threat_ratio: 1.5,
            threat_scale: 5.0,
            threat_cap: 15,
            weak_demander_ratio: 0.5,
            weak_demander: -8,
            abandoned_ally: -6,
            abandoned_allies_max: 3,
            prudence: -3,
        }
    }
}
