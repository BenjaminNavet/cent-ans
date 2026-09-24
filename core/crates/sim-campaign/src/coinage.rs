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

/// `set_coinage`: strikes the faction's money at `level` from this season
/// on; at most one change per calendar year.
pub fn set_coinage(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    level: CoinageLevel,
) -> Result<(), CoinageError> {
    let year = state.year;
    let f = state.factions.get(faction).expect("checked by apply_order");
    if f.coinage == level {
        return Err(CoinageError::Unchanged);
    }
    if f.coinage_changed_year == Some(year) {
        return Err(CoinageError::AlreadyChangedThisYear(year));
    }
    let f = state.factions.get_mut(faction).expect("checked above");
    f.coinage = level;
    f.coinage_changed_year = Some(year);
    let text = match level {
        CoinageLevel::Strong => format!(
            "{} ordonne une monnaie forte : refonte des espèces, les prix baisseront lentement.",
            faction_label(data, faction)
        ),
        CoinageLevel::Sound => format!(
            "{} revient à une monnaie saine.",
            faction_label(data, faction)
        ),
        CoinageLevel::Debased | CoinageLevel::HeavilyDebased => format!(
            "{} mue la monnaie ({}) : le seigneuriage remplit le trésor, les prix montent.",
            faction_label(data, faction),
            level.label_fr().to_lowercase()
        ),
    };
    state
        .pending_events
        .push(GameEvent::new(EventKind::Coinage, text).faction(faction));
    Ok(())
}

fn faction_label(data: &GameData, faction: &FactionId) -> String {
    data.factions
        .get(faction)
        .map_or_else(|| faction.to_string(), |f| f.name.display.clone())
}

/// Cost multiplier of the faction's prices (`price_level / 100`).
pub fn price_factor(state: &CampaignState, faction: &FactionId) -> f64 {
    state
        .factions
        .get(faction)
        .map_or(1.0, |f| f64::from(f.price_level) / f64::from(PRICE_BASE))
}

/// `amount` livres at the faction's current prices (rounded).
pub fn priced(state: &CampaignState, faction: &FactionId, amount: i64) -> i64 {
    (amount as f64 * price_factor(state, faction)).round() as i64
}

/// Seigniorage `level` would yield this season: a share of the tax income.
pub fn seigniorage_for(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    level: CoinageLevel,
) -> i64 {
    let percent = level.params().seigniorage_percent;
    if percent <= 0.0 {
        return 0;
    }
    let income = state.faction_income_effective(data, faction).max(0);
    (income as f64 * percent / 100.0).round() as i64
}

/// Recoinage cost `level` would take this season (strong money only).
pub fn recoinage_for(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    level: CoinageLevel,
) -> i64 {
    let percent = level.params().recoinage_percent;
    if percent <= 0.0 {
        return 0;
    }
    let income = state.faction_income_effective(data, faction).max(0);
    (income as f64 * percent / 100.0).round() as i64
}

/// Seigniorage of the faction's current coinage this season.
pub fn seigniorage(state: &CampaignState, data: &GameData, faction: &FactionId) -> i64 {
    let level = state.factions.get(faction).map(|f| f.coinage).unwrap_or_default();
    seigniorage_for(state, data, faction, level)
}

/// Recoinage cost of the faction's current coinage this season.
pub fn recoinage(state: &CampaignState, data: &GameData, faction: &FactionId) -> i64 {
    let level = state.factions.get(faction).map(|f| f.coinage).unwrap_or_default();
    recoinage_for(state, data, faction, level)
}

/// Price level after one season at `level` from `price_level`.
pub fn next_price_level(price_level: u32, level: CoinageLevel) -> u32 {
    let params = level.params();
    (price_level + params.inflation)
        .saturating_sub(params.deflation)
        .clamp(PRICE_BASE, PRICE_MAX)
}

