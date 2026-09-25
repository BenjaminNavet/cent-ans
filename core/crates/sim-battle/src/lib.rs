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
//! - Field sized by the head count ([`scale`], EP1: 1200 × 800 m up to
//!   2400 × 1600 m) with procedural hills,
//!   forests, mud and an optional river with two fords ([`Battlefield`]);
//!   weather drawn from the season ([`Weather`]). Lot B5 ([`site`]): the
//!   campaign site adds the ground of the season (mud, snow), a coast on a
//!   flank, marsh pools, a village or farm (cover, broken charges) with its
//!   hedges, fences and ditches, bocage hedgerows.
//! - Each regiment ([`Unit`]) is an oriented rectangle whose size follows its
//!   formation; soldiers are laid out in a grid for rendering.
//! - Movement slowed by slope, forest, mud, water and fatigue; shooting with
//!   range, line of sight, weather and armour; melee on contact with charge,
//!   flank (+50 %) and rear (+100 %) bonuses, pikes/schiltron against
//!   cavalry, archers' stakes; charge impacts that knock men down
//!   ([`impact`], lot BV2); morale, rout and rally; a general whose aura
//!   steadies nearby regiments.
//! - Siege battles ([`siege`], spec `docs/design/m8-sieges.md` § 2): town
//!   walls with towers and a gate, ladders, siege towers, ram, engines that
//!   breach the walls, victory by holding the central square.
//! - Siege fires ([`fire`], spec `docs/design/s2-incendies.md`): incendiary
//!   volleys, spread from house to house, heat, smoke, burnt ruins.
//! - Tactical battle AI ([`ai`], spec `docs/design/m9-ai.md` § 2) for the side
//!   the player does not command, every [`AI_PERIOD`] simulated seconds.
//! - Leader's orders ([`orders`], spec `docs/design/battle-orders.md`): war
//!   cry, no quarter, dismount, pavises, rally, from the catalogue of
//!   `data/battle_orders/` carried by [`BattleSetup::orders`].

pub mod ai;
pub mod command;
pub mod field;
pub mod fire;
pub mod formation_ai;
pub mod hydro;
pub mod impact;
pub mod naval;
pub mod orders;
pub mod outcome;
pub mod relief;
pub mod relief_ai;
pub mod rng;
pub mod scale;
pub mod setup;
pub mod shot;
pub mod siege;
pub mod siege_fx;
pub mod siege_layout;
pub mod sim;
pub mod site;
pub mod unit;

pub use command::{Command, CommandError};
pub use field::{
    Battlefield, Ford, River, Weather, Zone, ATTACKER_LINE_Z, DEFENDER_LINE_Z, FIELD_DEPTH,
    FIELD_WIDTH, GRID_RESOLUTION,
};
pub use fire::{Blaze, FireRules, FireState};
pub use hydro::{
    Bank, BankKind, Bridge, Crossing, Road, RoadKind, Stream, StreamKind, Water, WaterRules,
    WatersideSpot,
};
pub use impact::{ImpactEvent, ImpactKind, LossCause};
pub use orders::{OrderUse, OrderView};
pub use outcome::{BattleEvent, BattleOutcome, SideResult, StandardTrophy};
pub use relief::ReliefStyle;
pub use rng::BattleRng;
pub use scale::{BattleScale, BattleScaleRules, FieldSize, ScaleTier};
pub use setup::{
    BattleSeason, BattleSetup, GeneralSetup, SideId, SideSetup, SiegeSetup, UnitSetup,
};
pub use shot::{MissileKind, ShotCover, ShotEvent};
pub use siege::{PieceKind, SiegeWorks, Tower, WallPiece};
pub use siege_fx::{SiegeFx, SiegeFxKind};
pub use siege_layout::{LayoutError, LayoutGate, SiegeLandmark, SiegeLayout};
pub use sim::{
    BattleSim, DeploymentZone, Ladder, SetupError, AI_PERIOD, DT, FRIEND_GAP, MAX_DURATION,
    MAX_ON_FIELD, SIEGE_STANDOFF, ZONE_DEPTH,
};
pub use site::{
    Coast, FieldSite, Flank, Ground, House, HouseKind, Obstacle, ObstacleKind, Village,
};
pub use unit::{Formation, Unit, UnitFate, UnitState};
