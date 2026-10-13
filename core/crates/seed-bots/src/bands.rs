//! Balance bands (`data/balance/seed_bots_bands.yaml`) for the balance-drift oracle.
//!
//! The workspace has no YAML dependency and the file is a fixed two-level mapping, so a
//! small reader handles exactly that shape. Validation against the JSON schema is done by
//! `tools/tests/test_seed_bots_bands_schema.py`.

use std::collections::BTreeMap;
use std::path::Path;

/// One metric's band.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Band {
    /// Lower bound, if any.
    pub min: Option<f64>,
    /// Upper bound, if any.
    pub max: Option<f64>,
    /// `to_initialise` (not checked yet) or `initialised`.
    pub status: String,
}

impl Band {
    /// Whether the band is filled in and so must be checked.
    pub fn is_active(&self) -> bool {
        self.status == "initialised"
    }

    /// Whether `value` is outside the band (always false while not active).
    pub fn is_violated_by(&self, value: f64) -> bool {
        self.is_active()
            && (self.min.is_some_and(|min| value < min) || self.max.is_some_and(|max| value > max))
    }
}

/// Bands by metric name.
pub type Bands = BTreeMap<String, Band>;

/// Parses the YAML text of the bands file.
pub fn parse(text: &str) -> Result<Bands, String> {
    let mut bands = Bands::new();
    let mut current: Option<String> = None;
    for (index, raw) in text.lines().enumerate() {
        let line_no = index + 1;
        let line = raw.split('#').next().unwrap_or("").trim_end();
        if line.trim().is_empty() {
            continue;
        }
        let indented = line.starts_with(' ');
        let (key, value) = line
            .trim()
            .split_once(':')
            .ok_or_else(|| format!("ligne {line_no} : « : » attendu"))?;
        let (key, value) = (key.trim(), value.trim());
        if !indented {
            if !value.is_empty() {
                return Err(format!("ligne {line_no} : métrique « {key} » sans bloc"));
            }
            bands.insert(key.to_string(), Band::default());
            current = Some(key.to_string());
            continue;
        }
        let metric = current
            .as_ref()
            .ok_or_else(|| format!("ligne {line_no} : champ hors métrique"))?;
        let band = bands.get_mut(metric).expect("current metric was inserted");
        let number = |text: &str| -> Result<Option<f64>, String> {
            if text.is_empty() || text == "null" || text == "~" {
                Ok(None)
            } else {
                text.parse::<f64>()
                    .map(Some)
                    .map_err(|_| format!("ligne {line_no} : nombre attendu, trouvé « {text} »"))
            }
        };
        match key {
            "min" => band.min = number(value)?,
            "max" => band.max = number(value)?,
            "status" => band.status = value.trim_matches(['"', '\'']).to_string(),
            other => return Err(format!("ligne {line_no} : champ inconnu « {other} »")),
        }
    }
    if let Some((name, _)) = bands.iter().find(|(_, band)| band.status.is_empty()) {
        return Err(format!("métrique « {name} » sans status"));
    }
    Ok(bands)
}

/// Loads `<data_root>/balance/seed_bots_bands.yaml`.
pub fn load(data_root: &Path) -> Result<Bands, String> {
    let path = data_root.join("balance").join("seed_bots_bands.yaml");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_bands_load() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
        let bands = load(&root).unwrap();
        for metric in [
            "revolts_per_game",
            "bankruptcies_per_game",
            "war_turns_mean",
            "factions_eliminated",
            "leader_territory_share",
        ] {
            let band = &bands[metric];
            assert_eq!(band.status, "to_initialise");
            assert!(!band.is_violated_by(1e9));
        }
    }

    #[test]
    fn active_band_checks_bounds() {
        let bands = parse("m:\n  min: 1\n  max: 4.5 # c\n  status: initialised\n").unwrap();
        let band = &bands["m"];
        assert!(band.is_violated_by(0.5) && band.is_violated_by(5.0));
        assert!(!band.is_violated_by(2.0));
    }

    #[test]
    fn rejects_bad_input() {
        assert!(parse("m:\n  min: abc\n  status: x\n").is_err());
        assert!(parse("m:\n  oops: 1\n  status: x\n").is_err());
        assert!(parse("m:\n  min: 1\n").is_err());
    }
}
