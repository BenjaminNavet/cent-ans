//! Feudal tuning (lot FE, ADR 0098), mirroring
//! `data/schemas/feudal_rules.schema.json` (`data/rules/feudal.json`).
//!
//! The defaults are the values of the former `sim_campaign::diplomacy`
//! constants, so a data tree without the file behaves as before.

use serde::{Deserialize, Serialize};

/// Contents of `data/rules/feudal.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct FeudalRules {
    /// Share of a vassal's income paid to its direct suzerain each turn.
    pub vassal_tribute_percent: i64,
    /// Vassals below this loyalty may rebel.
    pub rebellion_loyalty: u8,
    /// Vassals below this loyalty ignore their suzerain's calls to arms.
    pub call_to_arms_loyalty: u8,
    /// Chance per turn (‰) that a disloyal vassal rebels.
    pub rebellion_permille: u32,
    /// Minimum power ratio to demand homage.
    pub vassalage_power_ratio: f64,
    /// Below this loyalty a vassal may refuse the host, revolt or pay
    /// homage to another liege (spec § 4.2).
    pub disloyal_threshold: u8,
    /// Turns a felony case stays open to a declaration of forfeiture (§ 4.4).
    pub felony_window_turns: u32,
    /// Turns of independence held for the generic victory (§ 4.8).
    pub independence_turns: u32,
    /// Turns as the realm's first vassal for the generic victory (§ 4.8).
    pub ascension_turns: u32,
    pub loyalty: LoyaltyWeights,
    /// Liege's war score against the felon, at the peace, needed to seize
    /// the forfeited titles (§ 4.4, lot F3).
    pub forfeiture_win_war_score: i32,
    /// Treaty value (negotiation points) of a title demanded in a peace,
    /// on top of its provinces (§ 4.6, lot F3).
    pub title_loss_penalty: i32,
    /// How a suzerain arbitrates a contested succession (§ 4.5, lot F3).
    pub arbitration: ArbitrationWeights,
    /// War escalation and private war (§ 4.3, lot F2).
    pub escalation: EscalationRules,
    /// Which direct vassals the host reaches (lot F8, ADR 0114).
    pub host: HostRules,
    /// Felony cases open at the start of the 1337 campaign (lot F8: Edward
    /// III harbours Robert of Artois, banished by Philip VI).
    pub start_felonies: Vec<StartFelony>,
}

/// Why a felony case was opened (§ 4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FelonyReason {
    RefusedHost,
    AlliedWithEnemy,
    Revolt,
    /// The vassal harbours a man banished by its suzerain (lot F8).
    HarbouredFelon,
}

/// A felony case open at the start of the campaign (lot F8).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartFelony {
    pub vassal: crate::FactionId,
    pub liege: crate::FactionId,
    pub reason: FelonyReason,
}

/// Which direct vassals a suzerain's host can summon (lot F8, ADR 0114):
/// a vassal too far from both the muster and the theatre, or strong enough
/// to be independent in fact, is not summoned (and commits no felony).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct HostRules {
    /// A vassal is summoned only if one of its settlements lies within this
    /// distance (km) of its suzerain's capital or of a settlement of the
    /// enemy. `0` removes the limit.
    pub max_muster_km: f64,
    /// A vassal whose power reaches this share of its suzerain's is
    /// independent in fact and not summoned. `0` removes the limit.
    pub independent_power_ratio: f64,
}

impl Default for HostRules {
    fn default() -> Self {
        HostRules {
            max_muster_km: 0.0,
            independent_power_ratio: 0.0,
        }
    }
}

impl Default for FeudalRules {
    fn default() -> Self {
        FeudalRules {
            vassal_tribute_percent: 10,
            rebellion_loyalty: 20,
            call_to_arms_loyalty: 30,
            rebellion_permille: 250,
            vassalage_power_ratio: 3.0,
            disloyal_threshold: 30,
            felony_window_turns: 8,
            independence_turns: 20,
            ascension_turns: 20,
            loyalty: LoyaltyWeights::default(),
            forfeiture_win_war_score: 10,
            title_loss_penalty: 30,
            arbitration: ArbitrationWeights::default(),
            escalation: EscalationRules::default(),
            host: HostRules::default(),
            start_felonies: Vec::new(),
        }
    }
}

/// Terms of a vassal's target loyalty towards its direct suzerain (§ 4.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct LoyaltyWeights {
    /// Starting point before any term.
    pub base: i32,
    /// Power ratio (suzerain / vassal) above which the balance of forces
    /// favours the suzerain.
    pub power_ratio: f64,
    /// Points per turn the loyalty moves towards its target.
    pub drift_per_turn: u8,
    /// Loyalty of a vassal right after it pays homage.
    pub homage_start: u8,
    /// Turns a remembered event (protection, forfeiture of a peer, defeat,
    /// rival claimant, title granted) weighs on the loyalty.
    pub memory_turns: u32,
    /// Suzerain more than `power_ratio` times as powerful as the vassal...
    pub power_favourable: i32,
    /// ... or not.
    pub power_unfavourable: i32,
    /// Excommunicated suzerain.
    pub excommunicated_liege: i32,
    /// Vassal embargoed by an enemy of its suzerain.
    pub embargo_squeeze: i32,
    /// Protection granted when the vassal was attacked (decays).
    pub protection_granted: i32,
    /// Protection refused (decays).
    pub protection_refused: i32,
    /// Rulers of the two factions are kin or married into each other.
    pub family_tie: i32,
    /// Vassal and suzerain share a culture.
    pub shared_culture: i32,
    /// The suzerain granted the vassal a title.
    pub title_granted: i32,
    /// A peer of the vassal was struck by forfeiture.
    pub peer_forfeiture: i32,
    /// Each recent defeat of the suzerain.
    pub liege_defeat: i32,
    /// A rival claimant to the suzerain's primary title exists.
    pub rival_claimant: i32,
}

