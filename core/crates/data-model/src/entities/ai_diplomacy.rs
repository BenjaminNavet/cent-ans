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

crate::bundled_rules!(AiDiplomacy, "ai/diplomacy.json", default);

/// Lot DP1 (ADR 0025): how treaties are valued, war goals scored and war
/// weariness accumulated. `enabled: false` (the default without the data
/// file) keeps the G5 peace of `plan_diplomacy`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

crate::bundled_rules!(NegotiationRules, "ai/diplomacy.json", at "/negotiation", default);

/// Lot DP2 (ADR 0029): an army ending its season in the lands of a faction
/// at peace, without military access, creates a diplomatic incident whose
/// malus grows with its duration and gives the victim a casus belli.
/// `enabled: false` (the default without the data file) ignores trespass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

crate::bundled_rules!(PassageRules, "ai/diplomacy.json", at "/passage", default);

/// Weights (in treaty points) of the reasons a recipient weighs in a treaty,
/// grouped by article kind (`negotiation::value` in `sim-campaign`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreatyWeights {
    pub peace: PeaceWeights,
    pub alliance: AllianceWeights,
    pub vassalage: VassalageWeights,
    pub marriage: MarriageWeights,
    pub exchange: ExchangeWeights,
    pub context: ContextWeights,
}

crate::bundled_rules!(TreatyWeights, "ai/diplomacy.json", at "/treaty_weights", default);

/// Peace, truce and papal mediation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

/// Alliance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

/// Vassalage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

/// Marriage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MarriageWeights {
    pub match_bonus: i32,
    /// Prestige difference per point of value, clamped to `prestige_cap`.
    pub prestige_divisor: i32,
    pub prestige_cap: i32,
}

/// Access, trade, money, captives and hostages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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

/// General considerations, whatever the articles.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
