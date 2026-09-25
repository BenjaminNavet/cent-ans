//! Naval battles (lot NV1, ADR 0028): cogs, nefs, galleys and barges
//! fighting as in the Channel wars of the Hundred Years' War — l'Écluse
//! (1340), Winchelsea (1350), La Rochelle (1372).
//!
//! # Model (summary)
//!
//! - A ship ([`Ship`]) has a hull, a fire level, a deck height (freeboard)
//!   and castles fore and aft; its fighting crew is made of the regiments
//!   embarked by the campaign ([`CrewSetup`]) plus its sailors.
//! - Approach: square-rigged ships cannot point into the wind; the fleet
//!   upwind holds the weather gauge (its arrows carry further and bite
//!   harder, [`Wind`]). Galleys row regardless of the wind, but tire.
//! - Shooting: archers and crossbowmen shoot from the castles (height bonus)
//!   and the deck; bulwarks and pavises stop part of the missiles; fire
//!   arrows set ships alight.
//! - Grapples and boarding: a ship ordered to board throws grapples when
//!   close, lashes alongside and a melee starts; climbing onto a higher deck
//!   costs (a high cog against a galley), castles help the defence, chained
//!   ships send reinforcements across.
//! - Fire and fireships: fire grows with the wind, spreads to lashed ships,
//!   burns hull and crew; the sailors fight it; a crew abandons a blazing
//!   ship. A fireship grapples, is set alight and left.
//! - Capture or sinking: a broken crew grappled to an enemy strikes (the
//!   ship is a prize), a free one flees; a ship without hull sinks, armoured
//!   men drown.
//! - [`auto_resolve`] plays the same formulas in phases, without positions.

pub mod ai;
pub mod auto;
pub mod combat;
pub mod outcome;
pub mod scenario;
pub mod setup;
pub mod ship;
pub mod sim;

pub use auto::auto_resolve;
pub use combat::Wind;
pub use outcome::{
    NavalEvent, NavalEventKind, NavalOutcome, NavalSideResult, ShipFate, ShipResult,
};
pub use scenario::NavalScenario;
pub use setup::{CrewSetup, NavalSetup, NavalSideSetup, ShipSetup};
pub use ship::{Crew, Ship, ShipOrder, ShipStatus};
pub use sim::{
    formation_slot, NavalCommand, NavalCommandError, NavalSetupError, NavalSim, ESCAPE_GRACE,
    NAVAL_AI_PERIOD, NAVAL_DT, OPENING_GAP,
};
