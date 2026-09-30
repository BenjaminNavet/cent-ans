//! CB2: unit modes (`data/rules/unit_modes.json`, `sim_battle::modes`).
//! Run makes every move a run; guard stops the pursuit (routing target,
//! broken melee); skirmish steps shooters back from melee troops closing
//! in (on by default for the `skirmish` ability, whose shot on the move goes
//! with it); melee sends shooters into contact; breach makes engines batter
//! walls harder and a little slower (the mangonel less so), never men. The
//! display states (charging, under fire, engaged, wavering) come from the
//! core. Replays with modes replay exactly; without modes, digests and JSON
//! are unchanged. The defensive AI puts its line on guard and its light
//! shooters in skirmish.

mod common;

use common::*;
use data_model::GameData;
use sim_battle::replay::state_digest;
use sim_battle::{
    BattleReplay, BattleSim, Command, CommandError, ReplayAction, ReplayPlayer, ReplayRecorder,
    ReplayStart, SideId, SiegeSetup, UnitMode, UnitModeRules, UnitState, DT,
};

type P = (f64, f64);

fn go(units: Vec<u32>, to: P) -> Command {
    Command::Move {
        units,
        x: to.0,
        z: to.1,
        run: false,
        facing: None,
        queue: false,
        width: None,
        match_speed: false,
        group_tag: None,
    }
}

fn attack(units: Vec<u32>, target: u32) -> Command {
    Command::Attack {
        units,
        target,
        run: true,
        queue: false,
    }
}

fn set_mode(units: Vec<u32>, mode: UnitMode, enabled: bool) -> Command {
    Command::SetMode {
        units,
        mode,
        enabled,
    }
}

fn pos(sim: &BattleSim, id: u32) -> P {
    let u = &sim.units()[id as usize];
    (u.x, u.z)
}

fn dist(a: P, b: P) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

/// Men-at-arms (0), longbowmen (1), mounted archers (2), crossbowmen (3)
/// against militia (4) and men-at-arms (5), far away.
fn lab_sim(seed: u64) -> BattleSim {
    let data = data();
    let mut sim = BattleSim::new(
        setup(
            units(
                &data,
                &[
                    "unit_men_at_arms_foot",
                    "unit_longbowmen",
                    "unit_mounted_archers",
                    "unit_crossbowmen",
                ],
            ),
            units(&data, &["unit_urban_militia", "unit_men_at_arms_foot"]),
            None,
        ),
        seed,
    )
    .unwrap();
    lab(&mut sim);
    hold_fire(&mut sim, SideId::Attacker);
    place(&mut sim, 0, 300.0, 300.0, 0.0);
    place(&mut sim, 1, 450.0, 300.0, 0.0);
    place(&mut sim, 2, 600.0, 300.0, 0.0);
    place(&mut sim, 3, 750.0, 300.0, 0.0);
    place(&mut sim, 4, 1100.0, 750.0, 0.0);
    place(&mut sim, 5, 100.0, 750.0, 0.0);
    sim
}

// ----- availability and commands ------------------------------------------

#[test]
fn set_mode_reads_and_writes_its_json() {
    let command: Command = serde_json::from_str(
        r#"{"type": "set_mode", "units": [1, 2], "mode": "skirmish", "enabled": true}"#,
    )
    .unwrap();
    assert_eq!(command, set_mode(vec![1, 2], UnitMode::Skirmish, true));
    let json = serde_json::to_string(&set_mode(vec![0], UnitMode::Breach, false)).unwrap();
    assert!(
        json.contains("\"set_mode\"") && json.contains("\"breach\""),
        "{json}"
    );
}

