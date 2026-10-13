//! Bankruptcy investigation: plays one AI-vs-AI game (the "player" is driven by
//! the strategic AI, as in `playthrough`) and writes per-event CSV rows plus a
//! per-faction summary on stdout.
//!
//! `cargo run --profile seed-bots -p ai --example bankruptcy_probe [seed] [turns] [faction] [events.csv]`

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::Write;

use data_model::{FactionId, GameData};
use sim_campaign::{CampaignState, EventKind};

#[derive(Default)]
struct Tally {
    events: u32,
    first: u32,
    last: u32,
    streak: u32,
    max_streak: u32,
    last_turn: u32,
    sum_income: i64,
    sum_army: i64,
    sum_build: i64,
    sum_admin: i64,
    sum_table: i64,
    sum_armies: u64,
    sum_prov: u64,
}

/// One planner call: faction, purse, all purses, order names.
type PlanCall = (FactionId, i64, BTreeMap<FactionId, i64>, Vec<String>);

fn f_id(t: &str) -> FactionId {
    FactionId::new(t).expect("id")
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let seed: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1337);
    let turns: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(150);
    let player = FactionId::new(args.get(3).map_or("fac_france", String::as_str)).expect("id");
    let csv_path = args
        .get(4)
        .cloned()
        .unwrap_or_else(|| "bankruptcy_events.csv".into());
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _) = GameData::load(&root).expect("data");
    let mut state = CampaignState::new_1337(&data, player.clone(), seed).expect("state");
    let mut csv = std::fs::File::create(&csv_path).expect("csv");
    writeln!(csv, "turn,faction,is_player,treasury_after,income,army_upkeep,building_upkeep,administration,table,trade,deficit_seasons,provinces,armies,units,unpaid_max").unwrap();
    let mut per_faction: BTreeMap<String, Tally> = BTreeMap::new();
    let mut per_turn: BTreeMap<u32, u32> = BTreeMap::new();
    let mut alive_factions = 0usize;
    // Treasury at the start of the turn (before the AI orders), per faction.
    let mut previous: BTreeMap<FactionId, i64> = BTreeMap::new();
    let trace: Option<String> = args.get(5).cloned();
    let mut onsets: Vec<String> = Vec::new();
    // Episodes of negative treasury (state-based, independent of the events).
    let mut open: BTreeMap<FactionId, (u32, i64)> = BTreeMap::new();
    let mut episode_len: Vec<u32> = Vec::new();
    let mut cause_count: BTreeMap<&'static str, u32> = BTreeMap::new();
    let mut negative_faction_turns = 0u32;
    let mut order_kinds: BTreeMap<String, u32> = BTreeMap::new();
    let mut order_spend_total: BTreeMap<&'static str, i64> = BTreeMap::new();
    let mut onset_kinds: BTreeMap<String, u32> = BTreeMap::new();
    let mut onsets_kinds_line: Vec<String> = Vec::new();
    for _ in 0..turns {
        for (id, f) in &state.factions {
            previous.insert(id.clone(), f.treasury);
        }
        for order in ai::plan_turn(&state, &data, &player) {
            let _ = state.submit_order(&data, order);
        }
        // Spending of the AI orders: purse of the faction when it plans, and
        // when the next faction plans (its orders are applied by then).
        let calls: RefCell<Vec<PlanCall>> = RefCell::new(Vec::new());
        let events = state.end_turn_with(&data, |st, d, f| {
            let orders = ai::plan_turn(st, d, f);
            let names: Vec<String> = orders
                .iter()
                .map(|o| {
                    let t = format!("{o:?}");
                    t.split(|c: char| !c.is_alphanumeric())
                        .next()
                        .unwrap_or("")
                        .to_string()
                })
                .collect();
            let purses = st
                .factions
                .iter()
                .map(|(i, x)| (i.clone(), x.treasury))
                .collect();
            calls
                .borrow_mut()
                .push((f.clone(), st.factions[f].treasury, purses, names));
            orders
        });
        let mut order_spend: BTreeMap<FactionId, (i64, BTreeMap<String, u32>)> = BTreeMap::new();
        {
            let calls = calls.borrow();
            for w in calls.windows(2) {
                let (f, t0, _, names) = &w[0];
                let after = w[1].2.get(f).copied().unwrap_or(*t0);
                let mut m = BTreeMap::new();
                for n in names {
                    *m.entry(n.clone()).or_insert(0u32) += 1;
                }
                order_spend.insert(f.clone(), (t0 - after, m));
            }
        }
        let turn = state.turn();
        if let Some(t) = &trace {
            if let Some(f) = state.factions.get(&FactionId::new(t.as_str()).expect("id")) {
                println!(
                    "trace t{turn} {t} treasury={} alive={} income={} army={} build={} admin={}",
                    f.treasury,
                    f.alive,
                    f.last_budget.income,
                    f.last_budget.army_upkeep,
                    f.last_budget.building_upkeep,
                    f.last_budget.administration
                );
                for e in events
                    .iter()
                    .filter(|e| e.faction.as_ref() == Some(&f_id(t)))
                {
                    println!("   event {:?}: {}", e.kind, e.text_fr);
                }
            }
        }
        alive_factions = state.factions.values().filter(|f| f.alive).count();
        for (id, f) in &state.factions {
            if !f.alive || id.is_rebels() {
                continue;
            }
            let before = previous.get(id).copied().unwrap_or(0);
            if f.treasury < 0 {
                negative_faction_turns += 1;
                if let Some(ep) = open.get(id) {
                    let _ = ep;
                } else {
                    let b = &f.last_budget;
                    let net = b.income + b.trade_income
                        - (b.army_upkeep + b.building_upkeep + b.administration + b.table);
                    let flow = f.treasury - before - net;
                    let texts: Vec<&str> = events
                        .iter()
                        .filter(|x| x.faction.as_ref() == Some(id))
                        .map(|x| x.text_fr.as_str())
                        .collect();
                    let cause = if texts
                        .iter()
                        .any(|t| t.contains("verse") || t.contains("Tribut"))
                    {
                        "treaty payment"
                    } else if texts
                        .iter()
                        .any(|t| t.contains("rançon") || t.contains("rachète"))
                    {
                        "ransom"
                    } else if before >= 0 && net < 0 && flow >= -net.abs() / 4 {
                        "structural deficit"
                    } else if before < 0 {
                        "already negative"
                    } else {
                        "other/spending"
                    };
                    *cause_count.entry(cause).or_default() += 1;
                    let mut kinds: Vec<String> = events
                        .iter()
                        .filter(|x| x.faction.as_ref() == Some(id))
                        .map(|x| format!("{:?}", x.kind))
                        .filter(|k| k != "Income" && k != "Bankruptcy")
                        .collect();
                    kinds.sort();
                    kinds.dedup();
                    if kinds.is_empty() {
                        kinds.push("(none)".into());
                    }
                    for k in &kinds {
                        *onset_kinds.entry(k.clone()).or_default() += 1;
                    }
                    let (os, om) = order_spend.get(id).cloned().unwrap_or_default();
                    for (k, n) in &om {
                        if matches!(
                            k.as_str(),
                            "Recruit" | "Build" | "Construct" | "Hire" | "Hired"
                        ) || os > 0
                        {
                            *order_kinds.entry(k.clone()).or_default() += n;
                        }
                    }
                    *order_spend_total.entry(cause).or_default() += os.max(0);
                    onsets_kinds_line.push(format!(
                        "t{turn} {id} before={before} order_spend={os} orders={om:?} total_other={}",
                        -flow
                    ));
                    onsets.push(format!(
                        "t{turn} {id} before={before} net={net} now={} other_flow={flow} cause={cause} provs={}",
                        f.treasury,
                        state.owned_provinces(id).len()
                    ));
                    open.insert(id.clone(), (turn, before));
                }
            } else if let Some((start, _)) = open.remove(id) {
                episode_len.push(turn - start);
            }
        }
        for e in events.iter().filter(|e| e.kind == EventKind::Bankruptcy) {
            let Some(id) = &e.faction else { continue };
            let f = &state.factions[id];
            let b = &f.last_budget;
            let provinces = state.owned_provinces(id).len();
            let mine: Vec<_> = state.armies.values().filter(|a| &a.faction == id).collect();
            let units: usize = mine.iter().map(|a| a.units.len()).sum();
            let unpaid = mine.iter().map(|a| a.unpaid_seasons).max().unwrap_or(0);
            writeln!(
                csv,
                "{turn},{id},{},{},{},{},{},{},{},{},{},{provinces},{},{units},{unpaid}",
                *id == player,
                f.treasury,
                b.income,
                b.army_upkeep,
                b.building_upkeep,
                b.administration,
                b.table,
                b.trade_income,
                f.deficit_seasons,
                mine.len()
            )
            .unwrap();
            *per_turn.entry(turn).or_default() += 1;
            let t = per_faction.entry(id.to_string()).or_default();
            if t.events == 0 {
                t.first = turn;
            }
            t.streak = if t.last_turn + 1 == turn {
                t.streak + 1
            } else {
                1
            };
            t.max_streak = t.max_streak.max(t.streak);
            t.last_turn = turn;
            t.last = turn;
            t.events += 1;
            t.sum_income += b.income;
            t.sum_army += b.army_upkeep;
            t.sum_build += b.building_upkeep;
            t.sum_admin += b.administration;
            t.sum_table += b.table;
            t.sum_armies += mine.len() as u64;
            t.sum_prov += provinces as u64;
        }
        if state.outcome.is_some() {
            break;
        }
    }
    let total: u32 = per_faction.values().map(|t| t.events).sum();
    println!("seed {seed} turns {} alive {alive_factions} total bankruptcy events {total} distinct factions {}", state.turn(), per_faction.len());
    println!("faction,events,first,last,max_streak,avg_prov,avg_armies,avg_income,avg_army,avg_build,avg_admin,avg_table");
    let mut rows: Vec<_> = per_faction.iter().collect();
    rows.sort_by_key(|(_, t)| std::cmp::Reverse(t.events));
    for (id, t) in rows {
        let n = i64::from(t.events);
        println!(
            "{id},{},{},{},{},{:.1},{:.1},{},{},{},{},{}",
            t.events,
            t.first,
            t.last,
            t.max_streak,
            t.sum_prov as f64 / f64::from(t.events),
            t.sum_armies as f64 / f64::from(t.events),
            t.sum_income / n,
            t.sum_army / n,
            t.sum_build / n,
            t.sum_admin / n,
            t.sum_table / n
        );
    }
    for (start, _) in open.values() {
        episode_len.push(state.turn() - start);
    }
    episode_len.sort_unstable();
    println!(
        "episodes >=3 turns: {}, >=6 turns: {}",
        episode_len.iter().filter(|l| **l >= 3).count(),
        episode_len.iter().filter(|l| **l >= 6).count()
    );
    println!(
        "negative faction-turns {negative_faction_turns}, episodes {}, median length {}, max {}, causes {cause_count:?}",
        episode_len.len(),
        episode_len.get(episode_len.len() / 2).copied().unwrap_or(0),
        episode_len.last().copied().unwrap_or(0)
    );
    println!("event kinds present at onsets: {onset_kinds:?}");
    println!("order kinds at onsets (spending turns): {order_kinds:?} spend {order_spend_total:?}");
    for l in &onsets_kinds_line {
        println!("K {l}");
    }
    println!("onsets (start of a negative-treasury episode):");
    for line in &onsets {
        println!("{line}");
    }
    println!("per-turn:");
    for (turn, n) in per_turn {
        print!("{turn}:{n} ");
    }
    println!();
}
