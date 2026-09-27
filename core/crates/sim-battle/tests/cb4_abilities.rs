//! CB4: active abilities of the regiments (plan
//! `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` § CB4,
//! `docs/research/cb4-capacites.md`): catalogue, cooldown, conditions, end
//! with its reason, each effect, replay determinism, AI.

mod common;

use common::*;
use data_model::{AbilityKind, BattleAbility, GameData};
use sim_battle::{BattleSim, SideId};

fn game_data() -> &'static GameData {
    static DATA: std::sync::OnceLock<GameData> = std::sync::OnceLock::new();
    DATA.get_or_init(data)
}

#[test]
fn the_catalogue_has_the_five_abilities_of_the_historian() {
    let data = game_data();
    let mut kinds: Vec<AbilityKind> = data.battle_abilities.values().map(|a| a.kind).collect();
    kinds.sort_by_key(|k| format!("{k:?}"));
    assert_eq!(
        kinds,
        vec![
            AbilityKind::AimedShot,
            AbilityKind::BannerRally,
            AbilityKind::CloseRanks,
            AbilityKind::Pavise,
            AbilityKind::PlantedPikes,
        ]
    );
    assert!(!data.battle_orders.contains_key("order_pavise"));
}

// ----- lab ------------------------------------------------------------------

use std::f64::consts::{FRAC_PI_2, PI};

