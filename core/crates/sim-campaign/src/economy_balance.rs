//! Net balance of a faction's purse (UI audit A3, E1/E4): the single source
//! of the « solde » shown by the top bar and the faction panel, so that the
//! interface never sums the budget lines itself.
//!
//! This adds the signed budget lines with the
//! season just resolved beside the projection, and a short history of the
//! purse (last [`BUDGET_HISTORY_SEASONS`] seasons) for the treasury curve.
//! Nothing here changes the economy: it only records and reports it.

use data_model::key_enum;
use std::collections::BTreeMap;

use data_model::FactionId;
use serde::{Deserialize, Serialize};

use crate::economy::FactionEconomy;
use crate::state::CampaignState;

/// Seasons of budget history kept per faction (three years).
pub const BUDGET_HISTORY_SEASONS: usize = 12;

/// One resolved season of a faction's purse. Costs are positive amounts;
/// `other` is the signed part of the treasury change the budget does not
/// explain (ransoms, tribute, agents, chronicle choices, plunder…).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetRecord {
    /// Turn that was resolved (0 = spring 1337).
    pub turn: u32,
    /// Treasury once the turn was resolved.
    pub treasury: i64,
    pub receipts: i64,
    pub armies: i64,
    pub buildings: i64,
    /// Court and administration, recoinage included.
    pub administration: i64,
    /// H3 « Table »: diets actually paid.
    pub table: i64,
    /// ECO2 (ADR 0777): the king's household.
    #[serde(default)]
    pub household: i64,
    /// ECO2: wages of governors and commanders.
    #[serde(default)]
    pub wages: i64,
    /// ECO2: cost of the sieges.
    #[serde(default)]
    pub siege: i64,
    /// ECO2: dowry, tribute and seizures.
    #[serde(default)]
    pub obligations: i64,
    /// ECO2: interest on the bankers' loans.
    #[serde(default)]
    pub interest: i64,
    /// ECO2: extraordinary taxes raised (outside `receipts`).
    #[serde(default)]
    pub levies: i64,
    /// ECO2: loans taken and repaid (outside `receipts`).
    #[serde(default)]
    pub borrowed: i64,
    #[serde(default)]
    pub repaid: i64,
    #[serde(default)]
    pub other: i64,
}

impl BudgetRecord {
    /// Receipts minus every upkeep: what the budget added to the treasury.
    pub fn net(&self) -> i64 {
        self.receipts
            - self.armies
            - self.buildings
            - self.administration
            - self.table
            - self.household
            - self.wages
            - self.siege
            - self.obligations
            - self.interest
    }

    /// Whole change of the treasury over the season (`net` plus `other`).
    pub fn change(&self) -> i64 {
        self.net() + self.other
    }
}

key_enum! {
/// Rubric of the budget table, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetLineKind {
    Receipts => "receipts",
    Armies => "armies",
    Buildings => "buildings",
    Table => "table",
    Administration => "administration",
    Household => "household",
    Wages => "wages",
    Siege => "siege",
    Obligations => "obligations",
    Interest => "interest",
    Levies => "levies",
    Loans => "loans",
    Other => "other",
}
}

impl BudgetLineKind {
    /// Receipts are positive, charges negative, `Other` either.
    pub fn is_charge(self) -> bool {
        !matches!(
            self,
            BudgetLineKind::Receipts
                | BudgetLineKind::Other
                | BudgetLineKind::Levies
                | BudgetLineKind::Loans
        )
    }
}

/// A signed budget line: the projection for the coming season and what was
/// booked by the season just resolved (`None` before the first turn).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetLine {
    pub kind: BudgetLineKind,
    pub projected: i64,
    pub last: Option<i64>,
}

impl BudgetLine {
    /// Change of the projection against the last season (`None` before the
    /// first turn).
    pub fn delta(&self) -> Option<i64> {
        self.last.map(|last| self.projected - last)
    }
}

