//! Campaign simulation: turn-based state of the Hundred Years' War campaign.
//!
//! The simulation is pure and deterministic: `state + orders + seed -> new
//! state`. All randomness goes through the seeded [`CampaignRng`] so that the
//! same seed and the same orders always produce the same state, and the whole
//! state is `serde`-serialisable (see [`CampaignState::save_json`]).
//!
//! # Public API (stable, used by `godot-bridge`)
//!
//! - [`CampaignState::new_1337`] builds the spring 1337 start from [`data_model::GameData`].
//! - [`CampaignState::submit_order`] validates and records a player [`Order`].
//! - [`CampaignState::end_turn`] resolves the turn (AI orders, movement,
//!   battles, sieges, raids, economy, attrition, characters) and returns the
//!   turn's [`GameEvent`]s.
//! - Queries: [`CampaignState::reachable`], [`CampaignState::find_path`],
//!   [`CampaignState::recruitable`], [`CampaignState::faction_summary`],
//!   [`CampaignState::province_state`], [`CampaignState::army`],
//!   [`CampaignState::armies`], [`CampaignState::events`],
//!   [`CampaignState::date_label`], [`CampaignState::turn`],
//!   [`CampaignState::player_faction`].
//! - Persistence: [`CampaignState::save_json`], [`CampaignState::load_json`].
//!
//! # Deviations from `docs/design/m2-campaign-loop.md`
//!
//! - The `ai` crate depends on this crate, so `end_turn` cannot call
//!   `ai::plan_turn` without a dependency cycle. The minimal M2 planner lives in
//!   [`ai_minimal::plan_turn`] (re-exported as `ai::plan_turn_minimal`) as a
//!   fallback; the strategic planner (M9) is `ai::plan_turn`, passed to
//!   [`CampaignState::end_turn_with`], which accepts any planner.
//! - Every faction (not only playable ones) starts with a main army so that AI
//!   factions can act; minor factions get 3 units.
//! - Vassal/overlord relations are treated as alliances (friendly territory).

pub mod agents;
pub mod ai_minimal;
pub mod ai_replay;
pub mod battle_auto;
pub mod battle_forecast;
pub mod battle_history;
pub mod battle_outcome;
pub mod battle_request;
pub mod buildings;
pub mod capture;
pub mod characters;
pub mod chivalry;
pub mod chronicle;
pub mod coinage;
pub mod crusade;
pub mod difficulty;
pub mod diplomacy;
pub mod dynasty;
pub mod economy;
pub mod economy_balance;
pub mod edicts;
pub mod encounter;
pub mod events;
pub mod feudal;
pub mod frontier;
pub mod holdings;
pub mod map_lens;
pub mod map_scenes;
pub mod march;
pub mod medicine;
pub mod mercenaries;
pub mod missions;
pub mod movement;
pub mod naval;
pub mod navigation;
pub mod negotiation;
pub mod news_relevance;
pub mod orders;
pub mod passage;
pub mod path_plan;
pub mod planning_scope;
pub mod population;
pub mod possession;
pub mod posture;
pub mod preview;
pub mod ransom;
pub mod reach;
pub mod recruit_pool;
pub mod religion;
pub mod replenish;
pub mod research;
pub mod retinue;
pub mod river_crossing;
pub mod rng;
pub mod rule_constants;
pub mod save;
pub mod sea_lanes;
pub mod settlements;
pub mod setup_1337;
pub mod siege;
pub mod siege_engines;
pub mod skills;
pub mod stance;
pub mod state;
pub mod table;
pub mod trade;
pub mod traditions;
pub mod treaty_explain;
pub mod turn;
pub mod victory;
pub mod vision;
pub mod voyage;
pub mod weather;

pub use agents::{
    AgentActionOption, AgentError, AgentId, AgentRecruitOption, AgentReport, AgentsState,
};
pub use ai_replay::{AiMoveKind, AiMoveNotability, AiMoveRecord};
pub use battle_auto::{
    resolve_auto, resolve_field, resolve_with, resolve_with_crossings, BattleContext, BattleResult,
    BattleUnit, FieldConditions, Side, SideOutcome, UnitFamily, UnitProfile, Winner,
};
pub use battle_request::{BattleRequestError, PendingBattle, NO_QUARTER_PIETY};
pub use buildings::{BuildOption, DemolitionPreview, EffectTotals, EffectValue, ProvinceCity};
pub use capture::{
    CaptureDecisionView, CaptureEffects, CaptureError, CaptureOptionView, CaptureOutcome,
    CaptureState, PendingCapture,
};
pub use chivalry::{ChivalryError, OrderState};
pub use chronicle::{
    ChronicleError, ChronicleState, Decision, DecisionOptionView, DecisionView, EventContext,
    PlagueWave,
};
pub use coinage::{CoinageError, CoinageLevel, CoinageParams};
pub use crusade::{CrusadeError, CrusadeState, CrusadeView, FervorChange, PendingPassage};
pub use diplomacy::{
    Claim, DiplomacyEntry, DiplomacyError, Evaluation, Offer, OpinionModifier, Proposal,
    RelationKind,
};
pub use dynasty::{
    CharacterView, ChildView, GovernorError, MarriageError, TraitView, MAJORITY_AGE,
    MARRIAGE_MIN_AGE,
};
pub use economy::{FactionEconomy, TaxRate};
pub use encounter::{
    EncounterBattle, EncounterError, EncounterOptionView, EncounterSite, EncounterSiteView,
    EncounterState, PendingEncounter, PendingEncounterView,
};
pub use events::{EventKind, GameEvent};
pub use frontier::GarrisonRole;
pub use march::{MoveReport, StopReason};
pub use mercenaries::{MercenaryMarket, MercenaryOption, MercenaryState};
pub use navigation::{Cell, GridPath};
pub use orders::{
    MoveOrderTarget, Order, OrderError, OrderOutcome, Place, RecruitGroup, RecruitOption,
    RecruitPrice, BASE_RECRUIT_SLOTS,
};
pub use path_plan::PathPlan;
pub use ransom::{CaptiveRank, RansomDebt, RansomError, RansomTerms};
pub use recruit_pool::PoolView;
pub use replenish::{FactorKind, ReplenishFactor, ReplenishPreview, Territory};
pub use research::{ResearchError, ResearchInfo, TechStatus, UnitTechBonus};
pub use rng::CampaignRng;
pub use save::{
    CampaignError, FREE_MOVEMENT_STATE_VERSION, SETTLEMENTS_STATE_VERSION, WIDE_MAP_STATE_VERSION,
};
pub use skills::LearnSkillError;
pub use state::{
    Army, ArmyId, ArmyPosition, BattleRequest, CampaignState, CharacterState, Construction,
    FactionState, FactionSummary, MoraleModifier, MoveTarget, ProvinceState, QueuedRecruit, Season,
    SettlementState, SiegeState, Stance, Unit, MAX_MOVEMENT_POINTS, START_YEAR, STATE_VERSION,
    TURNS_PER_YEAR,
};
pub use table::{DietChoice, DietError, DietOption, DEFAULT_DIET};
pub use trade::{faction_trade_income, trade_routes, TradeMode, TradeRouteView};
pub use traditions::{ArmyTraditions, TraditionError, TraditionOption, TraditionView};
pub use weather::{MapWeather, ProvinceWeather};

#[cfg(test)]
mod capture_tests;
#[cfg(test)]
mod review_tests;
#[cfg(test)]
mod rs_c_tests;
#[cfg(test)]
mod rs_n_tests;
