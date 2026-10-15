//! Sound bank (`data/audio/sound_bank.json`) and the voice-pool policy. Pure logic, no Bevy.
//! Port of game/scripts/audio/sound_bank.gd and voice_pool.gd.
//!
//! Files live under `game/assets/audio/<relative>.ogg` (or `.wav`); a missing file is silence.

use std::collections::HashMap;

use serde_json::Value;

use super::playlist::XorShift;

/// One-shot event of the bank (variants, priority, instance limit, random pitch, ducking).
#[derive(Debug, Clone)]
pub struct SfxEvent {
    pub files: Vec<String>,
    /// Godot bus name: `Interface`, `Bataille`, `Voix`, `Ambiance`.
    pub bus: String,
    pub priority: i32,
    pub volume_db: f32,
    pub pitch: [f32; 2],
    pub max_instances: usize,
    pub cooldown_s: f32,
    pub file_gain_db: HashMap<String, f32>,
    pub duck_music_db: f32,
    pub duck_seconds: f32,
    /// Events played together with this one (`contact` = shield bash + sword clash).
    pub layers: Vec<String>,
    /// Spatial events (battle): distance (m) at which the sound starts to fade, and the range.
    pub unit_size_m: f32,
    pub max_distance_m: f32,
}

impl SfxEvent {
    fn from_json(value: &Value) -> Self {
        let number =
            |key: &str, default: f64| value.get(key).and_then(Value::as_f64).unwrap_or(default);
        let pitch = value
            .get("pitch")
            .and_then(Value::as_array)
            .filter(|span| span.len() >= 2)
            .map(|span| {
                [
                    span[0].as_f64().unwrap_or(1.0) as f32,
                    span[1].as_f64().unwrap_or(1.0) as f32,
                ]
            })
            .unwrap_or([0.94, 1.06]);
        Self {
            files: value
                .get("files")
                .and_then(Value::as_array)
                .map(|files| {
                    files
                        .iter()
                        .filter_map(Value::as_str)
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default(),
            bus: value
                .get("bus")
                .and_then(Value::as_str)
                .unwrap_or("Bataille")
                .to_string(),
            priority: number("priority", 3.0) as i32,
            volume_db: number("volume_db", 0.0) as f32,
            pitch,
            max_instances: number("max_instances", 4.0) as usize,
            cooldown_s: number("cooldown_s", 0.0) as f32,
            file_gain_db: value
                .get("file_gain_db")
                .and_then(Value::as_object)
                .map(|gains| {
                    gains
                        .iter()
                        .filter_map(|(file, gain)| Some((file.clone(), gain.as_f64()? as f32)))
                        .collect()
                })
                .unwrap_or_default(),
            duck_music_db: number("duck_music_db", 0.0) as f32,
            duck_seconds: number("duck_seconds", 0.0) as f32,
            layers: value
                .get("layers")
                .and_then(Value::as_array)
                .map(|layers| {
                    layers
                        .iter()
                        .filter_map(Value::as_str)
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default(),
            unit_size_m: number("unit_size_m", 10.0) as f32,
            max_distance_m: number("max_distance_m", 300.0) as f32,
        }
    }
}

/// Looping ambience layer (`ambience` section) or battle bed (`beds`).
#[derive(Debug, Clone)]
pub struct LoopEntry {
    pub file: String,
    pub volume_db: f32,
}

#[derive(Debug, Clone, Default)]
pub struct SoundBank {
    pub events: HashMap<String, SfxEvent>,
    pub ambience: HashMap<String, LoopEntry>,
    pub beds: HashMap<String, LoopEntry>,
    /// Campaign-turn event kind -> clip, in decreasing priority (`event_sfx.order`).
    pub event_sfx_order: Vec<(String, String)>,
    pub max_voices: usize,
}

fn loop_entries(value: Option<&Value>) -> HashMap<String, LoopEntry> {
    value
        .and_then(Value::as_object)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|(name, entry)| {
                    Some((
                        name.clone(),
                        LoopEntry {
                            file: entry.get("file")?.as_str()?.to_string(),
                            volume_db: entry
                                .get("volume_db")
                                .and_then(Value::as_f64)
                                .unwrap_or(0.0) as f32,
                        },
                    ))
                })
                .collect()
        })
        .unwrap_or_default()
}

impl SoundBank {
    pub fn from_json(root: &Value) -> Self {
        let events = root
            .get("events")
            .and_then(Value::as_object)
            .map(|events| {
                events
                    .iter()
                    .map(|(name, entry)| (name.clone(), SfxEvent::from_json(entry)))
                    .collect()
            })
            .unwrap_or_default();
        let event_sfx_order = root
            .pointer("/event_sfx/order")
            .and_then(Value::as_array)
            .map(|pairs| {
                pairs
                    .iter()
                    .filter_map(|pair| {
                        Some((
                            pair.get("kind")?.as_str()?.to_string(),
                            pair.get("clip")?.as_str()?.to_string(),
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default();
        Self {
            events,
            ambience: loop_entries(root.get("ambience")),
            beds: loop_entries(root.get("beds")),
            event_sfx_order,
            max_voices: root
                .pointer("/voices/max_voices")
                .and_then(Value::as_u64)
                .unwrap_or(28) as usize,
        }
    }

    /// Clip of the most notable event kind of a campaign turn (`event_sfx`), "" if none.
    pub fn event_sfx(&self, kinds: &[&str]) -> Option<&str> {
        self.event_sfx_order
            .iter()
            .find(|(kind, _)| kinds.contains(&kind.as_str()))
            .map(|(_, clip)| clip.as_str())
    }

    /// Picks a variant at random, never the same as `last` twice in a row when possible.
    /// Returns the variant index and its file.
    pub fn pick_variant(
        &self,
        event: &str,
        last: Option<usize>,
        rng: &mut XorShift,
    ) -> Option<(usize, &str)> {
        let entry = self.events.get(event)?;
        if entry.files.is_empty() {
            return None;
        }
        let mut index = rng.below_inclusive(entry.files.len() - 1);
        if entry.files.len() > 1 && Some(index) == last {
            index = (index + 1) % entry.files.len();
        }
        Some((index, entry.files[index].as_str()))
    }

    /// Every referenced file (relative, without extension).
    pub fn all_files(&self) -> Vec<&str> {
        let mut files: Vec<&str> = self
            .events
            .values()
            .flat_map(|event| event.files.iter().map(String::as_str))
            .collect();
        files.extend(self.beds.values().map(|entry| entry.file.as_str()));
        files.extend(self.ambience.values().map(|entry| entry.file.as_str()));
        files
    }
}

/// A voice currently holding a sound.
#[derive(Debug, Clone, PartialEq)]
pub struct PoolVoice<T> {
    pub handle: T,
    pub event: String,
    pub priority: i32,
    pub started: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Claim {
    /// A free slot exists: start a new voice.
    Free,
    /// Steal the voice at this index (stop it, then start the new one).
    Replace(usize),
    /// Nothing can be taken: skip the sound.
    Refuse,
}

/// Policy of `VoicePool.claim`: beyond `max_instances` of the same event the oldest of that
/// event is replaced; else a free slot (`voices.len() < capacity`); else the lowest-priority
/// (then oldest) voice if its priority is <= the new sound's.
pub fn claim<T>(
    voices: &[PoolVoice<T>],
    capacity: usize,
    event: &str,
    priority: i32,
    max_instances: usize,
) -> Claim {
    let same: Vec<usize> = voices
        .iter()
        .enumerate()
        .filter(|(_, voice)| voice.event == event)
        .map(|(index, _)| index)
        .collect();
    if !same.is_empty() && same.len() >= max_instances.max(1) {
        let oldest = same
            .into_iter()
            .min_by(|a, b| voices[*a].started.total_cmp(&voices[*b].started))
            .unwrap();
        return Claim::Replace(oldest);
    }
    if voices.len() < capacity {
        return Claim::Free;
    }
    let victim = voices.iter().enumerate().min_by(|(_, a), (_, b)| {
        a.priority
            .cmp(&b.priority)
            .then(a.started.total_cmp(&b.started))
    });
    match victim {
        Some((index, voice)) if voice.priority <= priority => Claim::Replace(index),
        _ => Claim::Refuse,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn voice(event: &str, priority: i32, started: f32) -> PoolVoice<u32> {
        PoolVoice {
            handle: 0,
            event: event.into(),
            priority,
            started,
        }
    }

    #[test]
    fn claim_free_slot_then_replace_lowest_priority() {
        let mut voices = vec![voice("a", 2, 1.0)];
        assert_eq!(claim(&voices, 2, "b", 3, 2), Claim::Free);
        voices.push(voice("c", 1, 2.0));
        assert_eq!(claim(&voices, 2, "b", 3, 2), Claim::Replace(1));
        assert_eq!(claim(&voices, 2, "b", 0, 2), Claim::Refuse);
    }

    #[test]
    fn claim_respects_max_instances() {
        let voices = vec![voice("a", 5, 3.0), voice("a", 5, 1.0), voice("z", 1, 0.0)];
        assert_eq!(claim(&voices, 8, "a", 1, 2), Claim::Replace(1));
        assert_eq!(claim(&voices, 8, "a", 1, 3), Claim::Free);
    }

    #[test]
    fn bank_parses_defaults_and_event_sfx() {
        let bank = SoundBank::from_json(&json!({
            "events": {"ui_click": {"files": ["ui/click"], "bus": "Interface", "cooldown_s": 0.04},
                       "x": {"files": ["a", "b"], "file_gain_db": {"a": -2.0}}},
            "ambience": {"wind": {"file": "ambience/wind", "volume_db": -8}},
            "event_sfx": {"order": [{"kind": "battle", "clip": "sword_clash"}, {"kind": "death", "clip": "choir"}]}
        }));
        let click = &bank.events["ui_click"];
        assert_eq!(click.priority, 3);
        assert_eq!(click.max_instances, 4);
        assert_eq!(click.pitch, [0.94, 1.06]);
        assert_eq!(bank.ambience["wind"].volume_db, -8.0);
        assert_eq!(bank.event_sfx(&["death", "battle"]), Some("sword_clash"));
        assert_eq!(bank.event_sfx(&["peace"]), None);
        assert_eq!(bank.all_files().len(), 4);
    }

    #[test]
    fn variants_never_repeat_back_to_back() {
        let bank = SoundBank::from_json(&json!({"events": {"x": {"files": ["a", "b", "c"]}}}));
        let mut rng = XorShift::new(9);
        let mut last = None;
        for _ in 0..200 {
            let (index, _) = bank.pick_variant("x", last, &mut rng).unwrap();
            assert_ne!(Some(index), last);
            last = Some(index);
        }
        assert!(bank.pick_variant("missing", None, &mut rng).is_none());
    }
}
