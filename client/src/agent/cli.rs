//! Command line of the client (ADR 0357 § 6). Pure parsing, tested in `client/tests/`.

use std::path::{Path, PathBuf};

use bevy::prelude::Resource;

use crate::app::AppState;
use crate::sim::{DEFAULT_FACTION, DEFAULT_SEED};

/// Frames waited before a shot or a dump requested on the command line.
pub const DEFAULT_FRAMES: u32 = 60;

/// `--bench-map` and its options (port of `game/scripts/dev/map_bench.gd`, see `agent::bench`).
#[derive(Debug, Clone, PartialEq)]
pub struct BenchConfig {
    /// `--bench-out <file>`: JSON report (also printed in the log as `CampaignMap: bench_map`).
    pub out: Option<PathBuf>,
    /// `--bench-distance <n>`: camera distance of the pan (30).
    pub distance: f32,
    /// `--bench-seconds <n>`: duration of the pan (20 s).
    pub seconds: f32,
    /// `--bench-pan-only`: stop after the pan.
    pub pan_only: bool,
    /// `--bench-descent-only`: skip the pan.
    pub descent_only: bool,
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self {
            out: None,
            distance: 30.0,
            seconds: 20.0,
            pan_only: false,
            descent_only: false,
        }
    }
}

#[derive(Resource, Debug, Clone, PartialEq)]
pub struct Cli {
    /// `--state <name>`: first screen (default: main menu).
    pub state: Option<AppState>,
    /// `--faction <id>`.
    pub faction: String,
    /// `--seed <n>`.
    pub seed: u64,
    /// `--dump-state <file>`.
    pub dump_state: Option<PathBuf>,
    /// `--shot <file>`.
    pub shot: Option<PathBuf>,
    /// `--frames <n>`: frames to wait before the shot / dump.
    pub frames: u32,
    /// `--exit-after <n>`: quit after n frames whatever happens.
    pub exit_after: Option<u32>,
    /// `--script <file>`.
    pub script: Option<PathBuf>,
    /// `--offscreen`: render to an image (implied by `--shot`).
    pub offscreen: bool,
    /// `--bench-map`: scripted camera path over the campaign, frame-time report.
    pub bench: Option<BenchConfig>,
}

impl Default for Cli {
    fn default() -> Self {
        Self {
            state: None,
            faction: DEFAULT_FACTION.to_owned(),
            seed: DEFAULT_SEED,
            dump_state: None,
            shot: None,
            frames: DEFAULT_FRAMES,
            exit_after: None,
            script: None,
            offscreen: false,
            bench: None,
        }
    }
}

/// Relative paths are taken from the working directory (the `.app` bundle starts in `/`;
/// `run_bg.sh` makes them absolute beforehand).
pub fn absolute(path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().map_or_else(|_| path.to_path_buf(), |cwd| cwd.join(path))
    }
}

impl Cli {
    /// Parses the arguments after the program name.
    pub fn parse(args: &[String]) -> Result<Cli, String> {
        let mut cli = Cli::default();
        let mut index = 0;
        while index < args.len() {
            let flag = args[index].as_str();
            let mut value = || {
                index += 1;
                args.get(index)
                    .cloned()
                    .ok_or(format!("{flag} expects a value"))
            };
            match flag {
                "--state" => {
                    let name = value()?;
                    cli.state =
                        Some(AppState::from_name(&name).ok_or(format!("unknown state {name}"))?);
                }
                "--faction" => cli.faction = value()?,
                "--seed" => cli.seed = value()?.parse().map_err(|e| format!("--seed: {e}"))?,
                "--dump-state" => cli.dump_state = Some(absolute(&value()?)),
                "--shot" => cli.shot = Some(absolute(&value()?)),
                "--frames" => {
                    cli.frames = value()?.parse().map_err(|e| format!("--frames: {e}"))?
                }
                "--exit-after" => {
                    cli.exit_after =
                        Some(value()?.parse().map_err(|e| format!("--exit-after: {e}"))?);
                }
                "--script" => cli.script = Some(absolute(&value()?)),
                "--offscreen" => cli.offscreen = true,
                "--bench-map" => {
                    cli.bench.get_or_insert_with(BenchConfig::default);
                }
                "--bench-out" => {
                    let path = absolute(&value()?);
                    cli.bench.get_or_insert_with(BenchConfig::default).out = Some(path);
                }
                "--bench-distance" => {
                    let n = value()?
                        .parse()
                        .map_err(|e| format!("--bench-distance: {e}"))?;
                    cli.bench.get_or_insert_with(BenchConfig::default).distance = n;
                }
                "--bench-seconds" => {
                    let n = value()?
                        .parse()
                        .map_err(|e| format!("--bench-seconds: {e}"))?;
                    cli.bench.get_or_insert_with(BenchConfig::default).seconds = n;
                }
                "--bench-pan-only" => {
                    cli.bench.get_or_insert_with(BenchConfig::default).pan_only = true;
                }
                "--bench-descent-only" => {
                    cli.bench
                        .get_or_insert_with(BenchConfig::default)
                        .descent_only = true;
                }
                other => return Err(format!("unknown option {other}")),
            }
            index += 1;
        }
        Ok(cli)
    }

    /// Whether the main camera renders to an image rather than the window.
    pub fn wants_offscreen(&self) -> bool {
        self.offscreen || self.shot.is_some()
    }
}
