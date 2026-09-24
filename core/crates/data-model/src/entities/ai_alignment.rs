//! Tuning of the AI's historical side changes (lots G2 and G4), mirroring
//! `data/schemas/ai_alignment.schema.json` (file `data/ai/alignment.json`).

use serde::{Deserialize, Serialize};

/// Wool revolt: a vassal ruined by the embargo of its lord's enemy turns
/// to the embargoing crown (Artevelde, 1338-1340).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WoolRevoltRules {
    /// Below this loyalty (0-100) the squeezed vassal changes sides.
    pub loyalty: u8,
}

/// Defection towards an invader dominating the patron's realm (Troyes).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefectionRules {
    /// Below this loyalty (0-100) an opportunistic vassal may defect.
    pub loyalty: u8,
    /// Regions where a crown owned at least this many provinces in 1337
    /// form its realm.
    pub realm_region_provinces: usize,
    /// Share (0-1) of the realm's provinces an invader must hold to
    /// dominate it (or the 1337 capital).
    pub dominance_realm_share: f64,
    /// Share (0-1) of its own 1337 lands a crown in trouble has lost (or
    /// its war score is at the desertion threshold).
    pub crown_in_trouble_loss: f64,
}

/// Blood feud: a vassal or ally whose grievance against its patron runs
/// deep turns to the patron's enemy or pretender (Montereau, 1419).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrievanceRules {
    /// At or below this attitude (-100..100) towards its patron a prince
    /// holds a grievance.
    pub attitude: i32,
    /// Attitude the prince must have towards the patron's enemy above the
    /// one it has towards its patron.
    pub margin: i32,
}

/// Alliances with the in-laws and neighbours of the enemy (Hainaut, Brabant).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DynasticRules {
    /// Attitude margin a courted prince needs towards us over our enemy.
    pub margin: i32,
    /// Least attitude towards us of a courted prince.
    pub min_attitude: i32,
    /// Attitude towards one of our allies that makes a prince its in-law
    /// (no margin needed).
    pub in_law_attitude: i32,
}

/// Money fiefs: a crown at war buys the goodwill of a hesitant prince on
/// its enemy's border (Edward III's pensions to the Low Countries, 1337).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MoneyFiefRules {
    /// Gift in livres (its opinion bonus is the simulation's gift rule).
    pub amount: i64,
    /// The donor keeps at least this many times the gift in its treasury.
    pub treasury_multiple: i64,
    /// Least attitude towards us of a prince worth a pension.
    pub min_attitude: i32,
}

/// Contents of `data/ai/alignment.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AiAlignment {
    /// Chance (per mille) that a campaign sees a given side change at all
    /// (one roll per prince and per decade for defections).
    pub history_permille: u64,
    pub wool_revolt: WoolRevoltRules,
    pub defection: DefectionRules,
    pub grievance: GrievanceRules,
    pub dynastic: DynasticRules,
    pub money_fief: MoneyFiefRules,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
