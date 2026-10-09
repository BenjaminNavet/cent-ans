//! Precomputed JR4b fit of the 1337 garrisons (ADR 0233), mirroring
//! `data/schemas/starting_fit.schema.json` (`data/rules/starting_fit.json`):
//! what the starting budget removes from each realm, generated once by a
//! test of `sim-campaign` (which checks the file equals the computation).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ids::{FactionId, SettlementId};

/// Contents of `data/rules/starting_fit.json`. Index lists are positions in
/// the unfitted lists, ascending.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StartingFit {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Treasuries capped to a few seasons of income (A6-L3).
    #[serde(default)]
    pub treasuries: BTreeMap<FactionId, i64>,
    /// Garrison units sent home, by settlement.
    #[serde(default)]
    pub garrisons: BTreeMap<SettlementId, Vec<usize>>,
    /// Field units sent home, by army id (`army_NNNN`).
    #[serde(default)]
    pub armies: BTreeMap<String, Vec<usize>>,
    /// Buildings left unpaid for, by settlement.
    #[serde(default)]
    pub buildings: BTreeMap<SettlementId, Vec<usize>>,
}
