//! Style of the campaign places (port of `TownMaquetteData.style`, ADR 0158).
//!
//! `map.town_style` of `data/ui/campaign_map.json` (`maquette` or `real`), overridden by
//! `CENT_ANS_TOWN_STYLE=maquette|real` (Godot `--town-style=`):
//! - `maquette` (default): every place is a stylised maquette or its generated DN glb at constant
//!   world size, at every distance; no 1:1 town, no 1:1 landmark city, no far town mesh
//!   (`SettlementLayer._setup_towns` builds `TownMaquetteLayer` only). The markers lot draws the
//!   maquettes; the 1:1 layer, the landmark cities and the far layer stay off.
//! - `real`: the 1:1 towns at close zoom and their far meshes (VT, ADR 0138/0144); the maquettes
//!   sink under the town that replaces them (`TownsState::is_shown`).

use std::path::Path;

use bevy::prelude::*;
use serde_json::Value;

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TownStyle {
    #[default]
    Maquette,
    Real,
}

impl TownStyle {
    pub fn parse(text: &str) -> Option<Self> {
        match text.trim() {
            "maquette" => Some(Self::Maquette),
            "real" => Some(Self::Real),
            _ => None,
        }
    }

    /// Style of the data (`map.town_style`, default `maquette`), then the environment override.
    pub fn load(data_dir: &Path) -> Self {
        let from_data = std::fs::read(data_dir.join("ui/campaign_map.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .and_then(|v| v["map"]["town_style"].as_str().and_then(Self::parse))
            .unwrap_or_default();
        std::env::var("CENT_ANS_TOWN_STYLE")
            .ok()
            .and_then(|v| Self::parse(&v))
            .unwrap_or(from_data)
    }

    /// True when the 1:1 towns (and their far meshes, the landmark cities) are drawn.
    pub fn real(self) -> bool {
        self == Self::Real
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Maquette => "maquette",
            Self::Real => "real",
        }
    }
}

/// Inserts the style resource (read once at start-up).
pub fn add(app: &mut App) {
    let style = crate::paths::find_root()
        .map(|root| TownStyle::load(&crate::paths::data_dir(&root)))
        .unwrap_or_default();
    app.insert_resource(style);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_styles() {
        assert_eq!(TownStyle::parse("maquette"), Some(TownStyle::Maquette));
        assert_eq!(TownStyle::parse("real"), Some(TownStyle::Real));
        assert_eq!(TownStyle::parse("x"), None);
        assert!(!TownStyle::default().real());
    }

    #[test]
    fn real_data_is_maquette() {
        let Ok(root) = crate::paths::find_root() else {
            return;
        };
        if std::env::var("CENT_ANS_TOWN_STYLE").is_ok() {
            return;
        }
        assert_eq!(
            TownStyle::load(&crate::paths::data_dir(&root)),
            TownStyle::Maquette
        );
    }
}
