//! UI views of the game (ADR 0357): plain `Serialize` Rust types built from `GameData` and
//! `CampaignState`, shared by every client. No engine dependency (no Bevy, no Godot); each
//! view is unit-tested here by `cargo test -p game-api`.

pub mod faction;

pub use faction::{faction_summary, FactionSummary};
