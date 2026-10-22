//! Dated grudges (ADR 0127 extension, ADR 0720): missions anchored in a past affront.
//!
//! Skeleton: API only, behaviour to be implemented.

use data_model::{FactionId, ProvinceId, SettlementId};
use serde::{Deserialize, Serialize};

/// Nature of the affront a grudge remembers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrudgeKind {
    /// A place was taken from us.
    LostSettlement,
    /// An army of ours was destroyed.
    DefeatedArmy,
    /// A province of ours was pillaged.
    SackedProvince,
}

/// A remembered affront.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grudge {
    pub kind: GrudgeKind,
    /// Faction at fault.
    pub culprit: FactionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settlement: Option<SettlementId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub province: Option<ProvinceId>,
    /// Campaign turn of the affront.
    pub turn: u32,
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "M1 skeleton"]
    fn tx_m1_age_multiplier_follows_thresholds() {}

    #[test]
    #[ignore = "M1 skeleton"]
    fn tx_m1_grudge_cancelled_on_peace_without_penalty() {}

    #[test]
    #[ignore = "M1 skeleton"]
    fn tx_m1_emblematic_loss_doubles_prestige() {}
}
