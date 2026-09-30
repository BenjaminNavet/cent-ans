//! Lot M3: time each AI faction takes to play its turn on the real grid
//! (marches resumed, planning, orders executed), spec
//! `docs/design/2026-09-24-mouvement-libre.md` § 4: under 50 ms per faction
//! and turn in release.
//!
//! Each faction's turn is played `repeats` times on copies of the state and
//! the fastest run is kept (the others measure the machine's load, not the
//! AI), then once for real.
//!
//! Usage: `turn_perf [--sequential] [turns] [repeats] [seed...]` (default
//! 50 turns, 3 repeats, seed 1). France is the "player", driven by the same
//! AI. OMR R1: `--sequential` plans on the calling thread only and times
//! the thread's CPU (on Unix): a measure of the work that a loaded machine
//! barely disturbs, for before/after comparisons.
use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};

use data_model::{FactionId, GameData};
use sim_campaign::CampaignState;

/// CPU time of the calling thread (Unix), else the wall clock.
#[cfg(unix)]
fn clock() -> Duration {
    #[repr(C)]
    struct Timespec {
        tv_sec: i64,
        tv_nsec: i64,
    }
    extern "C" {
        fn clock_gettime(clock: i32, time: *mut Timespec) -> i32;
    }
    #[cfg(target_os = "macos")]
    const THREAD_CPU: i32 = 16;
    #[cfg(not(target_os = "macos"))]
    const THREAD_CPU: i32 = 3;
    let mut time = Timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `time` is a valid, writable timespec.
    unsafe { clock_gettime(THREAD_CPU, &mut time) };
    Duration::new(time.tv_sec as u64, time.tv_nsec as u32)
}

#[cfg(not(unix))]
fn clock() -> Duration {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    START.get_or_init(Instant::now).elapsed()
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let sequential = args.first().is_some_and(|a| a == "--sequential");
    if sequential {
        args.remove(0);
    }
    let planner: fn(&CampaignState, &GameData, &FactionId) -> Vec<sim_campaign::Order> =
        if sequential {
            ai::plan_turn_sequential
        } else {
            ai::plan_turn
        };
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
    // IA night: and the cover map, as the game does at load (bridge).
    let _ = data.cover_map();
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
                    if sequential {
                        let started = clock();
                        copy.play_ai_turn(&data, &faction, &planner, &mut scratch);
                        best = best.min(clock().saturating_sub(started));
                    } else {
                        let started = Instant::now();
                        copy.play_ai_turn(&data, &faction, &planner, &mut scratch);
                        best = best.min(started.elapsed());
                    }
                }
                state.play_ai_turn(&data, &faction, &planner, &mut events);
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
        "{}{} faction turns ({} turns x {} seed(s), best of {}): mean {:.2} ms, median {:.2} ms, p95 {:.2} ms, p99 {:.2} ms, max {:.2} ms",
        if sequential { "[sequential, thread CPU] " } else { "" },
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
    slowest.sort_by_key(|a| std::cmp::Reverse(a.1 .1));
    for (faction, (total, max, count)) in slowest.iter().take(8) {
        println!(
            "  {faction}: mean {:.2} ms, max {:.2} ms",
            ms(*total) / f64::from(*count),
            ms(*max)
        );
    }
}
