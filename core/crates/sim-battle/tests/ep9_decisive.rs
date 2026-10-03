//! EP9: field battles decide themselves (ADR 0056).
//!
//! Probes of the Q3 stalemate: battles with no player order (the player's
//! side idle, or both sides under the AI), several seeds, the three scale
//! tiers, with and without a river or heights. A battle must end within
//! 5-12 minutes of battle time.
//!
//! `cargo test --release -p sim-battle --test ep9_decisive -- --ignored --nocapture survey`
//! prints the survey table (duration, winner, how it ended).

mod common;

use common::*;
use data_model::Terrain;
use sim_battle::{BattleEnd, BattleScale, BattleSetup, BattleSim, GeneralSetup, SideId, UnitSetup};

/// Upper bound of the survey runs (simulated seconds).
const CAP: f64 = 1800.0;

const KINDS: [&str; 5] = [
    "unit_men_at_arms_foot",
    "unit_longbowmen",
    "unit_knights",
    "unit_urban_militia",
    "unit_crossbowmen",
];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tier {
    Standard,
    Large,
    Epic,
}

impl Tier {
    fn regiments(self) -> usize {
        match self {
            Tier::Standard => 10,
            Tier::Large => 24,
            Tier::Epic => 60,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Tier::Standard => "skirmish",
            Tier::Large => "large",
            Tier::Epic => "epic",
        }
    }
}

/// Who gives orders: both AIs, or the player's side left idle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mode {
    AiVsAi,
    /// The player attacks (campaign: he engaged) and gives no order.
    IdleAttacker,
    /// The player defends and gives no order.
    IdleDefender,
}

fn general(name: &str) -> GeneralSetup {
    GeneralSetup {
        character: format!("chr_{name}"),
        name: name.to_owned(),
        command: 5,
        unit_index: 0,
        morale_bonus: 0.0,
        charge_percent: 0.0,
        ranged_percent: 0.0,
        defense_percent: 0.0,
        sovereign: false,
    }
}

fn army(tier: Tier, offset: usize) -> Vec<UnitSetup> {
    let data = data();
    let ids: Vec<&str> = (0..tier.regiments())
        .map(|i| KINDS[(i + offset) % KINDS.len()])
        .collect();
    let mut list = units(&data, &ids);
    if tier == Tier::Epic {
        for unit in &mut list {
            unit.soldiers = 120;
            unit.max_soldiers = 120;
        }
    }
    list
}

pub fn battle(tier: Tier, terrain: Terrain, river: bool, mode: Mode) -> BattleSetup {
    let mut setup = setup(army(tier, 0), army(tier, 2), None);
    setup.terrain = terrain;
    setup.river = river;
    setup.attacker.general = Some(general("attacker"));
    setup.defender.general = Some(general("defender"));
    setup.player_side = match mode {
        Mode::AiVsAi => None,
        Mode::IdleAttacker => Some(SideId::Attacker),
        Mode::IdleDefender => Some(SideId::Defender),
    };
    setup
}

pub fn sim(tier: Tier, terrain: Terrain, river: bool, mode: Mode, seed: u64) -> BattleSim {
    let scale = BattleScale::named(tier.key()).unwrap();
    BattleSim::new_scaled(battle(tier, terrain, river, mode), seed, scale).unwrap()
}

/// Runs to the end (or [`CAP`]): `(seconds, winner, last journal line)`.
pub fn play(sim: &mut BattleSim) -> (f64, Option<SideId>, String) {
    let mut first_melee = None;
    while !sim.is_finished() && sim.elapsed() < CAP {
        sim.step();
        if first_melee.is_none()
            && sim
                .units()
                .iter()
                .any(|u| u.state == sim_battle::UnitState::Melee)
        {
            first_melee = Some(sim.elapsed());
        }
    }
    if std::env::var("EP9_VERBOSE").is_ok() {
        println!(
            "    first melee {first_melee:?}, fighting share att {:.2} def {:.2}, end {:?}",
            sim.fighting_share(SideId::Attacker),
            sim.fighting_share(SideId::Defender),
            sim.end_kind()
        );
    }
    let last = sim
        .events()
        .last()
        .map(|e| e.text_fr.clone())
        .unwrap_or_default();
    (sim.elapsed(), sim.winner(), last)
}

