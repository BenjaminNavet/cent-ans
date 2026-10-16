//! `data/ui/front_end.json` (schema `data/schemas/front_end.schema.json`): faction pitches, start
//! dates, recommended and special starts, difficulty labels, prologue cards, as read by
//! game/scripts/ui/front_end_data.gd. Display texts only, no rule.

use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct StartDate {
    pub id: String,
    pub year: i32,
    pub season: String,
    pub title: String,
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct FactionPitch {
    pub id: String,
    #[serde(default)]
    pub tagline: String,
    #[serde(default)]
    pub intro: String,
    #[serde(default)]
    pub strengths: Vec<String>,
    #[serde(default)]
    pub weaknesses: Vec<String>,
    /// 0..3 index into `difficulty_labels`.
    #[serde(default)]
    pub difficulty: usize,
    /// `res://assets/...` in the file; stored relative to the asset root.
    #[serde(default)]
    pub illustration: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SpecialStarts {
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub factions: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct IntroCard {
    #[serde(default)]
    pub date: String,
    pub title: String,
    pub text: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Intro {
    #[serde(default)]
    pub cards: Vec<IntroCard>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct FrontEnd {
    #[serde(default)]
    pub start_dates: Vec<StartDate>,
    #[serde(default)]
    pub recommended_factions: Vec<String>,
    #[serde(default)]
    pub special_starts: SpecialStarts,
    #[serde(default)]
    pub factions: Vec<FactionPitch>,
    #[serde(default)]
    pub difficulty_labels: Vec<String>,
    #[serde(default)]
    pub intro: Intro,
}

impl FrontEnd {
    /// Reads `data/ui/front_end.json`; `assets` (game/assets) is used to drop illustrations that
    /// do not exist (the card then shows the heraldry, as Godot does).
    pub fn load(data_dir: &Path, assets: &Path) -> Result<Self, String> {
        let path = data_dir.join("ui/front_end.json");
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let value: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        Self::from_value(value, assets)
    }

    /// Builds the data from the parsed document (also used by the `data://` asset reload).
    pub fn from_value(value: serde_json::Value, assets: &Path) -> Result<Self, String> {
        let mut parsed: FrontEnd = serde_json::from_value(value).map_err(|e| e.to_string())?;
        for pitch in &mut parsed.factions {
            let relative = pitch
                .illustration
                .take()
                .map(|p| p.trim_start_matches("res://assets/").to_owned())
                .unwrap_or_else(|| format!("illustrations/{}.jpg", pitch.id));
            if assets.join(&relative).is_file() {
                pitch.illustration = Some(relative);
            }
        }
        Ok(parsed)
    }

    pub fn pitch(&self, id: &str) -> Option<&FactionPitch> {
        self.factions.iter().find(|p| p.id == id)
    }

    /// Recommended starts that have a pitch; else the first three pitches.
    pub fn recommended(&self) -> Vec<String> {
        let list: Vec<String> = self
            .recommended_factions
            .iter()
            .filter(|id| self.pitch(id).is_some())
            .cloned()
            .collect();
        if list.is_empty() {
            self.factions.iter().take(3).map(|p| p.id.clone()).collect()
        } else {
            list
        }
    }

    /// Special starts that have a pitch.
    pub fn special(&self) -> Vec<String> {
        self.special_starts
            .factions
            .iter()
            .filter(|id| self.pitch(id).is_some())
            .cloned()
            .collect()
    }

    pub fn difficulty_label(&self, level: usize) -> &str {
        self.difficulty_labels
            .get(level.min(self.difficulty_labels.len().saturating_sub(1)))
            .map_or("", String::as_str)
    }

    /// « Printemps 1337 — La querelle de Guyenne » for the first start date.
    pub fn start_date_label(&self) -> String {
        self.start_dates.first().map_or_else(String::new, |d| {
            let mut season = d.season.clone();
            if let Some(first) = season.get(..1) {
                season = first.to_uppercase() + &season[1..];
            }
            format!("{season} {} — {}", d.year, d.title)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths;

    #[test]
    fn real_file_parses() {
        let root = paths::find_root().expect("repo root");
        let front = FrontEnd::load(&paths::data_dir(&root), &paths::asset_root(&root)).unwrap();
        assert!(front.factions.len() > 100);
        assert!(front.recommended().contains(&"fac_england".to_owned()));
        assert_eq!(front.difficulty_labels.len(), 4);
        assert!(front.start_date_label().starts_with("Printemps 1337"));
        assert!(!front.intro.cards.is_empty());
    }
}
