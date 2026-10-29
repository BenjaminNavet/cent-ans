//! R1-0 (audit `docs/audits/2026-10-29-campagne.md`): the campaign chronicle probe.
//!
//! `cargo run --profile seed-bots -p seed-bots --example campaign_timeline -- [--faction fac_france]
//!   [--seeds 1,2,3] [--profile passive|ai] [--turns 470] [--ambitions id,id] [--csv DIR] [--quiet]`
//!
//! Prints, per game, one row per year, the outcome, the bilan of the acts, the events of the
//! game and a `METRICS` line (`flips_per_year`, `player_flips_per_year`, `fr_en_war_share`,
//! `truce_share`, `score_frozen_years`, `raids_per_game`, `ransoms_per_game`,
//! `field_men_median`, `years_under_300_men`, plus the act openings and final scores). With
//! `--csv DIR`, one CSV file of the yearly rows per game. `--quiet` prints the metrics only.
//! The games of several seeds run in parallel.

use std::path::{Path, PathBuf};

use data_model::GameData;
use seed_bots::timeline::{self, ProbeConfig, Profile};

fn main() {
    let mut faction = "fac_france".to_owned();
    let mut seeds = vec![1u64];
    let mut profile = Profile::Ai;
    let mut turns = 470;
    let mut ambitions: Vec<String> = Vec::new();
    let mut csv: Option<PathBuf> = None;
    let mut quiet = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--faction" => faction = args.next().expect("--faction id"),
            "--seeds" => {
                seeds = args
                    .next()
                    .expect("--seeds a,b")
                    .split(',')
                    .map(|s| s.parse().expect("seed"))
                    .collect();
            }
            "--profile" => {
                profile = Profile::parse(&args.next().expect("--profile passive|ai"))
                    .expect("--profile passive|ai");
            }
            "--turns" => turns = args.next().and_then(|s| s.parse().ok()).expect("--turns N"),
            "--ambitions" => {
                ambitions = args
                    .next()
                    .expect("--ambitions a,b")
                    .split(',')
                    .map(str::to_owned)
                    .collect();
            }
            "--csv" => csv = Some(PathBuf::from(args.next().expect("--csv DIR"))),
            "--quiet" => quiet = true,
            other => panic!("unknown argument {other}"),
        }
    }
    let root = std::env::var("CENT_ANS_DATA_DIR").map_or_else(
        |_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data"),
        PathBuf::from,
    );
    let (data, _) = GameData::load(&root).expect("game data");
    let data: &GameData = &data;
    let timelines: Vec<timeline::Timeline> = std::thread::scope(|scope| {
        let handles: Vec<_> = seeds
            .iter()
            .map(|&seed| {
                let mut config = ProbeConfig::new(&faction, seed, profile);
                config.turns = turns;
                config.ambitions = ambitions.clone();
                scope.spawn(move || timeline::run(data, &config))
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("probe run"))
            .collect()
    });
    for t in &timelines {
        if quiet {
            println!("{}", t.metrics_line());
        } else {
            print!("{}", t.report());
        }
        if let Some(dir) = &csv {
            std::fs::create_dir_all(dir).expect("csv dir");
            let path = dir.join(format!("{}_{}_{}.csv", t.faction, t.profile, t.seed));
            std::fs::write(&path, t.csv()).expect("csv written");
        }
    }
}
