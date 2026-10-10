//! Real-time-with-pause battle simulation (spec `docs/design/m7-battles.md` § 1).
//!
//! Pure and deterministic: a [`BattleSim`] is built from a [`BattleSetup`] and
//! a seed, advanced by fixed ticks of [`DT`] seconds and driven by
//! [`Command`]s. The same setup, seed and commands issued at the same ticks
//! always give the same battle. No Godot dependency: `godot-bridge` wraps the
//! simulation, `sim-campaign` builds the setup and applies the
//! [`BattleOutcome`].
//!
//! # Module organisation
//!
//! Setup and outcome
//! - `setup` (the [`BattleSetup`] the campaign hands over), `outcome` (result and
//!   journal), `command` (player and AI orders), `replay`, `custom` (menu
//!   battle), `historical` (Crécy, Poitiers...), `opening`, `decision`, `duel`.
//!
//! Regiments and their rules
//! - `unit` (state, formation geometry, soldier grid), `formations`,
//!   `formation_width`, `group_formation`, `modes`, `abilities`, `orders`,
//!   `queue`, `deployment`, `morale`, `rout`, `missile_morale`, `pace`,
//!   `movement_rules`, `push`, `impact`, `crest`, `horse_wait`, `hover`,
//!   `preview`.
//!
//! Field and environment
//! - `field` (height grid, forests, mud, river, weather), `hydro`, `relief`,
//!   `terrain_rules`, `scale` (field size by head count), `site` (campaign
//!   site: season ground, coast, marsh, village), `time_of_day`, `geom`, `rng`.
//! - `decor` and `decor_gen` (hamlets, mills, manors, fields, camps and
//!   baggage), `position` (worth of a defensive position), `relief_ai`.
//!
//! Sieges
//! - `siege` (walls, towers, gate), `siege_layout`, `siege_fx`, `capture`
//!   (capture points), `fire` (incendiary volleys, spread, smoke), `town` and
//!   `props` (dense town, solid street furniture).
//!
//! Simulation
//! - `sim` is the [`BattleSim`] itself, split by phase of the fixed tick
//!   (movement, shooting, melee, morale, sieges, reinforcements...). Shooting
//!   events for the renderer live in `shot`, charge impacts in `impact`.
//!
//! Battle AI
//! - `ai` (tactical AI for the side the player does not command, every
//!   [`AI_PERIOD`] simulated seconds: roles, plans, shooters, cavalry, siege),
//!   with its tuning constants in `ai_rules`, and `formation_ai`, `alerts`.
//!
//! Naval
//! - `naval` auto-resolves a sea battle (fleets, ships, boarding, outcome); no
//!   3D simulation.
//!
//! Everything is data-driven: constants come from `data/`, not from the code.

pub(crate) mod abilities;
pub mod ai;
pub mod ai_rules;
pub mod alerts;
pub(crate) mod capture;
pub(crate) mod command;
pub(crate) mod crest;
pub mod custom;
pub(crate) mod decision;
pub(crate) mod decor;
mod decor_gen;
pub mod deployment;
pub(crate) mod duel;
pub(crate) mod field;
pub(crate) mod fire;
pub mod formation_ai;
pub(crate) mod formation_width;
pub(crate) mod formations;
pub(crate) mod geom;
pub mod group_formation;
pub(crate) mod historical;
pub(crate) mod horse_wait;
pub(crate) mod hover;
pub(crate) mod hydro;
pub mod impact;
pub mod missile_arc;
pub(crate) mod missile_morale;
pub(crate) mod modes;
pub(crate) mod morale;
pub(crate) mod movement_rules;
pub mod naval;
pub mod opening;
pub(crate) mod orders;
pub(crate) mod outcome;
pub(crate) mod pace;
pub mod position;
pub(crate) mod preview;
pub(crate) mod props;
pub(crate) mod push;
pub(crate) mod queue;
pub(crate) mod relief;
pub mod relief_ai;
pub mod replay;
pub(crate) mod rng;
pub(crate) mod rout;
pub(crate) mod scale;
pub(crate) mod setup;
pub mod shot;
pub mod siege;
pub mod siege_fx;
pub(crate) mod siege_layout;
pub mod sim;
pub mod site;
pub mod spear_wall;
pub mod speech;
pub(crate) mod terrain_rules;
pub mod time_of_day;
pub mod town;
pub mod unit;

