//! Music playlists: pure selection logic (no Bevy), port of the `next_track` / rotation part of
//! game/scripts/audio/audio_director.gd. Data: `data/audio/music.json`.
//!
//! A context (`menu`, `campaign`, `campaign_<region>`, `war`, `court`, `battle`) has a `primary`
//! and a `fallback` list. Tracks are drawn from a shuffled bag (a list is reshuffled only once
//! empty), `primary` first; every `fallback_every` primary tracks the `fallback` list goes first.
//! A track never plays twice in a row. Track paths are stored relative to the Bevy asset root
//! (`game/assets`): the leading `assets/` of the JSON is stripped.

use std::collections::HashMap;

use serde_json::Value;

pub const DEFAULT_CROSSFADE_SECONDS: f32 = 3.0;
pub const DEFAULT_FALLBACK_EVERY: u32 = 4;
pub const TIERS: [&str; 2] = ["primary", "fallback"];

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Playlist {
    pub primary: Vec<String>,
    pub fallback: Vec<String>,
}

impl Playlist {
    pub fn tier(&self, tier: &str) -> &[String] {
        if tier == "fallback" {
            &self.fallback
        } else {
            &self.primary
        }
    }
}

#[derive(Debug, Clone)]
pub struct MusicData {
    pub playlists: HashMap<String, Playlist>,
    /// Culture id -> musical region (`campaign_<region>`).
    pub culture_regions: HashMap<String, String>,
    /// Per-track gain in dB (loudness matching).
    pub track_gain_db: HashMap<String, f32>,
    pub crossfade_seconds: f32,
    pub fallback_every: u32,
}

impl Default for MusicData {
    fn default() -> Self {
        Self {
            playlists: HashMap::new(),
            culture_regions: HashMap::new(),
            track_gain_db: HashMap::new(),
            crossfade_seconds: DEFAULT_CROSSFADE_SECONDS,
            fallback_every: DEFAULT_FALLBACK_EVERY,
        }
    }
}

/// `assets/audio/x.ogg` -> `audio/x.ogg` (path relative to the Bevy asset root).
pub fn asset_relative(path: &str) -> String {
    path.strip_prefix("assets/").unwrap_or(path).to_string()
}

fn string_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(asset_relative)
                .collect()
        })
        .unwrap_or_default()
}

impl MusicData {
    pub fn from_json(root: &Value) -> Self {
        let mut data = MusicData::default();
        if let Some(playlists) = root.get("playlists").and_then(Value::as_object) {
            for (context, entry) in playlists {
                let playlist = match entry {
                    Value::Array(_) => Playlist {
                        primary: string_list(Some(entry)),
                        fallback: Vec::new(),
                    },
                    _ => Playlist {
                        primary: string_list(entry.get("primary")),
                        fallback: string_list(entry.get("fallback")),
                    },
                };
                data.playlists.insert(context.clone(), playlist);
            }
        }
        if let Some(gains) = root.get("track_gain_db").and_then(Value::as_object) {
            for (track, gain) in gains {
                if let Some(gain) = gain.as_f64() {
                    data.track_gain_db
                        .insert(asset_relative(track), gain as f32);
                }
            }
        }
        if let Some(regions) = root.get("culture_regions").and_then(Value::as_object) {
            for (culture, region) in regions {
                if let Some(region) = region.as_str() {
                    data.culture_regions
                        .insert(culture.clone(), region.to_string());
                }
            }
        }
        if let Some(seconds) = root.get("crossfade_seconds").and_then(Value::as_f64) {
            data.crossfade_seconds = seconds as f32;
        }
        if let Some(every) = root.get("fallback_every").and_then(Value::as_u64) {
            data.fallback_every = every as u32;
        }
        data
    }

    pub fn gain_db(&self, track: &str) -> f32 {
        self.track_gain_db.get(track).copied().unwrap_or(0.0)
    }

    /// Tracks of a list for a context. `war` is completed by the same list of `war_blend`
    /// (the player's regional peace playlist), without duplicates.
    pub fn tier_tracks(&self, context: &str, tier: &str, war_blend: &str) -> Vec<String> {
        let mut tracks: Vec<String> = self
            .playlists
            .get(context)
            .map(|playlist| playlist.tier(tier).to_vec())
            .unwrap_or_default();
        if context == "war"
            && !war_blend.is_empty()
            && let Some(blend) = self.playlists.get(war_blend)
        {
            for track in blend.tier(tier) {
                if !tracks.contains(track) {
                    tracks.push(track.clone());
                }
            }
        }
        tracks
    }

    /// Peace context of a faction culture: `campaign_<region>` if that playlist exists.
    pub fn campaign_context(&self, culture: &str) -> String {
        let context = self
            .culture_regions
            .get(culture)
            .map(|region| format!("campaign_{region}"));
        match context {
            Some(context) if self.playlists.contains_key(&context) => context,
            _ => "campaign".to_string(),
        }
    }
}

