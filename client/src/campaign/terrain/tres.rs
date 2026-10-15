//! Minimal reader of Godot `.tres` resources: the `key = value` lines of the `[resource]`
//! section (numbers, booleans and `PackedFloat32Array(...)`). The game's tunables stay in
//! `game/resources/*.tres` (single source of truth, shared with the Godot client during the
//! migration); this reads them instead of duplicating the numbers.

use std::collections::HashMap;
use std::path::Path;

/// The scalar and array properties of a `[resource]` section.
#[derive(Debug, Default, Clone)]
pub struct Tres {
    scalars: HashMap<String, f64>,
    arrays: HashMap<String, Vec<f64>>,
}

impl Tres {
    /// Parses `text`; unknown value shapes (strings, `ExtResource`) are skipped.
    pub fn parse(text: &str) -> Tres {
        let mut out = Tres::default();
        let mut in_resource = false;
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                in_resource = line == "[resource]";
                continue;
            }
            if !in_resource {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let (key, value) = (key.trim(), value.trim());
            if let Some(inner) = value
                .strip_prefix("PackedFloat32Array(")
                .and_then(|v| v.strip_suffix(')'))
            {
                let items = inner
                    .split(',')
                    .filter_map(|v| v.trim().parse::<f64>().ok())
                    .collect();
                out.arrays.insert(key.to_owned(), items);
            } else if value == "true" || value == "false" {
                out.scalars
                    .insert(key.to_owned(), if value == "true" { 1.0 } else { 0.0 });
            } else if let Ok(number) = value.parse::<f64>() {
                out.scalars.insert(key.to_owned(), number);
            }
        }
        out
    }

    /// Reads and parses a file; an unreadable file gives an empty resource (defaults apply).
    pub fn load(path: &Path) -> Tres {
        std::fs::read_to_string(path)
            .map(|text| Tres::parse(&text))
            .unwrap_or_default()
    }

    pub fn f32(&self, key: &str, default: f32) -> f32 {
        self.scalars.get(key).map_or(default, |v| *v as f32)
    }

    pub fn i32(&self, key: &str, default: i32) -> i32 {
        self.scalars.get(key).map_or(default, |v| *v as i32)
    }

    pub fn bool(&self, key: &str, default: bool) -> bool {
        self.scalars.get(key).map_or(default, |v| *v != 0.0)
    }

    pub fn array(&self, key: &str) -> Option<Vec<f32>> {
        self.arrays
            .get(key)
            .map(|v| v.iter().map(|x| *x as f32).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_scalars_arrays_and_ignores_other_sections() {
        let text = "[gd_resource]\nfoo = 9\n[resource]\nscript = ExtResource(\"1\")\nenabled = true\ngain = -0.1\nlevels = PackedFloat32Array(22, 9, 5.5)\n";
        let tres = Tres::parse(text);
        assert!(tres.bool("enabled", false));
        assert_eq!(tres.f32("gain", 0.0), -0.1);
        assert_eq!(tres.f32("foo", 7.0), 7.0);
        assert_eq!(tres.array("levels"), Some(vec![22.0, 9.0, 5.5]));
        assert_eq!(tres.i32("missing", 3), 3);
    }
}
