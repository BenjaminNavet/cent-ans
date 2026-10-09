//! SC BA13: movement constants of the regiments (contact range, speeds by
//! terrain, slope, turning, charge distance). Rules in
//! `data/rules/battle_movement.json` (schema
//! `data/schemas/battle_movement_rules.schema.json`).

use serde::{Deserialize, Serialize};

/// Contents of `data/rules/battle_movement.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MovementRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Distance centre à centre au-delà de laquelle deux régiments ne se touchent jamais (m).
    pub contact_range_m: f64,
    /// Mètres par seconde par point de vitesse de l'unité.
    pub base_speed_per_stat: f64,
    /// Vitesse minimale d'une tour de siège poussée (m/s).
    pub siege_tower_min_speed: f64,
    /// Facteur de vitesse d'une cavalerie qui charge.
    pub cavalry_charge_speed_factor: f64,
    /// Facteur de vitesse d'une course ou d'une déroute (autres cas que la charge de cavalerie).
    pub run_or_rout_speed_factor: f64,
    /// Facteur de vitesse d'un fantassin en forêt.
    pub forest_speed_factor_foot: f64,
    /// Facteur de vitesse d'un cavalier en forêt.
    pub forest_speed_factor_mounted: f64,
    /// Facteur de vitesse dans la boue (temps sec).
    pub mud_speed_factor: f64,
    /// Facteur de vitesse dans la boue sous la pluie.
    pub mud_speed_factor_rain: f64,
    /// Facteur de vitesse sous la neige.
    pub snow_speed_factor: f64,
    /// Distance en avant où l'on mesure la pente (m).
    pub grade_probe_m: f64,
    /// Coefficient k du ralentissement en montée : vitesse / (1 + pente * k).
    pub uphill_grade_penalty: f64,
    /// Bonus maximal de vitesse en descente (fraction).
    pub downhill_bonus_cap: f64,
    /// Diviseur de la fatigue : vitesse * (1 - fatigue / diviseur).
    pub fatigue_speed_divisor: f64,
    /// Vitesse de virage de l'infanterie (degrés par seconde).
    pub turn_rate_foot_deg_per_s: f64,
    /// Vitesse de virage de la cavalerie (degrés par seconde).
    pub turn_rate_mounted_deg_per_s: f64,
    /// Distance de la cible sous laquelle l'infanterie en course charge (m).
    pub charge_distance_foot_m: f64,
    /// Distance de la cible sous laquelle la cavalerie en course charge (m).
    pub charge_distance_cavalry_m: f64,
}

data_model::bundled_rules!(MovementRules, "rules/battle_movement.json", default);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_rules_keep_the_original_values() {
        let rules = MovementRules::default();
        assert_eq!(rules.contact_range_m, 250.0);
        assert_eq!(rules.base_speed_per_stat, 0.04);
        assert_eq!(rules.forest_speed_factor_mounted, 0.4);
        assert_eq!(rules.mud_speed_factor_rain, 0.45);
        assert_eq!(rules.charge_distance_cavalry_m, 120.0);
    }
}
