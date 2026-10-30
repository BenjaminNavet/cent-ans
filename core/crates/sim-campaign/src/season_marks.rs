//! XP-POURQUOI (R4): « the cause shown when the effect shows ».
//!
//! At the start of each turn resolution [`Baseline::capture`] notes what the player saw (the
//! figures of the top bar, the unrest of each of his provinces, what each AI faction thinks of
//! him). At its end [`record`] compares: a change of the treasury larger than
//! `treasury_pct`, of a province's unrest larger than `unrest_points`, or an AI faction moving
//! to another tier of attitude, becomes a [`Cause`] of the journal (kinds `TreasurySwing`,
//! `UnrestSwing`, `AttitudeShift`), its factors being the heaviest terms of the change. Only
//! the player's realm is watched, at most `max_per_season_player` causes a season (the heaviest
//! first), so the journal fills without drowning the real events. Thresholds are data
//! (`data/rules/cause_thresholds.json`).
//!
//! The marks of the player's figures stay in the state ([`SeasonMarks`]) so that the top bar
//! can show the change « since last season » without keeping a history of its own. Nothing
//! here changes a rule or uses randomness.

use std::collections::BTreeMap;

use data_model::{FactionId, GameData, ProvinceId};
use serde::{Deserialize, Serialize};

use crate::cause_builders;
use crate::causes::Cause;
use crate::state::CampaignState;

/// The player's top-bar figures as they stood at the start of the turn just resolved: what the
/// bar showed during that season, to compare with what it shows now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeasonMarks {
    /// Turn the marks were taken at (the one that was then resolved).
    pub turn: u32,
    pub treasury: i64,
    /// Projected net balance of the season (receipts minus all charges).
    pub net_income: i64,
    pub prestige: i32,
    /// Men under arms.
    pub men: u64,
    /// Population-weighted mean unrest, 0-100.
    pub unrest: u8,
    pub provinces: u32,
}

/// What was true when the player ended his turn.
pub(crate) struct Baseline {
    marks: Option<SeasonMarks>,
    purses: BTreeMap<FactionId, i64>,
    unrest: BTreeMap<ProvinceId, f64>,
    attitudes: BTreeMap<FactionId, i32>,
}

impl Baseline {
    pub(crate) fn capture(
        state: &CampaignState,
        data: &GameData,
        purses: &BTreeMap<FactionId, i64>,
    ) -> Self {
        let player = &state.player_faction;
        let marks = state.faction_summary(player).map(|summary| SeasonMarks {
            turn: state.turn,
            treasury: summary.treasury,
            net_income: state
                .faction_economy(data, player)
                .map_or(0, |economy| economy.net_income()),
            prestige: summary.prestige,
            men: summary.soldiers,
            unrest: summary.mean_unrest,
            provinces: u32::try_from(summary.provinces_count).unwrap_or(u32::MAX),
        });
        let unrest = state
            .provinces
            .iter()
            .filter(|(id, _)| state.controls_province(player, id))
            .map(|(id, province)| {
                (
                    id.clone(),
                    crate::population::weighted_unrest(&province.population),
                )
            })
            .collect();
        Baseline {
            marks,
            purses: purses.clone(),
            unrest,
            attitudes: attitudes_towards(state, data, player)
                .into_iter()
                .map(|(id, (value, _))| (id, value))
                .collect(),
        }
    }
}

/// Attitude of every other living faction towards `player`, with its reasons.
fn attitudes_towards(
    state: &CampaignState,
    data: &GameData,
    player: &FactionId,
) -> BTreeMap<FactionId, (i32, Vec<(String, i32)>)> {
    let cache = crate::plan_cache::PlanCache::new(state);
    state
        .factions
        .iter()
        .filter(|(id, faction)| faction.alive && *id != player && !id.is_rebels())
        .map(|(id, _)| (id.clone(), cache.attitude(data, id, player)))
        .collect()
}

/// Signed livres of every budget line of a season (they sum to the change of the treasury).
pub fn budget_terms(record: &crate::economy_balance::BudgetRecord) -> Vec<(&'static str, i64)> {
    let mut terms = vec![
        ("receipts", record.receipts),
        ("armies", -record.armies),
        ("buildings", -record.buildings),
        ("administration", -record.administration),
        ("table", -record.table),
        ("household", -record.household),
        ("wages", -record.wages),
        ("siege", -record.siege),
        ("obligations", -record.obligations),
        ("interest", -record.interest),
        ("levies", record.levies),
        ("loans", record.borrowed - record.repaid),
        ("other", record.other),
    ];
    terms.retain(|(_, amount)| *amount != 0);
    terms
}

