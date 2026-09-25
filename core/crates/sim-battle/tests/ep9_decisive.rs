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
use sim_battle::{BattleScale, BattleSetup, BattleSim, GeneralSetup, SideId, UnitSetup};

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
#[ignore]
fn q3_demo_battle_without_orders_ends() {
    let setup: BattleSetup =
        serde_json::from_str(include_str!("fixtures/demo_battle_1337.json")).unwrap();
    for seed in 0..4 {
        let mut sim = BattleSim::new(setup.clone(), 1337 + seed).unwrap();
        let (t, w, last) = play(&mut sim);
        println!("demo seed {seed}: {t:.0} s {w:?} « {last} »");
        assert!(sim.is_finished(), "seed {seed}: still running at {t:.0} s");
        assert!(t <= 720.0, "seed {seed}: {t:.0} s");
    }
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
    let crest = sim_battle::DEFENDER_LINE_Z - 20.0;
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

/// Trace of one demo battle (ignored): `EP9_SEED`.
#[test]
#[ignore]
fn trace_demo() {
    let setup: BattleSetup =
        serde_json::from_str(include_str!("fixtures/demo_battle_1337.json")).unwrap();
    let seed: u64 = std::env::var("EP9_SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    let mut sim = BattleSim::new(setup, 1337 + seed).unwrap();
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
            println!(
                "t {:4.0} quiet {:4.0} share att {:.2} def {:.2}",
                sim.elapsed(),
                sim.quiet_time(),
                sim.fighting_share(SideId::Attacker),
                sim.fighting_share(SideId::Defender)
            );
        }
    }
}
