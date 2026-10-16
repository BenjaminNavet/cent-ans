//! Fade between two successive gestures of a servant (NT10 `role_blend_s`; port of the `prev` /
//! `prev_since` / `fade_y` part of `siege_crew_fx.gd`).
//!
//! When a servant changes gesture, the previous clip keeps playing (from its own start) for
//! `role_blend_s` seconds while the new one fades in. Godot packs this into `INSTANCE_CUSTOM`
//! (`BattleSkinned.pack_fade`); the Bevy skinned shader reads the same information from the
//! `volley_time` slot of the record in the fixed (Pick) mode, as [`pack`] writes it:
//!
//! `packed = (prev_pick + 1) + 128 * delta_q + 128 * 256 * stagger_q`
//!
//! - `prev_pick`: slot of the previous clip in the batch's clip set;
//! - `delta_q`: `at - prev_since` (time the previous gesture had been playing when the change
//!   happened) in 1/16 s, at most 255;
//! - `stagger_q`: `at - since` (the new gesture starts a little before `at` for the first
//!   servants, see `crew.rs`) in 1/16 s, at most 15.
//!
//! `0.0` or less (the batches use `-1000`) means no fade. The decoder lives in `skinned.wesl`
//! (`anim_pose`, mode 3) and [`unpack`] is its tested twin.

use std::collections::HashMap;
use std::path::Path;

/// Fade length of the roles, `role_blend_s` of `data/fx/battle_animation.json` (0 = hard cut).
pub fn role_blend_s(data_dir: &Path) -> f32 {
    std::fs::read_to_string(data_dir.join("fx/battle_animation.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|doc| doc.get("role_blend_s").and_then(serde_json::Value::as_f64))
        .map_or(0.0, |v| v as f32)
}

/// Steps per second of the packed durations.
const STEPS: f32 = 16.0;

/// Packs the previous clip slot and the two durations (see the module doc).
pub fn pack(prev_pick: usize, delta_s: f32, stagger_s: f32) -> f32 {
    let q = |s: f32, max: f32| (s.max(0.0) * STEPS).round().min(max);
    (prev_pick.min(126) + 1) as f32 + 128.0 * q(delta_s, 255.0) + 128.0 * 256.0 * q(stagger_s, 15.0)
}

/// Decodes [`pack`]: `(prev_pick, delta_s, stagger_s)`, `None` when there is no fade.
pub fn unpack(packed: f32) -> Option<(usize, f32, f32)> {
    if packed < 0.5 {
        return None;
    }
    let whole = (packed + 0.5).floor();
    let stagger_q = (whole / (128.0 * 256.0)).floor();
    let rest = whole - stagger_q * 128.0 * 256.0;
    let delta_q = (rest / 128.0).floor();
    let prev = rest - delta_q * 128.0 - 1.0;
    (prev >= 0.0).then_some((prev as usize, delta_q / STEPS, stagger_q / STEPS))
}

/// What a servant plays: the current gesture, and the previous one while it fades out.
#[derive(Debug, Clone, PartialEq)]
pub struct Gesture {
    pub clip: String,
    /// Instant the clip starts (the staggered start).
    pub since: f32,
    /// Instant the gesture changed.
    pub at: f32,
    /// Previous clip and the instant it started.
    pub prev: Option<(String, f32)>,
}

impl Gesture {
    /// The fade still running at `now`, as `(previous clip, packed record value)`;
    /// `None` once it is over.
    pub fn fade(
        &self,
        now: f32,
        blend_s: f32,
        slot_of: impl Fn(&str) -> Option<usize>,
    ) -> Option<f32> {
        let (clip, prev_since) = self.prev.as_ref()?;
        if blend_s <= 0.0 || now < self.at || now - self.at > blend_s + 0.1 {
            return None;
        }
        let prev_pick = slot_of(clip)?;
        Some(pack(prev_pick, self.at - prev_since, self.at - self.since))
    }
}

/// Gestures of all the servants by stable key.
#[derive(Debug, Default)]
pub struct Gestures {
    by_key: HashMap<String, Gesture>,
}

impl Gestures {
    /// The gesture of `key` doing `clip` at `now`; a change starts a new gesture whose clip
    /// restarts at `now - stagger_s` and remembers the previous one for the fade.
    pub fn update(
        &mut self,
        key: &str,
        clip: &str,
        now: f32,
        stagger_s: f32,
        blend_s: f32,
    ) -> &Gesture {
        let entry = self.by_key.entry(key.to_string());
        let changed = match &entry {
            std::collections::hash_map::Entry::Occupied(o) => o.get().clip != clip,
            std::collections::hash_map::Entry::Vacant(_) => true,
        };
        let gesture = entry.or_insert_with(|| Gesture {
            clip: clip.to_string(),
            since: now - stagger_s,
            at: now,
            prev: None,
        });
        if changed && gesture.clip != clip {
            let prev = (blend_s > 0.0).then(|| (gesture.clip.clone(), gesture.since));
            *gesture = Gesture {
                clip: clip.to_string(),
                since: now - stagger_s,
                at: now,
                prev,
            };
        }
        gesture
    }

    pub fn clear(&mut self) {
        self.by_key.clear();
    }

    pub fn len(&self) -> usize {
        self.by_key.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_key.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packing_round_trips() {
        for (prev, delta, stagger) in [(0, 0.0, 0.0), (5, 2.5, 0.26), (17, 15.9, 0.52)] {
            let (p, d, s) = unpack(pack(prev, delta, stagger)).expect("fade");
            assert_eq!(p, prev);
            assert!((d - delta).abs() <= 1.0 / 32.0 + 1e-4, "{d} vs {delta}");
            assert!((s - stagger).abs() <= 1.0 / 32.0 + 1e-4);
        }
        assert!(pack(126, 15.9, 0.9) < 16_777_216.0, "exact in f32");
        assert_eq!(unpack(-1000.0), None);
        assert_eq!(unpack(0.0), None);
    }

    #[test]
    fn a_change_keeps_the_previous_clip_for_the_blend_only() {
        let mut gestures = Gestures::default();
        let slot = |c: &str| match c {
            "idle" => Some(0),
            "crank" => Some(1),
            _ => None,
        };
        let g = gestures.update("e1/0", "idle", 10.0, 0.0, 0.25).clone();
        assert!(g.prev.is_none());
        assert_eq!(
            g.fade(10.0, 0.25, slot),
            None,
            "first gesture: nothing to fade from"
        );
        let g = gestures.update("e1/0", "idle", 11.0, 0.0, 0.25).clone();
        assert_eq!(g.since, 10.0, "same clip: unchanged");
        let g = gestures.update("e1/0", "crank", 12.0, 0.13, 0.25).clone();
        assert_eq!(g.prev, Some(("idle".to_string(), 10.0)));
        let packed = g.fade(12.1, 0.25, slot).expect("fading");
        let (prev, delta, stagger) = unpack(packed).expect("decoded");
        assert_eq!(prev, 0);
        assert!((delta - 2.0).abs() < 0.04 && (stagger - 0.13).abs() < 0.04);
        assert_eq!(g.fade(12.5, 0.25, slot), None, "over after blend + 0.1 s");
        assert_eq!(g.fade(12.1, 0.0, slot), None, "role_blend_s = 0: hard cut");
    }

    #[test]
    fn the_roles_blend_is_read_from_the_data() {
        let root = crate::paths::find_root().expect("root");
        let blend = role_blend_s(&crate::paths::data_dir(&root));
        assert!((blend - 0.25).abs() < 1e-6);
    }
}