/// Key of the term that gathers what the named terms do not explain.
pub const OTHERS_KEY: &str = "others";

/// One signed term under a figure of the top bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Term {
    /// Stable key (`armies`, `receipts`, `table`, `tax`, ..., [`OTHERS_KEY`]).
    pub key: String,
    /// French label when the core knows it (building names); empty: the UI maps `key`.
    pub label: String,
    pub amount: i64,
}

/// The top-bar figures that have a trend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Figure {
    Treasury,
    NetIncome,
    Men,
    Prestige,
    Unrest,
}

impl Figure {
    pub const ALL: [Figure; 5] = [
        Figure::Treasury,
        Figure::NetIncome,
        Figure::Men,
        Figure::Prestige,
        Figure::Unrest,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Figure::Treasury => "treasury",
            Figure::NetIncome => "net_income",
            Figure::Men => "men",
            Figure::Prestige => "prestige",
            Figure::Unrest => "unrest",
        }
    }
}

/// A figure of the top bar, its change since last season and the terms behind it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trend {
    pub figure: Figure,
    pub current: i64,
    /// What the bar showed during the season just resolved (`None` before the first turn).
    pub previous: Option<i64>,
    /// At most `top_terms` terms, heaviest first; they sum to `delta()` when `last_season`.
    pub terms: Vec<Term>,
    /// The terms explain the change since last season (`true`) or weigh on the coming one.
    pub last_season: bool,
}

impl Trend {
    pub fn delta(&self) -> Option<i64> {
        self.previous.map(|previous| self.current - previous)
    }
}

/// Keeps the `max` heaviest terms of a change of `total`: the last of the `max` slots is
/// [`OTHERS_KEY`] when the named terms do not add up to `total`, so the result always does.
pub fn top_terms(mut terms: Vec<Term>, total: i64, max: usize) -> Vec<Term> {
    terms.retain(|term| term.amount != 0);
    terms.sort_by(|a, b| {
        b.amount
            .abs()
            .cmp(&a.amount.abs())
            .then_with(|| a.key.cmp(&b.key))
    });
    let max = max.max(1);
    let sum_all: i64 = terms.iter().map(|t| t.amount).sum();
    if terms.len() <= max && sum_all == total {
        return terms;
    }
    terms.truncate(max - 1);
    let rest = total - terms.iter().map(|t| t.amount).sum::<i64>();
    if rest != 0 {
        terms.push(Term {
            key: OTHERS_KEY.to_owned(),
            label: String::new(),
            amount: rest,
        });
    }
    terms
}

/// The trends of the top-bar figures of `faction` since the season just resolved. Only the
/// player's figures are marked: for any other faction every `previous` is `None`.
pub fn trends(state: &CampaignState, data: &GameData, faction: &FactionId) -> Vec<Trend> {
    let max = data.cause_thresholds.top_terms as usize;
    let marks = state
        .season_marks
        .as_ref()
        .filter(|_| faction == &state.player_faction);
    let Some(summary) = state.faction_summary(faction) else {
        return Vec::new();
    };
    let economy = state.faction_economy(data, faction);
    let net_now = economy.as_ref().map_or(0, |e| e.net_income());
    let record = state
        .last_budget(faction)
        .filter(|r| marks.is_some_and(|m| m.turn == r.turn));
    Figure::ALL
        .into_iter()
        .map(|figure| {
            let (current, previous) = match figure {
                Figure::Treasury => (summary.treasury, marks.map(|m| m.treasury)),
                Figure::NetIncome => (net_now, marks.map(|m| m.net_income)),
                Figure::Men => (
                    i64::try_from(summary.soldiers).unwrap_or(i64::MAX),
                    marks.map(|m| i64::try_from(m.men).unwrap_or(i64::MAX)),
                ),
                Figure::Prestige => (
                    i64::from(summary.prestige),
                    marks.map(|m| i64::from(m.prestige)),
                ),
                Figure::Unrest => (
                    i64::from(summary.mean_unrest),
                    marks.map(|m| i64::from(m.unrest)),
                ),
            };
            let delta = previous.map(|p| current - p);
            let (terms, last_season) = match figure {
                Figure::Treasury => match (record, delta) {
                    (Some(record), Some(delta)) => (
                        top_terms(
                            budget_terms(record)
                                .into_iter()
                                .map(|(key, amount)| Term {
                                    key: key.to_owned(),
                                    label: String::new(),
                                    amount,
                                })
                                .collect(),
                            delta,
                            max,
                        ),
                        true,
                    ),
                    _ => (Vec::new(), true),
                },
                Figure::NetIncome => match (&economy, record, delta) {
                    (Some(economy), Some(record), Some(delta)) => {
                        let lines = economy
                            .budget_lines(Some(record))
                            .into_iter()
                            .filter(|line| {
                                !matches!(
                                    line.kind,
                                    crate::economy_balance::BudgetLineKind::Levies
                                        | crate::economy_balance::BudgetLineKind::Loans
                                        | crate::economy_balance::BudgetLineKind::Other
                                )
                            })
                            .filter_map(|line| {
                                line.delta().map(|amount| Term {
                                    key: line.kind.key().to_owned(),
                                    label: String::new(),
                                    amount,
                                })
                            })
                            .collect();
                        (top_terms(lines, delta, max), true)
                    }
                    _ => (Vec::new(), true),
                },
                Figure::Prestige => {
                    let terms = crate::forecast::prestige_contributions(state, data, faction)
                        .into_iter()
                        .map(|c| Term {
                            key: c.key,
                            label: c.label,
                            amount: c.amount.round() as i64,
                        })
                        .collect::<Vec<_>>();
                    let total = terms.iter().map(|t| t.amount).sum();
                    (top_terms(terms, total, max), false)
                }
                Figure::Unrest => {
                    let terms = crate::population::unrest_pull(state, data, faction)
                        .into_iter()
                        .map(|(key, amount)| Term {
                            key: key.to_owned(),
                            label: String::new(),
                            amount: amount.round() as i64,
                        })
                        .collect::<Vec<_>>();
                    let total = terms.iter().map(|t| t.amount).sum();
                    (top_terms(terms, total, max), false)
                }
                // The men under arms: the phases that move them are only measured by the
                // forecast (a copy of the state each); the bar's tooltip keeps using it.
                Figure::Men => (Vec::new(), false),
            };
            Trend {
                figure,
                current,
                previous,
                terms,
                last_season,
            }
        })
        .collect()
}

