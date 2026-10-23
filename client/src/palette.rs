//! The palette bible of the game (`data/fx/palette.json`, ADR 0755, `docs/design/palette.md`):
//! one set of reference colours (ground, stone, leather, textile, sky) for the campaign and the
//! battle, a reference season (spring) and a reference hour (midday). The battle ground, grass
//! and horizon read the season ground from here; the common grade and the measurement targets
//! are checked against `data/fx/atmosphere.json` by the tests. Purely visual.

use std::sync::OnceLock;

use serde::Deserialize;

use crate::paths;

const BUNDLED: &str = include_str!("../../data/fx/palette.json");

/// Season keys of `ground`.
pub const SEASONS: [&str; 4] = ["spring", "summer", "autumn", "winter"];

#[derive(Debug, Clone, Deserialize)]
pub struct SeasonGround {
    pub spring: [f32; 3],
    pub summer: [f32; 3],
    pub autumn: [f32; 3],
    pub winter: [f32; 3],
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommonGrade {
    pub contrast: f32,
    pub saturation: f32,
    pub shadow_saturation: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Palette {
    pub reference_season: String,
    pub reference_hour: String,
    pub ground: SeasonGround,
    pub common_grade: CommonGrade,
}

impl Palette {
    /// The file under `data/fx`, or the bundled copy.
    pub fn load() -> Palette {
        let from_disk = paths::find_root()
            .ok()
            .and_then(|root| std::fs::read(paths::data_dir(&root).join("fx/palette.json")).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok());
        from_disk.unwrap_or_else(|| serde_json::from_str(BUNDLED).expect("bundled palette"))
    }

    /// Mean albedo of the ground and grass of a season (`spring`, `summer`, `autumn`, `winter`).
    pub fn ground_of(&self, season: &str) -> [f32; 3] {
        match season {
            "summer" => self.ground.summer,
            "autumn" => self.ground.autumn,
            "winter" => self.ground.winter,
            _ => self.ground.spring,
        }
    }
}

/// The palette, read once.
pub fn palette() -> &'static Palette {
    static PALETTE: OnceLock<Palette> = OnceLock::new();
    PALETTE.get_or_init(Palette::load)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules() -> serde_json::Value {
        let root = paths::find_root().unwrap();
        let bytes = std::fs::read(paths::data_dir(&root).join("fx/atmosphere.json")).unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    fn hsv(c: [f32; 3]) -> (f32, f32, f32) {
        let max = c[0].max(c[1]).max(c[2]);
        let min = c[0].min(c[1]).min(c[2]);
        let d = max - min;
        let hue = if d < 1e-6 {
            0.0
        } else if max == c[0] {
            60.0 * (((c[1] - c[2]) / d).rem_euclid(6.0))
        } else if max == c[1] {
            60.0 * ((c[2] - c[0]) / d + 2.0)
        } else {
            60.0 * ((c[0] - c[1]) / d + 4.0)
        };
        (hue, if max > 0.0 { d / max } else { 0.0 }, max)
    }

    #[test]
    fn reference_is_spring_midday_and_the_grass_is_green_not_yellow() {
        let p = Palette::load();
        assert_eq!(p.reference_season, "spring");
        assert_eq!(p.reference_hour, "midday");
        let (hue, sat, _) = hsv(p.ground.spring);
        assert!((85.0..110.0).contains(&hue), "spring grass hue {hue}");
        assert!(sat > 0.4);
        // Summer drifts towards yellow, autumn towards ochre: the hue goes down with the year.
        let (summer, _, _) = hsv(p.ground.summer);
        let (autumn, _, _) = hsv(p.ground.autumn);
        assert!(hue > summer && summer > autumn, "{hue} {summer} {autumn}");
    }

    #[test]
    fn the_battle_reads_its_season_ground_from_the_palette() {
        use sim_battle::BattleSeason;
        let p = Palette::load();
        for (season, key) in [
            (BattleSeason::Spring, "spring"),
            (BattleSeason::Summer, "summer"),
            (BattleSeason::Autumn, "autumn"),
            (BattleSeason::Winter, "winter"),
        ] {
            assert_eq!(
                crate::battle::scene::terrain::season_ground(season),
                p.ground_of(key)
            );
        }
    }

    #[test]
    fn campaign_and_battle_share_the_common_grade() {
        let p = Palette::load();
        let rules = rules();
        let tolerance = |key: &str| {
            let root = paths::find_root().unwrap();
            let bytes = std::fs::read(paths::data_dir(&root).join("fx/palette.json")).unwrap();
            let doc: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            doc["common_grade"]["tolerance"][key].as_f64().unwrap() as f32
        };
        let num = |v: &serde_json::Value| v.as_f64().unwrap() as f32;
        // Spring grade of the campaign and of the battle (season x clear weather).
        let campaign = &rules["campaign"]["seasons"]["spring"]["grade"];
        let battle_season = &rules["battle_seasons"]["spring"]["grade"];
        let battle_clear = &rules["battle"]["clear"]["grade"];
        let battle_saturation =
            num(&battle_season["saturation"]) * num(&battle_clear["saturation"]);
        let battle_contrast = num(&battle_clear["contrast"]);
        for (what, saturation, contrast) in [
            (
                "campaign",
                num(&campaign["saturation"]),
                num(&campaign["contrast"]),
            ),
            ("battle", battle_saturation, battle_contrast),
        ] {
            assert!(
                (saturation - p.common_grade.saturation).abs() <= tolerance("saturation"),
                "{what} saturation {saturation} vs {}",
                p.common_grade.saturation
            );
            assert!(
                (contrast - p.common_grade.contrast).abs() <= tolerance("contrast"),
                "{what} contrast {contrast} vs {}",
                p.common_grade.contrast
            );
        }
    }
}
