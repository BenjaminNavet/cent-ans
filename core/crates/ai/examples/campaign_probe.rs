//! Official campaign measurement tool (ADR 0246): the strategic AI plays every
//! faction (France included, answering offers as the AI would) for N seeds x T
//! turns and prints the balance measures as text, or JSON with `--json`.
//!
//! `cargo run --release -p ai --example campaign_probe -- [--turns T] [--seeds 1,2,3]
//!   [--json] [--full] [--top K] [--verbose]`
//!
//! - `--full`: keep playing after the player's victory/defeat/end outcome
//!   (victory conditions disabled: the outcome is still recorded, but the
//!   measure covers the whole span). Without it a run stops at the outcome.
//! - `CENT_ANS_DATA_DIR` plays another copy of the data (variants of a rule).
//!
//! WH `diplob` measures: seasons under a league against a hegemon, leagues
//! formed, ultimatums sent to France, calls of allies to France.
//!
//! Measures per seed: outcome and turn, France-England war share of turns,
//! active wars (distinct pairs) and declarations per turn, factions
//! eliminated, revolts, bankruptcies, top-K income and treasury at the end,
//! provinces of the largest faction (end and peak), seconds per turn, and the
//! sieges (RX iaplay): begun, ended by capture, by peace (no war left between
//! besieger and holder) or otherwise (lifted, army lost), plus the share of
//! sieges that end in a capture.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Instant;

use data_model::{FactionId, GameData};
use serde_json::{json, Value};
use sim_campaign::{CampaignState, EventKind, Order};

fn fid(raw: &str) -> FactionId {
    FactionId::new(raw).expect("well-formed id")
}

/// Distinct (unordered) war pairs among living factions (ids as strings).
fn war_pairs(state: &CampaignState) -> BTreeSet<(String, String)> {
    let mut pairs: BTreeSet<(&FactionId, &FactionId)> = BTreeSet::new();
    for (a, fa) in &state.factions {
        if !fa.alive {
            continue;
        }
        for b in &fa.at_war_with {
            if state.factions.get(b).is_some_and(|f| f.alive) {
                pairs.insert(if a < b { (a, b) } else { (b, a) });
            }
        }
    }
    pairs
        .into_iter()
        .map(|(a, b)| (a.as_str().to_owned(), b.as_str().to_owned()))
        .collect()
}

/// Largest living faction (excluding rebels) and its province count.
fn biggest(state: &CampaignState) -> (String, usize) {
    state
        .factions
        .iter()
        .filter(|(f, s)| s.alive && f.as_str() != "fac_rebels")
        .map(|(f, _)| (f.as_str().to_owned(), state.owned_provinces(f).len()))
        .max_by_key(|(name, n)| (*n, std::cmp::Reverse(name.clone())))
        .unwrap_or_default()
}

/// Walled sieges in progress: settlement -> (besieger, holder, turns, armies).
/// Villages are left out: any hostile army standing on one "besieges" it,
/// which is a passage, not a siege.
fn sieges(
    state: &CampaignState,
) -> BTreeMap<String, (FactionId, FactionId, u32, Vec<sim_campaign::ArmyId>)> {
    state
        .settlements
        .iter()
        .filter(|(id, _)| state.settlement_kind(id) != data_model::SettlementKind::Village)
        .filter_map(|(id, s)| {
            s.siege.as_ref().map(|siege| {
                (
                    id.as_str().to_owned(),
                    (
                        siege.attacker.clone(),
                        s.controller.clone(),
                        siege.turns_elapsed,
                        state
                            .armies
                            .iter()
                            .filter(|(_, a)| a.faction == siege.attacker && a.is_at(id))
                            .map(|(aid, _)| aid.clone())
                            .collect(),
                    ),
                )
            })
        })
        .collect()
}

/// Province share of the largest faction and its military power over the
/// second power (what the league of WH `diplob` weighs).
fn dominance(state: &CampaignState) -> (f64, f64) {
    let (name, count) = biggest(state);
    let total = state
        .provinces
        .keys()
        .filter(|p| state.province_owner(p).is_some())
        .count();
    let power = |f: &FactionId| state.faction_power(f);
    let top = fid(&name);
    let second = state
        .factions
        .iter()
        .filter(|(f, s)| s.alive && **f != top && !f.is_rebels() && f.as_str() != "fac_papacy")
        .map(|(f, _)| power(f))
        .fold(0.0, f64::max);
    (
        count as f64 / total.max(1) as f64,
        power(&top) / second.max(1.0),
    )
}

