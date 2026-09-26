//! PB3f (ADR 0091): a fingerprint of each turn (saved state and events) and
//! the time of the whole end of turn, to compare two builds of the core bit
//! for bit (before/after a performance change) and time them.
//!
//! Usage: `turn_digest [--sequential] [turns] [seed...]` (default 20
//! turns, seed 1); `--sequential` plans on the calling thread only.
//! France is the "player", driven by the same AI. Prints one line per turn
//! (`seed turn digest ms`), then the median end of turn.
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::time::Instant;

use data_model::{FactionId, GameData};
use sim_campaign::{CampaignState, Order};

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let sequential = args.first().is_some_and(|a| a == "--sequential");
    if sequential {
        args.remove(0);
    }
    let planner: fn(&CampaignState, &GameData, &FactionId) -> Vec<Order> = if sequential {
        ai::plan_turn_sequential
    } else {
        ai::plan_turn
    };
    let turns: u32 = args.first().and_then(|a| a.parse().ok()).unwrap_or(20);
    let mut seeds: Vec<u64> = args.iter().skip(1).filter_map(|a| a.parse().ok()).collect();
    if seeds.is_empty() {
        seeds.push(1);
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let data = GameData::load(&root).expect("game data loads").0;
    let _ = data.navgrid().component(0, 0);
    let france = FactionId::new("fac_france").expect("id");
    let mut times = Vec::new();
    for seed in &seeds {
        let mut state = CampaignState::new_1337(&data, france.clone(), *seed).expect("state");
        state.interactive_battles = false;
        for turn in 0..turns {
            for order in planner(&state, &data, &france) {
                let _ = state.submit_order(&data, order);
            }
            let started = Instant::now();
            let events = state.end_turn_with(&data, planner);
            let ms = started.elapsed().as_secs_f64() * 1000.0;
            times.push(ms);
            let mut hasher = DefaultHasher::new();
            state.save_json().hash(&mut hasher);
            format!("{events:?}").hash(&mut hasher);
            println!("{seed} {turn} {:016x} {ms:.1}", hasher.finish());
        }
    }
    times.sort_by(f64::total_cmp);
    println!("median end of turn {:.1} ms", times[times.len() / 2]);
}
