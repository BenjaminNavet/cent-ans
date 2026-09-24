//! Lot M3: time each AI faction takes to play its turn on the real grid
//! (marches resumed, planning, orders executed), spec
//! `docs/design/2026-09-24-mouvement-libre.md` § 4: under 50 ms per faction
//! and turn in release.
//!
//! Each faction's turn is played `repeats` times on copies of the state and
//! the fastest run is kept (the others measure the machine's load, not the
//! AI), then once for real.
//!
//! Usage: `turn_perf [turns] [repeats] [seed...]` (default 50 turns, 3
//! repeats, seed 1). France is the "player", driven by the same AI.
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};

use data_model::{FactionId, GameData};
use sim_campaign::CampaignState;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let turns: u32 = args.first().and_then(|a| a.parse().ok()).unwrap_or(50);
    let repeats: u32 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(3).max(1);
    let mut seeds: Vec<u64> = args.iter().skip(2).filter_map(|a| a.parse().ok()).collect();
    if seeds.is_empty() {
        seeds.push(1);
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let data = GameData::load(&root).expect("game data loads").0;
    // Decode the grid and label its components once, outside the measures.
    let _ = data.navgrid().component(0, 0);
    let france = FactionId::new("fac_france").expect("id");
    let mut all: Vec<Duration> = Vec::new();
    let mut by_faction: BTreeMap<FactionId, (Duration, Duration, u32)> = BTreeMap::new();
    let mut worst: Option<(Duration, u64, u32, FactionId)> = None;
    for seed in &seeds {
        let mut state = CampaignState::new_1337(&data, france.clone(), *seed).expect("state");
        state.interactive_battles = false;
        for turn in 0..turns {
            for order in ai::plan_turn(&state, &data, &france) {
                let _ = state.submit_order(&data, order);
            }
            let mut events = Vec::new();
            let factions: Vec<FactionId> = state
                .factions
                .iter()
                .filter(|(id, f)| f.alive && **id != france)
                .map(|(id, _)| id.clone())
                .collect();
            for faction in factions {
                let mut best = Duration::MAX;
                for _ in 0..repeats {
                    let mut copy = state.clone();
                    let mut scratch = Vec::new();
                    let started = Instant::now();
                    copy.play_ai_turn(&data, &faction, &ai::plan_turn, &mut scratch);
                    best = best.min(started.elapsed());
                }
                state.play_ai_turn(&data, &faction, &ai::plan_turn, &mut events);
                all.push(best);
                if worst.as_ref().is_none_or(|w| best > w.0) {
                    worst = Some((best, *seed, turn, faction.clone()));
                }
                let entry = by_faction.entry(faction).or_default();
                entry.0 += best;
                entry.1 = entry.1.max(best);
                entry.2 += 1;
            }
            state.resolve_end_of_turn(&data, &mut events);
        }
    }
    all.sort();
    let ms = |d: Duration| d.as_secs_f64() * 1000.0;
    let pick = |q: f64| all[((all.len() as f64 - 1.0) * q).round() as usize];
    println!(
        "{} faction turns ({} turns x {} seed(s), best of {}): mean {:.2} ms, median {:.2} ms, p95 {:.2} ms, p99 {:.2} ms, max {:.2} ms",
        all.len(),
        turns,
        seeds.len(),
        repeats,
        ms(all.iter().sum::<Duration>()) / all.len().max(1) as f64,
        ms(pick(0.5)),
        ms(pick(0.95)),
        ms(pick(0.99)),
        ms(pick(1.0)),
    );
    if let Some((time, seed, turn, faction)) = worst {
        println!(
            "  slowest: {faction}, seed {seed}, turn {turn}: {:.2} ms",
            ms(time)
        );
    }
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
