//! EP13: battle replays on the GDExtension side (ADR 0072).
//!
//! Every battle built by `BattleSim.setup` / `setup_historical` is recorded
//! (`sim_battle::ReplayRecorder`): the bridge notes each input that changes
//! the battle (`note`) and fingerprints the state as time moves on.
//!
//! - `save_replay(dir, title)` writes the recording as JSON in `dir` (the
//!   user folder `user://replays`) and keeps the `keep_count` latest;
//! - `list_replays(dir)` lists them, newest first (menu « Rejeux »);
//! - `load_replay(path)` / `start_replay()` switch this `BattleSim` to
//!   playback: the same getters show the re-simulated battle, `tick` plays
//!   it back, orders are refused;
//! - `replay_seek(seconds)` jumps along the time bar, `get_replay()` tells
//!   the duration, the position and any divergence (rules changed since the
//!   recording).

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use godot::prelude::*;
use sim_battle::{BattleReplay, ReplayAction, ReplayPlayer, ReplayRecorder, ReplayRules};

use crate::battle_sim::{result_dict, BattleSim};

/// Refusal of every order during a replay.
pub(crate) const REPLAY_REFUSAL: &str = "rejeu : on regarde la bataille, on ne la commande pas";

/// File names of the replays: `rejeu-<unix time>[-n].json`.
const PREFIX: &str = "rejeu-";

/// Notes `action` in the recording, if any (before applying it to `sim`).
pub(crate) fn note(
    recorder: &mut Option<ReplayRecorder>,
    sim: &sim_battle::BattleSim,
    action: ReplayAction,
) {
    if let Some(recorder) = recorder {
        recorder.record(sim, action);
    }
}

/// Replay files of `dir`, oldest first.
fn replay_files(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    p.extension().is_some_and(|e| e == "json")
                        && p.file_name()
                            .and_then(|n| n.to_str())
                            .is_some_and(|n| n.starts_with(PREFIX))
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort_by_key(|p| order_key(p));
    files
}

/// `(time, n)` of `rejeu-<time>[-n].json`: files of the same second keep
/// their order of writing.
fn order_key(path: &Path) -> (u64, u64, String) {
    let name = path
        .file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .trim_start_matches(PREFIX);
    let mut parts = name.split('-');
    let stamp = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let n = parts.next().and_then(|p| p.parse().ok()).unwrap_or(1);
    (stamp, n, name.to_owned())
}

/// Writes `replay` in `dir` and removes the oldest beyond `keep`.
pub(crate) fn store(dir: &Path, replay: &BattleReplay, keep: usize) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let stamp = replay.header.recorded_at;
    // After the latest file of the same second (the older ones may have
    // been removed: never reuse a lower number).
    let last = replay_files(dir)
        .iter()
        .map(|p| order_key(p))
        .filter(|(s, _, _)| *s == stamp)
        .map(|(_, n, _)| n)
        .max();
    let path = match last {
        None => dir.join(format!("{PREFIX}{stamp:010}.json")),
        Some(n) => dir.join(format!("{PREFIX}{stamp:010}-{}.json", n + 1)),
    };
    std::fs::write(&path, replay.to_json()).map_err(|e| e.to_string())?;
    let files = replay_files(dir);
    let excess = files.len().saturating_sub(keep.max(1));
    for old in &files[..excess] {
        let _ = std::fs::remove_file(old);
    }
    Ok(path)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl BattleSim {
    /// Applies `action` to the live battle and records it (no-op during a
    /// replay).
    pub(crate) fn drive(&mut self, action: ReplayAction) {
        if self.player.is_some() {
            return;
        }
        let Some(sim) = &mut self.sim else {
            return;
        };
        note(&mut self.recorder, sim, action.clone());
        action.apply(sim);
    }

    fn begin_playback(&mut self, replay: BattleReplay) -> Result<(), String> {
        let site = replay.start.site.clone();
        let (player, sim) = ReplayPlayer::load(replay).map_err(|e| e.to_string())?;
        self.historical = site;
        self.sim = Some(sim);
        self.player = Some(player);
        Ok(())
    }

    /// Discards what the renderer has not read yet (after a jump).
    fn drain_renderer_queues(&mut self) {
        if let Some(sim) = &mut self.sim {
            sim.take_shots();
            sim.take_impacts();
            sim.take_new_siege_fx();
            sim.take_new_events();
        }
    }
}

#[godot_api(secondary)]
impl BattleSim {
    /// Writes the recording of this battle in `dir` (created if needed) as
    /// `rejeu-<time>.json`, titled `title`; keeps the `keep_count` latest
    /// (`data/rules/battle_replay.json`). → the file path, empty on failure
    /// or during a replay.
    #[func]
    fn save_replay(&mut self, dir: GString, title: GString) -> GString {
        let Some(recorder) = &self.recorder else {
            return GString::new();
        };
        if self.player.is_some() {
            return GString::new();
        }
        let replay = recorder.finish(&title.to_string(), now());
        let keep = ReplayRules::bundled().keep_count;
        match store(&PathBuf::from(dir.to_string()), &replay, keep) {
            Ok(path) => GString::from(path.to_string_lossy().as_ref()),
            Err(error) => {
                godot_warn!("BattleSim.save_replay: {error}");
                GString::new()
            }
        }
    }

