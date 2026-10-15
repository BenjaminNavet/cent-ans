//! The Codex screen (AppState::Codex): entries, images and bubbles.
//!
//! Stub (V0). Will port: game/scripts/codex/ (codex_window.gd, codex_store.gd, codex_text.gd, codex_bubbles.gd) and game/scripts/ui/codex_hub.gd.
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.

use bevy::prelude::*;

pub struct CodexPlugin;

impl Plugin for CodexPlugin {
    fn build(&self, _app: &mut App) {}
}

pub mod rules;
pub mod store;
pub mod text;