impl FactionEconomy {
    /// Projected net balance for the coming season: receipts (seigniorage
    /// included) minus armies, buildings, court and administration
    /// (recoinage included) and the Table. This is exactly what
    /// `resolve_economy` will add to the treasury if nothing changes.
    pub fn net_income(&self) -> i64 {
        self.projected_income + self.trade_income
            - self.army_upkeep
            - self.building_upkeep
            - self.administration_upkeep
            - self.table_upkeep
            - self.household
            - self.wages
            - self.siege
            - self.obligations
            - self.interest
    }

    /// Signed budget lines (receipts +, charges −) with the season just
    /// resolved beside the projection. `Other` has no projection (0).
    pub fn budget_lines(&self, last: Option<&BudgetRecord>) -> Vec<BudgetLine> {
        BudgetLineKind::ALL
            .iter()
            .map(|&kind| {
                let (projected, booked) = match kind {
                    BudgetLineKind::Receipts => (
                        // C5: taxes plus the trade routes.
                        self.projected_income + self.trade_income,
                        last.map(|r| r.receipts),
                    ),
                    BudgetLineKind::Armies => (-self.army_upkeep, last.map(|r| -r.armies)),
                    BudgetLineKind::Buildings => {
                        (-self.building_upkeep, last.map(|r| -r.buildings))
                    }
                    BudgetLineKind::Table => (-self.table_upkeep, last.map(|r| -r.table)),
                    BudgetLineKind::Administration => {
                        (-self.administration_upkeep, last.map(|r| -r.administration))
                    }
                    BudgetLineKind::Household => (-self.household, last.map(|r| -r.household)),
                    BudgetLineKind::Wages => (-self.wages, last.map(|r| -r.wages)),
                    BudgetLineKind::Siege => (-self.siege, last.map(|r| -r.siege)),
                    BudgetLineKind::Obligations => {
                        (-self.obligations, last.map(|r| -r.obligations))
                    }
                    BudgetLineKind::Interest => (-self.interest, last.map(|r| -r.interest)),
                    // Raised by orders: no projection.
                    BudgetLineKind::Levies => (0, last.map(|r| r.levies)),
                    BudgetLineKind::Loans => (0, last.map(|r| r.borrowed - r.repaid)),
                    BudgetLineKind::Other => (0, last.map(|r| r.other)),
                };
                BudgetLine {
                    kind,
                    projected,
                    last: booked,
                }
            })
            .collect()
    }
}

/// Sign of the livre tournois (design § economy), used by every amount shown
/// to the player.
pub const LIVRE_SIGN: &str = "₶";

/// `-6000` → « −6 000 ₶ », `1250` → « +1 250 ₶ »: explicit sign (true minus
/// sign), digits grouped by three with no-break spaces.
pub fn signed_livres(value: i64) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut grouped = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push('\u{a0}');
        }
        grouped.push(digit);
    }
    let sign = if value < 0 { '\u{2212}' } else { '+' };
    format!("{sign}{grouped}\u{a0}{LIVRE_SIGN}")
}

/// Unsigned amount in livres tournois: `30018` → « 30 018 ₶ » (no-break spaces).
pub fn livres<T: Into<i64>>(value: T) -> String {
    let value: i64 = value.into();
    let signed = signed_livres(value);
    if value < 0 {
        signed
    } else {
        signed.trim_start_matches('+').to_owned()
    }
}

impl CampaignState {
    /// Net balance actually booked by the last resolved turn (receipts minus
    /// every upkeep, the Table included); `None` for an unknown faction.
    pub fn faction_net_last_turn(&self, id: &FactionId) -> Option<i64> {
        let faction = self.factions.get(id)?;
        Some(
            faction.last_budget.income + faction.last_budget.trade_income
                - faction.last_budget.upkeep(),
        )
    }

