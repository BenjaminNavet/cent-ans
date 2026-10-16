//! Generated speech clips: `audio/voice/speech/<voice>/<sha1(text)[..12]>.ogg` (ADR 0145). The
//! same file serves the end of a general's speech and the war cry repeated in chorus.
//! Port of `VoiceLines.speech_path`, `war_cry` and `speech_voice` (voice_lines.gd).

use bevy::prelude::*;
use serde_json::Value;

use super::{AudioDirector, DuckMusic, PlayVoice, read_json};

/// Hex length of the hash in a speech file name (`SPEECH_HASH_LEN`).
pub const HASH_LEN: usize = 12;

/// SHA-1 of `data`, lower-case hex (Godot's `String.sha1_text`).
pub fn sha1_hex(data: &[u8]) -> String {
    let mut state: [u32; 5] = [
        0x6745_2301,
        0xEFCD_AB89,
        0x98BA_DCFE,
        0x1032_5476,
        0xC3D2_E1F0,
    ];
    let mut padded = data.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for block in padded.chunks(64) {
        let mut words = [0u32; 80];
        for (index, word) in block.chunks(4).enumerate() {
            words[index] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for index in 16..80 {
            words[index] =
                (words[index - 3] ^ words[index - 8] ^ words[index - 14] ^ words[index - 16])
                    .rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = state;
        for (index, word) in words.iter().enumerate() {
            let (f, k) = match index {
                0..=19 => ((b & c) | (!b & d), 0x5A82_7999),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
                _ => (b ^ c ^ d, 0xCA62_C1D6),
            };
            let next = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = next;
        }
        for (slot, value) in state.iter_mut().zip([a, b, c, d, e]) {
            *slot = slot.wrapping_add(value);
        }
    }
    state.iter().map(|word| format!("{word:08x}")).collect()
}

/// `speech/<voice>/<hash>`, relative to `audio/voice/` (no extension).
pub fn speech_path(voice: &str, text: &str) -> String {
    let hash = sha1_hex(text.as_bytes());
    format!("speech/{voice}/{}", &hash[..HASH_LEN])
}

/// Text of a faction's war cry (`order_war_cry.json` `labels_by_faction`, else the default cry of
/// `speeches/battle_speeches.json`).
pub fn war_cry_text(war_cries: &Value, speeches: &Value, faction: &str) -> String {
    war_cries
        .pointer(&format!("/labels_by_faction/{faction}"))
        .and_then(Value::as_str)
        .or_else(|| speeches.get("default_cry").and_then(Value::as_str))
        .unwrap_or("")
        .to_string()
}

/// Casting voices of a faction (`speech_voices.json` `casting`, else `default`).
pub fn casting_voices(speech_voices: &Value, faction: &str) -> Vec<String> {
    let casting = speech_voices.get("casting");
    casting
        .and_then(|table| table.get(faction))
        .or_else(|| casting.and_then(|table| table.get("default")))
        .and_then(|entry| entry.get("voices"))
        .and_then(Value::as_array)
        .map(|voices| {
            voices
                .iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

/// Voice of a general: one of the casting voices of his faction, picked by his id (the same
/// general keeps the same voice). Port of `VoiceLines.speech_voice`.
pub fn speech_voice(speech_voices: &Value, faction: &str, general_id: &str) -> String {
    let voices = casting_voices(speech_voices, faction);
    if voices.is_empty() {
        return String::new();
    }
    let hash = sha1_hex(general_id.as_bytes());
    let pick = u32::from_str_radix(&hash[..8], 16).unwrap_or(0) as usize;
    voices[pick % voices.len()].clone()
}

/// A line of the general's speech (or his war cry): voiced by the faction casting, music ducked.
/// The pre-battle speech screen (battle lot) sends one per sentence; the clip is
/// `speech/<voice>/<sha1(text)[..12]>`, absent = silent (subtitle only, handled by the sender).
#[derive(Message, Debug, Clone)]
pub struct PlaySpeech {
    pub faction: String,
    pub general_id: String,
    pub text: String,
}

/// Voice-over duration estimate (s) of a spoken line, used to hold the music duck.
pub fn spoken_seconds(text: &str) -> f32 {
    text.chars().count() as f32 / 13.0 + 0.6
}

/// How long the music duck is held for a line: the real length of the clip when it exists
/// (probed from the file header), else the characters-per-second estimate.
fn speech_hold(director: &mut AudioDirector, clip: &str, text: &str) -> f32 {
    director
        .resolve_clip("audio/voice/", clip)
        .and_then(|path| director.clip_length(&path))
        .unwrap_or_else(|| spoken_seconds(text))
}

const SPEECH_DUCK_DB: f32 = -9.0;
const SPEECH_PRIORITY: i32 = 7;

/// Plays [`PlaySpeech`] lines through the voice pool.
pub fn handle_speech(
    mut requests: MessageReader<PlaySpeech>,
    mut director: ResMut<AudioDirector>,
    mut voices: MessageWriter<PlayVoice>,
    mut ducks: MessageWriter<DuckMusic>,
) {
    for request in requests.read() {
        let casting =
            read_json(&director.data_dir.join("voice/speech_voices.json")).unwrap_or(Value::Null);
        let voice = speech_voice(&casting, &request.faction, &request.general_id);
        if voice.is_empty() || request.text.is_empty() {
            continue;
        }
        let clip = speech_path(&voice, &request.text);
        director.speech_log.push(clip.clone());
        if director.speech_log.len() > 16 {
            director.speech_log.remove(0);
        }
        let hold = speech_hold(&mut director, &clip, &request.text);
        voices.write(PlayVoice {
            clip,
            priority: SPEECH_PRIORITY,
        });
        ducks.write(DuckMusic {
            db: SPEECH_DUCK_DB,
            hold,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha1_known_vectors() {
        assert_eq!(sha1_hex(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");
        assert_eq!(sha1_hex(b""), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        let long = vec![b'a'; 1000];
        assert_eq!(sha1_hex(&long).len(), 40);
    }

    #[test]
    fn paths_and_lookups() {
        assert_eq!(speech_path("onyx", "abc"), "speech/onyx/a9993e364706");
        let cries = serde_json::json!({"labels_by_faction": {"fac_england": "Saint George !"}});
        let speeches = serde_json::json!({"default_cry": "Montjoie !"});
        assert_eq!(
            war_cry_text(&cries, &speeches, "fac_england"),
            "Saint George !"
        );
        assert_eq!(war_cry_text(&cries, &speeches, "fac_x"), "Montjoie !");
        let voices = serde_json::json!({"casting": {"default": {"voices": ["onyx", "ash"]}}});
        assert_eq!(casting_voices(&voices, "fac_x"), ["onyx", "ash"]);
        let first = speech_voice(&voices, "fac_x", "char_edward");
        assert!(["onyx", "ash"].contains(&first.as_str()));
        assert_eq!(first, speech_voice(&voices, "fac_x", "char_edward"));
        assert_eq!(speech_voice(&serde_json::json!({}), "fac_x", "a"), "");
    }
}
