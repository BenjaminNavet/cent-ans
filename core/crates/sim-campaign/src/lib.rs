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
//!   [`ai_minimal::plan_turn`] and the `ai` crate re-exports it as
//!   `ai::plan_turn`. [`CampaignState::end_turn_with`] accepts any planner.
//! - Every faction (not only playable ones) starts with a main army so that AI
//!   factions can act; minor factions get 3 units.
//! - Vassal/overlord relations are treated as alliances (friendly territory).
//! - Population is stored per province but does not evolve yet (M3).
//! - No assault order: sieges are only resolved by duration.

pub mod ai_minimal;
pub mod battle_auto;
pub mod buildings;
pub mod characters;
pub mod economy;
pub mod events;
pub mod movement;
pub mod orders;
pub mod population;
pub mod rng;
pub mod save;
pub mod setup_1337;
pub mod siege;
pub mod state;
pub mod turn;

pub use battle_auto::{
    resolve_auto, BattleContext, BattleResult, BattleUnit, Side, SideOutcome, Winner,
};
pub use buildings::{BuildOption, EffectTotals, EffectValue, ProvinceCity};
pub use economy::{FactionEconomy, TaxRate};
pub use events::{EventKind, GameEvent};
pub use orders::{Order, OrderError, RecruitOption};
pub use rng::CampaignRng;
pub use save::CampaignError;
pub use state::{
    Army, ArmyId, BattleRequest, CampaignState, CharacterState, Construction, FactionState,
    FactionSummary, ProvinceState, Season, SiegeState, Stance, Unit, MAX_MOVEMENT_POINTS,
    START_YEAR, STATE_VERSION, TURNS_PER_YEAR,
};
