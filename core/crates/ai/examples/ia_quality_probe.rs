//! Campaign AI quality probe: how well the AI plays (not balance), AI
//! against AI from 1337. Skeleton: the metrics are filled in next.
//!
//! Usage: `ia_quality_probe [turns] [seed...]` (default 60 turns, seeds
//! 1-4). `VERBOSE=1` lists examples.

fn main() {
    let mut args = std::env::args().skip(1);
    let turns: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(60);
    let mut seeds: Vec<u64> = args.filter_map(|s| s.parse().ok()).collect();
    if seeds.is_empty() {
        seeds = vec![1, 2, 3, 4];
    }
    println!("ia_quality_probe: {turns} turns, seeds {seeds:?} (not implemented yet)");
}
