//! Voice data: unit barks (`data/voice/barks.json`), advisor lines (`data/voice/advisor.json`).
//! Pure lookups, port of game/scripts/audio/voice_lines.gd. Clips are
//! `audio/voice/barks/<id>.ogg`, `audio/voice/advisor/<id>.ogg` (relative id `barks/<id>`).
//! Speech clips (`speech/<voice>/<sha1(text)[..12]>`) are not ported here (see report).

use serde_json::Value;

pub const BARKS_FILE: &str = "voice/barks.json";
pub const ADVISOR_FILE: &str = "voice/advisor.json";

#[derive(Debug, Clone, Default)]
pub struct VoiceData {
    pub barks: Value,
    pub advisor: Value,
}

/// A candidate bark line.
#[derive(Debug, Clone, PartialEq)]
pub struct BarkLine {
    pub id: String,
    pub text: String,
}

/// Situation settings (`select`, `move`, `attack`, `charge`, `rout`...).
#[derive(Debug, Clone, PartialEq)]
pub struct Situation {
    pub priority: i32,
    pub cooldown_s: f32,
    pub chance: f32,
    pub spatial: bool,
}

impl VoiceData {
    /// Language of a regiment: regional unit, else noble (English knights: Anglo-Norman), else
    /// the faction's language.
    pub fn language_for(&self, unit_type: &str, faction: &str, is_general: bool) -> String {
        if let Some(language) = self
            .barks
            .pointer(&format!("/unit_language/{unit_type}"))
            .and_then(Value::as_str)
        {
            return language.to_string();
        }
        let noble = self
            .barks
            .get("noble_units")
            .and_then(Value::as_array)
            .is_some_and(|units| units.iter().any(|unit| unit.as_str() == Some(unit_type)));
        if (is_general || noble)
            && let Some(language) = self
                .barks
                .pointer(&format!("/noble_language/{faction}"))
                .and_then(Value::as_str)
        {
            return language.to_string();
        }
        let by_faction = self.barks.get("faction_language");
        by_faction
            .and_then(|table| table.get(faction))
            .or_else(|| by_faction.and_then(|table| table.get("default")))
            .and_then(Value::as_str)
            .unwrap_or("fr")
            .to_string()
    }

    /// Possible lines for a language / situation / unit category, falling back along
    /// `fallback_language` (3 hops) when a regional language has nothing for the situation.
    pub fn bark_candidates(
        &self,
        language: &str,
        situation: &str,
        category: &str,
    ) -> Vec<BarkLine> {
        let mut current = language.to_string();
        for _ in 0..3 {
            let lines = self
                .barks
                .pointer(&format!("/lines/{current}/{situation}"))
                .and_then(Value::as_array);
            let result: Vec<BarkLine> = lines
                .into_iter()
                .flatten()
                .filter(|line| {
                    let only = line.get("category").and_then(Value::as_str).unwrap_or("");
                    only.is_empty() || only == category
                })
                .filter_map(|line| {
                    Some(BarkLine {
                        id: line.get("id")?.as_str()?.to_string(),
                        text: line
                            .get("text")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                    })
                })
                .collect();
            if !result.is_empty() {
                return result;
            }
            match self
                .barks
                .pointer(&format!("/fallback_language/{current}"))
                .and_then(Value::as_str)
            {
                Some(next) => current = next.to_string(),
                None => break,
            }
        }
        Vec::new()
    }

    pub fn situation(&self, name: &str) -> Option<Situation> {
        let entry = self.barks.pointer(&format!("/situations/{name}"))?;
        Some(Situation {
            priority: entry.get("priority").and_then(Value::as_i64).unwrap_or(1) as i32,
            cooldown_s: entry
                .get("cooldown_s")
                .and_then(Value::as_f64)
                .unwrap_or(2.0) as f32,
            chance: entry.get("chance").and_then(Value::as_f64).unwrap_or(1.0) as f32,
            spatial: entry
                .get("spatial")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        })
    }

    /// Advisor line id and text for a trigger (faction-specific line first, else generic).
    pub fn advisor_line(&self, trigger: &str, faction: &str, event_kind: &str) -> Option<BarkLine> {
        let mut generic: Option<BarkLine> = None;
        for line in self.advisor.get("lines")?.as_array()? {
            if line.get("trigger").and_then(Value::as_str) != Some(trigger) {
                continue;
            }
            if trigger == "alert" && line.get("event").and_then(Value::as_str) != Some(event_kind) {
                continue;
            }
            let only = line.get("faction").and_then(Value::as_str).unwrap_or("");
            let entry = BarkLine {
                id: line.get("id")?.as_str()?.to_string(),
                text: line
                    .get("text")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            };
            if !only.is_empty() && only == faction {
                return Some(entry);
            }
            if only.is_empty() && generic.is_none() {
                generic = Some(entry);
            }
        }
        generic
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn data() -> VoiceData {
        VoiceData {
            barks: json!({
                "faction_language": {"fac_england": "en", "default": "fr"},
                "noble_language": {"fac_england": "an"},
                "noble_units": ["unit_knights"],
                "unit_language": {"unit_welsh": "cy"},
                "fallback_language": {"an": "fr", "cy": "en"},
                "lines": {
                    "fr": {"select": [{"id": "fr_1", "text": "Sire ?"}, {"id": "fr_k", "text": "k", "category": "knight"}]},
                    "en": {"select": [{"id": "en_1", "text": "My lord?"}]}
                },
                "situations": {"charge": {"priority": 4, "cooldown_s": 4.0, "chance": 0.7, "spatial": true}}
            }),
            advisor: json!({"lines": [
                {"id": "g", "trigger": "alert", "event": "war", "text": "generic"},
                {"id": "f", "trigger": "alert", "event": "war", "faction": "fac_x", "text": "faction"},
                {"id": "s", "trigger": "campaign_start", "text": "start"}
            ]}),
        }
    }

    #[test]
    fn language_resolution() {
        let data = data();
        assert_eq!(data.language_for("unit_welsh", "fac_england", false), "cy");
        assert_eq!(
            data.language_for("unit_knights", "fac_england", false),
            "an"
        );
        assert_eq!(
            data.language_for("unit_archers", "fac_england", false),
            "en"
        );
        assert_eq!(
            data.language_for("unit_archers", "fac_unknown", false),
            "fr"
        );
        assert_eq!(data.language_for("unit_archers", "fac_england", true), "an");
    }

    #[test]
    fn candidates_fall_back_and_filter_category() {
        let data = data();
        assert_eq!(data.bark_candidates("fr", "select", "foot").len(), 1);
        assert_eq!(data.bark_candidates("fr", "select", "knight").len(), 2);
        assert_eq!(data.bark_candidates("cy", "select", "foot")[0].id, "en_1");
        assert_eq!(data.bark_candidates("an", "select", "foot")[0].id, "fr_1");
        assert!(data.bark_candidates("fr", "nothing", "foot").is_empty());
    }

    #[test]
    fn situation_and_advisor() {
        let data = data();
        let charge = data.situation("charge").unwrap();
        assert!(charge.spatial && charge.priority == 4);
        assert_eq!(data.advisor_line("alert", "fac_x", "war").unwrap().id, "f");
        assert_eq!(data.advisor_line("alert", "fac_y", "war").unwrap().id, "g");
        assert!(data.advisor_line("alert", "fac_y", "peace").is_none());
        assert_eq!(data.advisor_line("campaign_start", "", "").unwrap().id, "s");
    }
}