/// Small deterministic xorshift generator (no `rand` dependency).
#[derive(Debug, Clone)]
pub struct XorShift(u64);

impl XorShift {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniform in `0..=max`.
    pub fn below_inclusive(&mut self, max: usize) -> usize {
        (self.next_u64() % (max as u64 + 1)) as usize
    }

    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Fisher-Yates shuffle.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for index in (1..items.len()).rev() {
            let other = self.below_inclusive(index);
            items.swap(index, other);
        }
    }
}

/// Shuffled-bag rotation state.
#[derive(Debug, Clone)]
pub struct Rotation {
    bags: HashMap<String, Vec<String>>,
    /// Last drawn track, never replayed immediately (even in another context).
    pub current: String,
    streak: HashMap<String, u32>,
    rng: XorShift,
}

impl Rotation {
    pub fn new(seed: u64) -> Self {
        Self {
            bags: HashMap::new(),
            current: String::new(),
            streak: HashMap::new(),
            rng: XorShift::new(seed),
        }
    }

    /// State to persist (`user://music_rotation.cfg`): the remaining bags and the last track.
    pub fn to_json(&self) -> Value {
        let mut bags: Vec<(&String, &Vec<String>)> = self.bags.iter().collect();
        bags.sort();
        serde_json::json!({
            "rotation": bags.into_iter().map(|(key, bag)| (key.clone(), serde_json::json!(bag))).collect::<serde_json::Map<_, _>>(),
            "last_track": self.current,
        })
    }

    /// Restores [`Self::to_json`]; unreadable entries are ignored (bags are re-filtered against
    /// the playlists at each draw).
    pub fn load_json(&mut self, state: &Value) {
        self.bags.clear();
        if let Some(bags) = state.get("rotation").and_then(Value::as_object) {
            for (key, bag) in bags {
                if let Some(list) = bag.as_array() {
                    self.bags.insert(
                        key.clone(),
                        list.iter()
                            .filter_map(Value::as_str)
                            .map(String::from)
                            .collect(),
                    );
                }
            }
        }
        self.current = state
            .get("last_track")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
    }

    pub fn streak(&self, context: &str) -> u32 {
        self.streak.get(context).copied().unwrap_or(0)
    }

    /// Tier order for the next draw: `primary` first, except every `fallback_every` primary
    /// tracks when a `fallback` list exists (ADR 0247).
    fn tier_order(&self, data: &MusicData, context: &str, war_blend: &str) -> [&'static str; 2] {
        if data.fallback_every > 0
            && self.streak(context) >= data.fallback_every
            && !data.tier_tracks(context, "fallback", war_blend).is_empty()
        {
            ["fallback", "primary"]
        } else {
            TIERS
        }
    }

    /// Draws the next track of `context` and removes it from its bag. `exists` filters tracks
    /// that have no file. `None` if nothing is playable.
    pub fn next_track(
        &mut self,
        data: &MusicData,
        context: &str,
        war_blend: &str,
        exists: &dyn Fn(&str) -> bool,
    ) -> Option<String> {
        for tier in self.tier_order(data, context, war_blend) {
            let tracks = data.tier_tracks(context, tier, war_blend);
            let key = format!("{context}/{tier}");
            let mut bag: Vec<String> = Vec::new();
            for path in self.bags.get(&key).into_iter().flatten() {
                if tracks.contains(path) && !bag.contains(path) {
                    bag.push(path.clone());
                }
            }
            let only_current = bag.len() == 1 && bag[0] == self.current;
            if bag.is_empty() || (tracks.len() > 1 && only_current) {
                bag = tracks.clone();
                self.rng.shuffle(&mut bag);
            }
            for path in bag.clone() {
                if path == self.current && bag.len() > 1 {
                    continue;
                }
                bag.retain(|candidate| *candidate != path);
                if exists(&path) {
                    self.bags.insert(key, bag);
                    self.current = path.clone();
                    let streak = self.streak.entry(context.to_string()).or_insert(0);
                    *streak = if tier == "primary" { *streak + 1 } else { 0 };
                    return Some(path);
                }
            }
            self.bags.insert(key, bag);
        }
        None
    }
}