pub use abilities::{AbilityView, ActiveAbility, EndedAbility, UnitAbilities};
pub use capture::{CapturePoint, CapturePointKind, CaptureRules, PointStatus};
pub use command::{Command, CommandError};
pub use custom::{CustomBattle, CustomBattleRules, CustomData, CustomReport, CustomSide};
pub use decision::{BattleEnd, DecisionRules};
pub use decor::{
    Area, AreaKind, Camp, Decor, DecorHover, DecorItem, DecorPlan, DecorProp, DecorPropKind,
    DecorRules, FieldState, Hamlet, HamletLayout, Moat, Mound,
};
pub use duel::DuelRules;
pub use field::{
    Battlefield, Ford, River, Weather, Zone, ATTACKER_LINE_Z, DEFENDER_LINE_Z, FIELD_DEPTH,
    FIELD_WIDTH, GRID_RESOLUTION,
};
pub use fire::{Blaze, BurnChoice, FireRules, FireState};
pub use formation_width::{split_widths, FormationWidthRules, RankBounds};
pub use formations::{AiRole, FormationDef, FormationRules, FormationShape, Reform};
pub use group_formation::{FormationSlot, GroupFormationRules, Preset, Role, Stance};
pub use historical::HistoricalMap;
pub use hover::{
    Advantage, Compare, CompareSide, HoverContext, HoverKind, HoverRules, RangeArcRules,
};
pub use hydro::{
    Bank, BankKind, Bridge, Crossing, Road, RoadKind, Stream, StreamKind, Water, WaterRules,
    WatersideSpot,
};
pub use impact::{ImpactEvent, ImpactKind, LossCause};
pub use modes::{UnitMode, UnitModeRules, UnitStatus};
pub use morale::MoraleRules;
pub use opening::OpeningRules;
pub use orders::{OrderUse, OrderView};
pub use outcome::{BattleEvent, BattleOutcome, SideResult, StandardTrophy};
pub use preview::{PreviewError, PreviewLeg};
pub use push::{PushRules, PushShape};
pub use queue::{QueueRules, QueuedOrder};
pub use relief::ReliefStyle;
pub use replay::{
    BattleReplay, Divergence, ReplayAction, ReplayError, ReplayPlayer, ReplayRecorder, ReplayRules,
    ReplayStart,
};
pub use rng::BattleRng;
pub use scale::{BattleScale, BattleScaleRules, FieldSize, ScaleTier};
pub use setup::{
    BattleCrossing, BattleOpening, BattleSeason, BattleSetup, CrossingStructure, EntryEdge,
    GeneralSetup, SideId, SideSetup, SiegeEngineSetup, SiegeSetup, UnitSetup,
};
pub use shot::{MissileKind, ShotCover, ShotEvent};
pub use siege::{PieceKind, SiegeWorkRules, SiegeWorks, Tower, WallPiece};
pub use siege_fx::{SiegeFx, SiegeFxKind};
pub use siege_layout::{LayoutError, LayoutGate, SiegeLandmark, SiegeLayout};
pub use sim::{
    AmbushLayout, BattleSim, CampState, DeploymentZone, Ladder, SetupError, SiegeEngineKind,
    SiegeEngineView, AI_PERIOD, DT, FRIEND_GAP, MAX_DURATION, MAX_ON_FIELD, SIEGE_STANDOFF,
    ZONE_DEPTH,
};
pub use site::{Coast, FieldSite, Flank, Ground, House, HouseKind, Obstacle, ObstacleKind};
pub use time_of_day::{DayPhase, TimeOfDayRules};
pub use town::{Footprint, Prop, PropKind, TownRules};
pub use unit::{Formation, Unit, UnitFate, UnitState};