#[test]
fn each_mode_goes_to_the_regiments_that_can_take_it() {
    let mut sim = lab_sim(1);
    // Everyone runs and guards.
    sim.apply_command(set_mode(vec![0, 1, 2, 3], UnitMode::Run, true), None)
        .unwrap();
    sim.apply_command(set_mode(vec![0, 1], UnitMode::Guard, true), None)
        .unwrap();
    // Skirmish and melee are for shooters.
    let refused = sim.apply_command(set_mode(vec![0], UnitMode::Skirmish, true), None);
    assert_eq!(
        refused,
        Err(CommandError::ModeUnavailable {
            unit: 0,
            mode: UnitMode::Skirmish
        })
    );
    assert!(refused.unwrap_err().to_string().contains("escarmouche"));
    assert!(sim
        .apply_command(set_mode(vec![0], UnitMode::Melee, true), None)
        .is_err());
    // Breach is for engines, in a siege.
    assert!(sim
        .apply_command(set_mode(vec![1], UnitMode::Breach, true), None)
        .is_err());
    let u = &sim.units()[0];
    assert!(u.mode_run && u.guard && !u.skirmish && !u.melee_mode);
    let modes: Vec<UnitMode> = sim.available_modes(&sim.units()[1]);
    assert_eq!(
        modes,
        vec![
            UnitMode::Run,
            UnitMode::Guard,
            UnitMode::Skirmish,
            UnitMode::Melee
        ]
    );
}

#[test]
fn guard_skirmish_and_melee_exclude_each_other() {
    let mut sim = lab_sim(2);
    sim.apply_command(set_mode(vec![1], UnitMode::Guard, true), None)
        .unwrap();
    sim.apply_command(set_mode(vec![1], UnitMode::Skirmish, true), None)
        .unwrap();
    let u = &sim.units()[1];
    assert!(u.skirmish && !u.guard);
    sim.apply_command(set_mode(vec![1], UnitMode::Melee, true), None)
        .unwrap();
    let u = &sim.units()[1];
    assert!(u.melee_mode && !u.skirmish);
    sim.apply_command(set_mode(vec![1], UnitMode::Guard, true), None)
        .unwrap();
    let u = &sim.units()[1];
    assert!(u.guard && u.melee_mode, "guard and melee go together");
}

#[test]
fn modes_can_be_set_while_deploying() {
    let data = data();
    let mut battle = setup(
        units(&data, &["unit_longbowmen"]),
        units(&data, &["unit_urban_militia"]),
        None,
    );
    battle.player_side = Some(SideId::Attacker);
    let mut sim = BattleSim::new(battle, 3).unwrap();
    if !sim.is_deploying() {
        return;
    }
    sim.issue_command(set_mode(vec![0], UnitMode::Skirmish, true))
        .unwrap();
    assert!(sim.units()[0].skirmish);
}

// ----- run --------------------------------------------------------------

#[test]
fn the_run_mode_runs_every_move_and_the_move_under_way() {
    let mut walk = lab_sim(3);
    let mut run_mode = lab_sim(3);
    run_mode
        .apply_command(set_mode(vec![0], UnitMode::Run, true), None)
        .unwrap();
    for sim in [&mut walk, &mut run_mode] {
        sim.apply_command(go(vec![0], (300.0, 600.0)), None)
            .unwrap();
        run(sim, 10.0);
    }
    assert!(run_mode.units()[0].running && !walk.units()[0].running);
    let walked = dist(pos(&walk, 0), (300.0, 300.0));
    let ran = dist(pos(&run_mode, 0), (300.0, 300.0));
    assert!(ran > walked * 1.5, "ran {ran:.1} m, walked {walked:.1} m");
    // Turned off, the move under way goes back to the walk.
    run_mode
        .apply_command(set_mode(vec![0], UnitMode::Run, false), None)
        .unwrap();
    assert!(!run_mode.units()[0].running);
    // Turned on during a walk, the regiment starts running.
    walk.apply_command(set_mode(vec![0], UnitMode::Run, true), None)
        .unwrap();
    assert!(walk.units()[0].running);
}

// ----- guard ------------------------------------------------------------