fn row(tier: Tier, terrain: Terrain, river: bool, mode: Mode, seeds: u64) -> Vec<f64> {
    // `EP9_ONLY=large,river,IdleAttacker` keeps the matching rows only.
    let label = format!("{} {} river={river} {mode:?}", tier.key(), terrain.key());
    if let Ok(only) = std::env::var("EP9_ONLY") {
        if !only.split(',').all(|token| label.contains(token)) {
            return Vec::new();
        }
    }
    let mut times = Vec::new();
    let (mut att, mut def, mut open) = (0, 0, 0);
    let mut ends = std::collections::BTreeMap::new();
    for seed in 0..seeds {
        let mut s = sim(tier, terrain, river, mode, seed);
        let (t, w, last) = play(&mut s);
        times.push(t);
        *ends
            .entry(s.end_kind().map_or("open", |e| e.key()))
            .or_insert(0) += 1;
        match w {
            Some(SideId::Attacker) => att += 1,
            Some(SideId::Defender) => def += 1,
            None => open += 1,
        }
        if std::env::var("EP9_VERBOSE").is_ok() {
            println!("  seed {seed}: {t:.0} s {w:?} « {last} »");
        }
    }
    let mut sorted = times.clone();
    sorted.sort_by(f64::total_cmp);
    println!(
        "{:9} {:9} river={:5} {:13} min {:4.0} med {:4.0} max {:4.0}  att {att:2} def {def:2} open {open:2} {ends:?}",
        tier.key(),
        terrain.key(),
        river,
        format!("{mode:?}"),
        sorted[0],
        sorted[sorted.len() / 2],
        sorted[sorted.len() - 1],
    );
    times
}

