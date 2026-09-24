//! Siege fires (lot S2, spec `docs/design/s2-incendies.md`): the state of a
//! burning house or gate and the tuning read from `data/rules/siege_fire.json`.
//!
//! The rules themselves (ignition, spread, heat, smoke) run in the battle
//! tick (`sim/fire.rs`); this module only holds plain data and the pure
//! burning law of one blaze, so that the bridge and the tests share it.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::field::Weather;

/// Where a house (or the gate) stands in its fire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FireState {
    #[default]
    Intact,
    Burning,
    /// Burnt out: a ruin that no longer blocks movement.
    Burnt,
}

impl FireState {
    pub fn key(self) -> &'static str {
        match self {
            FireState::Intact => "intact",
            FireState::Burning => "burning",
            FireState::Burnt => "burnt",
        }
    }
}

/// The fire of one house or of the gate.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Blaze {
    pub state: FireState,
    /// 0-1: drives spread, heat, smoke and the rendering.
    pub intensity: f64,
    /// 1 at ignition, 0 when burnt out.
    pub fuel: f64,
}

impl Blaze {
    pub fn burning(&self) -> bool {
        self.state == FireState::Burning
    }

    pub fn burnt(&self) -> bool {
        self.state == FireState::Burnt
    }

    /// Sets an intact blaze on fire; `false` when already burning or burnt.
    pub fn ignite(&mut self, intensity: f64) -> bool {
        if self.state != FireState::Intact {
            return false;
        }
        self.state = FireState::Burning;
        self.intensity = intensity.clamp(0.01, 1.0);
        self.fuel = 1.0;
        true
    }

    /// Advances a burning blaze by `dt` seconds (spec § 2.1); returns `true`
    /// when it burns out during this step.
    pub fn advance(&mut self, law: &BlazeRules, weather: &WeatherFire, dt: f64) -> bool {
        if !self.burning() {
            return false;
        }
        let decline = law.decline_fuel.max(1e-6);
        if self.fuel > decline {
            self.intensity = (self.intensity + law.growth_per_s * weather.growth * dt).min(1.0);
        }
        self.intensity = self.intensity.min(self.fuel / decline).max(0.0);
        let duration = (law.burn_duration_s * weather.duration).max(1e-6);
        self.fuel -= self.intensity.max(0.05) * dt / duration;
        if self.fuel <= 0.0 {
            self.state = FireState::Burnt;
            self.fuel = 0.0;
            self.intensity = 0.0;
            return true;
        }
        false
    }
}

/// Burning law of a house or of the gate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlazeRules {
    pub initial_intensity: f64,
    pub growth_per_s: f64,
    pub burn_duration_s: f64,
    pub decline_fuel: f64,
    /// Gate only: HP lost per second at full intensity.
    #[serde(default)]
    pub damage_per_s: f64,
}

/// Ignition by the besiegers' volleys.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IgnitionRules {
    /// Chance per volley by unit type id.
    pub by_unit_type: BTreeMap<String, f64>,
    /// Fallback by category key (`ranged`, `siege`...).
    pub by_category: BTreeMap<String, f64>,
    pub overshoot_m: f64,
    pub reach_m: f64,
    pub gate_reach_m: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SpreadRules {
    pub period_s: f64,
    pub chance_per_period: f64,
    pub edge_distance_m: f64,
    pub wind_strength_max: f64,
}

/// Weather multipliers of the fire.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WeatherFire {
    pub ignition: f64,
    pub spread: f64,
    pub growth: f64,
    pub duration: f64,
}

impl Default for WeatherFire {
    /// Neutral factors (a weather missing from the file).
    fn default() -> Self {
        WeatherFire {
            ignition: 1.0,
            spread: 1.0,
            growth: 1.0,
            duration: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeatRules {
    pub radius_m: f64,
    pub loss_per_s: f64,
    pub morale_per_s: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SmokeRules {
    pub min_intensity: f64,
    pub margin_m: f64,
    pub accuracy_factor: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TorchRules {
    pub reach_m: f64,
    pub chance: f64,
    pub initial_intensity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuburbRules {
    pub count: u32,
    pub distance_m: f64,
    pub spacing_m: f64,
    pub radius_m: f64,
    pub ai_burn_chance: f64,
}

/// Contents of `data/rules/siege_fire.json` (schema
/// `data/schemas/siege_fire_rules.schema.json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FireRules {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub ignition: IgnitionRules,
    pub house: BlazeRules,
    pub gate: BlazeRules,
    pub spread: SpreadRules,
    /// By weather key (`clear`, `rain`, `fog`, `snow`).
    pub weather: BTreeMap<String, WeatherFire>,
    pub heat: HeatRules,
    pub smoke: SmokeRules,
    pub torch: TorchRules,
    pub suburbs: SuburbRules,
}

/// The rules file, embedded at compile time.
const BUNDLED: &str = include_str!("../../../../data/rules/siege_fire.json");

impl FireRules {
    /// `data/rules/siege_fire.json` as compiled into the crate.
    pub fn bundled() -> &'static FireRules {
        static RULES: OnceLock<FireRules> = OnceLock::new();
        RULES.get_or_init(|| {
            serde_json::from_str(BUNDLED).expect("data/rules/siege_fire.json is valid")
        })
    }

    /// Weather factors (neutral when the weather is not listed).
    pub fn weather(&self, weather: Weather) -> WeatherFire {
        self.weather.get(weather.key()).cloned().unwrap_or_default()
    }

    /// Ignition chance per volley of a unit type (then its category).
    pub fn ignition_chance(&self, unit_type: &str, category: &str) -> f64 {
        self.ignition
            .by_unit_type
            .get(unit_type)
            .or_else(|| self.ignition.by_category.get(category))
            .copied()
            .unwrap_or(0.0)
    }
}