/// Men-at-arms (0) attack the militia (4) placed in front of them, which
/// then routs.
fn pursuit(guard: bool) -> BattleSim {
    let mut sim = lab_sim(4);
    place(&mut sim, 4, 300.0, 330.0, std::f64::consts::PI);
    if guard {
        sim.apply_command(set_mode(vec![0], UnitMode::Guard, true), None)
            .unwrap();
    }
    sim.apply_command(attack(vec![0], 4), None).unwrap();
    run(&mut sim, 3.0);
    let militia = &mut sim.units_mut()[4];
    militia.morale = 5.0;
    run(&mut sim, 6.0);
    assert_eq!(sim.units()[4].state, UnitState::Routing);
    sim
}

#[test]
fn guard_does_not_pursue_a_routing_target() {
    let chase = pursuit(false);
    let hold = pursuit(true);
    assert_eq!(chase.units()[0].target, Some(4), "without guard: pursuit");
    assert_eq!(hold.units()[0].target, None, "on guard: no pursuit");
    assert!(hold.units()[0].z < chase.units()[0].z);
}

#[test]
fn guard_holds_once_a_melee_breaks() {
    let mut sims = [lab_sim(5), lab_sim(5)];
    for (k, sim) in sims.iter_mut().enumerate() {
        if k == 1 {
            sim.apply_command(set_mode(vec![0], UnitMode::Guard, true), None)
                .unwrap();
        }
        sim.apply_command(attack(vec![0], 5), None).unwrap();
        // In melee last step, the opponent now pulling out, 60 m away.
        sim.units_mut()[0].state = UnitState::Melee;
        place(sim, 5, 300.0, 360.0, std::f64::consts::PI);
        sim.apply_command(go(vec![5], (300.0, 700.0)), None)
            .unwrap();
        sim.units_mut()[5].disengaging = true;
        sim.step();
    }
    assert_eq!(sims[0].units()[0].target, Some(5), "follows without guard");
    assert_eq!(sims[1].units()[0].target, None, "holds on guard");
    assert_eq!(sims[1].units()[0].state, UnitState::Idle);
}

// ----- skirmish ----------------------------------------------------------

/// Crossbowmen (3) with militia (4) marching at them from 70 m, for 16 s.
fn threatened(skirmish: bool) -> BattleSim {
    let mut sim = lab_sim(6);
    place(&mut sim, 4, 750.0, 370.0, std::f64::consts::PI);
    if skirmish {
        sim.apply_command(set_mode(vec![3], UnitMode::Skirmish, true), None)
            .unwrap();
    }
    sim.apply_command(attack(vec![4], 3), None).unwrap();
    run(&mut sim, 16.0);
    sim
}

#[test]
fn skirmishers_step_back_from_melee_troops_closing_in() {
    let stay = threatened(false);
    let back = threatened(true);
    assert!(
        (stay.units()[3].z - 300.0).abs() < 1.0,
        "without the mode the crossbowmen stand"
    );
    let rules = &UnitModeRules::bundled().skirmish;
    assert!(
        back.units()[3].z < 300.0 - rules.min_retreat_m,
        "stepped back to z {:.1}",
        back.units()[3].z
    );
    // Facing the threat on arrival.
    let facing = back.units()[3]
        .destination_facing
        .unwrap_or(back.units()[3].facing);
    assert!(facing.cos() > 0.9, "facing {facing}");
}

#[test]
fn skirmishers_hold_against_shooters_and_distant_troops() {
    let mut sim = lab_sim(7);
    sim.apply_command(set_mode(vec![3], UnitMode::Skirmish, true), None)
        .unwrap();
    // The militia marches past, 300 m away.
    sim.apply_command(go(vec![4], (1000.0, 600.0)), None)
        .unwrap();
    run(&mut sim, 6.0);
    assert!((sim.units()[3].z - 300.0).abs() < 1.0);
}

