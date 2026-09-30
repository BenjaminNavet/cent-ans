//! CB1: formation width set by a right-drag, and grouped pace. A `Move`
//! with `width` forms a Line that wide (files = width / spacing), its
//! ranks within the bounds of `data/rules/formation_width.json` (pikemen at
//! least 4, archers at least 2), every soldier kept; without width the
//! formation is unchanged. A group drag shares the width in proportion to
//! strength. `match_speed` keeps a group at the pace of its slowest
//! regiment. Queued orders carry the width; replays recorded before CB1
//! still read, and plain orders write the same JSON as before.

mod common;

use common::*;
use sim_battle::replay::state_digest;
use sim_battle::{
    split_widths, BattleReplay, BattleSim, Command, Formation, FormationWidthRules, QueuedOrder,
    ReplayAction, ReplayPlayer, ReplayRecorder, ReplayStart, SideId, DT,
};

type P = (f64, f64);

fn go(units: Vec<u32>, to: P, facing: Option<f64>, width: Option<f64>) -> Command {
    Command::Move {
        units,
        x: to.0,
        z: to.1,
        run: false,
        facing,
        queue: false,
        width,
        match_speed: false,
        group_tag: None,
    }
}

/// Men-at-arms (0), pikemen (1), longbowmen (2), knights (3) against
/// militia far away (4).
fn lab_sim(seed: u64) -> BattleSim {
    let data = data();
    let mut sim = BattleSim::new(
        setup(
            units(
                &data,
                &[
                    "unit_men_at_arms_foot",
                    "unit_flemish_pikemen",
                    "unit_longbowmen",
                    "unit_knights",
                ],
            ),
            units(&data, &["unit_urban_militia"]),
            None,
        ),
        seed,
    )
    .unwrap();
    lab(&mut sim);
    hold_fire(&mut sim, SideId::Attacker);
    place(&mut sim, 0, 300.0, 300.0, 0.0);
    place(&mut sim, 1, 400.0, 300.0, 0.0);
    place(&mut sim, 2, 500.0, 300.0, 0.0);
    place(&mut sim, 3, 600.0, 300.0, 0.0);
    place(&mut sim, 4, 1100.0, 900.0, 0.0);
    sim
}

fn ranks_files(sim: &BattleSim, id: u32) -> (u32, u32) {
    let u = &sim.units()[id as usize];
    u.ranks_files(u.soldiers())
}

#[test]
fn a_width_sets_the_files_of_the_line() {
    let mut sim = lab_sim(1);
    let n = sim.units()[0].soldiers();
    let width = 44.0; // 40 files of 1.1 m
    sim.apply_command(go(vec![0], (300.0, 360.0), Some(0.0), Some(width)), None)
        .unwrap();
    let u = &sim.units()[0];
    assert_eq!(u.formation, Formation::Line);
    let files = u.line_files.expect("a width sets the files");
    let (ranks, f) = ranks_files(&sim, 0);
    assert_eq!(f, files);
    assert_eq!(ranks, n.div_ceil(files), "ranks follow from the width");
    assert!((u.extent().0 - width).abs() <= 1.1, "{:?}", u.extent());
}

#[test]
fn widths_out_of_bounds_are_brought_back_and_every_soldier_kept() {
    let rules = FormationWidthRules::bundled();
    for (id, name) in [
        (0u32, "infantry"),
        (1, "pikemen"),
        (2, "ranged"),
        (3, "cavalry"),
    ] {
        for width in [0.5, 3.0, 20.0, 80.0, 400.0, 5000.0] {
            let mut sim = lab_sim(2);
            sim.apply_command(go(vec![id], (300.0, 400.0), None, Some(width)), None)
                .unwrap();
            let u = &sim.units()[id as usize];
            let bounds = rules.bounds(u);
            let n = u.soldiers();
            let (ranks, files) = ranks_files(&sim, id);
            assert!(ranks * files >= n, "{name} {width}: {ranks}×{files} < {n}");
            assert!(
                ranks >= bounds.min_ranks.min(n) && ranks <= bounds.max_ranks,
                "{name} {width}: {ranks} ranks outside {bounds:?}"
            );
            assert_eq!(u.soldier_positions().len() as u32, n, "{name} {width}");
        }
    }
}