impl Default for LoyaltyWeights {
    fn default() -> Self {
        LoyaltyWeights {
            base: 55,
            power_ratio: 2.0,
            drift_per_turn: 5,
            homage_start: 60,
            memory_turns: 12,
            power_favourable: 10,
            power_unfavourable: -10,
            excommunicated_liege: -20,
            embargo_squeeze: -25,
            protection_granted: 15,
            protection_refused: -20,
            family_tie: 10,
            shared_culture: 5,
            title_granted: 10,
            peer_forfeiture: -10,
            liege_defeat: -5,
            rival_claimant: -15,
        }
    }
}

/// Scores of each claimant when a suzerain arbitrates a contested
/// succession (spec § 4.5); the best score wins, the designated heir on a tie.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ArbitrationWeights {
    /// The claimant (or the claimant's spouse) is kin of the arbiter's
    /// ruler or belongs to the arbiter's court.
    pub family_tie: i32,
    /// The claimant is the heir by the succession law.
    pub law_heir: i32,
    /// The claimant is the heir designated by the late ruler.
    pub designated_heir: i32,
}

impl Default for ArbitrationWeights {
    fn default() -> Self {
        ArbitrationWeights {
            family_tie: 20,
            law_heir: 10,
            designated_heir: 5,
        }
    }
}

/// War escalation (§ 4.3): a suzerain called to protect an attacked vassal
/// intervenes or shirks; a private war between two vassals of the same
/// lord is arbitrated by that lord.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct EscalationRules {
    /// Prestige change of the ruler of a suzerain who shirks protection.
    pub shirk_prestige: i32,
    /// Loyalty lost at once by every direct vassal of a shirking suzerain.
    pub shirk_loyalty_drop: u8,
    /// Prestige change of the ruler of a suzerain who intervenes.
    pub intervene_prestige: i32,
    /// Turns the attacked vassal remembers protection granted or refused
    /// (opinion modifier worth `loyalty.protection_granted` / `_refused`).
    pub protection_memory_turns: u32,
    /// Turns the player has to answer a call for protection or arbitration.
    pub answer_turns: u32,
    /// Provisional AI score of a call for protection (replaced in F5).
    pub score: ProtectionScore,
    /// Private war arbitration.
    pub arbitration: ArbitrationRules,
}

impl Default for EscalationRules {
    fn default() -> Self {
        EscalationRules {
            shirk_prestige: -20,
            shirk_loyalty_drop: 10,
            intervene_prestige: 5,
            protection_memory_turns: 20,
            answer_turns: 2,
            score: ProtectionScore::default(),
            arbitration: ArbitrationRules::default(),
        }
    }
}

/// Terms of the provisional AI score of a call for protection: the
/// suzerain intervenes when the sum reaches `intervene_at`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ProtectionScore {
    /// Feudal duty: starting point.
    pub base: i32,
    /// Suzerain at least `power_ratio` times as strong as the aggressor...
    pub power_favourable: i32,
    /// ... or not.
    pub power_unfavourable: i32,
    /// Power ratio (suzerain / aggressor) counted as favourable.
    pub power_ratio: f64,
    /// Attitude of the suzerain towards the vassal is divided by this.
    pub attitude_divisor: i32,
    /// Suzerain with a negative treasury.
    pub empty_treasury: i32,
    /// Each war the suzerain already wages.
    pub per_ongoing_war: i32,
    /// Suzerain allied with the aggressor.
    pub allied_with_aggressor: i32,
    /// Score from which the suzerain intervenes.
    pub intervene_at: i32,
    /// Distance from `intervene_at` beyond which the preview is certain
    /// (`likely` above, `unlikely` below, `uncertain` in between).
    pub certainty_margin: i32,
}

impl Default for ProtectionScore {
    fn default() -> Self {
        ProtectionScore {
            base: 30,
            power_favourable: 15,
            power_unfavourable: -25,
            power_ratio: 1.0,
            attitude_divisor: 2,
            empty_treasury: -20,
            per_ongoing_war: -15,
            allied_with_aggressor: -60,
            intervene_at: 20,
            certainty_margin: 15,
        }
    }
}

/// Private war between two direct vassals of the same lord (§ 4.3.5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ArbitrationRules {
    /// Truce imposed with the peace.
    pub truce_turns: u32,
    /// The AI lord imposes peace when at least this many times as strong
    /// as the attacker.
    pub impose_peace_power_ratio: f64,
    /// The AI lord takes a side when its attitude towards one party exceeds
    /// its attitude towards the other by at least this much.
    pub take_side_attitude_gap: i32,
    /// Loyalty lost at once by the attacker forced into peace.
    pub imposed_peace_loyalty_drop: u8,
    /// Loyalty lost at once by the vassal its lord fights.
    pub opposed_loyalty_drop: u8,
}

impl Default for ArbitrationRules {
    fn default() -> Self {
        ArbitrationRules {
            truce_turns: 8,
            impose_peace_power_ratio: 1.5,
            take_side_attitude_gap: 40,
            imposed_peace_loyalty_drop: 5,
            opposed_loyalty_drop: 20,
        }
    }
}
