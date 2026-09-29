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
//! - Dense besieged town and solid street furniture ([`town`], [`props`],
//!   lot BR3, ADR 0047): house blocks and props are oriented rectangles that
//!   block the pathing and push the figures out.
//! - Countryside of the field ([`decor`], lot EP6): hamlets, mills, church
//!   and churchyard, moated manor, vineyards, orchards, ploughland, meadows;
//!   cover, speed and defence of each area; each army's camp and baggage,
//!   looted by an enemy who holds it unguarded (morale).
//! - Tactical battle AI ([`ai`], spec `docs/design/m9-ai.md` § 2) for the side
//!   the player does not command, every [`AI_PERIOD`] simulated seconds.
//! - Leader's orders ([`orders`], spec `docs/design/battle-orders.md`): war
//!   cry, no quarter, dismount, pavises, rally, from the catalogue of
//!   `data/battle_orders/` carried by [`BattleSetup::orders`].

pub mod abilities;
pub mod ai;
pub mod alerts;
pub mod capture;
pub mod command;
pub mod crest;
pub mod custom;
pub mod decision;
pub mod decor;
mod decor_gen;
pub mod duel;
pub mod field;
pub mod fire;
pub mod formation_ai;
pub mod formation_width;
pub mod group_formation;
pub mod historical;
pub mod horse_wait;
pub mod hover;
pub mod hydro;
pub mod impact;
pub mod missile_arc;
pub mod missile_morale;
pub mod modes;
pub mod naval;
pub mod opening;
pub mod orders;
pub mod outcome;
pub mod position;
pub mod preview;
pub mod props;
pub mod push;
pub mod queue;
pub mod relief;
pub mod relief_ai;
pub mod replay;
pub mod rng;
pub mod rout;
pub mod scale;
pub mod setup;
pub mod shot;
pub mod siege;
pub mod siege_fx;
pub mod siege_layout;
pub mod siege_layouts;
pub mod sim;
pub mod site;
pub mod time_of_day;
pub mod town;
pub mod unit;

pub use abilities::{AbilityView, ActiveAbility, EndedAbility, UnitAbilities};
pub use capture::{CapturePoint, CapturePointKind, CaptureRules, PointStatus};
pub use command::{Command, CommandError};
pub use custom::{CustomBattle, CustomBattleRules, CustomData, CustomReport, CustomSide};
pub use decision::{BattleEnd, DecisionRules};
pub use decor::{
    Area, AreaKind, Camp, Decor, DecorItem, DecorPlan, DecorProp, DecorPropKind, DecorRules,
    FieldState, Hamlet, HamletLayout, Moat, Mound,
};
pub use duel::DuelRules;
pub use field::{
    Battlefield, Ford, River, Weather, Zone, ATTACKER_LINE_Z, DEFENDER_LINE_Z, FIELD_DEPTH,
    FIELD_WIDTH, GRID_RESOLUTION,
};
pub use fire::{Blaze, BurnChoice, FireRules, FireState};
pub use formation_width::{split_widths, FormationWidthRules, RankBounds};
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
    BattleOpening, BattleSeason, BattleSetup, GeneralSetup, SideId, SideSetup, SiegeEngineSetup,
    SiegeSetup, UnitSetup,
};
pub use shot::{MissileKind, ShotCover, ShotEvent};
pub use siege::{PieceKind, SiegeWorkRules, SiegeWorks, Tower, WallPiece};
pub use siege_fx::{SiegeFx, SiegeFxKind};
pub use siege_layout::{LayoutError, LayoutGate, SiegeLandmark, SiegeLayout};
pub use siege_layouts::{place_seed, BoroughRules, CastleRules, PlaceKind, PlaceRules};
pub use sim::{
    AmbushLayout, BattleSim, CampState, DeploymentZone, Ladder, SetupError, SiegeEngineKind,
    SiegeEngineView, AI_PERIOD, DT, FRIEND_GAP, MAX_DURATION, MAX_ON_FIELD, SIEGE_STANDOFF,
    ZONE_DEPTH,
};
pub use site::{
    Coast, FieldSite, Flank, Ground, House, HouseKind, Obstacle, ObstacleKind, Village,
};
pub use time_of_day::{DayPhase, TimeOfDayRules};
pub use town::{Footprint, Prop, PropKind, TownRules};
pub use unit::{Formation, Unit, UnitFate, UnitState};
