//! Faction currencies (lot HIST4, `data/economy/currencies.json`).
//!
//! The value of every amount is one common unit of account (the livre
//! tournois of ADR 0793); only the *name* shown to the player changes with
//! the faction (`Faction::currency`): hyperpère, dinar, florin, marc…
//!
//! Text built deep in the simulation (journal lines, refusals) has no player
//! at hand, so the client sets the *display currency* once per campaign
//! ([`set_display`]) and the text reads it through [`MONEY`] (plural name),
//! [`symbol`] and [`plural`]. The default is the livre, so the core and its
//! tests are unaffected unless a front end opts in.

use std::fmt;
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

/// One currency of the catalogue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CurrencyDef {
    pub id: String,
    pub singular: String,
    pub plural: String,
    /// Short abbreviation for tight spaces (« ₶ », « din. »).
    pub symbol: String,
}

impl CurrencyDef {
    /// The livre, used when no catalogue is loaded.
    pub fn livre() -> Self {
        Self {
            id: "livre".to_owned(),
            singular: "livre".to_owned(),
            plural: "livres".to_owned(),
            symbol: "₶".to_owned(),
        }
    }

    /// `singular` for one unit (or none), `plural` beyond.
    pub fn name_for(&self, count: i64) -> &str {
        if count.unsigned_abs() > 1 {
            &self.plural
        } else {
            &self.singular
        }
    }
}

/// Contents of `data/economy/currencies.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Currencies {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Currency of the factions without `currency`.
    pub default: String,
    pub currencies: Vec<CurrencyDef>,
}

impl Currencies {
    /// The currency `id`, if defined.
    pub fn get(&self, id: &str) -> Option<&CurrencyDef> {
        self.currencies.iter().find(|c| c.id == id)
    }

    /// Currency named by `id`, the catalogue default, or the livre.
    pub fn resolve(&self, id: Option<&str>) -> CurrencyDef {
        id.and_then(|id| self.get(id))
            .or_else(|| self.get(&self.default))
            .cloned()
            .unwrap_or_else(CurrencyDef::livre)
    }
}

static DISPLAY: RwLock<Option<CurrencyDef>> = RwLock::new(None);

/// Sets the currency shown by [`MONEY`], [`symbol`] and [`plural`] (the
/// player's faction); `None` restores the livre.
pub fn set_display(currency: Option<CurrencyDef>) {
    if let Ok(mut slot) = DISPLAY.write() {
        *slot = currency;
    }
}

/// The display currency (the livre unless [`set_display`] was called).
pub fn display() -> CurrencyDef {
    DISPLAY
        .read()
        .ok()
        .and_then(|slot| slot.clone())
        .unwrap_or_else(CurrencyDef::livre)
}

/// Plural name of the display currency.
pub fn plural() -> String {
    display().plural
}

/// Short symbol of the display currency.
pub fn symbol() -> String {
    display().symbol
}

/// `text` with its `{money}` placeholder named after the display currency (data texts such as
/// « Amasser 60 000 {money}. »).
pub fn fill(text: &str) -> String {
    if text.contains("{money}") {
        text.replace("{money}", &plural())
    } else {
        text.to_owned()
    }
}

/// Plural name of the display currency as a format argument: write
/// `"{amount} {MONEY}"` in a `format!` where `MONEY` is imported.
pub struct Money;

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&plural())
    }
}

/// See [`Money`].
pub static MONEY: Money = Money;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_known_default_and_unknown_ids() {
        let catalogue = Currencies {
            description: None,
            default: "livre".to_owned(),
            currencies: vec![
                CurrencyDef::livre(),
                CurrencyDef {
                    id: "dinar".to_owned(),
                    singular: "dinar".to_owned(),
                    plural: "dinars".to_owned(),
                    symbol: "din.".to_owned(),
                },
            ],
        };
        assert_eq!(catalogue.resolve(Some("dinar")).plural, "dinars");
        assert_eq!(catalogue.resolve(Some("nope")).plural, "livres");
        assert_eq!(catalogue.resolve(None).symbol, "₶");
        assert_eq!(catalogue.get("dinar").unwrap().name_for(1), "dinar");
        assert_eq!(catalogue.get("dinar").unwrap().name_for(2), "dinars");
    }
}
