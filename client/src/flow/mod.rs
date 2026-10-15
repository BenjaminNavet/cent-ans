//! Campaign ↔ battle flow: encounter → pre-battle → battle or auto-resolve → result back to the campaign.
//!
//! Stub (V0). Will port: game/scripts/ (battle launch / result return glue, pre-battle dialog).
//! Owned by a single lot of agent (ADR 0357); other lots do not edit this folder.
//!
//! Shared interface (fixed before wave 3, other lots may use it):
//! - `battle::scene::PendingBattle` is what flow fills before entering `AppState::Battle`;
//! - the battle HUD's result screen sends `LeaveBattle` when the player clicks « Continuer »;
//!   flow then applies the outcome to the campaign and returns to `AppState::Campaign`.

use bevy::prelude::*;

/// Sent by the battle result screen (battle/hud) when the player leaves a finished (or
/// abandoned) battle. Flow owns what happens next.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeaveBattle {
    /// True when the player quit before the end (counts as a retreat).
    pub abandoned: bool,
}

pub struct FlowPlugin;

impl Plugin for FlowPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<LeaveBattle>();
    }
}