#[test]
fn pikemen_keep_four_ranks_and_archers_two() {
    let mut sim = lab_sim(3);
    sim.apply_command(go(vec![1], (400.0, 400.0), None, Some(5000.0)), None)
        .unwrap();
    sim.apply_command(go(vec![2], (500.0, 400.0), None, Some(5000.0)), None)
        .unwrap();
    assert!(ranks_files(&sim, 1).0 >= 4, "{:?}", ranks_files(&sim, 1));
    assert!(ranks_files(&sim, 2).0 >= 2, "{:?}", ranks_files(&sim, 2));
}

#[test]
fn without_width_the_formation_is_unchanged() {
    let mut sim = lab_sim(4);
    let before: Vec<_> = sim
        .units()
        .iter()
        .map(|u| (u.formation, u.extent()))
        .collect();
    sim.apply_command(go(vec![0, 1, 2, 3], (450.0, 420.0), Some(0.0), None), None)
        .unwrap();
    let after: Vec<_> = sim
        .units()
        .iter()
        .map(|u| (u.formation, u.extent()))
        .collect();
    assert_eq!(before, after);
    assert!(sim.units().iter().all(|u| u.line_files.is_none()));
    // A width, then a plain order: the width stays; a formation order
    // brings back the default depth.
    sim.apply_command(go(vec![0], (300.0, 400.0), None, Some(60.0)), None)
        .unwrap();
    let wide = sim.units()[0].extent();
    sim.apply_command(go(vec![0], (320.0, 400.0), None, None), None)
        .unwrap();
    assert_eq!(sim.units()[0].extent(), wide);
    sim.apply_command(
        Command::Formation {
            units: vec![0],
            kind: Formation::Line,
        },
        None,
    )
    .unwrap();
    assert_eq!(sim.units()[0].line_files, None);
    assert_eq!(sim.units()[0].extent(), before[0].1);
}

#[test]
fn a_group_drag_shares_the_width_by_strength() {
    let mut sim = lab_sim(5);
    let total = 160.0;
    sim.apply_command(go(vec![0, 2], (400.0, 500.0), Some(0.0), Some(total)), None)
        .unwrap();
    let counts = [sim.units()[0].soldiers(), sim.units()[2].soldiers()];
    let gap = FormationWidthRules::bundled().group_gap_m;
    let shares = split_widths(&counts, total, gap);
    for (k, id) in [0u32, 2].into_iter().enumerate() {
        let u = &sim.units()[id as usize];
        assert_eq!(u.extent().0, u.extent_for_width(Some(shares[k])).0);
        assert!(
            (u.extent().0 - shares[k]).abs() < 2.5,
            "{} vs {}",
            u.extent().0,
            shares[k]
        );
    }
    // Left to right along the line, the gap between neighbours kept.
    let (a, b) = (&sim.units()[0], &sim.units()[2]);
    let (da, db) = (a.destination.unwrap(), b.destination.unwrap());
    let apart = (db.0 - da.0).abs();
    assert!(
        (apart - (a.extent().0 + b.extent().0) * 0.5 - 10.0).abs() < 1e-6,
        "{apart}"
    );
}

#[test]
fn the_preview_shows_the_frontage_taken() {
    let mut sim = lab_sim(6);
    let legs = sim.preview_group_width(&[0, 2], 400.0, 500.0, Some(0.0), Some(160.0), false);
    sim.apply_command(go(vec![0, 2], (400.0, 500.0), Some(0.0), Some(160.0)), None)
        .unwrap();
    for leg in legs {
        let u = &sim.units()[leg.unit.unwrap() as usize];
        assert_eq!(leg.extent, Some(u.extent()));
        assert_eq!(leg.path.unwrap().last().copied(), u.destination);
    }
}

