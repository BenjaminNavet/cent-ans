//! Illustrated plates (port of game/scripts/ui/art_plates.gd): `data/ui/illustrations.json`,
//! schema `data/schemas/illustrations.schema.json`.
//!
//! Loading screens by context (battle, siege, naval, campaign), event vignettes (by event kind of
//! the core, file order = priority) and ending screens. Reading and choosing only, no rule.
//! Replaces the ad hoc parses of `flow/loading.rs` and `battle/hud/aftermath.rs` for new users;
//! those keep working unchanged. Paths are given relative to `game/assets`.

use bevy::prelude::*;
use serde_json::{Value, json};

use crate::agent::dump::DumpAppExt;
use crate::paths;

/// Chronicler's quote of a loading screen.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlateQuote {
    pub text: String,
    pub author: String,
    pub source: String,
    pub date: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LoadingPlate {
    pub id: String,
    pub contexts: Vec<String>,
    pub image: String,
    pub title: String,
    pub quote: Option<PlateQuote>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Vignette {
    pub id: String,
    pub title: String,
    pub kinds: Vec<String>,
    pub image: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ending {
    pub outcome: String,
    pub image: String,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct ArtPlates {
    pub min_seconds: f32,
    pub max_seconds: f32,
    loading: Vec<LoadingPlate>,
    vignettes: Vec<Vignette>,
    endings: Vec<Ending>,
}

fn text(value: &Value) -> String {
    value.as_str().unwrap_or_default().to_owned()
}

fn strings(value: &Value) -> Vec<String> {
    value
        .as_array()
        .map(|list| list.iter().map(text).collect())
        .unwrap_or_default()
}

/// `res://assets/x.jpg` -> `x.jpg` (relative to `game/assets`).
fn asset_path(value: &Value) -> String {
    text(value).trim_start_matches("res://assets/").to_owned()
}

impl ArtPlates {
    pub fn parse(json: &Value) -> Self {
        let loading = &json["loading"];
        Self {
            min_seconds: loading["min_seconds"].as_f64().unwrap_or(3.0) as f32,
            max_seconds: loading["max_seconds"].as_f64().unwrap_or(4.0) as f32,
            loading: loading["screens"]
                .as_array()
                .map(|list| {
                    list.iter()
                        .map(|s| LoadingPlate {
                            id: text(&s["id"]),
                            contexts: strings(&s["contexts"]),
                            image: asset_path(&s["image"]),
                            title: text(&s["title"]),
                            quote: s["quote"].is_object().then(|| PlateQuote {
                                text: text(&s["quote"]["text"]),
                                author: text(&s["quote"]["author"]),
                                source: text(&s["quote"]["source"]),
                                date: text(&s["quote"]["date"]),
                            }),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            vignettes: json["vignettes"]
                .as_array()
                .map(|list| {
                    list.iter()
                        .map(|v| Vignette {
                            id: text(&v["id"]),
                            title: text(&v["title"]),
                            kinds: strings(&v["kinds"]),
                            image: asset_path(&v["image"]),
                        })
                        .collect()
                })
                .unwrap_or_default(),
            endings: json["endings"]
                .as_array()
                .map(|list| {
                    list.iter()
                        .map(|e| Ending {
                            outcome: text(&e["outcome"]),
                            image: asset_path(&e["image"]),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    /// Loads `data/ui/illustrations.json` (empty when missing).
    pub fn load() -> Self {
        let parsed = paths::find_root().ok().and_then(|root| {
            let raw = std::fs::read_to_string(paths::data_dir(&root).join("ui/illustrations.json"))
                .ok()?;
            serde_json::from_str::<Value>(&raw).ok()
        });
        parsed.map(|json| Self::parse(&json)).unwrap_or_default()
    }

    /// Loading screens of a context.
    pub fn loading_screens(&self, context: &str) -> Vec<&LoadingPlate> {
        self.loading
            .iter()
            .filter(|screen| screen.contexts.iter().any(|c| c == context))
            .collect()
    }

    /// A loading screen of the context, chosen by `pick` (a random number, so the caller owns the RNG).
    pub fn loading_screen(&self, context: &str, pick: usize) -> Option<&LoadingPlate> {
        let screens = self.loading_screens(context);
        (!screens.is_empty()).then(|| screens[pick % screens.len()])
    }

    /// Vignette of an event kind.
    pub fn vignette_for_kind(&self, kind: &str) -> Option<&Vignette> {
        self.vignettes
            .iter()
            .find(|v| v.kinds.iter().any(|k| k == kind))
    }

    /// Vignette of the highest-priority kind (file order) among `kinds`.
    pub fn vignette_for_events<'a>(
        &self,
        kinds: impl IntoIterator<Item = &'a str>,
    ) -> Option<&Vignette> {
        let present: Vec<&str> = kinds.into_iter().collect();
        self.vignettes
            .iter()
            .find(|v| v.kinds.iter().any(|k| present.contains(&k.as_str())))
    }

    /// Ending screen: `battle_victory`, `battle_defeat`, `campaign_victory`, `campaign_defeat`.
    pub fn ending(&self, outcome: &str) -> Option<&Ending> {
        self.endings.iter().find(|e| e.outcome == outcome)
    }
}

pub fn plugin(app: &mut App) {
    app.insert_resource(ArtPlates::load()).register_dump(dump);
}

fn dump(world: &World) -> (String, Value) {
    let plates = world.resource::<ArtPlates>();
    (
        "art_plates".into(),
        json!({
            "loading": plates.loading.len(),
            "battle_screens": plates.loading_screens("battle").len(),
            "vignettes": plates.vignettes.len(),
            "endings": plates.endings.len(),
            "min_seconds": plates.min_seconds,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ArtPlates {
        ArtPlates::parse(&json!({
            "loading": {"min_seconds": 2.0, "screens": [
                {"id": "a", "contexts": ["battle"], "image": "res://assets/art/a.jpg", "title": "A",
                 "quote": {"text": "t", "author": "x"}},
                {"id": "b", "contexts": ["battle", "siege"], "image": "res://assets/art/b.jpg"}]},
            "vignettes": [
                {"id": "v1", "kinds": ["plague"], "image": "res://assets/art/p.jpg"},
                {"id": "v2", "kinds": ["war", "plague"], "image": "res://assets/art/w.jpg"}],
            "endings": [{"outcome": "campaign_victory", "image": "res://assets/art/e.jpg"}]
        }))
    }

    #[test]
    fn contexts_priority_and_endings() {
        let plates = sample();
        assert_eq!(plates.min_seconds, 2.0);
        assert_eq!(plates.loading_screens("battle").len(), 2);
        assert_eq!(plates.loading_screens("siege").len(), 1);
        assert_eq!(plates.loading_screen("battle", 3).unwrap().id, "b");
        assert!(plates.loading_screen("naval", 0).is_none());
        assert_eq!(plates.loading_screens("battle")[0].image, "art/a.jpg");
        assert_eq!(
            plates.vignette_for_events(["war", "plague"]).unwrap().id,
            "v1"
        );
        assert_eq!(plates.vignette_for_kind("war").unwrap().id, "v2");
        assert_eq!(
            plates.ending("campaign_victory").unwrap().image,
            "art/e.jpg"
        );
        assert!(plates.ending("nope").is_none());
    }

    #[test]
    fn real_file_has_screens() {
        let plates = ArtPlates::load();
        assert!(!plates.loading_screens("battle").is_empty());
        assert!(plates.ending("battle_victory").is_some());
    }
}
