//! Balance bands (`data/balance/seed_bots_bands.json`) for the balance-drift oracle.
//!
//! Validated against `data/schemas/seed_bots_bands.schema.json` by
//! `tools/tests/test_seed_bots_bands_schema.py`.

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

/// Whether a band is filled in yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BandStatus {
    /// Not checked until the reference long pass sets it.
    ToInitialise,
    /// Checked.
    Initialised,
}

/// One metric's band.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Band {
    /// Lower bound, if any.
    #[serde(default)]
    pub min: Option<f64>,
    /// Upper bound, if any.
    #[serde(default)]
    pub max: Option<f64>,
    /// Whether the band is checked.
    pub status: BandStatus,
}

impl Band {
    /// Whether the band is filled in and so must be checked.
    pub fn is_active(&self) -> bool {
        self.status == BandStatus::Initialised
    }

    /// Whether `value` is outside the band (always false while not active).
    pub fn is_violated_by(&self, value: f64) -> bool {
        self.is_active()
            && (self.min.is_some_and(|min| value < min) || self.max.is_some_and(|max| value > max))
    }
}

/// Bands by metric name.
pub type Bands = BTreeMap<String, Band>;

/// Parses the JSON text of the bands file.
pub fn parse(text: &str) -> Result<Bands, String> {
    serde_json::from_str(text).map_err(|error| format!("bandes d'équilibre invalides : {error}"))
}

/// Loads `<data_root>/balance/seed_bots_bands.json`.
pub fn load(data_root: &Path) -> Result<Bands, String> {
    let path = data_root.join("balance/seed_bots_bands.json");
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("{} illisible : {error}", path.display()))?;
    parse(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_bands_load() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
        let bands = load(&root).expect("bands");
        assert_eq!(bands.len(), 5);
        assert!(bands.contains_key("revolts_per_game"));
    }

    #[test]
    fn active_band_checks_bounds() {
        let band = parse(r#"{"m": {"min": 1.0, "max": 3.0, "status": "initialised"}}"#).unwrap()
            ["m"]
            .clone();
        assert!(band.is_violated_by(0.5));
        assert!(band.is_violated_by(3.5));
        assert!(!band.is_violated_by(2.0));
        let idle = parse(r#"{"m": {"min": 1.0, "status": "to_initialise"}}"#).unwrap()["m"].clone();
        assert!(!idle.is_violated_by(0.0));
    }

    #[test]
    fn rejects_bad_input() {
        assert!(parse(r#"{"m": {"status": "maybe"}}"#).is_err());
        assert!(parse(r#"{"m": {"min": 1.0}}"#).is_err());
        assert!(parse(r#"{"m": {"status": "initialised", "mean": 2}}"#).is_err());
    }
}
