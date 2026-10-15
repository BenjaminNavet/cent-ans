//! Data the effects of this folder read: `data/fx/battle_gore.json` (deaths, wounded, corpse and
//! dropped arms limits) and the melee entries of `data/fx/battle_animation.json`.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

/// Death clips of one loss cause (`deaths` of battle_gore.json).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Death {
    #[serde(default)]
    pub foot: Vec<String>,
    #[serde(default)]
    pub mounted: Vec<String>,
    #[serde(default)]
    pub impulse: Vec<f32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WoundedSettings {
    #[serde(default)]
    pub share: HashMap<String, f32>,
    #[serde(default)]
    pub clips: HashMap<String, f32>,
    pub max_total: usize,
    pub animated_seconds: f32,
}

impl Default for WoundedSettings {
    fn default() -> Self {
        Self {
            share: HashMap::new(),
            clips: HashMap::new(),
            max_total: 1500,
            animated_seconds: 7.0,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CorpseLimits {
    pub max_per_layer: usize,
    pub max_total: usize,
    pub spawn_per_update: usize,
    pub far_m: f32,
}

impl Default for CorpseLimits {
    fn default() -> Self {
        Self {
            max_per_layer: 3000,
            max_total: 14000,
            spawn_per_update: 60,
            far_m: 380.0,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct DroppedArmsSettings {
    pub max_per_kind: usize,
    pub max_per_rout: usize,
    pub scatter_m: f32,
    pub far_m: f32,
}

impl Default for DroppedArmsSettings {
    fn default() -> Self {
        Self {
            max_per_kind: 500,
            max_per_rout: 80,
            scatter_m: 0.9,
            far_m: 220.0,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct GoreSettings {
    #[serde(default)]
    pub deaths: HashMap<String, Death>,
    #[serde(default)]
    pub wounded: WoundedSettings,
    #[serde(default)]
    pub corpses: CorpseLimits,
    #[serde(default)]
    pub dropped_arms: DroppedArmsSettings,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct MeleeHit {
    pub window_s: f32,
    pub share: f32,
    pub cavalry_share: f32,
}

impl Default for MeleeHit {
    fn default() -> Self {
        Self {
            window_s: 0.7,
            share: 0.35,
            cavalry_share: 0.12,
        }
    }
}

/// Everything of this module read from the data folder.
#[derive(Debug, Clone, Default)]
pub struct FxSettings {
    pub gore: GoreSettings,
    pub cycle_blend_s: f32,
    pub melee_hit: MeleeHit,
}

#[derive(Deserialize, Default)]
struct AnimationFx {
    #[serde(default)]
    cycle_blend_s: f32,
    #[serde(default)]
    melee_hit: MeleeHit,
}

impl FxSettings {
    pub fn load(data_dir: &Path) -> Self {
        fn read<T: for<'de> Deserialize<'de> + Default>(path: std::path::PathBuf) -> T {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|text| serde_json::from_str(&text).ok())
                .unwrap_or_default()
        }
        let animation: AnimationFx = read(data_dir.join("fx/battle_animation.json"));
        Self {
            gore: read(data_dir.join("fx/battle_gore.json")),
            cycle_blend_s: animation.cycle_blend_s,
            melee_hit: animation.melee_hit,
        }
    }

    /// Death of `cause`, falling back on `other`.
    pub fn death(&self, cause: &str) -> Option<&Death> {
        self.gore
            .deaths
            .get(cause)
            .or_else(|| self.gore.deaths.get("other"))
    }
}

/// Reaction clips ("hit" set of the melee state) per animation style.
pub fn hit_clips(style: &str) -> &'static [&'static str] {
    match style {
        "sword" => &["hit", "hit_b", "hit_c"],
        "pike" | "militia" => &["hit", "hit_b"],
        "bow" => &["hit", "hit_b"],
        "crossbow" => &["hit", "hit_c"],
        "lance" | "horse_bow" | "horse_javelin" => &["c_rear"],
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_data_parses() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../data");
        let fx = FxSettings::load(&dir);
        assert!(fx.cycle_blend_s > 0.0);
        assert!(fx.gore.deaths.contains_key("arrow"));
        assert!(fx.death("unknown").is_some(), "falls back on other");
        assert!(fx.gore.wounded.clips.contains_key("crawl"));
        assert_eq!(fx.gore.corpses.max_total, 14000);
        assert_eq!(hit_clips("lance"), ["c_rear"]);
    }
}
