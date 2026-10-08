//! Siege fires (lot S2, spec `docs/design/s2-incendies.md`): the state of a
//! burning house or gate and the tuning read from `data/rules/siege_fire.json`.
//!
//! The rules themselves (ignition, spread, heat, smoke) run in the battle
//! tick (`sim/fire.rs`); this module only holds plain data and the pure
//! burning law of one blaze, so that the bridge and the tests share it.

use data_model::key_enum;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};

use crate::field::Weather;

key_enum! {
/// Where a house (or the gate) stands in its fire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FireState {
    #[default]
    Intact => "intact",
    Burning => "burning",
    /// Burnt out: a ruin that no longer blocks movement.
    Burnt => "burnt",
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
        self.fuel -= dt / duration;
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
    /// BR3b: share of the heat that reaches a regiment on the wall walk
    /// (raised above the street, behind the parapet).
    #[serde(default = "full_heat")]
    pub wall_walk_factor: f64,
}

fn full_heat() -> f64 {
    1.0
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

/// RS-F: what the « Incendier » order of the battle bar would set on fire
/// (`BattleSim::burn_choice`): the regiment carrying the torch and its
/// target, `house` (index in `siege.houses`) or the gate.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct BurnChoice {
    pub unit: u32,
    pub house: Option<usize>,
    pub gate: bool,
    /// The house is one of the suburbs.
    pub suburb: bool,
    /// From the regiment to the edge of its target, in metres.
    pub distance_m: f64,
}

data_model::bundled_rules!(FireRules, "rules/siege_fire.json");

/// Path of the rules file under the data folder.
pub const FIRE_RULES_PATH: &str = "rules/siege_fire.json";

/// Rules read from the data folder at load time ([`FireRules::install`]);
/// `None`: the bundled ones.
static INSTALLED: RwLock<Option<Arc<FireRules>>> = RwLock::new(None);

impl FireRules {
    /// Parses the contents of a rules file.
    pub fn from_json(text: &str) -> Result<FireRules, String> {
        serde_json::from_str(text).map_err(|e| format!("{FIRE_RULES_PATH}: {e}"))
    }

    /// Reads `rules/siege_fire.json` under `data_dir`.
    pub fn load(data_dir: &Path) -> Result<FireRules, String> {
        let path = data_dir.join(FIRE_RULES_PATH);
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::from_json(&text)
    }

    /// Rules of the battles built from now on (`None`: back to the bundled
    /// ones). The bridge installs the file of the data folder when it loads
    /// the game data, so the fire can be tuned without recompiling.
    pub fn install(rules: Option<FireRules>) {
        let mut guard = INSTALLED.write().unwrap_or_else(|e| e.into_inner());
        *guard = rules.map(Arc::new);
    }

    /// The installed rules, else the bundled ones.
    pub fn current() -> Arc<FireRules> {
        let guard = INSTALLED.read().unwrap_or_else(|e| e.into_inner());
        guard
            .clone()
            .unwrap_or_else(|| Arc::new(Self::bundled().clone()))
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

#[cfg(test)]
mod key_enum_tests {
    use super::*;
    use data_model::key_enum::assert_keys_match_serde;

    #[test]
    fn keys_match_serde_names() {
        assert_keys_match_serde::<FireState>();
    }
}
