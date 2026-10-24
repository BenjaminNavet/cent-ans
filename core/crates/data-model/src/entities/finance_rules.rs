//! Royal finances, mirroring `data/schemas/finance_rules.schema.json`
//! (`data/rules/finance.json`, ADR 0777): household, officers' wages, siege
//! costs, bankers' credit, royal default, extraordinary taxes and dowries.
//! The whole file is optional: without it every part is switched off.

use serde::{Deserialize, Serialize};

/// One step of the household scale: realms of at least `min_provinces`
/// provinces keep a court costing `percent` of their gross tax income.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HouseholdTier {
    pub min_provinces: u32,
    pub percent: f64,
}

/// The king's household (« maison du roi »): a share of the gross tax income
/// that grows with the rank of the realm and the ruler's prestige.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct HouseholdRules {
    pub tiers: Vec<HouseholdTier>,
    /// Extra percent of the gross income per 10 points of ruler prestige
    /// (positive prestige only).
    pub percent_per_10_prestige: f64,
    /// Ceiling of the household, percent of the gross income.
    pub max_percent: f64,
}

/// Wages of the officers of the crown.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct OfficerRules {
    /// Livres per season for each governor of a province.
    pub governor_wage: i64,
    /// Livres per season for each army led by a general.
    pub commander_wage: i64,
    /// Ceiling of all wages, percent of the gross income.
    pub max_percent: f64,
}

/// Cost of the sieges the faction conducts.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct SiegeCostRules {
    /// Livres per season for each siege under way (stakes, engines, forage).
    pub per_siege: i64,
    /// Extra percent of that cost for each season the siege has lasted.
    pub growth_percent_per_season: i64,
    /// Ceiling of the extra, percent.
    pub growth_max_percent: i64,
    /// Ceiling of all siege costs, percent of the gross income.
    pub max_percent: f64,
}

/// A banking house the crown can borrow from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lender {
    pub id: String,
    pub name: String,
    /// Interest per season, in thousandths of the principal, at full credit
    /// rating.
    pub rate_permille: u32,
    /// What this house lends at most, in seasons of the borrower's gross tax
    /// income (all its loans together).
    pub limit_seasons: f64,
    /// Smallest loan.
    pub min_loan: i64,
}

/// What a royal default does.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct DefaultRules {
    /// A default happens when the treasury is negative and the debt exceeds
    /// this many seasons of gross tax income.
    pub debt_seasons: f64,
    /// Share of the debt the bankers lose (percent).
    pub write_off_percent: i64,
    /// The negative treasury is wiped out (the crown stops paying).
    pub clear_treasury: bool,
    /// Turns during which no banker lends.
    pub freeze_turns: u32,
    /// Credit points lost.
    pub rating_loss: u32,
    /// Share of the gross tax income the bankers seize each season ...
    pub seizure_percent: f64,
    /// ... for this many seasons.
    pub seizure_turns: u32,
    /// Unrest added to every province of the realm ...
    pub unrest: f64,
    /// ... for this many seasons.
    pub unrest_turns: u32,
    /// Prestige lost by the ruler.
    pub prestige_loss: i32,
}

/// Bankers' credit.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct CreditRules {
    pub lenders: Vec<Lender>,
    /// Credit rating of a realm that never defaulted (0-100).
    pub start_rating: u32,
    /// Rating regained per season without default.
    pub rating_recovery: u32,
    /// Extra interest (permille per season) for each point of rating below 100.
    pub rate_penalty_permille_per_point: f64,
    /// A loan is refused when the debt would exceed the rating's share of the
    /// limit: limit x rating / 100.
    pub rating_scales_limit: bool,
    pub default: DefaultRules,
}

/// An extraordinary tax (taille, gabelle, fouage...).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Levy {
    pub id: String,
    pub name: String,
    /// First year the tax can be raised (0: always).
    #[serde(default)]
    pub from_year: i32,
    /// Yield, in seasons of the realm's gross tax income.
    pub income_seasons: f64,
    /// Unrest added to every province of the realm ...
    pub unrest: f64,
    /// ... for this many seasons.
    pub unrest_turns: u32,
    /// Seasons before the same tax can be raised again.
    pub cooldown_turns: u32,
}

/// Dowry of a marriage between two realms.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct DowryRules {
    /// Total, in seasons of the paying realm's gross tax income.
    pub income_seasons: f64,
    /// Number of seasonal instalments.
    pub installments: u32,
}

/// What the AI does with its finances.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct FinanceAiRules {
    /// The AI borrows when its projected treasury falls under this many
    /// seasons of gross income (and levies a tax before borrowing if the
    /// realm is calm).
    pub borrow_below_seasons: f64,
    /// It borrows up to this many seasons of income at a time.
    pub borrow_seasons: f64,
    /// It repays when the treasury is above this many seasons of income.
    pub repay_above_seasons: f64,
    /// It levies an extraordinary tax only when the mean unrest of its
    /// provinces is under this value.
    pub levy_max_unrest: u8,
}

/// Contents of `data/rules/finance.json`.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct FinanceRules {
    pub description: String,
    pub household: HouseholdRules,
    pub officers: OfficerRules,
    pub siege: SiegeCostRules,
    pub credit: CreditRules,
    pub levies: Vec<Levy>,
    pub dowry: DowryRules,
    pub ai: FinanceAiRules,
}

impl FinanceRules {
    pub fn lender(&self, id: &str) -> Option<&Lender> {
        self.credit.lenders.iter().find(|l| l.id == id)
    }

    pub fn levy(&self, id: &str) -> Option<&Levy> {
        self.levies.iter().find(|l| l.id == id)
    }
}
