//! Net balance of a faction's purse (UI audit A3, E1/E4): the single source
//! of the « solde » shown by the top bar and the faction panel, so that the
//! interface never sums the budget lines itself.
//!
//! Lot U3 (« économie lisible ») adds the signed budget lines with the
//! season just resolved beside the projection, and a short history of the
//! purse (last [`BUDGET_HISTORY_SEASONS`] seasons) for the treasury curve.
//! Nothing here changes the economy: it only records and reports it.

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
    #[serde(default)]
    pub other: i64,
}

impl BudgetRecord {
    /// Receipts minus every upkeep: what the budget added to the treasury.
    pub fn net(&self) -> i64 {
        self.receipts - self.armies - self.buildings - self.administration - self.table
    }

    /// Whole change of the treasury over the season (`net` plus `other`).
    pub fn change(&self) -> i64 {
        self.net() + self.other
    }
}

/// Rubric of the budget table, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetLineKind {
    Receipts,
    Armies,
    Buildings,
    Table,
    Administration,
    Other,
}

impl BudgetLineKind {
    pub const ALL: [BudgetLineKind; 6] = [
        BudgetLineKind::Receipts,
        BudgetLineKind::Armies,
        BudgetLineKind::Buildings,
        BudgetLineKind::Table,
        BudgetLineKind::Administration,
        BudgetLineKind::Other,
    ];

    /// Stable key for the bridge (the interface owns the French labels).
    pub fn key(self) -> &'static str {
        match self {
            BudgetLineKind::Receipts => "receipts",
            BudgetLineKind::Armies => "armies",
            BudgetLineKind::Buildings => "buildings",
            BudgetLineKind::Table => "table",
            BudgetLineKind::Administration => "administration",
            BudgetLineKind::Other => "other",
        }
    }

    /// Receipts are positive, charges negative, `Other` either.
    pub fn is_charge(self) -> bool {
        !matches!(self, BudgetLineKind::Receipts | BudgetLineKind::Other)
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
        self.projected_income
            - self.army_upkeep
            - self.building_upkeep
            - self.administration_upkeep
            - self.table_upkeep
    }

    /// Signed budget lines (receipts +, charges −) with the season just
    /// resolved beside the projection. `Other` has no projection (0).
    pub fn budget_lines(&self, last: Option<&BudgetRecord>) -> Vec<BudgetLine> {
        BudgetLineKind::ALL
            .iter()
            .map(|&kind| {
                let (projected, booked) = match kind {
                    BudgetLineKind::Receipts => (self.projected_income, last.map(|r| r.receipts)),
                    BudgetLineKind::Armies => (-self.army_upkeep, last.map(|r| -r.armies)),
                    BudgetLineKind::Buildings => {
                        (-self.building_upkeep, last.map(|r| -r.buildings))
                    }
                    BudgetLineKind::Table => (-self.table_upkeep, last.map(|r| -r.table)),
                    BudgetLineKind::Administration => {
                        (-self.administration_upkeep, last.map(|r| -r.administration))
                    }
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

impl CampaignState {
    /// Net balance actually booked by the last resolved turn (receipts minus
    /// every upkeep, the Table included); `None` for an unknown faction.
    pub fn faction_net_last_turn(&self, id: &FactionId) -> Option<i64> {
        let faction = self.factions.get(id)?;
        Some(faction.income_last_turn - faction.upkeep_last_turn)
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
            let table = faction.table_upkeep_last_turn;
            let armies = faction.army_upkeep_last_turn;
            let buildings = faction.building_upkeep_last_turn;
            let mut record = BudgetRecord {
                turn,
                treasury: faction.treasury,
                receipts: faction.income_last_turn,
                armies,
                buildings,
                administration: faction.upkeep_last_turn - armies - buildings - table,
                table,
                other: 0,
            };
            let before = purses_before
                .get(id)
                .copied()
                .unwrap_or(faction.treasury - record.net());
            record.other = faction.treasury - before - record.net();
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
        };
        assert_eq!(record.net(), 200);
        assert_eq!(record.change(), 80);
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