fn run(data: &GameData, seed: u64, turns: u32, full: bool, top: usize) -> Value {
    let started = Instant::now();
    let france = fid("fac_france");
    let england = fid("fac_england");
    let mut state = CampaignState::new_1337(data, france.clone(), seed).expect("state");
    let factions_start = state.factions.values().filter(|f| f.alive).count();
    let (mut war_turns, mut played) = (0u32, 0u32);
    let (mut revolts, mut bankruptcies, mut declarations, mut eliminated) =
        (0u32, 0u32, 0u32, 0u32);
    let (mut wars_sum, mut wars_max) = (0usize, 0usize);
    let (mut league_turns, mut leagues, mut ultimatums, mut ally_calls) = (0u32, 0u32, 0u32, 0u32);
    let mut seen_offers: BTreeSet<u32> = BTreeSet::new();
    // WR `ai-diplo`: pacts running, province unrest and per-province taxes.
    let (mut pact_pair_turns, mut unrest_hot, mut unrest_max) = (0u64, 0u64, 0.0f64);
    let (mut override_turns, mut unrest_seen) = (0u64, 0u64);
    let mut peak = (String::new(), 0usize, 0u32);
    let mut series: Vec<Value> = Vec::new();
    // War lifetimes: start turn of the open pairs, end turn of the last war
    // of a pair, completed durations, redeclarations within 12 turns.
    let mut open: std::collections::BTreeMap<(String, String), u32> = Default::default();
    let mut ended: std::collections::BTreeMap<(String, String), u32> = Default::default();
    let (mut durations, mut starts, mut fast_redeclared) = (Vec::<u32>::new(), 0u32, 0u32);
    let mut live_sieges = sieges(&state);
    let (mut sieges_begun, mut sieges_captured, mut sieges_peace, mut sieges_other) =
        (0u32, 0u32, 0u32, 0u32);
    let mut sieges_walked_off = 0u32;
    let (mut sieges_relieved, mut sieges_abandoned, mut abandoned_turns) = (0u32, 0u32, 0u64);
    for _ in 0..turns {
        for offer in state.factions[&france].offers.clone() {
            let accept = sim_campaign::negotiation::evaluate_treaty(
                &state,
                data,
                &offer.from,
                &france,
                &offer.proposal.articles,
            )
            .accept;
            let _ = state.submit_order(
                data,
                Order::AnswerOffer {
                    offer: offer.id,
                    accept,
                },
            );
        }
        for order in ai::plan_turn(&state, data, &france) {
            let _ = state.submit_order(data, order);
        }
        let events = state.end_turn_with(data, ai::plan_turn);
        played += 1;
        let now = sieges(&state);
        for (id, (attacker, holder, _, _)) in &now {
            if live_sieges
                .get(id)
                .is_none_or(|(a, h, _, _)| a != attacker || h != holder)
            {
                sieges_begun += 1;
            }
        }
        for (id, (attacker, holder, elapsed, armies)) in &live_sieges {
            if now
                .get(id)
                .is_some_and(|(a, h, _, _)| a == attacker && h == holder)
            {
                continue;
            }
            let controller = state
                .settlements
                .get(&data_model::SettlementId::new(id).expect("id"))
                .map(|s| &s.controller);
            if controller == Some(attacker) {
                sieges_captured += 1;
            } else if !state.is_at_war(attacker, holder) {
                sieges_peace += 1;
            } else {
                sieges_other += 1;
                let sid = data_model::SettlementId::new(id).expect("id");
                if state.friendly_armies_at(holder, &sid).is_empty() {
                    if armies.iter().all(|a| state.armies.contains_key(a)) {
                        sieges_walked_off += 1;
                    }
                    sieges_abandoned += 1;
                    abandoned_turns += u64::from(*elapsed);
                } else {
                    sieges_relieved += 1;
                }
            }
        }
        live_sieges = now;
        if state.league.is_some() {
            league_turns += 1;
        }
        for (id, f) in &state.factions {
            pact_pair_turns += f
                .ledger
                .non_aggression
                .iter()
                .filter(|(other, until)| **until > state.turn && id < *other)
                .count() as u64;
        }
        for (id, p) in &state.provinces {
            if state.province_controller(id).is_none_or(|c| c.is_rebels()) {
                continue;
            }
            let unrest = sim_campaign::population::weighted_unrest(&p.population);
            unrest_seen += 1;
            unrest_max = unrest_max.max(unrest);
            if unrest > 60.0 {
                unrest_hot += 1;
            }
            if state.has_province_tax(id) {
                override_turns += 1;
            }
        }
        for offer in &state.factions[&france].offers {
            if !seen_offers.insert(offer.id) {
                continue;
            }
            if offer.proposal.is_ultimatum() {
                ultimatums += 1;
            }
            if offer.proposal.is_ally_call() {
                ally_calls += 1;
            }
        }
        for e in &events {
            if e.text_fr.contains("se liguent") {
                leagues += 1;
            }
            match e.kind {
                EventKind::Revolt if !e.text_fr.contains("passe aux mains") => revolts += 1,
                EventKind::Bankruptcy => bankruptcies += 1,
                EventKind::WarDeclared => declarations += 1,
                EventKind::FactionDestroyed => eliminated += 1,
                _ => {}
            }
        }
        if state.is_at_war(&france, &england) {
            war_turns += 1;
        }
        let now = war_pairs(&state);
        for pair in &now {
            if !open.contains_key(pair) {
                starts += 1;
                if ended
                    .get(pair)
                    .is_some_and(|end| state.turn.saturating_sub(*end) <= 12)
                {
                    fast_redeclared += 1;
                }
                open.insert(pair.clone(), state.turn);
            }
        }
        let closed: Vec<(String, String)> =
            open.keys().filter(|p| !now.contains(*p)).cloned().collect();
        for pair in closed {
            if let Some(start) = open.remove(&pair) {
                durations.push(state.turn.saturating_sub(start));
                ended.insert(pair, state.turn);
            }
        }
        let wars = now.len();
        wars_sum += wars;
        wars_max = wars_max.max(wars);
        let (name, n) = biggest(&state);
        if n > peak.1 {
            peak = (name, n, state.turn);
        }
        if state.turn.is_multiple_of(40) {
            let (share, ratio) = dominance(&state);
            series.push(json!({"turn": state.turn, "year": state.year, "wars": wars,
                "biggest": biggest(&state).0, "biggest_provinces": n,
                "biggest_share": share, "power_ratio": ratio}));
        }
        if state.outcome.is_some() && !full {
            break;
        }
    }
    let mut rows: Vec<(i64, i64, String, usize)> = state
        .factions
        .iter()
        .filter(|(f, s)| s.alive && f.as_str() != "fac_rebels")
        .map(|(f, s)| {
            (
                s.last_budget.income,
                s.treasury,
                f.as_str().to_owned(),
                state.owned_provinces(f).len(),
            )
        })
        .collect();
    rows.sort_by_key(|r| std::cmp::Reverse((state.owned_provinces(&fid(&r.2)).len(), r.0)));
    let top_rows: Vec<Value> = rows
        .iter()
        .take(top)
        .map(|(income, treasury, name, provinces)| {
            json!({"faction": name, "provinces": provinces, "income": income, "treasury": treasury})
        })
        .collect();
    let (big_name, big_n) = biggest(&state);
    let outcome = state.outcome.as_ref().map_or(
        Value::Null,
        |o| json!({"kind": format!("{:?}", o.kind), "turn": o.turn, "score": o.score}),
    );
    let denom = f64::from(played.max(1));
    let signed = |article: &str| -> usize {
        state
            .factions
            .values()
            .flat_map(|f| f.ledger.history.iter())
            .filter(|r| r.proposed && r.accepted && r.articles.iter().any(|a| a == article))
            .count()
    };
    let (pacts_signed, join_wars_signed) = (signed("non_aggression"), signed("join_war"));
    durations.sort_unstable();
    let median_war = durations.get(durations.len() / 2).copied().unwrap_or(0);
    json!({
        "war_median_turns": median_war,
        "wars_ended": durations.len(),
        "wars_started": starts,
        "fast_redeclared_share": f64::from(fast_redeclared) / f64::from(starts.max(1)),
        "seed": seed,
        "turns_played": played,
        "end_year": state.year,
        "outcome": outcome,
        "fr_en_war_share": f64::from(war_turns) / denom,
        "wars_active_mean": wars_sum as f64 / denom,
        "wars_active_max": wars_max,
        "wars_declared": declarations,
        "wars_declared_per_40_turns": f64::from(declarations) * 40.0 / denom,
        "factions_start": factions_start,
        "factions_end": state.factions.values().filter(|f| f.alive).count(),
        "eliminated": eliminated,
        "league_turns": league_turns,
        "leagues_formed": leagues,
        "ultimatums_to_player": ultimatums,
        "ally_calls_to_player": ally_calls,
        "revolts": revolts,
        "pact_pairs_mean": pact_pair_turns as f64 / denom,
        "pacts_signed": pacts_signed,
        "join_wars_signed": join_wars_signed,
        "province_unrest_max": unrest_max,
        "province_share_unrest_over_60": unrest_hot as f64 / (unrest_seen.max(1) as f64),
        "province_tax_overrides_mean": override_turns as f64 / denom,
        "bankruptcies": bankruptcies,
        "biggest_end": {"faction": big_name, "provinces": big_n},
        "biggest_peak": {"faction": peak.0, "provinces": peak.1, "turn": peak.2},
        "sieges_begun": sieges_begun,
        "sieges_captured": sieges_captured,
        "sieges_peace": sieges_peace,
        "sieges_other": sieges_other,
        "sieges_other_relieved": sieges_relieved,
        "sieges_other_abandoned": sieges_abandoned,
        "sieges_abandoned_army_alive": sieges_walked_off,
        "abandoned_mean_turns": abandoned_turns as f64 / f64::from(sieges_abandoned.max(1)),
        "siege_capture_share": f64::from(sieges_captured)
            / f64::from((sieges_captured + sieges_peace + sieges_other).max(1)),
        "top": top_rows,
        "series_every_40_turns": series,
        "seconds": started.elapsed().as_secs_f64(),
        "seconds_per_turn": started.elapsed().as_secs_f64() / denom,
    })
}