/// Compares the end of the turn with `baseline`; the causes found enter the journal.
pub(crate) fn record(state: &mut CampaignState, data: &GameData, baseline: Baseline) {
    let thresholds = data.cause_thresholds.clone();
    let player = state.player_faction.clone();
    state.season_marks = baseline.marks.clone();
    let mut found: Vec<Cause> = Vec::new();

    // The treasury (the booked season carries the terms).
    if let Some(&before) = baseline.purses.get(&player) {
        if let Some(faction) = state.factions.get(&player) {
            let after = faction.treasury;
            let change = (after - before).abs();
            let limit = (thresholds.treasury_pct * before.abs() as f64).max(0.0);
            if change as f64 > limit && change >= thresholds.treasury_min {
                if let Some(record) = faction.budget_history.last() {
                    found.push(cause_builders::treasury_swing_cause(
                        &player,
                        before,
                        after,
                        &budget_terms(record),
                    ));
                }
            }
        }
    }

    // The unrest of each province the player still holds.
    for (id, before) in &baseline.unrest {
        let Some(province) = state.provinces.get(id) else {
            continue;
        };
        if !state.controls_province(&player, id) {
            continue;
        }
        let after = crate::population::weighted_unrest(&province.population);
        if (after - before).abs() >= thresholds.unrest_points {
            let terms = crate::population::province_unrest_terms(state, data, id);
            found.push(cause_builders::unrest_swing_cause(
                &player, id, *before, after, &terms,
            ));
        }
    }

    // The AI factions that changed tier of attitude towards the player.
    for (id, (after, reasons)) in attitudes_towards(state, data, &player) {
        let Some(&before) = baseline.attitudes.get(&id) else {
            continue;
        };
        let (tier_before, tier_after) = (
            thresholds.attitude_tier(before),
            thresholds.attitude_tier(after),
        );
        if tier_before == tier_after {
            continue;
        }
        let mut reasons = reasons;
        reasons.retain(|(_, value)| *value != 0);
        found.push(cause_builders::attitude_shift_cause(
            &id,
            &player,
            (before, after),
            tier_after,
            &reasons,
        ));
    }

    // The heaviest first; the order of discovery (treasury, provinces, factions: all sorted)
    // breaks ties, so the choice is deterministic.
    found.sort_by_key(|cause| std::cmp::Reverse(cause.severity));
    found.truncate(thresholds.max_per_season_player as usize);
    for cause in found {
        state.note_cause(cause);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::economy_balance::BudgetRecord;

    use crate::causes::CauseKind;
    use crate::test_support::{idle, start_quiet};

    fn swings(state: &CampaignState, kind: CauseKind) -> Vec<&Cause> {
        state
            .cause_journal()
            .records()
            .map(|r| &r.cause)
            .filter(|c| c.kind == kind)
            .collect()
    }

    #[test]
    fn a_doubled_payroll_makes_a_treasury_swing_led_by_the_armies() {
        let data = data_model::test_support::game_data();
        let mut state = start_quiet(data, "fac_france", 1337);
        let player = state.player_faction.clone();
        state.factions.get_mut(&player).unwrap().treasury = 20_000;
        // Four times the men under arms: the pay swallows the receipts.
        for army in state.armies.values_mut().filter(|a| a.faction == player) {
            let units = army.units.clone();
            for _ in 0..3 {
                army.units.extend(units.clone());
            }
        }
        state.end_turn_with(data, idle);
        let found = swings(&state, CauseKind::TreasurySwing);
        assert_eq!(found.len(), 1, "one treasury swing expected");
        assert_eq!(found[0].subject.as_ref(), Some(&player));
        assert_eq!(found[0].factors[0].key, "treasury_armies");
        assert!(found[0].factors[0].value < 0);
    }

    #[test]
    fn a_calm_season_has_no_treasury_swing_and_marks_are_kept() {
        let data = data_model::test_support::game_data();
        let mut state = start_quiet(data, "fac_england", 1337);
        let player = state.player_faction.clone();
        state.factions.get_mut(&player).unwrap().treasury = 1_000_000;
        state.end_turn_with(data, idle);
        assert!(swings(&state, CauseKind::TreasurySwing).is_empty());
        let marks = state.season_marks.as_ref().expect("marks after a turn");
        assert_eq!(marks.turn, 0);
        assert_eq!(marks.treasury, 1_000_000);
    }

    #[test]
    fn swing_causes_of_the_player_are_capped_per_season() {
        let data = data_model::test_support::game_data();
        let mut state = start_quiet(data, "fac_france", 1337);
        for _ in 0..12 {
            let before = state.cause_journal().len();
            state.end_turn_with(data, idle);
            let added = state
                .cause_journal()
                .records()
                .skip(before)
                .filter(|r| {
                    matches!(
                        r.cause.kind,
                        CauseKind::TreasurySwing | CauseKind::UnrestSwing | CauseKind::AttitudeShift
                    )
                })
                .count();
            assert!(added <= data.cause_thresholds.max_per_season_player as usize);
        }
    }

    fn term(key: &str, amount: i64) -> Term {
        Term {
            key: key.to_owned(),
            label: String::new(),
            amount,
        }
    }

    #[test]
    fn top_terms_keep_at_most_max_and_always_sum_to_the_total() {
        let terms = vec![term("a", -50), term("b", 30), term("c", -20), term("d", 5)];
        let top = top_terms(terms, -35, 3);
        assert_eq!(top.len(), 3);
        assert_eq!(top[0].key, "a");
        assert_eq!(top[2].key, OTHERS_KEY);
        assert_eq!(top.iter().map(|t| t.amount).sum::<i64>(), -35);
        // Few terms that already add up: no « others ».
        let top = top_terms(vec![term("a", 10), term("b", -4)], 6, 3);
        assert_eq!(top.len(), 2);
        // A shortfall is gathered, even with a single term.
        let top = top_terms(vec![term("a", 10)], 25, 3);
        assert_eq!(top.last().unwrap().amount, 15);
    }

    #[test]
    fn treasury_trend_terms_explain_the_change_since_last_season() {
        let data = data_model::test_support::game_data();
        let mut state = start_quiet(data, "fac_england", 1337);
        let player = state.player_faction.clone();
        let before = trends(&state, data, &player);
        assert!(before.iter().all(|t| t.previous.is_none() && t.delta().is_none()));
        state.end_turn_with(data, idle);
        let after = trends(&state, data, &player);
        for trend in &after {
            assert!(trend.terms.len() <= data.cause_thresholds.top_terms as usize);
        }
        let treasury = after.iter().find(|t| t.figure == Figure::Treasury).unwrap();
        let delta = treasury.delta().expect("marked after a turn");
        assert_eq!(treasury.terms.iter().map(|t| t.amount).sum::<i64>(), delta);
        assert!(!treasury.terms.is_empty());
    }

    #[test]
    fn budget_terms_sum_to_the_change_of_the_treasury() {
        let record = BudgetRecord {
            receipts: 5000,
            armies: 3000,
            buildings: 400,
            administration: 200,
            table: 100,
            household: 50,
            levies: 700,
            borrowed: 1000,
            repaid: 300,
            other: -250,
            ..Default::default()
        };
        let terms = budget_terms(&record);
        let expected = record.change() + record.levies + record.borrowed - record.repaid;
        assert_eq!(terms.iter().map(|(_, v)| v).sum::<i64>(), expected);
    }
}