/// Phase after the economy: prices move with the coinage, the ruler gains
/// (strong) or loses (heavily debased) prestige; the player's journal
/// reports every 25-point threshold crossed.
pub(crate) fn resolve_coinage(state: &mut CampaignState, events: &mut Vec<GameEvent>) {
    let player = state.player_faction.clone();
    let ids: Vec<FactionId> = state
        .factions
        .iter()
        .filter(|(_, f)| f.alive)
        .map(|(id, _)| id.clone())
        .collect();
    for id in ids {
        let f = state.factions.get_mut(&id).expect("listed above");
        let before = f.price_level;
        let after = next_price_level(before, f.coinage);
        f.price_level = after;
        let prestige = f.coinage.params().prestige;
        let ruler = f.ruler.clone();
        if prestige != 0 {
            if let Some(r) = ruler.and_then(|r| state.characters.get_mut(&r)) {
                r.prestige += prestige;
            }
        }
        if id == player && before / 25 != after / 25 {
            let text = if after > before {
                format!("Les prix montent : ils atteignent {after} % de ceux de 1337. Bourgeois et clercs, qui vivent de rentes fixes, s'appauvrissent.")
            } else {
                format!("La monnaie forte fait baisser les prix : {after} % de ceux de 1337.")
            };
            events.push(GameEvent::new(EventKind::Coinage, text).faction(&id));
        }
    }
}

/// Coinage policy of the AI factions (both planners, like the diets):
/// in debt, debase (heavily when deep in debt); once out of debt, return
/// to sound money; with a comfortable surplus and high prices, strike
/// strong money until prices are back near 100. Deterministic.
pub fn ai_choose_coinage(state: &CampaignState, data: &GameData, faction: &FactionId) -> Vec<Order> {
    let Some(f) = state.factions.get(faction).filter(|f| f.alive) else {
        return Vec::new();
    };
    if f.coinage_changed_year == Some(state.year) {
        return Vec::new();
    }
    let income = state.faction_income_effective(data, faction).max(1);
    let wanted = if f.treasury < -4 * income && f.price_level < 200 {
        CoinageLevel::HeavilyDebased
    } else if f.treasury < 0 && f.price_level < 160 {
        CoinageLevel::Debased.max_weakness(f.coinage)
    } else if f.treasury >= 4 * income && f.price_level > 130 {
        CoinageLevel::Strong
    } else if f.coinage == CoinageLevel::Strong && f.price_level > 105 {
        CoinageLevel::Strong
    } else if f.treasury >= 0 {
        CoinageLevel::Sound
    } else {
        f.coinage
    };
    if wanted == f.coinage {
        Vec::new()
    } else {
        vec![Order::SetCoinage { level: wanted }]
    }
}

impl CoinageLevel {
    /// The weaker of `self` and `other` (an AI in debt never strengthens).
    fn max_weakness(self, other: CoinageLevel) -> CoinageLevel {
        if (other as u8) > (self as u8) {
            other
        } else {
            self
        }
    }
}

/// Population effects of the controller's money on `class` (target
/// shifts, merged like the H3 diets): inflation angers and impoverishes
/// burghers and clergy (fixed money rents); debasement itself angers the
/// burghers, a strong coinage wins them over.
pub fn class_effects(state: &CampaignState, faction: &FactionId, class: SocialClass) -> EffectTotals {
    let mut totals = EffectTotals::default();
    let Some(f) = state.factions.get(faction) else {
        return totals;
    };
    let excess = f64::from(f.price_level.saturating_sub(PRICE_BASE));
    if matches!(class, SocialClass::Burghers | SocialClass::Clergy) && excess > 0.0 {
        totals.unrest.flat += (excess / INFLATION_UNREST_DIVISOR).min(INFLATION_UNREST_MAX);
        totals.wealth.flat -= (excess / INFLATION_WEALTH_DIVISOR).min(INFLATION_WEALTH_MAX);
    }
    if class == SocialClass::Burghers {
        totals.unrest.flat += f.coinage.params().burgher_unrest;
    }
    totals
}