#[test]
fn the_skirmish_ability_starts_the_mode_and_its_shot_on_the_move() {
    let mut sim = lab_sim(8);
    let archers = &sim.units()[2];
    assert!(archers.skirmish, "horse archers start skirmishing");
    assert!(archers.shoots_on_move());
    assert!(!sim.units()[1].skirmish && !sim.units()[1].shoots_on_move());
    sim.apply_command(set_mode(vec![2], UnitMode::Skirmish, false), None)
        .unwrap();
    assert!(
        !sim.units()[2].shoots_on_move(),
        "the shot goes with the mode"
    );
    // Longbowmen in skirmish still do not shoot on the move (no ability).
    sim.apply_command(set_mode(vec![1], UnitMode::Skirmish, true), None)
        .unwrap();
    assert!(!sim.units()[1].shoots_on_move());
}

// ----- melee -------------------------------------------------------------

#[test]
fn melee_mode_sends_shooters_into_contact() {
    let mut shoot = lab_sim(9);
    let mut close = lab_sim(9);
    close
        .apply_command(set_mode(vec![1], UnitMode::Melee, true), None)
        .unwrap();
    for sim in [&mut shoot, &mut close] {
        place(sim, 5, 450.0, 420.0, std::f64::consts::PI);
        sim.apply_command(attack(vec![1], 5), None).unwrap();
        run(sim, 4.0);
    }
    assert_eq!(shoot.units()[1].state, UnitState::Shooting);
    assert!((shoot.units()[1].z - 300.0).abs() < 1.0);
    let ammo = close.units()[1].ammo;
    assert_eq!(ammo, close.units()[1].stats.ammo, "no volley in melee mode");
    assert!(close.units()[1].z > 310.0, "closes in");
    run(&mut close, 20.0);
    assert!(matches!(
        close.units()[1].state,
        UnitState::Melee | UnitState::Charging | UnitState::Routing
    ));
}

// ----- breach -------------------------------------------------------------

const BESIEGERS: [&str; 3] = ["unit_men_at_arms_foot", "unit_trebuchet", "unit_mangonel"];

fn siege_sim(data: &GameData, breach: bool) -> (BattleSim, usize) {
    let battle = setup(
        units(data, &BESIEGERS),
        units(data, &["unit_urban_militia"]),
        Some(SiegeSetup {
            fortification: 3,
            breach: 0,
            ..Default::default()
        }),
    );
    let mut sim = BattleSim::new(battle, 21).unwrap();
    lab(&mut sim);
    let works = sim.siege().unwrap().clone();
    let piece = works.front_walls()[0];
    let (mx, mz) = works.pieces[piece].midpoint();
    let (nx, nz) = works.pieces[piece].outward();
    for (k, id) in [1u32, 2].into_iter().enumerate() {
        let side = (k as f64 - 0.5) * 30.0;
        place(
            &mut sim,
            id,
            mx + nx * 150.0 + nz * side,
            mz + nz * 150.0 - nx * side,
            (-nx).atan2(-nz),
        );
    }
    // The garrison far inside, out of the engines' way.
    let (cx, cz) = works.center;
    place(&mut sim, 3, cx, cz, 0.0);
    if breach {
        sim.apply_command(set_mode(vec![1, 2], UnitMode::Breach, true), None)
            .unwrap();
    }
    (sim, piece)
}

/// Wall damage done by the engine `id` alone over `seconds`.
fn battered(data: &GameData, id: u32, breach: bool, seconds: f64) -> f64 {
    let (mut sim, piece) = siege_sim(data, breach);
    let other = if id == 1 { 2 } else { 1 };
    sim.apply_command(
        Command::FireAtWill {
            units: vec![other],
            enabled: false,
        },
        None,
    )
    .unwrap();
    if breach {
        sim.apply_command(set_mode(vec![other], UnitMode::Breach, false), None)
            .unwrap();
    }
    sim.apply_command(
        Command::TargetWall {
            units: vec![id],
            piece,
        },
        None,
    )
    .unwrap();
    let before = sim.siege().unwrap().pieces[piece].hp;
    run(&mut sim, seconds);
    before - sim.siege().unwrap().pieces[piece].hp
}