fn distance_walked(match_speed: bool) -> (f64, f64) {
    let mut sim = lab_sim(7);
    let order = Command::Move {
        units: vec![2, 3],
        x: 550.0,
        z: 700.0,
        run: false,
        facing: None,
        queue: false,
        width: None,
        match_speed,
        group_tag: None,
    };
    sim.apply_command(order, None).unwrap();
    let start: Vec<P> = sim.units().iter().map(|u| (u.x, u.z)).collect();
    run(&mut sim, 20.0);
    let d = |id: usize| (sim.units()[id].x - start[id].0).hypot(sim.units()[id].z - start[id].1);
    (d(2), d(3))
}

#[test]
fn match_speed_keeps_the_pace_of_the_slowest() {
    let (bows, knights) = distance_walked(false);
    assert!(
        knights > bows * 1.1,
        "knights {knights} faster than bows {bows}"
    );
    let (bows_m, knights_m) = distance_walked(true);
    assert!((bows_m - bows).abs() < 1e-6, "the slowest walks as before");
    assert!(
        (knights_m - bows_m).abs() < bows_m * 0.03,
        "knights {knights_m} keep the bows' pace {bows_m}"
    );
}

#[test]
fn locked_group_moves_share_a_tag() {
    let mut sim = lab_sim(8);
    for (id, x) in [(2u32, 500.0), (3, 600.0)] {
        let order = Command::Move {
            units: vec![id],
            x,
            z: 700.0,
            run: false,
            facing: Some(0.0),
            queue: false,
            width: None,
            match_speed: true,
            group_tag: Some(7),
        };
        sim.apply_command(order, None).unwrap();
    }
    assert_eq!(sim.units()[2].group_tag, Some(7));
    assert_eq!(sim.units()[3].group_tag, Some(7));
    let start = (sim.units()[2].z, sim.units()[3].z);
    run(&mut sim, 20.0);
    let bows = sim.units()[2].z - start.0;
    let knights = sim.units()[3].z - start.1;
    assert!(bows > 10.0);
    assert!((knights - bows).abs() < bows * 0.03, "{knights} vs {bows}");
    // Another order leaves the group.
    sim.apply_command(Command::Halt { units: vec![3] }, None)
        .unwrap();
    assert_eq!(sim.units()[3].group_tag, None);
    assert!(!sim.units()[3].match_speed);
}

#[test]
fn a_queued_move_carries_its_width() {
    let mut sim = lab_sim(9);
    sim.apply_command(go(vec![0], (300.0, 330.0), None, None), None)
        .unwrap();
    let queued = Command::Move {
        units: vec![0],
        x: 300.0,
        z: 380.0,
        run: false,
        facing: Some(0.0),
        queue: true,
        width: Some(70.0),
        match_speed: false,
        group_tag: None,
    };
    sim.apply_command(queued, None).unwrap();
    assert!(matches!(
        sim.units()[0].order_queue[0],
        QueuedOrder::Move { width: Some(w), .. } if w == 70.0
    ));
    assert_eq!(
        sim.units()[0].line_files,
        None,
        "not before the order starts"
    );
    for _ in 0..(90.0 / DT) as usize {
        sim.step();
        if sim.units()[0].line_files.is_some() {
            break;
        }
    }
    let u = &sim.units()[0];
    assert!(
        u.line_files.is_some(),
        "the width applies when the order starts"
    );
    assert_eq!(u.destination, Some((300.0, 380.0)));
}

