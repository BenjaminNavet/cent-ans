//! Lot M3: time each AI faction takes to play its turn on the real grid
//! (marches resumed, planning, orders executed), spec
//! `docs/design/2026-09-24-mouvement-libre.md` § 4: under 50 ms per faction
//! and turn in release.
//!
//! Usage: `turn_perf [turns] [seed...]` (default 30 turns, seed 1). France
//! is the "player", driven by the same AI before each `end_turn`.
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use data_model::{FactionId, GameData};
use sim_campaign::CampaignState;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let turns: u32 = args.first().and_then(|a| a.parse().ok()).unwrap_or(30);
    let mut seeds: Vec<u64> = args.iter().skip(1).filter_map(|a| a.parse().ok()).collect();
    if seeds.is_empty() {
        seeds.push(1);
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let data = GameData::load(&root).expect("game data loads").0;
    // Decode the grid once, outside the measures.
    let _ = data.navgrid();
    let france = FactionId::new("fac_france").expect("id");
    let mut all: Vec<Duration> = Vec::new();
    let mut by_faction: BTreeMap<FactionId, (Duration, Duration, u32)> = BTreeMap::new();
    for seed in &seeds {
        let mut state = CampaignState::new_1337(&data, france.clone(), *seed).expect("state");
        state.interactive_battles = false;
        for _ in 0..turns {
            for order in ai::plan_turn(&state, &data, &france) {
                let _ = state.submit_order(&data, order);
            }
            let (_, timings) = state.end_turn_profiled(&data, ai::plan_turn);
            for (faction, time) in timings {
                all.push(time);
                let entry = by_faction.entry(faction).or_default();
                entry.0 += time;
                entry.1 = entry.1.max(time);
                entry.2 += 1;
            }
        }
    }
    all.sort();
    let ms = |d: Duration| d.as_secs_f64() * 1000.0;
    let pick = |q: f64| all[((all.len() as f64 - 1.0) * q).round() as usize];
    println!(
        "{} faction turns ({} turns x {} seed(s)): mean {:.2} ms, median {:.2} ms, p95 {:.2} ms, max {:.2} ms",
        all.len(),
        turns,
        seeds.len(),
        ms(all.iter().sum::<Duration>()) / all.len().max(1) as f64,
        ms(pick(0.5)),
        ms(pick(0.95)),
        ms(pick(1.0)),
    );
    let mut slowest: Vec<_> = by_faction.into_iter().collect();
    slowest.sort_by(|a, b| b.1 .1.cmp(&a.1 .1));
    for (faction, (total, max, count)) in slowest.iter().take(8) {
        println!(
            "  {faction}: mean {:.2} ms, max {:.2} ms",
            ms(*total) / f64::from(*count),
            ms(*max)
        );
    }
}