#[test]
fn breach_batters_walls_harder_the_mangonel_less_so() {
    let data = data();
    let seconds = 300.0;
    let trebuchet = battered(&data, 1, false, seconds);
    let trebuchet_breach = battered(&data, 1, true, seconds);
    let mangonel = battered(&data, 2, false, seconds);
    let mangonel_breach = battered(&data, 2, true, seconds);
    assert!(trebuchet > 0.0 && mangonel > 0.0);
    let heavy = trebuchet_breach / trebuchet;
    let light = mangonel_breach / mangonel;
    println!(
        "trebuchet {trebuchet:.0} -> {trebuchet_breach:.0}, mangonel {mangonel:.0} -> {mangonel_breach:.0}"
    );
    assert!(heavy > 1.1, "trebuchet ×{heavy:.2}");
    assert!(
        light > 1.0 && light < heavy,
        "mangonel ×{light:.2} < ×{heavy:.2}"
    );
}

/// Shots of the trebuchet (1) over a minute, 450 m out from the walls
/// (none in range) with a garrison regiment (3) 150 m in front of it.
fn engine_shots_at_men(breach: bool) -> Vec<sim_battle::ShotEvent> {
    let data = data();
    let (mut sim, piece) = siege_sim(&data, breach);
    let works = sim.siege().unwrap().clone();
    let (mx, mz) = works.pieces[piece].midpoint();
    let (nx, nz) = works.pieces[piece].outward();
    let (ex, ez) = (mx + nx * 450.0, mz + nz * 450.0);
    place(&mut sim, 1, ex, ez, (-nx).atan2(-nz));
    place(&mut sim, 2, ex + 600.0, ez, 0.0);
    place(&mut sim, 3, ex - nx * 150.0, ez - nz * 150.0, 0.0);
    sim.take_shots();
    run(&mut sim, 60.0);
    sim.take_shots()
        .into_iter()
        .filter(|s| s.shooter == 1)
        .collect()
}

#[test]
fn breach_is_for_siege_engines_and_never_shoots_men() {
    let data = data();
    let (mut sim, _) = siege_sim(&data, true);
    // Not for the foot, nor outside a siege (see above).
    assert!(sim
        .apply_command(set_mode(vec![0], UnitMode::Breach, true), None)
        .is_err());
    assert!(sim.units()[1].breach && sim.units()[2].breach);
    // At will, the engine shoots the men in range; battering, never.
    assert!(!engine_shots_at_men(false).is_empty(), "shoots men at will");
    assert!(
        engine_shots_at_men(true).is_empty(),
        "battering: walls only"
    );
    // An attack order is the player's call: it ends the mode.
    sim.apply_command(attack(vec![1], 3), None).unwrap();
    assert!(!sim.units()[1].breach);
    assert_eq!(sim.units()[1].target, Some(3));
}

// ----- states ------------------------------------------------------------

#[test]
fn the_core_computes_the_display_states() {
    let mut sim = lab_sim(10);
    let rules = &UnitModeRules::bundled().status;
    assert_eq!(sim.unit_status(&sim.units()[0]), Default::default());
    {
        let u = &mut sim.units_mut()[0];
        u.missile_timer = rules.under_fire_seconds * 0.5;
        u.morale = rules.wavering_morale - 1.0;
        u.state = UnitState::Charging;
    }
    let status = sim.unit_status(&sim.units()[0]);
    assert!(status.under_fire && status.wavering && status.charging && !status.engaged);
    let u = &mut sim.units_mut()[0];
    u.state = UnitState::Routing;
    u.missile_timer = rules.under_fire_seconds * 2.0;
    let status = sim.unit_status(&sim.units()[0]);
    assert!(
        !status.wavering && !status.under_fire,
        "a rout is not a waver"
    );
}

// ----- replay ------------------------------------------------------------

