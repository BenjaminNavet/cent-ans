//! Cent Ans Bevy client (ADR 0357). The binary (`main.rs`) only registers the plugins; the
//! library exists so that `client/tests/` can reach the pure parts (CLI, script, paths).

pub mod agent;
pub mod app;
pub mod audio;
pub mod battle;
pub mod campaign;
pub mod codex;
pub mod dataassets;
pub mod flow;
pub mod menu;
pub mod paths;
pub mod settings;
pub mod sim;
pub mod ui;