use sim_battle::replay::state_digest;
use sim_battle::{
    AbilityView, BattleReplay, Command, CommandError, ImpactKind, ReplayAction, ReplayPlayer,
    ReplayRecorder, ReplayStart, UnitState, DT,
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

fn use_ability(units: Vec<u32>, ability: &str) -> Command {
    Command::UseAbility {
        units,
        ability: ability.to_owned(),
    }
}

fn pos(sim: &BattleSim, id: u32) -> P {
    let u = &sim.units()[id as usize];
    (u.x, u.z)
}

fn dist(a: P, b: P) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

fn views(sim: &BattleSim, id: u32) -> Vec<AbilityView> {
    sim.ability_views(&sim.units()[id as usize])
}

fn view(sim: &BattleSim, id: u32, ability: &str) -> AbilityView {
    views(sim, id)
        .into_iter()
        .find(|v| v.id == ability)
        .unwrap_or_else(|| panic!("unit {id} has no {ability}"))
}

fn refusal(result: Result<(), CommandError>) -> String {
    match result {
        Err(CommandError::AbilityUnavailable { reason, .. }) => reason,
        other => panic!("expected a refused ability, got {other:?}"),
    }
}

/// Crossbowmen (0), longbowmen (1), knights (2), men-at-arms (3), Flemish
/// militia with goedendags (4) against longbowmen (5), knights (6) and
/// militia (7), all far apart, AI off, abilities in the setup.
fn lab_sim(seed: u64) -> BattleSim {
    let data = game_data();
    let mut battle = setup(
        units(
            data,
            &[
                "unit_crossbowmen",
                "unit_longbowmen",
                "unit_knights",
                "unit_men_at_arms_foot",
                "unit_goedendag_militia",
            ],
        ),
        units(
            data,
            &["unit_longbowmen", "unit_knights", "unit_urban_militia"],
        ),
        None,
    );
    battle.abilities = data.battle_abilities.values().cloned().collect();
    let mut sim = BattleSim::new(battle, seed).unwrap();
    lab(&mut sim);
    hold_fire(&mut sim, SideId::Attacker);
    hold_fire(&mut sim, SideId::Defender);
    for (id, x) in [(0, 150.0), (1, 350.0), (2, 550.0), (3, 750.0), (4, 950.0)] {
        place(&mut sim, id, x, 150.0, 0.0);
    }
    for (id, x) in [(5, 150.0), (6, 550.0), (7, 950.0)] {
        place(&mut sim, id, x, 850.0, PI);
    }
    sim
}

#[test]
fn use_ability_reads_and_writes_its_json() {
    let command: Command = serde_json::from_str(
        r#"{"type": "use_ability", "units": [0, 4], "ability": "ability_pavise"}"#,
    )
    .unwrap();
    assert_eq!(command, use_ability(vec![0, 4], "ability_pavise"));
    let json = serde_json::to_value(&command).unwrap();
    assert_eq!(json["type"], "use_ability");
    // Units without abilities in use leave no trace in their JSON.
    let sim = lab_sim(1);
    let unit = serde_json::to_string(&sim.units()[0]).unwrap();
    assert!(!unit.contains("ability_state"), "{unit}");
}

#[test]
fn each_regiment_gets_the_abilities_of_its_kind() {
    let mut sim = lab_sim(1);
    let ids = |sim: &BattleSim, id: u32| -> Vec<String> {
        views(sim, id).into_iter().map(|v| v.id).collect()
    };
    assert_eq!(ids(&sim, 0), ["ability_pavise"]);
    assert_eq!(ids(&sim, 1), ["ability_aimed_shot"]);
    assert_eq!(ids(&sim, 2), ["ability_banner_rally"]);
    assert_eq!(ids(&sim, 3), ["ability_close_ranks"]);
    // By rank: the pikes first, then the close ranks.
    assert_eq!(
        ids(&sim, 4),
        ["ability_planted_pikes", "ability_close_ranks"]
    );
    assert_eq!(ids(&sim, 7), ["ability_close_ranks"]);
    for v in views(&sim, 4) {
        assert!(v.available && !v.active && v.reason.is_empty(), "{v:?}");
    }
    // Dismounted knights close their ranks, and no longer rally on horseback.
    sim.units_mut()[2].dismount(35, 0);
    assert_eq!(ids(&sim, 2), ["ability_close_ranks"]);
    // No catalogue, no ability (older replays, hand-made setups).
    let bare = BattleSim::new(
        setup(
            units(game_data(), &["unit_crossbowmen"]),
            units(game_data(), &["unit_longbowmen"]),
            None,
        ),
        1,
    )
    .unwrap();
    assert!(bare.ability_views(&bare.units()[0]).is_empty());
}

#[test]
fn a_regiment_without_the_ability_cannot_use_it() {
    let mut sim = lab_sim(1);
    let why = refusal(sim.apply_command(use_ability(vec![3], "ability_pavise"), None));
    assert_eq!(why, "aucune des unités désignées n'a cette capacité");
    assert!(matches!(
        sim.apply_command(use_ability(vec![0], "ability_fireball"), None),
        Err(CommandError::UnknownAbility(_))
    ));
    // A mixed selection: only those that have it use it.
    sim.apply_command(use_ability(vec![0, 3], "ability_pavise"), None)
        .unwrap();
    assert!(view(&sim, 0, "ability_pavise").active);
    assert!(sim.units()[3].ability_state.active.is_none());
}

#[test]
fn pavises_halt_plant_then_lift_at_the_first_move_and_recharge() {
    let mut sim = lab_sim(1);
    sim.apply_command(go(vec![0], (150.0, 400.0)), None)
        .unwrap();
    run(&mut sim, 1.0);
    sim.apply_command(use_ability(vec![0], "ability_pavise"), None)
        .unwrap();
    let u = &sim.units()[0];
    assert!(u.destination.is_none() && u.state != UnitState::Marching);
    assert!(u.pavise.is_some());
    let v = view(&sim, 0, "ability_pavise");
    assert!(v.active && !v.effective, "{v:?}");
    assert!((v.setup_remaining - 4.0).abs() < 1e-6);
    run(&mut sim, 4.1);
    assert!(view(&sim, 0, "ability_pavise").effective);
    // Still standing: the pavises stay up (no duration).
    run(&mut sim, 30.0);
    assert!(view(&sim, 0, "ability_pavise").active);
    // A move lifts them; the reason shows, the cooldown runs.
    sim.apply_command(go(vec![0], (150.0, 300.0)), None)
        .unwrap();
    run(&mut sim, DT);
    let v = view(&sim, 0, "ability_pavise");
    assert!(!v.active && !v.available, "{v:?}");
    assert!(!v.ended_reason.is_empty(), "{v:?}");
    assert!(v.cooldown_remaining > 9.0 && v.cooldown_remaining <= 10.0);
    assert!(v.reason.starts_with("recharge"), "{v:?}");
    let why = refusal(sim.apply_command(use_ability(vec![0], "ability_pavise"), None));
    assert!(why.starts_with("recharge, encore"), "{why}");
    run(&mut sim, 10.0);
    assert!(view(&sim, 0, "ability_pavise").available);
    assert!(sim.units()[0].pavise.is_none());
}

#[test]
fn a_second_use_lifts_the_ability() {
    let mut sim = lab_sim(1);
    sim.apply_command(use_ability(vec![0], "ability_pavise"), None)
        .unwrap();
    run(&mut sim, 1.0);
    sim.apply_command(use_ability(vec![0], "ability_pavise"), None)
        .unwrap();
    let v = view(&sim, 0, "ability_pavise");
    assert!(!v.active && v.ended_reason.is_empty(), "{v:?}");
    assert!(v.cooldown_remaining > 9.0);
    assert!(sim.units()[0].pavise.is_none());
}

#[test]
fn pavises_cover_from_the_front_only_once_planted() {
    let mut sim = lab_sim(1);
    let front = |sim: &BattleSim| sim.missile_cover(&sim.units()[0], 0);
    let flank = |sim: &BattleSim| sim.missile_cover(&sim.units()[0], 1);
    // Standing crossbowmen: the passive cover of their pavise ability.
    assert!((front(&sim) - 0.6).abs() < 1e-9);
    sim.apply_command(use_ability(vec![0], "ability_pavise"), None)
        .unwrap();
    run(&mut sim, 1.0);
    assert!((front(&sim) - 0.6).abs() < 1e-9, "not planted yet");
    run(&mut sim, 3.5);
    assert!((front(&sim) - 0.35).abs() < 1e-9);
    assert!(
        (flank(&sim) - 0.6).abs() < 1e-9,
        "nothing more from the flank"
    );
}

/// Missile casualties of the crossbowmen (0), pavises planted, under the
/// English longbows (5) for 30 s, shot at from the front or from the flank.
fn pavise_losses(flank: bool) -> f64 {
    let mut sim = lab_sim(5);
    let facing = if flank { FRAC_PI_2 } else { 0.0 };
    place(&mut sim, 0, 350.0, 700.0, facing);
    sim.apply_command(use_ability(vec![0], "ability_pavise"), None)
        .unwrap();
    run(&mut sim, 5.0);
    place(&mut sim, 5, 350.0, 810.0, PI);
    sim.apply_command(
        Command::FireAtWill {
            units: vec![5],
            enabled: true,
        },
        None,
    )
    .unwrap();
    let before = sim.units()[0].hp;
    run(&mut sim, 30.0);
    assert!(view(&sim, 0, "ability_pavise").active);
    before - sim.units()[0].hp
}

#[test]
fn pavises_spare_the_crossbowmen_shot_in_front_not_on_the_flank() {
    let front = pavise_losses(false);
    let flank = pavise_losses(true);
    assert!(front > 0.0);
    assert!(front < flank * 0.75, "front {front} vs flank {flank}");
}

#[test]
fn the_aimed_shot_shortens_the_range_and_hits_horses_harder() {
    let mut sim = lab_sim(1);
    let range = |sim: &BattleSim| {
        let u = &sim.units()[1];
        sim.effective_range(u, u.x, u.z + 50.0)
    };
    let long = range(&sim);
    sim.apply_command(use_ability(vec![1], "ability_aimed_shot"), None)
        .unwrap();
    assert!((range(&sim) / long - 0.55).abs() < 1e-6);
    // It lasts 30 s, then the cooldown (45 s) runs.
    run(&mut sim, 29.0);
    assert!(view(&sim, 1, "ability_aimed_shot").active);
    run(&mut sim, 1.2);
    let v = view(&sim, 1, "ability_aimed_shot");
    assert!(!v.active && v.ended_reason.is_empty(), "{v:?}");
    assert!(v.cooldown_remaining > 43.0);
    assert!((range(&sim) - long).abs() < 1e-6);
}

/// Casualties of the French knights (6 is the English knights: here the
/// English longbows (5) shoot at the French knights (2) at 90 m for 12 s.
fn knights_shot(aimed: bool) -> f64 {
    let mut sim = lab_sim(5);
    place(&mut sim, 2, 350.0, 700.0, PI);
    place(&mut sim, 5, 350.0, 790.0, PI);
    // Face the knights.
    sim.units_mut()[5].facing = PI;
    sim.units_mut()[2].facing = 0.0;
    if aimed {
        sim.apply_command(use_ability(vec![5], "ability_aimed_shot"), None)
            .unwrap();
    }
    sim.apply_command(
        Command::FireAtWill {
            units: vec![5],
            enabled: true,
        },
        None,
    )
    .unwrap();
    let before = sim.units()[2].hp;
    run(&mut sim, 12.0);
    before - sim.units()[2].hp
}

#[test]
fn aimed_arrows_bring_down_more_horsemen_at_close_range() {
    let plain = knights_shot(false);
    let aimed = knights_shot(true);
    assert!(plain > 0.0);
    assert!(aimed > plain * 1.2, "aimed {aimed} vs plain {plain}");
}

#[test]
fn the_aimed_shot_needs_arrows_and_no_melee() {
    let mut sim = lab_sim(1);
    sim.units_mut()[1].state = UnitState::Melee;
    let why = refusal(sim.apply_command(use_ability(vec![1], "ability_aimed_shot"), None));
    assert_eq!(why, "au contact de l'ennemi");
    assert_eq!(view(&sim, 1, "ability_aimed_shot").reason, why);
    sim.units_mut()[1].state = UnitState::Idle;
    sim.units_mut()[1].ammo = 0;
    let why = refusal(sim.apply_command(use_ability(vec![1], "ability_aimed_shot"), None));
    assert_eq!(why, "plus de traits");
    // In use, it stops when the arrows run out, with the reason.
    sim.units_mut()[1].ammo = 3;
    sim.apply_command(use_ability(vec![1], "ability_aimed_shot"), None)
        .unwrap();
    run(&mut sim, 1.0);
    sim.units_mut()[1].ammo = 0;
    run(&mut sim, DT);
    let v = view(&sim, 1, "ability_aimed_shot");
    assert!(!v.active);
    assert_eq!(v.ended_reason, "plus de traits");
}

#[test]
fn knights_rally_at_their_banner_standing_still() {
    let rallied = |use_it: bool| -> (f64, BattleSim) {
        let mut sim = lab_sim(1);
        sim.units_mut()[2].morale = 30.0;
        sim.apply_command(go(vec![2], (550.0, 400.0)), None)
            .unwrap();
        run(&mut sim, 1.0);
        let before = sim.units()[2].morale;
        if use_it {
            sim.apply_command(use_ability(vec![2], "ability_banner_rally"), None)
                .unwrap();
            assert!(sim.units()[2].destination.is_none(), "halted");
            assert!(sim.units()[2].morale >= before + 5.0 - 1e-9);
        }
        run(&mut sim, 5.0);
        (sim.units()[2].morale - before, sim)
    };
    let (plain, _) = rallied(false);
    let (banner, sim) = rallied(true);
    assert!(banner > plain + 5.0, "banner {banner} vs plain {plain}");
    assert!(view(&sim, 2, "ability_banner_rally").active);
    // A move ends it, with its reason.
    let mut sim = sim;
    sim.apply_command(go(vec![2], (550.0, 300.0)), None)
        .unwrap();
    run(&mut sim, DT);
    let v = view(&sim, 2, "ability_banner_rally");
    assert!(!v.active);
    assert_eq!(v.ended_reason, "l'unité s'est mise en mouvement");
}

#[test]
fn close_ranks_hold_the_front_and_pay_on_the_flanks() {
    let mut sim = lab_sim(1);
    sim.apply_command(use_ability(vec![3], "ability_close_ranks"), None)
        .unwrap();
    let (me, knights) = (&sim.units()[3], &sim.units()[6]);
    assert!((sim.ability_melee_factor(knights, me, 0) - 0.8).abs() < 1e-9);
    assert!((sim.ability_melee_factor(knights, me, 1) - 1.25).abs() < 1e-9);
    assert!((sim.ability_charge_taken(me) - 0.6).abs() < 1e-9);
    assert!((sim.missile_cover(me, 0) - 1.15).abs() < 1e-9);
    // It lasts 60 s.
    run(&mut sim, 60.2);
    assert!(!view(&sim, 3, "ability_close_ranks").active);
}

#[test]
fn close_ranks_march_slower_and_never_run() {
    let walked = |close: bool| -> f64 {
        let mut sim = lab_sim(1);
        if close {
            sim.apply_command(use_ability(vec![3], "ability_close_ranks"), None)
                .unwrap();
        }
        let start = pos(&sim, 3);
        sim.apply_command(
            Command::Move {
                units: vec![3],
                x: 750.0,
                z: 600.0,
                run: true,
                facing: None,
                queue: false,
                width: None,
                match_speed: false,
                group_tag: None,
            },
            None,
        )
        .unwrap();
        run(&mut sim, 10.0);
        if close {
            assert!(!sim.units()[3].running);
            assert!(
                view(&sim, 3, "ability_close_ranks").active,
                "moving is fine"
            );
        }
        dist(start, pos(&sim, 3))
    };
    let open = walked(false);
    let close = walked(true);
    assert!(close < open * 0.5, "close {close} vs open {open}");
}

/// Kind of the first impact of the English knights (6) charging the
/// Flemish militia (4) from the front, pikes planted or not.
fn charge_on_militia(pikes: bool) -> ImpactKind {
    let mut sim = lab_sim(2);
    place(&mut sim, 6, 950.0, 420.0, PI);
    if pikes {
        sim.apply_command(use_ability(vec![4], "ability_planted_pikes"), None)
            .unwrap();
        run(&mut sim, 2.5);
        assert!(view(&sim, 4, "ability_planted_pikes").effective);
    }
    sim.apply_command(
        Command::Attack {
            units: vec![6],
            target: 4,
            run: true,
            queue: false,
        },
        None,
    )
    .unwrap();
    for _ in 0..600 {
        sim.step();
        if let Some(hit) = sim.take_impacts().into_iter().find(|h| h.attacker == 6) {
            if pikes {
                assert!(
                    view(&sim, 4, "ability_planted_pikes").active,
                    "kept in melee"
                );
            }
            return hit.kind;
        }
    }
    panic!("no charge impact");
}

#[test]
fn a_frontal_charge_breaks_on_planted_pikes() {
    assert_ne!(charge_on_militia(false), ImpactKind::Pikes);
    assert_eq!(charge_on_militia(true), ImpactKind::Pikes);
}

#[test]
fn planted_pikes_need_no_contact_to_start_and_stand_still() {
    let mut sim = lab_sim(1);
    sim.units_mut()[4].state = UnitState::Melee;
    let why = refusal(sim.apply_command(use_ability(vec![4], "ability_planted_pikes"), None));
    assert_eq!(why, "au contact de l'ennemi");
    sim.units_mut()[4].state = UnitState::Idle;
    sim.apply_command(use_ability(vec![4], "ability_planted_pikes"), None)
        .unwrap();
    run(&mut sim, 3.0);
    // The other ability of the militia replaces it.
    sim.apply_command(use_ability(vec![4], "ability_close_ranks"), None)
        .unwrap();
    assert!(view(&sim, 4, "ability_close_ranks").active);
    let pikes = view(&sim, 4, "ability_planted_pikes");
    assert!(
        !pikes.active && pikes.cooldown_remaining > 19.0,
        "{pikes:?}"
    );
}

#[test]
fn the_rout_ends_the_ability() {
    let mut sim = lab_sim(1);
    sim.apply_command(use_ability(vec![3], "ability_close_ranks"), None)
        .unwrap();
    sim.units_mut()[3].morale = 0.0;
    run(&mut sim, 0.3);
    assert_eq!(sim.units()[3].state, UnitState::Routing);
    let v = view(&sim, 3, "ability_close_ranks");
    assert!(!v.active && !v.available);
    assert_eq!(v.reason, "en déroute");
}

// ----- replay -----------------------------------------------------------------

fn record_with_abilities(seed: u64) -> (BattleSim, BattleReplay) {
    let mut setup: sim_battle::BattleSetup =
        serde_json::from_str(include_str!("fixtures/demo_battle_1337.json")).unwrap();
    setup.abilities = game_data().battle_abilities.values().cloned().collect();
    let start = ReplayStart::plain(setup, seed);
    let mut sim = start.build().unwrap();
    let mut recorder = ReplayRecorder::new(start, &sim);
    let drive = |sim: &mut BattleSim, recorder: &mut ReplayRecorder, action: ReplayAction| {
        recorder.record(sim, action.clone());
        action.apply(sim);
    };
    drive(&mut sim, &mut recorder, ReplayAction::StartBattle);
    let player = sim.setup().player_side.unwrap_or(SideId::Attacker);
    let ours: Vec<u32> = sim
        .units()
        .iter()
        .filter(|u| u.side == player && u.present())
        .map(|u| u.id)
        .collect();
    let mut used = 0;
    for ability in game_data().battle_abilities.keys() {
        let having: Vec<u32> = ours
            .iter()
            .copied()
            .filter(|&id| views(&sim, id).iter().any(|v| &v.id == ability))
            .collect();
        if having.is_empty() {
            continue;
        }
        used += 1;
        drive(
            &mut sim,
            &mut recorder,
            ReplayAction::Command {
                command: use_ability(having, ability),
            },
        );
    }
    assert!(used >= 2, "the demo army has abilities to use");
    for _ in 0..600 {
        sim.tick(DT);
        recorder.observe(&sim);
    }
    (sim, recorder.finish("Capacités", 1_790_000_000))
}

#[test]
fn a_replay_with_abilities_replays_exactly() {
    let (live, replay) = record_with_abilities(1337);
    let text = replay.to_json();
    assert!(
        text.contains("\"use_ability\""),
        "ability commands recorded"
    );
    let read = BattleReplay::from_json(&text).unwrap();
    assert_eq!(read, replay);
    let (mut player, mut sim) = ReplayPlayer::load(read).unwrap();
    while player.advance(&mut sim, 1.0) > 0 {}
    assert!(player.divergence().is_none(), "{:?}", player.divergence());
    assert_eq!(sim.ticks(), live.ticks());
    assert_eq!(state_digest(&sim), state_digest(&live));
    let (again, _) = record_with_abilities(1337);
    assert_eq!(state_digest(&again), state_digest(&live), "deterministic");
}

#[test]
fn a_replay_recorded_before_cb4_still_reads() {
    let text = include_str!("fixtures/replay_sample.json");
    assert!(!text.contains("use_ability") && !text.contains("battle_abilities"));
    let replay = BattleReplay::from_json(text).expect("the pre-CB sample reads");
    let (mut player, mut sim) = ReplayPlayer::load(replay).expect("the sample battle rebuilds");
    assert!(sim.ability_catalog().is_empty());
    while player.advance(&mut sim, 1.0) > 0 {}
    // Same bound as `cb2_modes.rs`: CB4 must not make it part any earlier.
    assert!(
        player.divergence().is_none_or(|d| d.tick >= 1120),
        "{:?}",
        player.divergence()
    );
}

// ----- AI ---------------------------------------------------------------------

#[test]
fn ai_crossbowmen_raise_their_pavises_under_fire() {
    let data = game_data();
    let mut battle = setup(
        units(data, &["unit_men_at_arms_foot", "unit_genoese_crossbowmen"]),
        units(data, &["unit_men_at_arms_foot", "unit_longbowmen"]),
        None,
    );
    battle.abilities = data.battle_abilities.values().cloned().collect();
    let mut sim = BattleSim::new(battle, 9).unwrap();
    lab(&mut sim);
    sim.set_ai(SideId::Attacker, true);
    place(&mut sim, 0, 300.0, 100.0, 0.0);
    place(&mut sim, 1, 600.0, 300.0, 0.0);
    place(&mut sim, 2, 1100.0, 750.0, PI);
    place(&mut sim, 3, 600.0, 490.0, PI);
    let mut raised = false;
    for _ in 0..400 {
        sim.step();
        if sim.units()[1].ability_state.active_kind() == Some(AbilityKind::Pavise) {
            raised = true;
            break;
        }
    }
    assert!(
        raised,
        "the Genoese should raise their pavises under the arrows"
    );
}

#[test]
fn ai_pikemen_plant_their_pikes_before_the_horse() {
    let data = game_data();
    let mut battle = setup(
        units(data, &["unit_knights"]),
        units(data, &["unit_flemish_pikemen", "unit_men_at_arms_foot"]),
        None,
    );
    battle.abilities = data.battle_abilities.values().cloned().collect();
    let mut sim = BattleSim::new(battle, 4).unwrap();
    lab(&mut sim);
    sim.set_ai(SideId::Defender, true);
    place(&mut sim, 0, 600.0, 300.0, 0.0);
    place(&mut sim, 1, 600.0, 520.0, PI);
    place(&mut sim, 2, 900.0, 700.0, PI);
    sim.apply_command(
        Command::Attack {
            units: vec![0],
            target: 1,
            run: true,
            queue: false,
        },
        None,
    )
    .unwrap();
    let mut planted = false;
    for _ in 0..300 {
        sim.step();
        if sim.units()[1].ability_state.active_kind() == Some(AbilityKind::PlantedPikes) {
            planted = true;
            break;
        }
        if sim.units()[1].state == UnitState::Melee {
            break;
        }
    }
    assert!(
        planted,
        "the pikemen should plant their pikes before the charge"
    );
}

// ----- reference battles --------------------------------------------------

/// The ability catalogue, keeping the AI rule of the kinds in `ai_kinds`
/// only (every kind when `None`).
fn catalogue(ai_kinds: Option<&[AbilityKind]>) -> Vec<BattleAbility> {
    game_data()
        .battle_abilities
        .values()
        .cloned()
        .map(|mut a| {
            if ai_kinds.is_some_and(|kinds| !kinds.contains(&a.kind)) {
                a.ai = None;
            }
            a
        })
        .collect()
}

fn historical(id: &str, seed: u64, abilities: Vec<BattleAbility>) -> BattleSim {
    let data = game_data();
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
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
    setup.abilities = abilities;
    map.start(setup, seed).unwrap()
}

fn mixed(seed: u64, abilities: Vec<BattleAbility>) -> BattleSim {
    let data = game_data();
    let mut battle = setup(
        units(
            data,
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
            data,
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
    battle.abilities = abilities;
    let mut sim = BattleSim::new(battle, seed).unwrap();
    sim.set_ai(SideId::Attacker, true);
    sim.set_ai(SideId::Defender, true);
    sim
}

/// English victories out of 20 on each historical map, French out of 16 in
/// the EQ7 mixed battle (seeds of `ep7_historical` and `eq7_cavalry`).
fn margins(ai_kinds: Option<&[AbilityKind]>) -> [usize; 4] {
    let mut out = [0; 4];
    for (k, id) in ["crecy", "azincourt", "poitiers"].iter().enumerate() {
        out[k] = (1..21)
            .filter(|&seed| {
                let mut sim = historical(id, seed, catalogue(ai_kinds));
                run(&mut sim, 2400.0);
                sim.winner() == Some(SideId::Defender)
            })
            .count();
    }
    out[3] = (0..16)
        .filter(|&seed| {
            let mut sim = mixed(seed, catalogue(ai_kinds));
            run_to_end(&mut sim);
            sim.winner() == Some(SideId::Attacker)
        })
        .count();
    out
}

/// Probe (ignored): the margins with the AI of each ability alone.
/// `cargo test --release -p sim-battle --test cb4_abilities -- --ignored --nocapture probe_margins`
#[test]
#[ignore = "probe"]
fn probe_margins() {
    use AbilityKind as K;
    let variants: [(&str, Option<&[AbilityKind]>); 7] = [
        ("no AI", Some(&[])),
        ("pavise", Some(&[K::Pavise])),
        ("aimed shot", Some(&[K::AimedShot])),
        ("banner rally", Some(&[K::BannerRally])),
        ("close ranks", Some(&[K::CloseRanks])),
        ("planted pikes", Some(&[K::PlantedPikes])),
        ("all", None),
    ];
    // `CB4_ONLY=aimed,all`: only the variants whose name starts with one of these.
    let only = std::env::var("CB4_ONLY").unwrap_or_default();
    for (name, kinds) in variants {
        if !only.is_empty() && !only.split(',').any(|o| name.starts_with(o)) {
            continue;
        }
        let [c, a, p, e] = margins(kinds);
        println!("{name}: Crécy {c}/20, Azincourt {a}/20, Poitiers {p}/20, EQ7 {e}/16");
    }
}
