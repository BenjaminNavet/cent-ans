//! Music, ambience and UI/battle sounds (OGG from game/assets/audio).
//!
//! Stub (V0). Will port: game/scripts/audio/ (audio_director.gd, sound_bank.gd, campaign_ambience.gd, battle_audio.gd, voice_lines.gd, ui_sounds.gd).
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

use bevy::prelude::*;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, _app: &mut App) {}
}
