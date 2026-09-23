//! Real-time-with-pause battle simulation (spec `docs/design/m7-battles.md` § 1).
//!
//! Pure and deterministic: a [`BattleSim`] is built from a [`BattleSetup`] and
//! a seed, advanced by fixed ticks of [`DT`] seconds and driven by
//! [`Command`]s. The same setup, seed and commands issued at the same ticks
//! always give the same battle. No Godot dependency: `godot-bridge` wraps the
//! simulation, `sim-campaign` builds the setup and applies the
//! [`BattleOutcome`].
//!
//! # Model (summary)
//!
//! - Field of [`FIELD_WIDTH`] × [`FIELD_DEPTH`] metres with procedural hills,
//!   forests, mud and an optional river with two fords ([`Battlefield`]);
//!   weather drawn from the season ([`Weather`]).
//! - Each regiment ([`Unit`]) is an oriented rectangle whose size follows its
//!   formation; soldiers are laid out in a grid for rendering.
//! - Movement slowed by slope, forest, mud, water and fatigue; shooting with
//!   range, line of sight, weather and armour; melee on contact with charge,
//!   flank (+50 %) and rear (+100 %) bonuses, pikes/schiltron against
//!   cavalry, archers' stakes; morale, rout and rally; a general whose aura
//!   steadies nearby regiments.
//! - Minimal battle AI ([`ai`]) for the side the player does not command.

pub mod ai;
pub mod command;
pub mod field;
pub mod outcome;
pub mod rng;
pub mod setup;
pub mod siege;
pub mod sim;
pub mod unit;

pub use command::{Command, CommandError};
pub use field::{
    Battlefield, Ford, River, Weather, Zone, ATTACKER_LINE_Z, DEFENDER_LINE_Z, FIELD_DEPTH,
    FIELD_WIDTH, GRID_RESOLUTION,
};
pub use outcome::{BattleEvent, BattleOutcome, SideResult};
pub use rng::BattleRng;
pub use setup::{
    BattleSeason, BattleSetup, GeneralSetup, SideId, SideSetup, SiegeSetup, UnitSetup,
};
pub use siege::{PieceKind, SiegeWorks, Tower, WallPiece};
pub use sim::{BattleSim, SetupError, DT, MAX_DURATION};
pub use unit::{Formation, Unit, UnitState};
