//! EP13 (ADR 0072): a recorded battle replays to the same outcome; a rule
//! change is reported instead of silently showing another battle; jumps
//! along the time bar land on the same state as straight playback.

mod common;

use std::path::PathBuf;
use std::time::Instant;

use common::*;
use sim_battle::replay::{state_digest, ReplayRules};
use sim_battle::{
    BattleReplay, BattleSetup, BattleSim, Command, HistoricalMap, ReplayAction, ReplayError,
    ReplayPlayer, ReplayRecorder, ReplayStart, SideId, UnitState, DT,
};

fn demo_setup() -> BattleSetup {
    serde_json::from_str(include_str!("fixtures/demo_battle_1337.json")).unwrap()
}

/// Applies `action` to the live battle and records it, as the bridge does.
fn drive(sim: &mut BattleSim, recorder: &mut ReplayRecorder, action: ReplayAction) {
    recorder.record(sim, action.clone());
    action.apply(sim);
}

/// Advances the live battle by frames of `dt` seconds (like `_process`).
fn play_frames(sim: &mut BattleSim, recorder: &mut ReplayRecorder, seconds: f64, dt: f64) {
    let until = sim.elapsed() + seconds;
    while sim.elapsed() < until && !sim.is_finished() {
        sim.tick(dt);
        recorder.observe(sim);
    }
}

/// B6-like digest of a battle: end time, winner, strength per regiment.
fn b6_digest(sim: &BattleSim) -> String {
    let hp: Vec<i64> = sim.units().iter().map(|u| u.hp.round() as i64).collect();
    format!("{:.0} {:?} {:?}", sim.elapsed(), sim.winner(), hp)
}

/// The 1337 demo played by a "player" on the attacking side: deployment,
/// a march, an attack, a halt and a formation change, at uneven frame
/// rates, to the end.
fn record_demo(seed: u64) -> (BattleSim, BattleReplay) {
    let start = ReplayStart::plain(demo_setup(), seed);
    let mut sim = start.build().unwrap();
    let mut recorder = ReplayRecorder::new(start, &sim);
    drive(
        &mut sim,
        &mut recorder,
        ReplayAction::StartHour { hour: 9.5 },
    );
    drive(&mut sim, &mut recorder, ReplayAction::BeginDeployment);
    let ours: Vec<u32> = sim
        .units()
        .iter()
        .filter(|u| u.side == SideId::Attacker)
        .map(|u| u.id)
        .collect();
    let first = sim.unit(ours[0]).unwrap().clone();
    drive(
        &mut sim,
        &mut recorder,
        ReplayAction::DeployUnit {
            unit: first.id,
            x: first.x + 15.0,
            z: first.z,
            facing: None,
            width: None,
        },
    );
    drive(&mut sim, &mut recorder, ReplayAction::StartBattle);
    play_frames(&mut sim, &mut recorder, 20.0, 1.0 / 60.0);
    let (x, z) = {
        let enemy = sim
            .units()
            .iter()
            .find(|u| u.side == SideId::Defender)
            .unwrap();
        (enemy.x, enemy.z)
    };
    drive(
        &mut sim,
        &mut recorder,
        ReplayAction::Command {
            command: Command::Move {
                units: ours.clone(),
                x,
                z: (z + first.z) / 2.0,
                run: false,
                facing: None,
                queue: false,
                width: None,
                match_speed: false,
                group_tag: None,
            },
        },
    );
    play_frames(&mut sim, &mut recorder, 40.0, 1.0 / 30.0 * 4.0);
    let target = sim
        .units()
        .iter()
        .find(|u| u.side == SideId::Defender && u.state != UnitState::Routing)
        .map(|u| u.id)
        .unwrap();
    drive(
        &mut sim,
        &mut recorder,
        ReplayAction::Command {
            command: Command::Attack {
                units: ours[..ours.len() / 2].to_vec(),
                target,
                run: true,
                queue: false,
            },
        },
    );
    drive(
        &mut sim,
        &mut recorder,
        ReplayAction::Command {
            command: Command::Halt {
                units: vec![ours[ours.len() - 1]],
            },
        },
    );
    // A refused command is recorded too (it is refused again on replay).
    drive(
        &mut sim,
        &mut recorder,
        ReplayAction::Command {
            command: Command::Attack {
                units: vec![ours[0]],
                target: ours[1],
                run: true,
                queue: false,
            },
        },
    );
    // Let the AI finish the job for the player.
    drive(
        &mut sim,
        &mut recorder,
        ReplayAction::SetAi {
            side: SideId::Attacker,
            enabled: true,
        },
    );
    while !sim.is_finished() && sim.elapsed() < 1800.0 {
        sim.tick(0.1 * 8.0);
        recorder.observe(&sim);
    }
    assert!(sim.is_finished(), "the demo battle ends");
    let replay = recorder.finish("Bataille de test", 1_790_000_000);
    (sim, replay)
}

