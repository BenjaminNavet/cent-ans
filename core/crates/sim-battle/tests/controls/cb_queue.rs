//! CB-M3: queued orders (Shift + right click). A queued move or attack
//! waits behind the current order and starts when it ends; any plain order,
//! a halt or a rout empties the queue; at most `max_queued_orders`
//! (`data/rules/battle_queue.json`); replays record and replay the queue,
//! and replays recorded before CB still read.

use crate::common;

use common::*;
use sim_battle::replay::state_digest;
use sim_battle::{
    BattleReplay, BattleSim, Command, CommandError, QueueRules, QueuedOrder, ReplayAction,
    ReplayPlayer, ReplayRecorder, ReplayStart, SideId, UnitState, DT,
};

type P = (f64, f64);

fn go(units: Vec<u32>, to: P, queue: bool) -> Command {
    Command::Move {
        units,
        x: to.0,
        z: to.1,
        run: false,
        facing: None,
        queue,
        width: None,
        match_speed: false,
        group_tag: None,
    }
}

fn attack(units: Vec<u32>, target: u32, queue: bool) -> Command {
    Command::Attack {
        units,
        target,
        run: false,
        queue,
    }
}

/// Knights (0) and a second regiment (1) against militia far away (2).
fn lab_sim(seed: u64) -> BattleSim {
    let data = data();
    let mut sim = BattleSim::new(
        setup(
            units(data, &["unit_knights", "unit_knights"]),
            units(data, &["unit_urban_militia"]),
            None,
        ),
        seed,
    )
    .unwrap();
    lab(&mut sim);
    place(&mut sim, 0, 400.0, 300.0, 0.0);
    place(&mut sim, 1, 460.0, 300.0, 0.0);
    place(&mut sim, 2, 1100.0, 800.0, 0.0);
    sim
}

fn pos(sim: &BattleSim, id: u32) -> P {
    let u = &sim.units()[id as usize];
    (u.x, u.z)
}

fn dist(a: P, b: P) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

/// Steps until regiment `id` has no order left (or `seconds` pass);
/// returns the closest it came to each of `marks`.
fn walk_marks(sim: &mut BattleSim, id: u32, marks: &[P], seconds: f64) -> Vec<f64> {
    let mut best = vec![f64::MAX; marks.len()];
    for _ in 0..(seconds / DT) as usize {
        sim.step();
        let at = pos(sim, id);
        for (b, m) in best.iter_mut().zip(marks) {
            *b = b.min(dist(at, *m));
        }
        if !sim.units()[id as usize].busy() {
            break;
        }
    }
    best
}

#[test]
fn a_queued_move_waits_behind_the_current_order() {
    let mut sim = lab_sim(3);
    sim.apply_command(go(vec![0], (500.0, 400.0), false), None)
        .unwrap();
    sim.apply_command(go(vec![0], (600.0, 300.0), true), None)
        .unwrap();
    let u = &sim.units()[0];
    assert_eq!(u.destination, Some((500.0, 400.0)), "current order kept");
    assert_eq!(u.order_queue.len(), 1);
    assert!(matches!(
        u.order_queue[0],
        QueuedOrder::Move { x, z, .. } if x == 600.0 && z == 300.0
    ));
}

#[test]
fn a_queued_order_to_an_idle_regiment_starts_at_once() {
    let mut sim = lab_sim(3);
    sim.apply_command(go(vec![0], (500.0, 400.0), true), None)
        .unwrap();
    let u = &sim.units()[0];
    assert_eq!(u.destination, Some((500.0, 400.0)));
    assert!(u.order_queue.is_empty());
}

#[test]
fn queued_orders_follow_one_another() {
    let mut sim = lab_sim(5);
    let marks = [(500.0, 400.0), (600.0, 300.0), (520.0, 220.0)];
    sim.apply_command(go(vec![0], marks[0], false), None)
        .unwrap();
    for &m in &marks[1..] {
        sim.apply_command(go(vec![0], m, true), None).unwrap();
    }
    assert_eq!(sim.units()[0].order_queue.len(), 2);
    let best = walk_marks(&mut sim, 0, &marks, 600.0);
    for (k, b) in best.iter().enumerate() {
        assert!(*b < 2.0, "waypoint {k} reached (closest {b:.1} m)");
    }
    let u = &sim.units()[0];
    assert!(u.order_queue.is_empty() && u.destination.is_none());
    assert!(dist(pos(&sim, 0), marks[2]) < 2.0, "ends on the last point");
}

#[test]
fn a_group_queued_move_keeps_the_formation_from_the_queue_end() {
    let mut sim = lab_sim(5);
    sim.apply_command(go(vec![0, 1], (430.0, 500.0), false), None)
        .unwrap();
    let first = (
        sim.units()[0].destination.unwrap(),
        sim.units()[1].destination.unwrap(),
    );
    sim.apply_command(go(vec![0, 1], (630.0, 500.0), true), None)
        .unwrap();
    for (id, before) in [(0usize, first.0), (1, first.1)] {
        let QueuedOrder::Move { x, z, .. } = sim.units()[id].order_queue[0] else {
            panic!("a queued move");
        };
        assert!((x - before.0 - 200.0).abs() < 1e-9 && (z - before.1).abs() < 1e-9);
    }
}