fn main() {
    let mut turns = 464u32;
    let mut seeds: Vec<u64> = vec![1, 2, 3];
    let (mut json_out, mut full, mut top) = (false, false, 5usize);
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--turns" => turns = args.next().and_then(|s| s.parse().ok()).expect("--turns N"),
            "--seeds" => {
                seeds = args
                    .next()
                    .expect("--seeds 1,2")
                    .split(',')
                    .filter_map(|s| s.parse().ok())
                    .collect();
            }
            "--top" => top = args.next().and_then(|s| s.parse().ok()).expect("--top K"),
            "--json" => json_out = true,
            "--full" => full = true,
            other => panic!("unknown argument {other}"),
        }
    }
    let root = std::env::var("CENT_ANS_DATA_DIR").map_or_else(
        |_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data"),
        std::path::PathBuf::from,
    );
    let (data, _) = GameData::load(&root).expect("data");
    let reports: Vec<Value> = std::thread::scope(|scope| {
        let handles: Vec<_> = seeds
            .iter()
            .map(|seed| {
                let data = &data;
                scope.spawn(move || run(data, *seed, turns, full, top))
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("campaign"))
            .collect()
    });
    if json_out {
        println!("{}", serde_json::to_string_pretty(&reports).expect("json"));
        return;
    }
    for r in &reports {
        println!(
            "seed {} | {} turns (to {}) | outcome {} | FR-EN war {:.0} % | wars active {:.1} (max {}), declared {} | eliminated {} | revolts {} | bankruptcies {} | war median {} t, fast-redeclare {:.0} % | biggest {} {} prov (peak {} t{}) | sieges {} (captured {}, peace {}, other {} [relieved {}, abandoned {} (army alive {}) after {:.1} turns]: {:.0} % taken) | {:.2} s/turn",
            r["seed"],
            r["turns_played"],
            r["end_year"],
            r["outcome"],
            100.0 * r["fr_en_war_share"].as_f64().unwrap_or(0.0),
            r["wars_active_mean"].as_f64().unwrap_or(0.0),
            r["wars_active_max"],
            r["wars_declared"],
            r["eliminated"],
            r["revolts"],
            r["bankruptcies"],
            r["war_median_turns"],
            100.0 * r["fast_redeclared_share"].as_f64().unwrap_or(0.0),
            r["biggest_end"]["faction"],
            r["biggest_end"]["provinces"],
            r["biggest_peak"]["provinces"],
            r["biggest_peak"]["turn"],
            r["sieges_begun"],
            r["sieges_captured"],
            r["sieges_peace"],
            r["sieges_other"],
            r["sieges_other_relieved"],
            r["sieges_other_abandoned"],
            r["sieges_abandoned_army_alive"],
            r["abandoned_mean_turns"].as_f64().unwrap_or(0.0),
            100.0 * r["siege_capture_share"].as_f64().unwrap_or(0.0),
            r["seconds_per_turn"].as_f64().unwrap_or(0.0),
        );
        for s in r["series_every_40_turns"].as_array().into_iter().flatten() {
            println!(
                "    t{:<4} wars {:>3} | {} {} prov = {:.1} % of the map, power x{:.2} the second",
                s["turn"],
                s["wars"],
                s["biggest"].as_str().unwrap_or("?"),
                s["biggest_provinces"],
                100.0 * s["biggest_share"].as_f64().unwrap_or(0.0),
                s["power_ratio"].as_f64().unwrap_or(0.0)
            );
        }
        println!(
            "    league: {} leagues, {} turns | ultimatums to France {} | ally calls to France {}",
            r["leagues_formed"],
            r["league_turns"],
            r["ultimatums_to_player"],
            r["ally_calls_to_player"]
        );
        for t in r["top"].as_array().into_iter().flatten() {
            println!(
                "    {:<18} {:>3} prov  income {:>8}  treasury {:>9}",
                t["faction"].as_str().unwrap_or("?"),
                t["provinces"],
                t["income"],
                t["treasury"]
            );
        }
    }
}