fn play_to_end(replay: BattleReplay) -> (ReplayPlayer, BattleSim) {
    let (mut player, mut sim) = ReplayPlayer::load(replay).unwrap();
    while player.advance(&mut sim, 1.0) > 0 {}
    (player, sim)
}

#[test]
fn a_recorded_battle_replays_to_the_same_outcome() {
    let (live, replay) = record_demo(1337);
    assert!(replay.end.is_some());
    assert!(!replay.checkpoints.is_empty());
    assert_eq!(replay.header.winner, live.winner());
    // Through the file format.
    let text = replay.to_json();
    let read = BattleReplay::from_json(&text).unwrap();
    assert_eq!(read, replay, "the file round-trips exactly");
    let (player, sim) = play_to_end(read);
    assert!(player.divergence().is_none(), "{:?}", player.divergence());
    assert!(sim.is_finished());
    assert_eq!(sim.outcome(), live.outcome(), "same BattleOutcome");
    assert_eq!(b6_digest(&sim), b6_digest(&live), "same B6 digest");
    assert_eq!(state_digest(&sim), state_digest(&live));
    assert_eq!(sim.events(), live.events(), "same journal");
}

#[test]
fn replays_of_several_seeds_match() {
    for seed in [3, 11, 42] {
        let (live, replay) = record_demo(seed);
        let (player, sim) = play_to_end(replay);
        assert!(player.divergence().is_none(), "seed {seed}");
        assert_eq!(sim.outcome(), live.outcome(), "seed {seed}");
    }
}