#[test]
fn a_plain_order_or_a_halt_empties_the_queue() {
    let mut sim = lab_sim(3);
    sim.apply_command(go(vec![0], (500.0, 400.0), false), None)
        .unwrap();
    sim.apply_command(go(vec![0], (600.0, 300.0), true), None)
        .unwrap();
    sim.apply_command(attack(vec![0], 2, true), None).unwrap();
    assert_eq!(sim.units()[0].order_queue.len(), 2);
    sim.apply_command(go(vec![0], (450.0, 450.0), false), None)
        .unwrap();
    let u = &sim.units()[0];
    assert!(u.order_queue.is_empty());
    assert_eq!(u.destination, Some((450.0, 450.0)));

    sim.apply_command(go(vec![0], (600.0, 300.0), true), None)
        .unwrap();
    assert_eq!(sim.units()[0].order_queue.len(), 1);
    sim.apply_command(Command::Halt { units: vec![0] }, None)
        .unwrap();
    assert!(sim.units()[0].order_queue.is_empty());
    run(&mut sim, 5.0);
    assert!(sim.units()[0].destination.is_none(), "nothing restarts");
}

#[test]
fn a_rout_empties_the_queue() {
    let mut sim = lab_sim(3);
    sim.apply_command(go(vec![0], (500.0, 400.0), false), None)
        .unwrap();
    sim.apply_command(go(vec![0], (600.0, 300.0), true), None)
        .unwrap();
    {
        let u = &mut sim.units_mut()[0];
        u.morale = 0.0;
        u.morale_cap = 0.0;
    }
    run(&mut sim, 0.5);
    let u = &sim.units()[0];
    assert_eq!(u.state, UnitState::Routing);
    assert!(u.order_queue.is_empty());
}

#[test]
fn an_attack_on_a_fleeing_target_gives_way_to_the_next_order() {
    let mut sim = lab_sim(3);
    sim.apply_command(attack(vec![0], 2, false), None).unwrap();
    sim.apply_command(go(vec![0], (450.0, 450.0), true), None)
        .unwrap();
    run(&mut sim, 1.0);
    assert_eq!(sim.units()[0].target, Some(2));
    {
        let u = &mut sim.units_mut()[2];
        u.morale = 0.0;
        u.morale_cap = 0.0;
    }
    run(&mut sim, 1.0);
    assert_eq!(sim.units()[2].state, UnitState::Routing);
    let u = &sim.units()[0];
    assert_eq!(u.target, None, "the chase ends");
    assert_eq!(
        u.destination,
        Some((450.0, 450.0)),
        "the queued move starts"
    );
}

#[test]
fn the_queue_is_bounded_by_the_rules() {
    let max = QueueRules::bundled().max_queued_orders;
    assert_eq!(max, 8, "data/rules/battle_queue.json");
    let mut sim = lab_sim(3);
    sim.apply_command(go(vec![0, 1], (500.0, 400.0), false), None)
        .unwrap();
    for k in 0..max {
        let x = 450.0 + f64::from(k) * 10.0;
        sim.apply_command(go(vec![0, 1], (x, 500.0), true), None)
            .unwrap();
    }
    assert_eq!(sim.units()[0].order_queue.len(), max as usize);
    let error = sim
        .apply_command(go(vec![0, 1], (700.0, 500.0), true), None)
        .unwrap_err();
    assert_eq!(error, CommandError::QueueFull { unit: 0, max });
    assert!(error.to_string().contains("file d'ordres pleine"));
    let error = sim
        .apply_command(attack(vec![1], 2, true), None)
        .unwrap_err();
    assert_eq!(error, CommandError::QueueFull { unit: 1, max });
    assert_eq!(
        sim.units()[1].order_queue.len(),
        max as usize,
        "nothing added"
    );
    // A plain order is always accepted.
    sim.apply_command(go(vec![0], (700.0, 500.0), false), None)
        .unwrap();
}

#[test]
fn the_queue_preview_starts_from_the_last_queued_point() {
    let mut sim = lab_sim(3);
    sim.apply_command(go(vec![0], (500.0, 400.0), false), None)
        .unwrap();
    sim.apply_command(go(vec![0], (600.0, 300.0), true), None)
        .unwrap();
    assert_eq!(sim.queue_anchor(0), (600.0, 300.0));
    let path = sim
        .preview_path_from(0, Some((600.0, 300.0)), 700.0, 350.0)
        .unwrap();
    assert_eq!(path, vec![(700.0, 350.0)]);
    let legs = sim.preview_group_queued(&[0], 700.0, 350.0, None);
    assert_eq!(legs[0].from, (600.0, 300.0));
    assert_eq!(legs[0].path, Ok(vec![(700.0, 350.0)]));
    // Without `from`: the plain preview.
    assert_eq!(
        sim.preview_path_from(0, None, 700.0, 350.0),
        sim.preview_path(0, 700.0, 350.0)
    );
}