/// Records the 1337 demo with modes set during a minute of battle.
fn record_with_modes(seed: u64) -> (BattleSim, BattleReplay) {
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
    let crossbows: Vec<u32> = ours
        .iter()
        .copied()
        .filter(|&id| sim.units()[id as usize].can_shoot())
        .collect();
    for command in [
        set_mode(ours.clone(), UnitMode::Run, true),
        set_mode(ours[..2].to_vec(), UnitMode::Guard, true),
        set_mode(crossbows, UnitMode::Skirmish, true),
    ] {
        drive(&mut sim, &mut recorder, ReplayAction::Command { command });
    }
    let (x, z) = pos(&sim, ours[0]);
    drive(
        &mut sim,
        &mut recorder,
        ReplayAction::Command {
            command: go(ours[..3].to_vec(), (x, z + 80.0)),
        },
    );
    for _ in 0..600 {
        sim.tick(DT);
        recorder.observe(&sim);
    }
    (sim, recorder.finish("Modes", 1_790_000_000))
}

#[test]
fn a_replay_with_modes_replays_exactly() {
    let (live, replay) = record_with_modes(1337);
    let text = replay.to_json();
    assert!(text.contains("\"set_mode\""), "mode commands recorded");
    let read = BattleReplay::from_json(&text).unwrap();
    assert_eq!(read, replay);
    let (mut player, mut sim) = ReplayPlayer::load(read).unwrap();
    while player.advance(&mut sim, 1.0) > 0 {}
    assert!(player.divergence().is_none(), "{:?}", player.divergence());
    assert_eq!(sim.ticks(), live.ticks());
    assert_eq!(state_digest(&sim), state_digest(&live));
    let (again, _) = record_with_modes(1337);
    assert_eq!(state_digest(&again), state_digest(&live), "deterministic");
}

#[test]
fn the_digest_sees_what_modes_do_not_the_flags() {
    let mut one = lab_sim(11);
    let mut two = lab_sim(11);
    assert_eq!(state_digest(&one), state_digest(&two));
    // A flag alone changes nothing of the battle: same digest (replays
    // recorded before CB2, whose AI now guards its line, still match).
    one.apply_command(set_mode(vec![0], UnitMode::Guard, true), None)
        .unwrap();
    one.apply_command(set_mode(vec![0], UnitMode::Run, true), None)
        .unwrap();
    assert_eq!(state_digest(&one), state_digest(&two));
    // What it does shows: the run mode runs the same move.
    for sim in [&mut one, &mut two] {
        sim.apply_command(go(vec![0], (300.0, 600.0)), None)
            .unwrap();
        run(sim, 2.0);
    }
    assert_ne!(state_digest(&one), state_digest(&two), "the run counts");
    // Unit JSON: modes left out when off.
    let json = serde_json::to_string(&lab_sim(11).units()[0]).unwrap();
    for key in ["mode_run", "guard", "skirmish", "melee_mode", "breach"] {
        assert!(!json.contains(&format!("\"{key}\"")), "{key} in {json}");
    }
}

#[test]
fn a_replay_recorded_before_cb2_still_reads() {
    let text = include_str!("fixtures/replay_sample.json");
    assert!(!text.contains("set_mode"));
    let replay = BattleReplay::from_json(text).expect("the pre-CB sample reads");
    let (mut player, mut sim) = ReplayPlayer::load(replay).expect("the sample battle rebuilds");
    while player.advance(&mut sim, 1.0) > 0 {}
    // The sample parts from today's rules at tick 1120 on `main` before CB2
    // (see `cb1_width.rs`): CB2 must not make it part any earlier.
    // IA night: the attacker's horse under the arrows now waits behind its
    // foot; the sample parts from the rules at tick 704 (1 min 10 s).
    assert!(
        player.divergence().is_none_or(|d| d.tick >= 704),
        "{:?}",
        player.divergence()
    );
}

// ----- AI ----------------------------------------------------------------

