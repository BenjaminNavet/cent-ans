//! UI views of the game (ADR 0357): plain `Serialize` Rust types built from `GameData` and
//! `CampaignState`, shared by every client. No engine dependency (no Bevy, no Godot); each
//! view is unit-tested here by `cargo test -p game-api`.

pub mod army;
pub mod common;
pub mod diplomacy;
pub mod faction;
pub mod hud;
pub mod pprovince;
pub mod province;
pub mod recruit;
pub mod tech;
pub mod turn;

pub use army::{army_ids, army_sheet, ArmySheet};
pub use common::{FactionRef, KeyedAmount, UnitView};
pub use faction::{faction_sheet, faction_summary, FactionSheet, FactionSummary};
pub use hud::{
    current_difficulty, difficulty_levels, economy, forecast, income_top_provinces, top_bar,
    EconomyView, ForecastView, TopBar,
};
pub use province::{province_sheet, ProvinceSheet};
pub use tech::{tech_tree, TechTree};
pub use turn::{turn_readiness, Alert, TurnReadiness};
