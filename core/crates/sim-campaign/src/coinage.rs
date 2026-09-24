//! H5 « Monnaie » : mutations monétaires, seigneuriage et inflation.
//!
//! Every faction strikes its money at one of four [`CoinageLevel`]s
//! (`set_coinage`, one change per calendar year). Debasing yields
//! seigniorage every season (a share of the tax income) but raises the
//! faction's `price_level` (100 = base), which scales recruitment, upkeep and
//! construction costs and angers the classes living on fixed money rents
//! (burghers and clergy), who also lose wealth. A strong coinage costs a
//! recoinage every season, brings prices slowly back to 100, pleases the
//! burghers and adds to the ruler's prestige.
//!
//! The parameters are Rust constants, as for the other economic rules
//! (`economy.rs`); they are documented in `docs/design/h5-h6-api.md`.
//!
//! Calibration (a realm whose upkeep is ≈ 70 % of its income): debasing
//! (+15 % income, +3 prices a season) pays for about two years, then the
//! costs outrun the seigniorage; heavy debasement (+30 %, +8 a season)
//! breaks even within a year. Undoing 30 points of inflation takes fifteen
//! seasons of strong money.

use data_model::{FactionId, GameData, SocialClass};
use serde::{Deserialize, Serialize};

use crate::buildings::EffectTotals;
use crate::events::{EventKind, GameEvent};
use crate::orders::Order;
use crate::state::CampaignState;

/// Price level of a sound currency (per cent of the 1337 prices).
pub const PRICE_BASE: u32 = 100;
/// Ceiling of the price level.
pub const PRICE_MAX: u32 = 400;
/// Inflation (price-level points above 100) per point of burgher/clergy
/// unrest target: +1 unrest per 6 points.
pub const INFLATION_UNREST_DIVISOR: f64 = 6.0;
/// Ceiling of the inflation unrest.
pub const INFLATION_UNREST_MAX: f64 = 25.0;
/// Inflation points per point of burgher/clergy wealth lost (fixed rents).
pub const INFLATION_WEALTH_DIVISOR: f64 = 8.0;
/// Ceiling of the inflation wealth loss.
pub const INFLATION_WEALTH_MAX: f64 = 20.0;

/// How much silver the faction's coins hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoinageLevel {
    /// Monnaie forte (Charles V, the franc of 1360).
    Strong,
    /// Monnaie saine (default).
    #[default]
    Sound,
    /// Monnaie affaiblie.
    Debased,
    /// Monnaie fortement affaiblie (Philippe VI and Jean II at war).
    HeavilyDebased,
}

/// Seasonal rule of one coinage level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoinageParams {
    /// Seigniorage, per cent of the season's tax income.
    pub seigniorage_percent: f64,
    /// Price-level points gained per season.
    pub inflation: u32,
    /// Price-level points lost per season (never below [`PRICE_BASE`]).
    pub deflation: u32,
    /// Recoinage cost, per cent of the season's tax income.
    pub recoinage_percent: f64,
    /// Burgher unrest target shift (negative: loyalty).
    pub burgher_unrest: f64,
    /// Ruler prestige per season.
    pub prestige: i32,
}

impl CoinageLevel {
    /// Every level, from the strongest to the weakest.
    pub const ALL: [CoinageLevel; 4] = [
        CoinageLevel::Strong,
        CoinageLevel::Sound,
        CoinageLevel::Debased,
        CoinageLevel::HeavilyDebased,
    ];

    /// The `snake_case` key used by orders and the bridge.
    pub fn key(self) -> &'static str {
        match self {
            CoinageLevel::Strong => "strong",
            CoinageLevel::Sound => "sound",
            CoinageLevel::Debased => "debased",
            CoinageLevel::HeavilyDebased => "heavily_debased",
        }
    }

    /// French label for the UI.
    pub fn label_fr(self) -> &'static str {
        match self {
            CoinageLevel::Strong => "Monnaie forte",
            CoinageLevel::Sound => "Monnaie saine",
            CoinageLevel::Debased => "Monnaie affaiblie",
            CoinageLevel::HeavilyDebased => "Monnaie fortement affaiblie",
        }
    }

    /// Parses a [`CoinageLevel::key`].
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|level| level.key() == key)
    }

    /// The seasonal rule of this level (see the module documentation).
    pub fn params(self) -> CoinageParams {
        match self {
            CoinageLevel::Strong => CoinageParams {
                seigniorage_percent: 0.0,
                inflation: 0,
                deflation: 2,
                recoinage_percent: 6.0,
                burgher_unrest: -6.0,
                prestige: 1,
            },
            CoinageLevel::Sound => CoinageParams {
                seigniorage_percent: 0.0,
                inflation: 0,
                deflation: 0,
                recoinage_percent: 0.0,
                burgher_unrest: 0.0,
                prestige: 0,
            },
            CoinageLevel::Debased => CoinageParams {
                seigniorage_percent: 15.0,
                inflation: 3,
                deflation: 0,
                recoinage_percent: 0.0,
                burgher_unrest: 4.0,
                prestige: 0,
            },
            CoinageLevel::HeavilyDebased => CoinageParams {
                seigniorage_percent: 30.0,
                inflation: 8,
                deflation: 0,
                recoinage_percent: 0.0,
                burgher_unrest: 8.0,
                prestige: -1,
            },
        }
    }
}

/// Why `set_coinage` was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CoinageError {
    #[error("la monnaie a déjà été changée cette année ({0})")]
    AlreadyChangedThisYear(i32),
    #[error("la monnaie est déjà à ce niveau")]
    Unchanged,
}

pub(crate) fn default_price_level() -> u32 {
    PRICE_BASE
}

/// `set_coinage` (skeleton).
pub fn set_coinage(
    _state: &mut CampaignState,
    _data: &GameData,
    _faction: &FactionId,
    _level: CoinageLevel,
) -> Result<(), CoinageError> {
    Ok(())
}
