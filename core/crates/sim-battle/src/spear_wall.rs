//! Spear wall (`Ability::SpearWall`, lot TW bsim, ADR 0320): spearmen,
//! goedendag men, coutiliers and town militia hold their spears out in a
//! line. Against horsemen in front of them their blows weigh more and a
//! charge breaks in part (casualties and morale for the horse, a dulled
//! shock, a shorter charge bonus). Flank and rear are bare. Weaker than the
//! pikes (`PikeSquare`), which stop the charge outright. Numbers in
//! `data/rules/battle_spear_wall.json`.

use data_model::Ability;
use serde::{Deserialize, Serialize};

use crate::unit::Unit;

/// Contents of `data/rules/battle_spear_wall.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpearWallRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    /// Multiplier of the spearmen's blows at horsemen in front of them.
    pub vs_horse_front_factor: f64,
    /// Multiplier of the shock (men knocked down, cohesion) of a frontal charge.
    pub charge_taken_factor: f64,
    /// Share of the horsemen lost at the impact.
    pub charge_loss_hp_fraction: f64,
    /// Morale the horsemen lose at the impact.
    pub charge_morale_loss: f64,
    /// Share of the charge bonus duration that remains.
    pub charge_timer_factor: f64,
}

data_model::bundled_rules!(SpearWallRules, "rules/battle_spear_wall.json");

/// Does `defender` meet the horse `attacker` with a spear wall? Only for
/// foot soldiers (the wall is no help once they ride), against a mounted
/// attacker coming from the front (`angle` 0).
pub(crate) fn spear_wall_holds(attacker: &Unit, defender: &Unit, angle: u8) -> bool {
    angle == 0
        && attacker.is_cavalry()
        && attacker.mounted
        && !defender.mounted
        && defender.has(Ability::SpearWall)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_rules_are_a_partial_wall() {
        let rules = SpearWallRules::bundled();
        assert!(rules.vs_horse_front_factor > 1.0);
        assert!(rules.charge_taken_factor < 1.0);
        // Weaker than the pikes, which stop a charge outright.
        assert!(rules.charge_loss_hp_fraction < crate::impact::PIKE_STOP_LOSS);
    }
}
