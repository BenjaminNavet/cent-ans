//! Naval battles, auto-resolved only (ADR 0028, 0192): cogs, nefs, galleys
//! and barges fighting as in the Channel wars of the Hundred Years' War.
//!
//! # Model (summary)
//!
//! - A ship ([`Ship`]) has a hull, a fire level, a deck height (freeboard)
//!   and castles fore and aft; its fighting crew is made of the regiments
//!   embarked by the campaign ([`CrewSetup`]) plus its sailors.
//! - The fleet upwind holds the weather gauge ([`Fleets::gauge`]): its arrows
//!   carry further and bite harder, and it shoots more volleys.
//! - Shooting: archers and crossbowmen shoot from the castles (height bonus)
//!   and the deck; bulwarks and pavises stop part of the missiles; fire
//!   arrows set ships alight.
//! - Boarding: climbing onto a higher deck costs (a high cog against a
//!   galley), castles help the defence, chained ships send reinforcements.
//! - Fire and fireships: fire grows with the wind, spreads to lashed ships,
//!   burns hull and crew; a crew abandons a blazing ship.
//! - Capture or sinking: a broken crew strikes (the ship is a prize), a free
//!   one flees; a ship without hull sinks, armoured men drown.
//! - [`auto_resolve`] plays these formulas in phases, without positions.

pub mod auto;
pub mod combat;
pub mod fleet;
pub mod outcome;
pub mod setup;
pub mod ship;

pub use auto::auto_resolve;
pub use fleet::{Fleets, NavalSetupError};
pub use outcome::{NavalOutcome, NavalSideResult, ShipFate, ShipResult};
pub use setup::{CrewSetup, NavalSetup, NavalSideSetup, ShipSetup};
pub use ship::{Crew, Ship, ShipStatus};
