//! IA night (dev tool): A/B trials of campaign AI rules. A rule under trial
//! asks [`on`] whether the faction it plans plays it; the example
//! `ai_duel_probe` lets one faction play the trial rules while every other
//! keeps the current ones, and compares that faction's fate with the same
//! game played on the current rules only. Nobody plays a trial by default:
//! the game is not affected (ADR 0148).
//!
//! A trial rule reads, where it applies:
//! `if crate::experiment::on(ctx.faction, "my_rule") { new } else { current }`;
//! an adopted rule loses its gate.
use std::collections::BTreeSet;
use std::sync::RwLock;

use data_model::FactionId;

/// (factions, rules) playing a trial; `"*"` in either set means all.
type Trial = (BTreeSet<String>, BTreeSet<String>);

static TRIAL: RwLock<Option<Trial>> = RwLock::new(None);

/// `factions` play the trial `rules` from now on (`None`: nobody).
pub fn set(trial: Option<Trial>) {
    *TRIAL.write().unwrap_or_else(|p| p.into_inner()) = trial;
}

/// Does `faction` play trial rule `rule`?
pub fn on(faction: &FactionId, rule: &str) -> bool {
    TRIAL
        .read()
        .unwrap_or_else(|p| p.into_inner())
        .as_ref()
        .is_some_and(|(factions, rules)| {
            (factions.contains("*") || factions.contains(faction.as_str()))
                && (rules.contains("*") || rules.contains(rule))
        })
}