#[test]
fn deployment_takes_a_width() {
    let data = data();
    let army = ["unit_men_at_arms_foot", "unit_longbowmen"];
    let mut s = setup(units(&data, &army), units(&data, &army), None);
    s.player_side = Some(SideId::Attacker);
    let mut sim = BattleSim::new(s, 4).unwrap();
    assert!(sim.begin_deployment());
    let zone = sim.deployment_zone(SideId::Attacker);
    sim.deploy_unit_width(0, 300.0, zone.z1 - 10.0, Some(0.0), Some(80.0))
        .unwrap();
    let u = &sim.units()[0];
    assert!(u.line_files.is_some());
    assert!((u.extent().0 - 80.0).abs() <= 1.1);
}

/// Records the 1337 demo with dragged, grouped and queued orders.
fn record_with_widths(seed: u64) -> (BattleSim, BattleReplay) {
    let setup = serde_json::from_str(include_str!("fixtures/demo_battle_1337.json")).unwrap();
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
    let first = (
        sim.units()[ours[0] as usize].x,
        sim.units()[ours[0] as usize].z,
    );
    let orders = [
        Command::Move {
            units: ours[..2].to_vec(),
            x: first.0,
            z: first.1 + 40.0,
            run: false,
            facing: Some(0.0),
            queue: false,
            width: Some(150.0),
            match_speed: true,
            group_tag: None,
        },
        Command::Move {
            units: vec![ours[0]],
            x: first.0 + 30.0,
            z: first.1 + 80.0,
            run: false,
            facing: Some(0.2),
            queue: true,
            width: Some(40.0),
            match_speed: true,
            group_tag: Some(3),
        },
    ];
    for command in orders {
        drive(&mut sim, &mut recorder, ReplayAction::Command { command });
    }
    for _ in 0..600 {
        sim.tick(DT);
        recorder.observe(&sim);
    }
    (sim, recorder.finish("Largeur au glisser", 1_790_000_000))
}

#[test]
fn a_replay_with_widths_replays_exactly() {
    let (live, replay) = record_with_widths(1337);
    let text = replay.to_json();
    assert!(text.contains("\"width\":150.0"), "widths recorded");
    assert!(text.contains("\"match_speed\":true"));
    let read = BattleReplay::from_json(&text).unwrap();
    assert_eq!(read, replay);
    let (mut player, mut sim) = ReplayPlayer::load(read).unwrap();
    while player.advance(&mut sim, 1.0) > 0 {}
    assert!(player.divergence().is_none(), "{:?}", player.divergence());
    assert_eq!(state_digest(&sim), state_digest(&live));
    let (again, _) = record_with_widths(1337);
    assert_eq!(state_digest(&again), state_digest(&live));
}

#[test]
fn a_replay_recorded_before_cb1_still_reads() {
    let text = include_str!("fixtures/replay_sample.json");
    assert!(!text.contains("match_speed") && !text.contains("group_tag"));
    let replay = BattleReplay::from_json(text).expect("the pre-CB sample reads");
    for entry in &replay.actions {
        if let ReplayAction::Command {
            command:
                Command::Move {
                    width,
                    match_speed,
                    group_tag,
                    ..
                },
        } = &entry.action
        {
            assert_eq!((*width, *match_speed, *group_tag), (None, false, None));
        }
    }
    let (mut player, mut sim) = ReplayPlayer::load(replay).expect("the sample battle rebuilds");
    while player.advance(&mut sim, 1.0) > 0 {}
    // The sample already parts from today's rules at tick 1120 (1 min 52 s)
    // on `main` before CB1 (rule changes since EP13): CB1 must not make it
    // part any earlier.
    // IA night: the attacker's horse under the arrows now waits behind its
    // foot; the sample parts from the rules at tick 704 (1 min 10 s).
    assert!(
        player.divergence().is_none_or(|d| d.tick >= 704),
        "{:?}",
        player.divergence()
    );
    // A plain order written today is the same JSON as before CB1.
    let json = serde_json::to_string(&go(vec![0], (1.0, 2.0), None, None)).unwrap();
    assert!(
        !json.contains("width") && !json.contains("match_speed") && !json.contains("group_tag"),
        "{json}"
    );
}