/// True when the current track enters its last `fade` seconds (long enough for a fade: at
/// least 4 x `fade`) and the next one must blend over it.
pub fn should_crossfade(position: f32, length: f32, fade: f32) -> bool {
    fade > 0.0 && length > 4.0 * fade && length - position <= fade
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn data() -> MusicData {
        MusicData::from_json(&json!({
            "playlists": {
                "campaign": {"primary": ["assets/a.ogg", "assets/b.ogg", "assets/c.ogg"], "fallback": ["assets/f1.ogg", "assets/f2.ogg"]},
                "campaign_france": {"primary": ["assets/fr.ogg"], "fallback": []},
                "war": {"primary": ["assets/w.ogg"]},
                "menu": ["assets/m1.ogg", "assets/m2.ogg"],
                "battle": {"primary": ["assets/x.ogg"]}
            },
            "culture_regions": {"cul_french": "france", "cul_x": "nowhere"},
            "track_gain_db": {"assets/a.ogg": -3.5},
            "crossfade_seconds": 2.0,
            "fallback_every": 2
        }))
    }

    fn all(_: &str) -> bool {
        true
    }

    #[test]
    fn parses_and_strips_assets_prefix() {
        let data = data();
        assert_eq!(data.playlists["campaign"].primary[0], "a.ogg");
        assert_eq!(data.playlists["menu"].primary.len(), 2);
        assert_eq!(data.gain_db("a.ogg"), -3.5);
        assert_eq!(data.gain_db("zzz"), 0.0);
        assert_eq!(data.crossfade_seconds, 2.0);
    }

    #[test]
    fn campaign_context_by_culture() {
        let data = data();
        assert_eq!(data.campaign_context("cul_french"), "campaign_france");
        assert_eq!(data.campaign_context("cul_x"), "campaign");
        assert_eq!(data.campaign_context("unknown"), "campaign");
    }

    #[test]
    fn war_is_blended_without_duplicates() {
        let data = data();
        assert_eq!(
            data.tier_tracks("war", "primary", "campaign_france"),
            vec!["w.ogg", "fr.ogg"]
        );
        assert_eq!(data.tier_tracks("war", "primary", ""), vec!["w.ogg"]);
    }

    #[test]
    fn bag_plays_each_track_once_before_reshuffle_and_never_repeats() {
        let data = data();
        let mut rotation = Rotation::new(7);
        let mut seen = Vec::new();
        for _ in 0..2 {
            seen.push(rotation.next_track(&data, "menu", "", &all).unwrap());
        }
        seen.sort();
        assert_eq!(seen, vec!["m1.ogg", "m2.ogg"]);
        let mut previous = rotation.current.clone();
        for _ in 0..50 {
            let track = rotation.next_track(&data, "campaign", "", &all).unwrap();
            assert_ne!(track, previous);
            previous = track;
        }
    }

    #[test]
    fn rotation_state_round_trips() {
        let data = data();
        let mut first = Rotation::new(7);
        let a = first.next_track(&data, "campaign", "", &all).unwrap();
        let state = first.to_json();
        let mut restored = Rotation::new(99);
        restored.load_json(&state);
        assert_eq!(restored.current, a);
        // The remaining bag keeps going: no immediate repeat, still a primary track.
        let b = restored.next_track(&data, "campaign", "", &all).unwrap();
        assert_ne!(b, a);
        assert!(["a.ogg", "b.ogg", "c.ogg"].contains(&b.as_str()));
    }

    #[test]
    fn fallback_comes_every_n_primary_tracks() {
        let data = data(); // fallback_every = 2
        let mut rotation = Rotation::new(3);
        let tiers: Vec<bool> = (0..9)
            .map(|_| {
                let track = rotation.next_track(&data, "campaign", "", &all).unwrap();
                track.starts_with('f')
            })
            .collect();
        // p p f p p f p p f
        assert_eq!(
            tiers,
            vec![false, false, true, false, false, true, false, false, true]
        );
    }

    #[test]
    fn missing_files_fall_through_to_the_other_tier_then_none() {
        let data = data();
        let mut rotation = Rotation::new(1);
        let only_fallback = |p: &str| p.starts_with('f');
        let track = rotation
            .next_track(&data, "campaign", "", &only_fallback)
            .unwrap();
        assert!(track.starts_with('f'));
        let none = |_: &str| false;
        assert!(rotation.next_track(&data, "campaign", "", &none).is_none());
        assert!(rotation.next_track(&data, "nothing", "", &all).is_none());
    }

    #[test]
    fn single_track_list_repeats_instead_of_going_silent() {
        let data = data();
        let mut rotation = Rotation::new(1);
        assert_eq!(
            rotation.next_track(&data, "battle", "", &all).as_deref(),
            Some("x.ogg")
        );
        assert_eq!(
            rotation.next_track(&data, "battle", "", &all).as_deref(),
            Some("x.ogg")
        );
    }

    #[test]
    fn crossfade_window() {
        assert!(should_crossfade(97.5, 100.0, 3.0));
        assert!(!should_crossfade(50.0, 100.0, 3.0));
        assert!(!should_crossfade(9.0, 10.0, 3.0)); // too short
        assert!(!should_crossfade(99.0, 100.0, 0.0));
    }

    #[test]
    fn shuffle_is_a_permutation() {
        let mut rng = XorShift::new(42);
        let mut items: Vec<u32> = (0..20).collect();
        rng.shuffle(&mut items);
        let mut sorted = items.clone();
        sorted.sort();
        assert_eq!(sorted, (0..20).collect::<Vec<_>>());
        assert_ne!(items, sorted);
    }
}
