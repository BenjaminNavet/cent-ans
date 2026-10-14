//! Command line of the client (ADR 0357 § 6). Pure parsing, tested in `client/tests/`.

use std::path::{Path, PathBuf};

use bevy::prelude::Resource;

use crate::app::AppState;
use crate::sim::{DEFAULT_FACTION, DEFAULT_SEED};

/// Frames waited before a shot or a dump requested on the command line.
pub const DEFAULT_FRAMES: u32 = 60;

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