/// Survey table (ignored: long). `EP9_SEEDS` (default 12), `EP9_TIERS`
/// (`standard,large,epic`).
#[test]
#[ignore]
fn survey() {
    let seeds: u64 = std::env::var("EP9_SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(12);
    let tiers = std::env::var("EP9_TIERS").unwrap_or_else(|_| "standard,large,epic".into());
    for (tier, name) in [
        (Tier::Standard, "standard"),
        (Tier::Large, "large"),
        (Tier::Epic, "epic"),
    ] {
        if !tiers.contains(name) {
            continue;
        }
        for (terrain, river) in [
            (Terrain::Plains, false),
            (Terrain::Hills, false),
            (Terrain::Plains, true),
        ] {
            for mode in [Mode::AiVsAi, Mode::IdleAttacker, Mode::IdleDefender] {
                row(tier, terrain, river, mode, seeds);
            }
        }
    }
}

/// The Q3 battle: the demo battle of 1337 (France attacks, the player's
/// side) with the player giving no order.
#[test]
fn q3_demo_battle_without_orders_ends() {
    let setup: BattleSetup =
        serde_json::from_str(include_str!("fixtures/demo_battle_1337.json")).unwrap();
    for seed in 0..4 {
        let mut sim = BattleSim::new(setup.clone(), 1337 + seed).unwrap();
        let (t, w, last) = play(&mut sim);
        println!("demo seed {seed}: {t:.0} s {w:?} « {last} »");
        assert!(sim.is_finished(), "seed {seed}: still running at {t:.0} s");
        // L13b (ADR 0180): longer battles (approach x0.45), 20 minutes at most.
        assert!(t <= 1200.0, "seed {seed}: {t:.0} s");
    }
}

/// Standard tier, no order from the player (or both AIs), four seeds:
/// every battle ends within 20 minutes (23 with a river to cross, ADR 0180).
#[test]
fn battles_without_orders_end_in_time() {
    for (terrain, river, limit) in [
        (Terrain::Plains, false, 1200.0),
        (Terrain::Hills, false, 1200.0),
        (Terrain::Plains, true, 1380.0),
    ] {
        for mode in [Mode::AiVsAi, Mode::IdleAttacker, Mode::IdleDefender] {
            for seed in 0..4 {
                let mut sim = sim(Tier::Standard, terrain, river, mode, seed);
                let (t, _, last) = play(&mut sim);
                assert!(
                    sim.is_finished() && t <= limit,
                    "{} river={river} {mode:?} seed {seed}: {t:.0} s « {last} »",
                    terrain.key()
                );
            }
        }
    }
}

/// Large and epic tiers (up to 60 regiments a side): the idle side's
/// battle ends within 12 minutes too.
#[test]
fn large_and_epic_battles_without_orders_end_in_time() {
    for (tier, terrain, mode, seed) in [
        (Tier::Large, Terrain::Plains, Mode::IdleAttacker, 0),
        (Tier::Large, Terrain::Hills, Mode::IdleDefender, 1),
        (Tier::Epic, Terrain::Plains, Mode::IdleDefender, 0),
        (Tier::Epic, Terrain::Hills, Mode::IdleAttacker, 1),
        (Tier::Epic, Terrain::Hills, Mode::AiVsAi, 2),
    ] {
        let mut sim = sim(tier, terrain, false, mode, seed);
        let (t, _, last) = play(&mut sim);
        assert!(
            sim.is_finished() && t <= 1200.0,
            "{tier:?} {} {mode:?} seed {seed}: {t:.0} s « {last} »",
            terrain.key()
        );
    }
}

/// The attacker under the AI always engages: against an idle defender it
/// never lets the battle be refused (no river to cross). EP9b: the idle
/// army may also rout to the last regiment before it counts as broken.
#[test]
fn the_ai_attacker_always_engages() {
    for terrain in [Terrain::Plains, Terrain::Hills] {
        for seed in 0..4 {
            let mut sim = sim(Tier::Standard, terrain, false, Mode::IdleDefender, seed);
            play(&mut sim);
            assert!(
                matches!(sim.end_kind(), Some(BattleEnd::Broken | BattleEnd::Rout)),
                "{} seed {seed}: {:?}",
                terrain.key(),
                sim.end_kind()
            );
        }
    }
}

/// Nobody engages (the player attacks on paper and stays put, the AI
/// defender holds its heights): after five minutes the attacker withdraws,
/// the defender keeps the field. No rout, no loss, a small blow to morale.
#[test]
fn an_unfought_battle_is_refused() {
    let mut sim = sim(Tier::Standard, Terrain::Hills, false, Mode::IdleAttacker, 0);
    let (t, winner, last) = play(&mut sim);
    let rules = sim.decision_rules().clone();
    assert_eq!(sim.end_kind(), Some(BattleEnd::Refused), "« {last} »");
    // L13b (ADR 0180): the refusal clock runs at the patience factor of the
    // field pace (x1.5); a regiment of the defender repositioning slowly may
    // restart it once.
    assert!(
        (rules.refusal_seconds..=rules.refusal_seconds * 2.5).contains(&t),
        "{t:.0} s"
    );
    assert_eq!(winner, Some(SideId::Defender));
    assert!(last.contains("Bataille refusée"), "« {last} »");
    let outcome = sim.outcome().unwrap();
    assert_eq!(outcome.end, BattleEnd::Refused);
    assert_eq!(outcome.attacker.total_losses, 0);
    assert!(!outcome.attacker.routed && outcome.attacker.withdrew);
    assert_eq!(outcome.attacker.morale_delta, rules.refused_morale.attacker);
    assert_eq!(outcome.defender.morale_delta, rules.refused_morale.defender);
    // The result survives a round trip through JSON (Godot bridge, saves).
    let json = serde_json::to_string(&outcome).unwrap();
    assert!(json.contains("\"end\":\"refused\""), "{json}");
}

/// A beaten army breaks as a whole: its last regiments rout and the
/// journal says so.
#[test]
fn a_beaten_army_breaks() {
    let mut sim = sim(
        Tier::Standard,
        Terrain::Plains,
        false,
        Mode::IdleAttacker,
        0,
    );
    let (_, winner, last) = play(&mut sim);
    assert_eq!(sim.end_kind(), Some(BattleEnd::Broken));
    assert_eq!(winner, Some(SideId::Defender));
    assert!(last.contains("déroute générale"), "« {last} »");
    assert!(sim
        .units()
        .iter()
        .filter(|u| u.side == SideId::Attacker && u.present())
        .all(|u| !u.able()));
    let outcome = sim.outcome().unwrap();
    assert!(outcome.attacker.routed);
    assert_eq!(outcome.attacker.morale_delta, -20);
}

/// Crécy-like: the English on their ridge win most of the time.
#[test]
fn crecy_like_the_english_win_most_battles() {
    let english = (0..6)
        .filter(|&seed| {
            let mut sim = crecy(seed, false);
            play(&mut sim);
            sim.winner() == Some(SideId::Defender)
        })
        .count();
    assert!(english >= 5, "English won {english}/6");
}

/// The pre-EP9 end: no army break, no refusal (survey baseline).
pub fn legacy_rules(sim: &mut BattleSim) {
    let mut rules = sim.decision_rules().clone();
    rules.break_share = 0.0;
    rules.break_share_without_general = 0.0;
    rules.refusal_seconds = f64::INFINITY;
    sim.set_decision_rules(rules);
}

/// Crécy-like: English longbows and dismounted men-at-arms on a ridge,
/// French knights, crossbowmen and foot attacking up the glacis. Both AIs.
pub fn crecy(seed: u64, legacy: bool) -> BattleSim {
    let data = data();
    let french = [
        "unit_knights",
        "unit_knights",
        "unit_knights",
        "unit_knights",
        "unit_crossbowmen",
        "unit_crossbowmen",
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_urban_militia",
    ];
    let english = [
        "unit_longbowmen",
        "unit_longbowmen",
        "unit_longbowmen",
        "unit_longbowmen",
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_knights",
    ];
    let mut battle = setup(units(&data, &french), units(&data, &english), None);
    battle.village = Some(false);
    battle.attacker.general = Some(general("Philippe"));
    battle.defender.general = Some(general("Edouard"));
    battle.defender.general.as_mut().unwrap().unit_index = 4;
    let mut sim = BattleSim::new(battle, seed).unwrap();
    // The ridge lies 20 m in front of the English line wherever the field
    // battle scale puts it (ADR 0180: wider gap).
    let crest = sim
        .units()
        .iter()
        .filter(|u| u.side == SideId::Defender)
        .map(|u| u.z)
        .fold(f64::INFINITY, f64::min)
        - 20.0;
    let field = sim.field_mut();
    field.forests.clear();
    field.forest_parts.clear();
    field.mud.clear();
    field.mud_parts.clear();
    field.pools.clear();
    field.obstacles.clear();
    field.river = None;
    let (nx, res) = (field.nx, field.resolution);
    for (k, h) in field.heights.iter_mut().enumerate() {
        let z = (k / nx) as f64 * res;
        *h = 20.0 / (1.0 + (-(z - crest + 60.0) / 40.0).exp());
    }
    if legacy {
        legacy_rules(&mut sim);
    }
    sim
}

/// Crécy-like battles over 12 seeds, before (legacy end) and after.
#[test]
#[ignore]
fn survey_crecy() {
    for legacy in [true, false] {
        let (mut english, mut times) = (0, Vec::new());
        for seed in 0..12 {
            let mut sim = crecy(seed, legacy);
            let (t, w, _) = play(&mut sim);
            times.push(t.round() as i64);
            if w == Some(SideId::Defender) {
                english += 1;
            }
        }
        println!("crecy legacy={legacy}: English {english}/12, durations {times:?}");
    }
}

/// The reference battles of `ai.rs` (10 a side, river every other seed)
/// and `ep1_scale.rs` (60 a side, 120 men), legacy end against EP9.
#[test]
#[ignore]
fn survey_reference() {
    let data = data();
    for legacy in [true, false] {
        let mut line = Vec::new();
        for seed in 0..8u64 {
            let ids: Vec<&str> = (0..10).map(|i| KINDS[i % KINDS.len()]).collect();
            let mut battle = setup(units(&data, &ids), units(&data, &ids), None);
            battle.river = seed % 2 == 0;
            let mut sim = BattleSim::new(battle, seed).unwrap();
            if legacy {
                legacy_rules(&mut sim);
            }
            let (t, w, _) = play(&mut sim);
            line.push(format!(
                "{t:.0}{}",
                if w == Some(SideId::Attacker) {
                    "A"
                } else {
                    "D"
                }
            ));
        }
        println!("ai.rs legacy={legacy}: {}", line.join(" "));
        let mut line = Vec::new();
        for seed in [3u64, 5, 11] {
            let ids: Vec<&str> = (0..60).map(|i| KINDS[i % KINDS.len()]).collect();
            let mut battle = setup(units(&data, &ids), units(&data, &ids), None);
            for unit in battle
                .attacker
                .units
                .iter_mut()
                .chain(battle.defender.units.iter_mut())
            {
                unit.soldiers = 120;
                unit.max_soldiers = 120;
            }
            let mut sim = BattleSim::new(battle, seed).unwrap();
            if legacy {
                legacy_rules(&mut sim);
            }
            let mut melee = 0;
            while !sim.is_finished() && sim.elapsed() < CAP {
                sim.step();
                melee = melee.max(
                    sim.units()
                        .iter()
                        .filter(|u| u.state == sim_battle::UnitState::Melee)
                        .count(),
                );
            }
            line.push(format!(
                "seed {seed}: {:.0} s {:?} melee {melee}",
                sim.elapsed(),
                sim.winner()
            ));
        }
        println!("ep1 legacy={legacy}: {}", line.join(" | "));
    }
}

/// Trace of one battle (ignored): `EP9_CASE=demo,1` or
/// `EP9_CASE=epic,river,IdleDefender,9` (tier, `plains`/`hills`/`river`,
/// mode, seed).
#[test]
#[ignore]
fn trace() {
    let case = std::env::var("EP9_CASE").unwrap_or_else(|_| "demo,1".into());
    let parts: Vec<&str> = case.split(',').collect();
    let seed: u64 = parts.last().and_then(|s| s.parse().ok()).unwrap_or(1);
    let mut sim = if parts[0] == "demo" {
        let setup: BattleSetup =
            serde_json::from_str(include_str!("fixtures/demo_battle_1337.json")).unwrap();
        BattleSim::new(setup, 1337 + seed).unwrap()
    } else {
        let tier = match parts[0] {
            "large" => Tier::Large,
            "epic" => Tier::Epic,
            _ => Tier::Standard,
        };
        let (terrain, river) = match parts[1] {
            "hills" => (Terrain::Hills, false),
            "river" => (Terrain::Plains, true),
            _ => (Terrain::Plains, false),
        };
        let mode = match parts[2] {
            "IdleAttacker" => Mode::IdleAttacker,
            "IdleDefender" => Mode::IdleDefender,
            _ => Mode::AiVsAi,
        };
        sim(tier, terrain, river, mode, seed)
    };
    let mut next = 0.0;
    let mut seen = 0;
    while !sim.is_finished() && sim.elapsed() < CAP {
        sim.step();
        for e in &sim.events()[seen..] {
            println!("{:6.1} {:?} {}", e.time, e.side, e.text_fr);
        }
        seen = sim.events().len();
        if sim.elapsed() >= next {
            next += 30.0;
            let melee = sim
                .units()
                .iter()
                .filter(|u| u.state == sim_battle::UnitState::Melee)
                .count();
            let mean_z = |side: SideId| {
                let list: Vec<f64> = sim
                    .units()
                    .iter()
                    .filter(|u| u.side == side && u.able())
                    .map(|u| u.z)
                    .collect();
                list.iter().sum::<f64>() / list.len().max(1) as f64
            };
            println!(
                "t {:4.0} quiet {:4.0} share att {:.2} def {:.2} melee {melee} z att {:.0} def {:.0}",
                sim.elapsed(),
                sim.quiet_time(),
                sim.fighting_share(SideId::Attacker),
                sim.fighting_share(SideId::Defender),
                mean_z(SideId::Attacker),
                mean_z(SideId::Defender),
            );
        }
    }
}