/// A weak English side (two men-at-arms, horse archers) facing a strong
/// French one: the English AI stands on the defensive.
fn defensive_sim(seed: u64) -> BattleSim {
    let data = data();
    let french = [
        "unit_knights",
        "unit_knights",
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_crossbowmen",
        "unit_crossbowmen",
    ];
    let english = [
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_mounted_archers",
    ];
    let mut battle = setup(units(&data, &french), units(&data, &english), None);
    battle.village = Some(false);
    let mut sim = BattleSim::new(battle, seed).unwrap();
    sim.set_ai(SideId::Attacker, true);
    sim.set_ai(SideId::Defender, true);
    sim
}

#[test]
fn the_defensive_ai_guards_its_line_and_skirmishes_its_light_shooters() {
    let mut sim = defensive_sim(12);
    let english: Vec<u32> = sim
        .units()
        .iter()
        .filter(|u| u.side == SideId::Defender)
        .map(|u| u.id)
        .collect();
    let horse_archers = *english
        .iter()
        .find(|&&id| sim.units()[id as usize].can_shoot())
        .unwrap();
    sim.apply_command(
        set_mode(vec![horse_archers], UnitMode::Skirmish, false),
        None,
    )
    .unwrap();
    run(&mut sim, 5.0);
    for &id in &english {
        let u = &sim.units()[id as usize];
        if u.can_shoot() {
            assert!(u.skirmish, "light shooters skirmish");
        } else {
            assert!(u.guard, "{} on guard", u.name);
        }
    }
    // The advancing French line is not on guard.
    assert!(sim
        .units()
        .iter()
        .filter(|u| u.side == SideId::Attacker)
        .all(|u| !u.guard));
}

// ----- reference battles (probe) -------------------------------------------

fn historical_start(id: &str, seed: u64) -> BattleSim {
    use std::path::PathBuf;
    static DATA: std::sync::OnceLock<GameData> = std::sync::OnceLock::new();
    let data = DATA.get_or_init(data);
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../data/battle_maps")
        .join(format!("{id}.json"));
    let map =
        sim_battle::HistoricalMap::from_json(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut setup = map
        .battle_setup(
            &data.unit_types,
            data.battle_orders.values().cloned().collect(),
            Some(data.battle_standard_rules.clone()),
            None,
        )
        .unwrap();
    // CB4: the regiments' abilities, as in the game.
    setup.abilities = data.battle_abilities.values().cloned().collect();
    map.start(setup, seed).unwrap()
}

/// Probe (ignored): the margins of the reference battles, as asserted by
/// `ep7_historical` (English wins over seeds 1-20) and `eq7_cavalry`
/// (French wins over seeds 0-15).
/// `cargo test --release -p sim-battle --test cb2_modes -- --ignored --nocapture margins`
#[test]
#[ignore = "probe"]
fn reference_margins() {
    for id in ["crecy", "azincourt", "poitiers"] {
        let wins = (1..21)
            .filter(|&seed| {
                let mut sim = historical_start(id, seed);
                let steps = (2400.0 / DT).round() as u64;
                for _ in 0..steps {
                    if sim.is_finished() {
                        break;
                    }
                    sim.step();
                }
                sim.winner() == Some(SideId::Defender)
            })
            .count();
        println!("{id}: English {wins}/20");
    }
    let data = data();
    let french = (0..16)
        .filter(|&seed| {
            let mut battle = setup(
                units(
                    &data,
                    &[
                        "unit_knights",
                        "unit_men_at_arms_foot",
                        "unit_men_at_arms_foot",
                        "unit_crossbowmen",
                        "unit_crossbowmen",
                        "unit_knights",
                    ],
                ),
                units(
                    &data,
                    &[
                        "unit_men_at_arms_foot",
                        "unit_longbowmen",
                        "unit_longbowmen",
                        "unit_knights",
                    ],
                ),
                None,
            );
            battle.village = Some(false);
            battle.abilities = data.battle_abilities.values().cloned().collect();
            let mut sim = BattleSim::new(battle, seed).unwrap();
            sim.set_ai(SideId::Attacker, true);
            sim.set_ai(SideId::Defender, true);
            run_to_end(&mut sim);
            sim.winner() == Some(SideId::Attacker)
        })
        .count();
    println!("eq7: French {french}/16");
}
