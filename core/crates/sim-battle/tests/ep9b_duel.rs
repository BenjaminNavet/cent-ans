//! EP9b (ADR 0056, § EP9b): the attacker's archery duel. Mirrored epic
//! armies (60 regiments of 120 men a side, both AIs) on bare flat ground
//! without stakes: before EP9b the attacker's line walked into the arrows
//! after ``ATTACKER_DUEL_LIMIT` (180 s) even while its shooters were winning the
//! duel, its militia in front broke and the defender won 10 battles out
//! of 10.
//!
//! `cargo test --release -p sim-battle --test ep9b_duel -- --ignored --nocapture survey`
//! (`EP9B_SEEDS=1..11` by default).

mod common;

use common::*;
use data_model::Ability;
use sim_battle::{BattleEnd, BattleSim, SideId};

const KINDS: [&str; 5] = [
    "unit_men_at_arms_foot",
    "unit_longbowmen",
    "unit_knights",
    "unit_urban_militia",
    "unit_crossbowmen",
];

fn seeds() -> std::ops::Range<u64> {
    std::env::var("EP9B_SEEDS")
        .ok()
        .and_then(|s| {
            let (a, b) = s.split_once("..")?;
            Some(a.parse().ok()?..b.parse().ok()?)
        })
        .unwrap_or(1..11)
}

/// The symmetric case: 60 regiments of 120 men a side, flat ground, no
/// stakes, clear weather; the attacker's army cycles through
/// `attacker_kinds` (the defender keeps [`KINDS`]). `decor`: the countryside of EP6
/// (hamlets, plots, camps) is kept on the flattened field.
fn flat_battle_with(seed: u64, attacker_kinds: &[&str], decor: bool) -> BattleSim {
    let data = data();
    let army: Vec<&str> = (0..60).map(|i| KINDS[i % KINDS.len()]).collect();
    let attackers: Vec<&str> = (0..60)
        .map(|i| attacker_kinds[i % attacker_kinds.len()])
        .collect();
    let mut setup = setup(units(data, &attackers), units(data, &army), None);
    setup.village = if decor { None } else { Some(false) };
    for unit in setup
        .attacker
        .units
        .iter_mut()
        .chain(setup.defender.units.iter_mut())
    {
        unit.soldiers = 120;
        unit.max_soldiers = 120;
        unit.abilities.retain(|a| *a != Ability::Stakes);
    }
    let mut sim = BattleSim::new(setup, seed).unwrap();
    sim.set_weather(sim_battle::Weather::Clear);
    sim.set_ai(SideId::Attacker, true);
    sim.set_ai(SideId::Defender, true);
    let field = sim.field_mut();
    if !decor {
        field.forests.clear();
        field.forest_parts.clear();
        field.mud.clear();
        field.mud_parts.clear();
        field.pools.clear();
        field.obstacles.clear();
    }
    field.river = None;
    field.bridges.clear();
    for h in field.heights.iter_mut() {
        *h = 0.0;
    }
    sim
}

/// Winner, end and duration of a battle.
type Played = (Option<SideId>, Option<BattleEnd>, f64);

/// Plays a battle to its end (30 min cap); returns winner, end and time.
fn play(sim: &mut BattleSim) -> Played {
    while !sim.is_finished() && sim.elapsed() < 1800.0 {
        sim.step();
    }
    (sim.winner(), sim.end_kind(), sim.elapsed())
}

/// Attacker wins, all results, over the seeds (`decor`: EP6 countryside
/// kept on the flat field).
fn flat_results(decor: bool) -> (usize, Vec<Played>) {
    let results: Vec<_> = seeds()
        .map(|seed| play(&mut flat_battle_with(seed, &KINDS, decor)))
        .collect();
    let wins = results
        .iter()
        .filter(|r| r.0 == Some(SideId::Attacker))
        .count();
    (wins, results)
}

/// Symmetric epic battle on flat ground without stakes: the outcome is
/// open (the attacker wins 3 to 7 battles out of 10), every battle ends.
#[test]

fn symmetric_flat_battle_is_open() {
    let (wins, results) = flat_results(false);
    for r in &results {
        assert!(r.0.is_some() && r.2 < 1800.0, "undecided: {r:?}");
    }
    assert!(
        (3..=7).contains(&wins),
        "attacker won {wins}/10: {results:?}"
    );
}

/// Mean `z` of the attacker's regiments of `kind` still present.
fn mean_z(sim: &BattleSim, kind: &str) -> f64 {
    let zs: Vec<f64> = sim
        .units()
        .iter()
        .filter(|u| u.side == SideId::Attacker && &*u.unit_type == kind && u.present())
        .map(|u| u.z)
        .collect();
    zs.iter().sum::<f64>() / zs.len().max(1) as f64
}

/// The attacker's longbows replace its crossbows: it wins the archery duel.
const LONGBOW_ATTACKER: [&str; 5] = [
    "unit_men_at_arms_foot",
    "unit_longbowmen",
    "unit_knights",
    "unit_urban_militia",
    "unit_longbowmen",
];

/// Where the attacker's men-at-arms stand at `at` seconds, and how the
/// battle ends (`limited`: the duel is never prolonged).
fn won_duel(limited: bool, at: f64) -> (f64, Option<BattleEnd>, f64) {
    let mut sim = flat_battle_with(1, &LONGBOW_ATTACKER, false);
    if limited {
        let mut rules = sim.duel_rules().clone();
        rules.winning_duel_max_seconds = rules.duel_limit_seconds;
        sim.set_duel_rules(rules);
    }
    let mut z = f64::NAN;
    while !sim.is_finished() && sim.elapsed() < 1800.0 {
        sim.step();
        if z.is_nan() && sim.elapsed() >= at {
            z = mean_z(&sim, "unit_men_at_arms_foot");
        }
    }
    (z, sim.end_kind(), sim.elapsed())
}

/// While its shooters win the duel the attacker keeps its line back (it
/// does not walk into the arrows at 180 s); the battle still ends, and is
/// not refused (the defender's losses keep the engagement clock going).
#[test]
// L13b (ADR 0184): the duel clocks run x1.9 slower, measured at 460 s instead of 270 s.
fn a_won_duel_holds_the_line_then_the_battle_ends() {
    let (held, end, t) = won_duel(false, 460.0);
    let (walked, _, _) = won_duel(true, 460.0);
    assert!(
        walked - held > 20.0,
        "men-at-arms at {held:.0} (duel prolonged) vs {walked:.0} (limited)"
    );
    assert!(t < 1800.0 && end.is_some(), "undecided at {t:.0} s");
    assert_ne!(end, Some(BattleEnd::Refused));
}
