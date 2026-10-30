//! Divisions: vanguard, battle and rearguard, assigned at deployment by position.
//!
//! The regiments of a side are sorted by depth (the nearest to the enemy
//! first); the first share is the vanguard, the last the rearguard, the rest
//! the battle. The assignment is made once, when the battle starts, and kept:
//! a regiment stays in its division whatever it does afterwards.

use serde::{Deserialize, Serialize};

use crate::setup::SideId;
use crate::unit::Unit;
use data_model::UnitCategory;

/// The three divisions of an army.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DivisionKind {
    Vanguard,
    Battle,
    Rear,
}

/// Shares of the regiments in the vanguard and the rearguard.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DivisionShares {
    pub vanguard: f64,
    pub rear: f64,
}

/// A group of regiments sharing one plan role and one target.
#[derive(Debug, Clone, PartialEq)]
pub struct Division {
    pub kind: DivisionKind,
    /// Unit ids.
    pub regiments: Vec<u32>,
    /// The enemy regiment (unit id) the whole division works on.
    pub target: Option<u32>,
}

/// Cuts the regiments of `side` into divisions by depth. Siege engines and
/// battle-only regiments belong to none.
pub fn assign(units: &[Unit], side: SideId, shares: DivisionShares) -> Vec<Division> {
    let forward = if side == SideId::Attacker { 1.0 } else { -1.0 };
    let mut ranked: Vec<&Unit> = units
        .iter()
        .filter(|u| u.side == side && !u.synthetic && u.category != UnitCategory::Siege)
        .collect();
    ranked.sort_by(|a, b| {
        (b.z * forward)
            .total_cmp(&(a.z * forward))
            .then(a.id.cmp(&b.id))
    });
    let n = ranked.len();
    let (vanguard, rear) = split_counts(n, shares);
    let mut divisions = Vec::new();
    for (kind, range) in [
        (DivisionKind::Vanguard, 0..vanguard),
        (DivisionKind::Battle, vanguard..n - rear),
        (DivisionKind::Rear, n - rear..n),
    ] {
        if range.is_empty() {
            continue;
        }
        divisions.push(Division {
            kind,
            regiments: ranked[range].iter().map(|u| u.id).collect(),
            target: None,
        });
    }
    divisions
}

/// Regiments of the vanguard and of the rearguard among `n`: at least one
/// each from three regiments up, never the whole army.
fn split_counts(n: usize, shares: DivisionShares) -> (usize, usize) {
    if n < 2 {
        return (0, 0);
    }
    if n == 2 {
        return (1, 0);
    }
    let count = |share: f64| ((n as f64 * share).round() as usize).max(1);
    let vanguard = count(shares.vanguard).min(n - 2);
    let rear = count(shares.rear).min(n - 1 - vanguard);
    (vanguard, rear)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHARES: DivisionShares = DivisionShares {
        vanguard: 0.34,
        rear: 0.26,
    };

    #[test]
    fn counts_leave_a_battle_division() {
        assert_eq!(split_counts(0, SHARES), (0, 0));
        assert_eq!(split_counts(1, SHARES), (0, 0));
        assert_eq!(split_counts(2, SHARES), (1, 0));
        assert_eq!(split_counts(3, SHARES), (1, 1));
        assert_eq!(split_counts(7, SHARES), (2, 2));
        for n in 3..40 {
            let (v, r) = split_counts(n, SHARES);
            assert!(v >= 1 && r >= 1 && v + r < n, "n={n}: {v}+{r}");
        }
    }
}
