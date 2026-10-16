//! Real duration of a sound file, read from its container header (no decoding): Ogg Vorbis
//! (sample rate of the identification header, granule position of the last page) and PCM WAV
//! (`fmt ` byte rate, `data` size). Used for the voice-over duck hold (instead of the characters
//! per second estimate of [`super::speech::spoken_seconds`]) and for the virtual voice pool of
//! the muted mode. Godot reads `AudioStream.get_length()`; `bevy_audio` has no such probe.

use std::path::Path;

/// Duration (s) of an `.ogg` (Vorbis) or `.wav` file; `None` when the format is unknown.
pub fn clip_seconds(path: &Path) -> Option<f32> {
    let bytes = std::fs::read(path).ok()?;
    match path.extension()?.to_str()? {
        "ogg" => ogg_seconds(&bytes),
        "wav" => wav_seconds(&bytes),
        _ => None,
    }
}

/// Ogg Vorbis: duration = last granule position / sample rate.
pub fn ogg_seconds(bytes: &[u8]) -> Option<f32> {
    // Identification header: packet byte 0x01, "vorbis", version u32, channels u8, rate u32.
    let marker = bytes.windows(7).position(|w| w == b"\x01vorbis")?;
    let rate_at = marker + 7 + 4 + 1;
    let rate = u32::from_le_bytes(bytes.get(rate_at..rate_at + 4)?.try_into().ok()?);
    if rate == 0 {
        return None;
    }
    // Last page: granule position at offset 6 of the last "OggS".
    let last = bytes.windows(4).rposition(|w| w == b"OggS")?;
    let granule = i64::from_le_bytes(bytes.get(last + 6..last + 14)?.try_into().ok()?);
    (granule > 0).then(|| granule as f32 / rate as f32)
}

/// PCM WAV: `data` size over the byte rate of the `fmt ` chunk.
pub fn wav_seconds(bytes: &[u8]) -> Option<f32> {
    if bytes.get(0..4)? != b"RIFF" || bytes.get(8..12)? != b"WAVE" {
        return None;
    }
    let mut at = 12;
    let mut byte_rate = 0u32;
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let size = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().ok()?) as usize;
        let body = at + 8;
        if id == b"fmt " {
            byte_rate = u32::from_le_bytes(bytes.get(body + 8..body + 12)?.try_into().ok()?);
        } else if id == b"data" {
            let available = bytes.len().saturating_sub(body);
            // Streamed writers leave 0 / 0xFFFFFFFF as the size: trust the file length then.
            let size = if size == 0 || size > available {
                available
            } else {
                size
            };
            return (byte_rate > 0).then(|| size as f32 / byte_rate as f32);
        }
        at = body + size + (size & 1);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(rate: u32, channels: u16, frames: u32) -> Vec<u8> {
        let byte_rate = rate * u32::from(channels) * 2;
        let data = frames * u32::from(channels) * 2;
        let mut out = b"RIFF".to_vec();
        out.extend((36 + data).to_le_bytes());
        out.extend(b"WAVEfmt ");
        out.extend(16u32.to_le_bytes());
        out.extend(1u16.to_le_bytes());
        out.extend(channels.to_le_bytes());
        out.extend(rate.to_le_bytes());
        out.extend(byte_rate.to_le_bytes());
        out.extend((channels * 2).to_le_bytes());
        out.extend(16u16.to_le_bytes());
        out.extend(b"data");
        out.extend(data.to_le_bytes());
        out.extend(vec![0u8; data as usize]);
        out
    }

    fn ogg(rate: u32, granule: i64) -> Vec<u8> {
        let mut out = b"OggS".to_vec();
        out.extend([0u8; 22]);
        out.extend(b"\x01vorbis");
        out.extend(0u32.to_le_bytes());
        out.push(2);
        out.extend(rate.to_le_bytes());
        out.extend([0u8; 12]);
        out.extend(b"OggS\0\x04");
        out.extend(granule.to_le_bytes());
        out.extend([0u8; 16]);
        out
    }

    #[test]
    fn wav_duration() {
        let seconds = wav_seconds(&wav(22050, 2, 44100)).unwrap();
        assert!((seconds - 2.0).abs() < 1e-4);
        assert!(wav_seconds(b"not a wav").is_none());
    }

    #[test]
    fn ogg_duration() {
        let seconds = ogg_seconds(&ogg(44100, 66150)).unwrap();
        assert!((seconds - 1.5).abs() < 1e-4);
        assert!(ogg_seconds(&ogg(44100, -1)).is_none());
        assert!(ogg_seconds(b"junk").is_none());
    }

    #[test]
    fn real_asset_if_present() {
        let Ok(root) = crate::paths::find_root() else {
            return;
        };
        let path = root.join("game/assets/audio/ui/alert.wav");
        if let Some(seconds) = clip_seconds(&path) {
            assert!(seconds > 0.0 && seconds < 30.0);
        }
    }
}