fn map(id: &str) -> HistoricalMap {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../data/battle_maps")
        .join(format!("{id}.json"));
    HistoricalMap::from_json(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn a_historical_battle_replays_without_divergence() {
    let data = data();
    let map = map("crecy");
    let setup = map
        .battle_setup(
            &data.unit_types,
            data.battle_orders.values().cloned().collect(),
            Some(data.battle_standard_rules.clone()),
            None,
        )
        .unwrap();
    let start = ReplayStart::historical(setup, 7, map);
    let mut live = start.build().unwrap();
    let mut recorder = ReplayRecorder::new(start, &live);
    play_frames(&mut live, &mut recorder, 150.0, 0.25);
    let replay = recorder.finish("Crécy", 0);
    assert!(replay.end.is_none() || live.is_finished());
    let text = replay.to_json();
    let (player, sim) = play_to_end(BattleReplay::from_json(&text).unwrap());
    assert!(player.divergence().is_none());
    assert_eq!(sim.ticks(), live.ticks());
    assert_eq!(state_digest(&sim), state_digest(&live));
}

#[test]
fn a_campaign_battle_on_a_site_replays() {
    let mut setup = demo_setup();
    setup.player_side = None;
    let start = ReplayStart::on_site(setup, 5, map("azincourt"));
    let mut live = start.build().unwrap();
    let mut recorder = ReplayRecorder::new(start, &live);
    play_frames(&mut live, &mut recorder, 90.0, 0.5);
    let (player, sim) = play_to_end(recorder.finish("", 0));
    assert!(player.divergence().is_none());
    assert_eq!(state_digest(&sim), state_digest(&live));
}

#[test]
fn a_rule_change_is_reported_not_silent() {
    let (_, replay) = record_demo(1337);
    // A later fingerprint no longer matches (a rule changed mid-battle).
    let mut changed = replay.clone();
    let index = changed.checkpoints.len() / 2;
    let tick = changed.checkpoints[index].tick;
    changed.checkpoints[index].digest = "0000000000000000".to_owned();
    let (player, _) = play_to_end(changed);
    let divergence = player.divergence().expect("reported");
    assert_eq!(divergence.tick, tick);
    assert!(
        divergence.message.contains("règles"),
        "{}",
        divergence.message
    );
    // The battle is built differently (field generation changed).
    let mut rebuilt = replay.clone();
    rebuilt.start.seed += 1;
    let (player, _) = ReplayPlayer::load(rebuilt).unwrap();
    assert_eq!(player.divergence().map(|d| d.tick), Some(0));
    // The recorded outcome differs from the one replayed.
    let mut outcome = replay.clone();
    if let Some(end) = outcome.end.as_mut() {
        end.outcome.duration += 1.0;
    }
    let (player, _) = play_to_end(outcome);
    assert!(player.divergence().is_some());
}

#[test]
fn another_format_is_refused_with_a_message() {
    let (_, replay) = record_demo(1337);
    let mut value: serde_json::Value = serde_json::from_str(&replay.to_json()).unwrap();
    value["header"]["format"] = serde_json::json!(99);
    let text = value.to_string();
    let error = BattleReplay::from_json(&text).unwrap_err();
    assert_eq!(error, ReplayError::Format(99));
    assert!(error.to_string().contains("plus récente"));
    // The list still reads the header.
    assert_eq!(
        BattleReplay::header_from_json(&text).unwrap().format,
        99,
        "the list shows it"
    );
    assert!(matches!(
        BattleReplay::from_json("{}"),
        Err(ReplayError::Invalid(_))
    ));
}

#[test]
fn jumps_land_on_the_state_of_straight_playback() {
    let (live, replay) = record_demo(1337);
    let (mut player, mut sim) = ReplayPlayer::load(replay.clone()).unwrap();
    // Straight playback to 60 s, fingerprint.
    player.seek(&mut sim, 60.0);
    assert!((sim.elapsed() - 60.0).abs() < DT);
    let at_60 = state_digest(&sim);
    // Forward to the end, back to 60 s, back to the start, forward again.
    player.seek(&mut sim, replay.duration());
    assert!(sim.is_finished());
    player.seek(&mut sim, 60.0);
    assert_eq!(state_digest(&sim), at_60, "back from a copy");
    player.seek(&mut sim, 0.0);
    assert_eq!(sim.ticks(), 0);
    assert!(!sim.is_deploying(), "the deployment inputs are applied");
    player.seek(&mut sim, 60.0);
    assert_eq!(state_digest(&sim), at_60);
    while player.advance(&mut sim, 2.0) > 0 {}
    assert!(player.divergence().is_none());
    assert_eq!(sim.outcome(), live.outcome());
    let rules = ReplayRules::bundled();
    assert!(player.keyframe_count() <= rules.max_keyframes + 1);
}

/// Probe (ignored): costs of replaying and jumping, for ADR 0072.
/// `cargo test -p sim-battle --release --test ep13_replay measure -- --ignored --nocapture`
#[test]
#[ignore]
fn measure() {
    let (live, replay) = record_demo(1337);
    println!(
        "demo: {} regiments, {:.0} s, {} inputs, {} checkpoints, file {} KB",
        live.units().len(),
        replay.duration(),
        replay.actions.len(),
        replay.checkpoints.len(),
        replay.to_json().len() / 1024
    );
    report(replay);
    let data = data();
    for id in ["crecy", "azincourt"] {
        let map = map(id);
        let setup = map
            .battle_setup(
                &data.unit_types,
                data.battle_orders.values().cloned().collect(),
                Some(data.battle_standard_rules.clone()),
                None,
            )
            .unwrap();
        let start = ReplayStart::historical(setup, 7, map);
        let mut live = start.build().unwrap();
        let mut recorder = ReplayRecorder::new(start, &live);
        let clock = Instant::now();
        while !live.is_finished() && live.elapsed() < 1800.0 {
            live.tick(0.8);
            recorder.observe(&live);
        }
        println!(
            "{id}: {} regiments, {:.0} s simulated in {:.2} s live, file {} KB",
            live.units().len(),
            live.elapsed(),
            clock.elapsed().as_secs_f64(),
            recorder.replay().to_json().len() / 1024
        );
        report(recorder.finish(id, 0));
    }
}

fn report(replay: BattleReplay) {
    let duration = replay.duration();
    let clock = Instant::now();
    let (mut player, mut sim) = ReplayPlayer::load(replay).unwrap();
    let load = clock.elapsed().as_secs_f64();
    let clock = Instant::now();
    player.seek(&mut sim, duration);
    let full = clock.elapsed().as_secs_f64();
    let clock = Instant::now();
    player.seek(&mut sim, duration * 0.5 + 7.0);
    let back = clock.elapsed().as_secs_f64();
    let clock = Instant::now();
    let copy = sim.clone();
    let clone = clock.elapsed().as_secs_f64();
    drop(copy);
    println!(
        "  load {load:.3} s, full re-simulation {full:.2} s ({:.0}× real time), \
         jump back {back:.3} s, {} copies, one copy {:.2} ms, divergence {:?}",
        duration / full.max(1e-6),
        player.keyframe_count(),
        clone * 1000.0,
        player.divergence()
    );
}

/// Sample replay file, validated by `tools/tests/test_battle_replay_schema.py`.
const SAMPLE: &str = "tests/fixtures/replay_sample.json";

/// Rewrites the sample (after a deliberate format change, with
/// `REPLAY_FORMAT` bumped): `cargo test -p sim-battle --test ep13_replay
/// write_sample -- --ignored`.
#[test]
#[ignore]
fn write_sample() {
    let (_, replay) = record_demo(1337);
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(SAMPLE);
    std::fs::write(path, serde_json::to_string_pretty(&replay).unwrap()).unwrap();
}

/// A file written by an earlier build still reads: changing the structure
/// of a replay without bumping `REPLAY_FORMAT` breaks this test.
#[test]
fn the_sample_file_still_reads() {
    let replay = BattleReplay::from_json(include_str!("fixtures/replay_sample.json"))
        .expect("the replay format changed: bump REPLAY_FORMAT and rewrite the sample");
    assert_eq!(replay.header.format, sim_battle::replay::REPLAY_FORMAT);
    let (player, sim) = ReplayPlayer::load(replay).expect("the sample battle rebuilds");
    assert!(sim.ticks() == 0 && player.duration() > 0.0);
}
