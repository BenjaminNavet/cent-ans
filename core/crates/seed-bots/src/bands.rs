//! Balance bands (`data/balance/seed_bots_bands.json`) for the balance-drift oracle.
//!
//! Validated against `data/schemas/seed_bots_bands.schema.json` by
//! `tools/tests/test_seed_bots_bands_schema.py`.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::metrics::{BotMetrics, METRIC_NAMES};

/// Whether a band is filled in yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BandStatus {
    /// Not checked until the reference long pass sets it.
    ToInitialise,
    /// Checked.
    Initialised,
}

/// One metric's band.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Band {
    /// Lower bound, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    /// Upper bound, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
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

/// Margin applied on each side of the observed range by [`propose`].
const MARGIN: f64 = 0.25;

/// Bands PROPOSED from a reference pass: for each metric, `[min observed over every bot - 25 %,
/// max observed + 25 %]`, `status: initialised`. Only the player validates them.
pub fn propose(bots: &[BotMetrics]) -> Result<Bands, String> {
    if bots.is_empty() {
        return Err("aucune métrique dans le rapport (passe sans --balance ?)".to_owned());
    }
    let mut bands = Bands::new();
    for name in METRIC_NAMES {
        let aggregates: Vec<_> = bots.iter().filter_map(|b| b.metrics.get(name)).collect();
        if aggregates.is_empty() {
            continue;
        }
        let min = aggregates
            .iter()
            .map(|a| a.min)
            .fold(f64::INFINITY, f64::min);
        let max = aggregates
            .iter()
            .map(|a| a.max)
            .fold(f64::NEG_INFINITY, f64::max);
        bands.insert(
            name.to_owned(),
            Band {
                min: Some(min * (1.0 - MARGIN)),
                max: Some(max * (1.0 + MARGIN)),
                status: BandStatus::Initialised,
            },
        );
    }
    Ok(bands)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_bands_load() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
        let bands = load(&root).expect("bands");
        assert_eq!(bands.len(), METRIC_NAMES.len());
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
    fn proposal_widens_the_observed_range_by_a_quarter() {
        use crate::metrics::{aggregate, GameMetrics};
        let game = |revolts, share| GameMetrics {
            revolts,
            leader_territory_share: share,
            ..Default::default()
        };
        let bots = aggregate(&[
            ("a".to_owned(), game(4, 0.2)),
            ("b".to_owned(), game(8, 0.4)),
            ("b".to_owned(), game(12, 0.1)),
        ]);
        let bands = propose(&bots).unwrap();
        assert_eq!(bands.len(), METRIC_NAMES.len());
        let revolts = &bands["revolts_per_game"];
        assert_eq!((revolts.min, revolts.max), (Some(3.0), Some(15.0)));
        assert_eq!(revolts.status, BandStatus::Initialised);
        assert!((bands["leader_territory_share"].min.unwrap() - 0.075).abs() < 1e-9);
        // Round trip through JSON as the file would be read back.
        let text = serde_json::to_string(&bands).unwrap();
        assert_eq!(parse(&text).unwrap(), bands);
        assert!(propose(&[]).is_err());
    }

    #[test]
    fn rejects_bad_input() {
        assert!(parse(r#"{"m": {"status": "maybe"}}"#).is_err());
        assert!(parse(r#"{"m": {"min": 1.0}}"#).is_err());
        assert!(parse(r#"{"m": {"status": "initialised", "mean": 2}}"#).is_err());
    }
}
