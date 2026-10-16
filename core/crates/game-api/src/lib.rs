//! UI views of the game (ADR 0357): plain `Serialize` Rust types built from `GameData` and
//! `CampaignState`, shared by every client. No engine dependency (no Bevy, no Godot); each
//! view is unit-tested here by `cargo test -p game-api`.

pub mod army;
pub mod chronicle;
pub mod common;
pub mod court;
pub mod diplomacy;
pub mod faction;
pub mod faction_panel;
pub mod feudal;
pub mod flow;
pub mod hud;
pub mod lens;
pub mod pprovince;
pub mod progress;
pub mod province;
pub mod ransoms;
pub mod reach;
pub mod recruit;
pub mod saves;
pub mod siege;
pub mod start_sheets;
pub mod tech;
pub mod turn;

pub use army::{army_ids, army_sheet, ArmySheet};
pub use chronicle::{
    chronicle_view, turn_journal, ChronicleView, JournalEntry, NewsLetter, NewsMode, TurnJournal,
};
pub use common::{FactionRef, KeyedAmount, UnitView};
pub use faction::{faction_sheet, faction_summary, FactionSheet, FactionSummary};
pub use hud::{
    current_difficulty, difficulty_levels, economy, forecast, income_top_provinces, top_bar,
    EconomyView, ForecastView, TopBar,
};
pub use lens::{lens_view, LensCell, LensView, LensWanted};
pub use province::{province_sheet, ProvinceSheet};
pub use reach::{reach_mask, ReachMask};
pub use siege::{siege_sheet, SiegeSheet};
pub use tech::{tech_tree, TechTree};
pub use turn::{turn_readiness, Alert, TurnReadiness};