    /// Treasury of every faction, taken at the start of a turn so that
    /// [`CampaignState::record_budget_history`] can tell the part of the
    /// change the budget does not explain.
    pub fn purses(&self) -> BTreeMap<FactionId, i64> {
        self.factions
            .iter()
            .map(|(id, faction)| (id.clone(), faction.treasury))
            .collect()
    }

    /// Appends the season just resolved to each living faction's history
    /// (at most [`BUDGET_HISTORY_SEASONS`] kept). `purses_before` comes from
    /// [`CampaignState::purses`] at the start of the turn; `turn` is the
    /// turn that was resolved.
    pub fn record_budget_history(&mut self, purses_before: &BTreeMap<FactionId, i64>, turn: u32) {
        for (id, faction) in self.factions.iter_mut() {
            if !faction.alive {
                continue;
            }
            let table = faction.last_budget.table;
            let armies = faction.last_budget.army_upkeep;
            let buildings = faction.last_budget.building_upkeep;
            let b = faction.last_budget;
            let mut record = BudgetRecord {
                turn,
                treasury: faction.treasury,
                // C5: trade routes are credited after the taxes, apart.
                receipts: b.income + b.trade_income,
                armies,
                buildings,
                administration: b.administration,
                table,
                household: b.household,
                wages: b.wages,
                siege: b.siege,
                obligations: b.obligations,
                interest: b.interest,
                levies: b.levy_income,
                borrowed: b.borrowed,
                repaid: b.repaid,
                other: 0,
            };
            let before = purses_before
                .get(id)
                .copied()
                .unwrap_or(faction.treasury - record.net());
            record.other =
                faction.treasury - before - record.net() - record.levies - record.borrowed
                    + record.repaid;
            faction.budget_history.push(record);
            let excess = faction
                .budget_history
                .len()
                .saturating_sub(BUDGET_HISTORY_SEASONS);
            faction.budget_history.drain(..excess);
        }
    }

    /// Last seasons of a faction's purse, oldest first (empty before the
    /// first turn or for an unknown faction).
    pub fn budget_history(&self, id: &FactionId) -> &[BudgetRecord] {
        self.factions
            .get(id)
            .map_or(&[], |faction| faction.budget_history.as_slice())
    }

    /// The season just resolved, if any.
    pub fn last_budget(&self, id: &FactionId) -> Option<&BudgetRecord> {
        self.budget_history(id).last()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_net_and_change() {
        let record = BudgetRecord {
            turn: 3,
            treasury: 1_000,
            receipts: 500,
            armies: 200,
            buildings: 50,
            administration: 40,
            table: 10,
            other: -120,
            ..BudgetRecord::default()
        };
        assert_eq!(record.net(), 200);
        assert_eq!(record.change(), 80);
    }

    #[test]
    fn livres_are_signed_and_grouped() {
        assert_eq!(signed_livres(-6000), "\u{2212}6\u{a0}000\u{a0}₶");
        assert_eq!(signed_livres(1_250_000), "+1\u{a0}250\u{a0}000\u{a0}₶");
        assert_eq!(signed_livres(0), "+0\u{a0}₶");
        assert_eq!(signed_livres(999), "+999\u{a0}₶");
    }

    #[test]
    fn livres_unsigned_for_event_texts() {
        assert_eq!(livres(30018), "30\u{a0}018\u{a0}₶");
        assert_eq!(livres(-5), "\u{2212}5\u{a0}₶");
    }

    #[test]
    fn line_delta_needs_a_past_season() {
        let line = BudgetLine {
            kind: BudgetLineKind::Armies,
            projected: -300,
            last: None,
        };
        assert_eq!(line.delta(), None);
        let line = BudgetLine {
            last: Some(-200),
            ..line
        };
        assert_eq!(line.delta(), Some(-100));
        assert!(BudgetLineKind::Armies.is_charge());
        assert!(!BudgetLineKind::Other.is_charge());
    }
}