    /// Replays of `dir`, newest first: `[{path, title, attacker, defender,
    /// player_side, winner, duration, recorded_at, format, readable,
    /// game_version}]` (`readable` false: another file format, shown but
    /// not playable).
    #[func]
    fn list_replays(&self, dir: GString) -> VarArray {
        let mut files = replay_files(&PathBuf::from(dir.to_string()));
        files.reverse();
        files
            .iter()
            .filter_map(|path| {
                let text = std::fs::read_to_string(path).ok()?;
                let header = BattleReplay::header_from_json(&text).ok()?;
                Some(
                    vdict! {
                        "path" => path.to_string_lossy().as_ref(),
                        "title" => header.title.as_str(),
                        "attacker" => header.attacker.as_str(),
                        "defender" => header.defender.as_str(),
                        "player_side" => header.player_side.map_or("", |s| s.key()),
                        "winner" => header.winner.map_or("", |s| s.key()),
                        "duration" => header.duration,
                        "recorded_at" => header.recorded_at as i64,
                        "format" => i64::from(header.format),
                        "readable" => header.format == sim_battle::replay::REPLAY_FORMAT,
                        "game_version" => header.game_version.as_str(),
                    }
                    .to_variant(),
                )
            })
            .collect()
    }

    /// Plays back the replay file `path` → `{ok, error}` (French error: not
    /// a replay, other format, battle cannot be rebuilt).
    #[func]
    fn load_replay(&mut self, path: GString) -> VarDictionary {
        let result = std::fs::read_to_string(path.to_string())
            .map_err(|e| format!("rejeu introuvable : {e}"))
            .and_then(|text| BattleReplay::from_json(&text).map_err(|e| e.to_string()))
            .and_then(|replay| {
                self.recorder = None;
                self.begin_playback(replay)
            });
        result_dict(result)
    }

    /// Plays back the battle just fought (from the recording in memory)
    /// → `{ok, error}`.
    #[func]
    fn start_replay(&mut self) -> VarDictionary {
        let Some(recorder) = &self.recorder else {
            return result_dict(Err("aucune bataille enregistrée".to_owned()));
        };
        let replay = recorder.finish("", now());
        result_dict(self.begin_playback(replay))
    }

    /// `true` while a replay is played back (orders refused).
    #[func]
    fn is_replay(&self) -> bool {
        self.player.is_some()
    }

    /// Playback state: `{duration, elapsed, at_end, title, attacker,
    /// defender, winner, recorded_at, keyframes, divergence: {time, message}
    /// (empty while the replay follows the recording)}`; empty outside a
    /// replay.
    #[func]
    fn get_replay(&self) -> VarDictionary {
        let (Some(player), Some(sim)) = (&self.player, &self.sim) else {
            return VarDictionary::new();
        };
        let header = &player.replay().header;
        let divergence = player
            .divergence()
            .map(|d| vdict! { "time" => d.time, "message" => d.message.as_str() })
            .unwrap_or_default();
        vdict! {
            "duration" => player.duration(),
            "elapsed" => sim.elapsed(),
            "at_end" => player.at_end(sim),
            "title" => header.title.as_str(),
            "attacker" => header.attacker.as_str(),
            "defender" => header.defender.as_str(),
            "winner" => header.winner.map_or("", |s| s.key()),
            "recorded_at" => header.recorded_at as i64,
            "keyframes" => player.keyframe_count() as i64,
            "divergence" => &divergence,
        }
    }

    /// Jumps to `seconds` of the replay (clamped); the renderer queues
    /// (volleys, impacts, journal) are emptied. → steps simulated.
    #[func]
    fn replay_seek(&mut self, seconds: f64) -> i64 {
        let (Some(player), Some(sim)) = (&mut self.player, &mut self.sim) else {
            return 0;
        };
        let steps = player.seek(sim, seconds);
        self.drain_renderer_queues();
        steps as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_keeps_the_latest() {
        let dir = std::env::temp_dir().join(format!("cent-ans-replays-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let setup: sim_battle::BattleSetup = serde_json::from_str(include_str!(
            "../../sim-battle/tests/fixtures/demo_battle_1337.json"
        ))
        .unwrap();
        let start = sim_battle::ReplayStart::plain(setup, 1);
        let sim = start.build().unwrap();
        let recorder = ReplayRecorder::new(start, &sim);
        for stamp in [100, 300, 200, 300] {
            store(&dir, &recorder.finish("t", stamp), 3).unwrap();
        }
        // The oldest of second 300 goes; the next one still sorts last.
        std::fs::remove_file(dir.join("rejeu-0000000300.json")).unwrap();
        store(&dir, &recorder.finish("t", 300), 3).unwrap();
        let files = replay_files(&dir);
        let names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            names,
            [
                "rejeu-0000000200.json",
                "rejeu-0000000300-2.json",
                "rejeu-0000000300-3.json"
            ]
        );
        let text = std::fs::read_to_string(&files[0]).unwrap();
        assert!(BattleReplay::from_json(&text).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
