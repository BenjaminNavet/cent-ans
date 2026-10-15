//! Per-unit-type figure variants (`data/fx/unit_looks.json`): share of soldiers in livery,
//! undyed cloth colours and horse coats. Rendering only. Port of
//! game/scripts/battle/battle_unit_looks.gd.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

/// At most this many colours per list (`MAX_COLOURS`).
pub const MAX_COLOURS: usize = 4;

/// Named coats (linear), same values as `COATS` of `battle_soldier_skinned.gdshader`.
pub fn named_coat(name: &str) -> Option<[f32; 3]> {
    Some(match name {
        "bay" => [0.16, 0.07, 0.03],
        "chestnut" => [0.26, 0.10, 0.03],
        "black" => [0.03, 0.025, 0.022],
        "grey" => [0.34, 0.33, 0.31],
        "dun" => [0.36, 0.26, 0.13],
        _ => return None,
    })
}

/// One declared variant.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Look {
    #[serde(default)]
    pub livery_share: Option<f32>,
    #[serde(default)]
    pub cloth: Vec<String>,
    #[serde(default)]
    pub coats: Vec<String>,
}

impl Look {
    /// Undyed cloth colours, linear (at most [`MAX_COLOURS`], bad hex ignored).
    pub fn cloth_colours(&self) -> Vec<[f32; 3]> {
        self.cloth
            .iter()
            .filter_map(|hex| hex_to_linear(hex))
            .take(MAX_COLOURS)
            .collect()
    }

    /// Coat colours, linear (unknown names ignored).
    pub fn coat_colours(&self) -> Vec<[f32; 3]> {
        self.coats
            .iter()
            .filter_map(|name| named_coat(name))
            .take(MAX_COLOURS)
            .collect()
    }
}

/// `#RRGGBB` sRGB to linear.
pub fn hex_to_linear(hex: &str) -> Option<[f32; 3]> {
    let digits = hex.strip_prefix('#')?;
    if digits.len() != 6 {
        return None;
    }
    let channel = |i: usize| u8::from_str_radix(&digits[i..i + 2], 16).ok();
    let (r, g, b) = (channel(0)?, channel(2)?, channel(4)?);
    let lin = |c: u8| {
        let c = f32::from(c) / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    Some([lin(r), lin(g), lin(b)])
}

/// Pads a list to [`MAX_COLOURS`] by repeating its first colour (shader arrays are fixed).
pub fn padded(colours: &[[f32; 3]]) -> [[f32; 3]; MAX_COLOURS] {
    let mut out = [colours.first().copied().unwrap_or([0.0; 3]); MAX_COLOURS];
    for (slot, colour) in out.iter_mut().zip(colours) {
        *slot = *colour;
    }
    out
}

/// All declared variants, by unit type id.
#[derive(Debug, Clone, Default)]
pub struct UnitLooks {
    looks: HashMap<String, Look>,
}

impl UnitLooks {
    pub fn load(data_dir: &Path) -> Self {
        #[derive(Deserialize)]
        struct File {
            #[serde(default)]
            looks: HashMap<String, Look>,
        }
        let looks = std::fs::read_to_string(data_dir.join("fx/unit_looks.json"))
            .ok()
            .and_then(|text| serde_json::from_str::<File>(&text).ok())
            .map(|file| file.looks)
            .unwrap_or_default();
        Self { looks }
    }

    pub fn get(&self, unit_type: &str) -> Option<&Look> {
        self.looks.get(unit_type)
    }

    pub fn len(&self) -> usize {
        self.looks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.looks.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_file_parses_with_clean_colours() {
        let root = crate::paths::find_root().expect("repo root");
        let looks = UnitLooks::load(&crate::paths::data_dir(&root));
        assert!(looks.len() >= 5);
        let mamluk = looks.get("unit_mamluk_cavalry").expect("mamluk");
        assert_eq!(mamluk.livery_share, Some(0.3));
        assert_eq!(mamluk.cloth_colours().len(), 4);
        assert_eq!(mamluk.coat_colours().len(), 3);
        for look in looks.looks.values() {
            assert_eq!(look.cloth_colours().len(), look.cloth.len().min(MAX_COLOURS));
            assert_eq!(look.coat_colours().len(), look.coats.len().min(MAX_COLOURS));
        }
    }

    #[test]
    fn hex_and_padding() {
        let white = hex_to_linear("#ffffff").unwrap();
        assert!((white[0] - 1.0).abs() < 1e-6);
        assert!(hex_to_linear("ffffff").is_none());
        let pad = padded(&[[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]);
        assert_eq!(pad[3], [1.0, 0.0, 0.0]);
        assert_eq!(pad[1], [0.0, 1.0, 0.0]);
    }
}