/// Records the 1337 demo with queued orders for a minute of battle.
fn record_with_queue(seed: u64) -> (BattleSim, BattleReplay) {
    let setup = serde_json::from_str(include_str!("../fixtures/demo_battle_1337.json")).unwrap();
    let start = ReplayStart::plain(setup, seed);
    let mut sim = start.build().unwrap();
    let mut recorder = ReplayRecorder::new(start, &sim);
    let drive = |sim: &mut BattleSim, recorder: &mut ReplayRecorder, action: ReplayAction| {
        recorder.record(sim, action.clone());
        action.apply(sim);
    };
    drive(&mut sim, &mut recorder, ReplayAction::StartBattle);
    let ours: Vec<u32> = sim
        .units()
        .iter()
        .filter(|u| u.side == SideId::Attacker && u.present())
        .map(|u| u.id)
        .collect();
    let first = pos(&sim, ours[0]);
    let enemy = sim
        .units()
        .iter()
        .find(|u| u.side == SideId::Defender && u.present())
        .map(|u| u.id)
        .unwrap();
    for (k, queue) in [(0.0, false), (40.0, true), (80.0, true)] {
        drive(
            &mut sim,
            &mut recorder,
            ReplayAction::Command {
                command: go(
                    ours[..2].to_vec(),
                    (first.0 + k, first.1 + 30.0 + k * 0.5),
                    queue,
                ),
            },
        );
    }
    drive(
        &mut sim,
        &mut recorder,
        ReplayAction::Command {
            command: attack(vec![ours[0]], enemy, true),
        },
    );
    assert_eq!(sim.units()[ours[0] as usize].order_queue.len(), 3);
    assert_eq!(sim.units()[ours[1] as usize].order_queue.len(), 2);
    for _ in 0..600 {
        sim.tick(DT);
        recorder.observe(&sim);
    }
    (sim, recorder.finish("File d'ordres", 1_790_000_000))
}

#[test]
fn a_replay_with_queued_orders_replays_exactly() {
    let (live, replay) = record_with_queue(1337);
    let text = replay.to_json();
    assert!(text.contains("\"queue\":true"), "queued commands recorded");
    let read = BattleReplay::from_json(&text).unwrap();
    assert_eq!(read, replay);
    let (mut player, mut sim) = ReplayPlayer::load(read).unwrap();
    while player.advance(&mut sim, 1.0) > 0 {}
    assert!(player.divergence().is_none(), "{:?}", player.divergence());
    assert_eq!(sim.ticks(), live.ticks());
    assert_eq!(state_digest(&sim), state_digest(&live));
}

#[test]
fn a_replay_recorded_before_cb_still_reads() {
    // The sample written by EP13 (before CB): commands without `queue`.
    let text = include_str!("../fixtures/replay_sample.json");
    assert!(!text.contains("\"queue\""));
    let replay = BattleReplay::from_json(text).expect("the pre-CB sample reads");
    let commands: Vec<&Command> = replay
        .actions
        .iter()
        .filter_map(|entry| match &entry.action {
            ReplayAction::Command { command } => Some(command),
            _ => None,
        })
        .collect();
    assert!(!commands.is_empty());
    for command in commands {
        match command {
            Command::Move { queue, .. } | Command::Attack { queue, .. } => assert!(!queue),
            _ => {}
        }
    }
    let (mut player, mut sim) = ReplayPlayer::load(replay).expect("the sample battle rebuilds");
    for _ in 0..30 {
        player.advance(&mut sim, 1.0);
    }
    assert!(sim.ticks() > 0);
    // A plain order written today is the same JSON as before CB.
    let json = serde_json::to_string(&go(vec![0], (1.0, 2.0), false)).unwrap();
    assert!(!json.contains("queue"), "{json}");
}

#[test]
fn queued_battles_are_deterministic_and_the_digest_sees_the_queue() {
    let (a, _) = record_with_queue(42);
    let (b, _) = record_with_queue(42);
    assert_eq!(state_digest(&a), state_digest(&b));

    let mut one = lab_sim(9);
    let mut two = lab_sim(9);
    for sim in [&mut one, &mut two] {
        sim.apply_command(go(vec![0], (500.0, 400.0), false), None)
            .unwrap();
    }
    assert_eq!(state_digest(&one), state_digest(&two));
    one.apply_command(go(vec![0], (600.0, 300.0), true), None)
        .unwrap();
    assert_ne!(state_digest(&one), state_digest(&two), "the queue counts");
}
