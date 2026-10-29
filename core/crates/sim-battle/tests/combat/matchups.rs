//! Lot COMBAT (ADR combat-matchups): outcome bands on reference matchups,
//! AI against AI on the full 3D battle. Bands are wide on purpose: they pin
//! plausibility (who wins most often, how lopsided), not exact figures.
//! `cargo test --release -p sim-battle --test combat -- --ignored --nocapture matchups::survey`
//! prints the survey (attacker wins out of N seeds, mean losses).

use crate::common;

use common::*;
use data_model::Terrain;
use sim_battle::{BattleSim, SideId};

const CAP: f64 = 1800.0;

pub struct Row {
    pub attacker_wins: u32,
    pub runs: u32,
    pub attacker_losses: f64,
    pub defender_losses: f64,
    pub attacker_start: f64,
    pub defender_start: f64,
}

pub fn matchup(
    attackers: &[(&str, usize)],
    defenders: &[(&str, usize)],
    terrain: Terrain,
    seeds: &[u64],
) -> Row {
    let data = data();
    let expand = |list: &[(&str, usize)]| -> Vec<sim_battle::UnitSetup> {
        list.iter()
            .flat_map(|(id, n)| std::iter::repeat_n(unit(data, id), *n))
            .collect()
    };
    let mut row = Row {
        attacker_wins: 0,
        runs: seeds.len() as u32,
        attacker_losses: 0.0,
        defender_losses: 0.0,
        attacker_start: 0.0,
        defender_start: 0.0,
    };
    for &seed in seeds {
        let mut battle = setup(expand(attackers), expand(defenders), None);
        battle.terrain = terrain;
        let mut sim = BattleSim::new(battle, seed).unwrap();
        let start = |side: SideId, sim: &BattleSim| -> f64 {
            sim.units()
                .iter()
                .filter(|u| u.side == side && !u.synthetic)
                .map(|u| u.initial_soldiers as f64)
                .sum()
        };
        row.attacker_start = start(SideId::Attacker, &sim);
        row.defender_start = start(SideId::Defender, &sim);
        while !sim.is_finished() && sim.elapsed() < CAP {
            sim.step();
        }
        if sim.winner() == Some(SideId::Attacker) {
            row.attacker_wins += 1;
        }
        let lost = |side: SideId| -> f64 {
            sim.units()
                .iter()
                .filter(|u| u.side == side && !u.synthetic)
                .map(|u| u.initial_soldiers.saturating_sub(u.soldiers()) as f64)
                .sum()
        };
        row.attacker_losses += lost(SideId::Attacker) / seeds.len() as f64;
        row.defender_losses += lost(SideId::Defender) / seeds.len() as f64;
    }
    row
}

pub const SEEDS: [u64; 6] = [0, 1, 2, 3, 4, 5];
pub const K: &str = "unit_knights";
pub const L: &str = "unit_longbowmen";
pub const M: &str = "unit_urban_militia";
pub const A: &str = "unit_men_at_arms_foot";
pub const X: &str = "unit_crossbowmen";
pub const MA: &str = "unit_mounted_archers";

type Comp = Vec<(&'static str, usize)>;

fn cases() -> Vec<(&'static str, Comp, Comp, Terrain)> {
    use Terrain::*;
    vec![
        (
            "fr 4K+4X+4M vs en 6L+4A plain",
            vec![(K, 4), (X, 4), (M, 4)],
            vec![(L, 6), (A, 4)],
            Plains,
        ),
        ("10K vs 10L", vec![(K, 10)], vec![(L, 10)], Plains),
        ("12M vs 12L", vec![(M, 12)], vec![(L, 12)], Plains),
        ("4K vs 20M", vec![(K, 4)], vec![(M, 20)], Plains),
        ("20M vs 4K", vec![(M, 20)], vec![(K, 4)], Plains),
        ("8K vs 8A foot", vec![(K, 8)], vec![(A, 8)], Plains),
        ("10MA vs 10M", vec![(MA, 10)], vec![(M, 10)], Plains),
        ("8A vs 12M", vec![(A, 8)], vec![(M, 12)], Plains),
        ("12M vs 8A", vec![(M, 12)], vec![(A, 8)], Plains),
        (
            "20M vs 12L+4A hills",
            vec![(M, 20)],
            vec![(L, 12), (A, 4)],
            Hills,
        ),
        ("8L vs 8L", vec![(L, 8)], vec![(L, 8)], Plains),
    ]
}

#[test]
#[ignore]
fn trace() {
    let data = data();
    let mut battle = setup(
        std::iter::repeat_n(
            unit(
                data,
                &format!("unit_{}", std::env::var("TA").unwrap_or("knights".into())),
            ),
            8,
        )
        .collect(),
        std::iter::repeat_n(
            unit(
                data,
                &format!(
                    "unit_{}",
                    std::env::var("TB").unwrap_or("men_at_arms_foot".into())
                ),
            ),
            8,
        )
        .collect(),
        None,
    );
    battle.terrain = Terrain::Plains;
    let mut sim = BattleSim::new(battle, 1).unwrap();
    let mut next = 0.0;
    while !sim.is_finished() && sim.elapsed() < CAP {
        sim.step();
        if sim.elapsed() >= next {
            next += 5.0;
            let line: Vec<String> = sim
                .units()
                .iter()
                .map(|u| {
                    format!(
                        "{}{}:{:.0}/{:?}/m{:.0}",
                        if u.side == SideId::Attacker { "A" } else { "D" },
                        u.id,
                        u.hp,
                        u.state,
                        u.morale
                    )
                })
                .collect();
            let sum = |side: SideId| -> (f64, usize, f64) {
                let us: Vec<_> = sim.units().iter().filter(|u| u.side == side).collect();
                (
                    us.iter().map(|u| u.hp).sum(),
                    us.iter()
                        .filter(|u| u.state == sim_battle::UnitState::Routing)
                        .count(),
                    us.iter().map(|u| u.morale).sum::<f64>() / us.len() as f64,
                )
            };
            println!(
                "t={:.0} A {:?} D {:?} {}",
                sim.elapsed(),
                sum(SideId::Attacker),
                sum(SideId::Defender),
                line.len()
            );
        }
    }
    for e in sim.events().iter().take(60) {
        println!("{:.0}s {}", e.time, e.text_fr);
    }
}

#[test]
#[ignore]
fn survey() {
    let filter = std::env::var("FILTER").unwrap_or_default();
    let seeds: Vec<u64> = (0..std::env::var("SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(6))
        .collect();
    for (name, att, def, terrain) in cases() {
        if !name.contains(&filter) {
            continue;
        }
        let r = matchup(&att, &def, terrain, &seeds);
        println!(
            "{name:<34} att wins {}/{}  losses {:>5.0}/{:.0} vs {:>5.0}/{:.0}",
            r.attacker_wins,
            r.runs,
            r.attacker_losses,
            r.attacker_start,
            r.defender_losses,
            r.defender_start
        );
    }
}

/// Reference matchups (AI against AI, 6 seeds, plains). Wide bands: they pin
/// plausibility (nobody is untouchable, nobody wins every time), not figures.
#[test]
fn outcome_bands() {
    use Terrain::Plains;
    let seeds = &SEEDS[..];
    // Crecy-like: the English archers beat a French mixed army on open
    // ground most of the time, but bleed for it.
    let r = matchup(&[(K, 4), (X, 4), (M, 4)], &[(L, 6), (A, 4)], Plains, seeds);
    assert!(
        r.attacker_wins <= 3,
        "French mixed army: {} wins",
        r.attacker_wins
    );
    assert!(
        r.defender_losses > 60.0,
        "archers untouched: {:.0}",
        r.defender_losses
    );
    assert!(
        r.attacker_losses > 150.0,
        "French losses too light: {:.0}",
        r.attacker_losses
    );
    // Ten knights against ten longbows: the horse must have a chance.
    let r = matchup(&[(K, 10)], &[(L, 10)], Plains, seeds);
    assert!(
        (1..=5).contains(&r.attacker_wins),
        "10K vs 10L: {} wins",
        r.attacker_wins
    );
    assert!(
        r.defender_losses > 60.0,
        "archers untouched: {:.0}",
        r.defender_losses
    );
    // A mass of militia holds against a few knights, and costs them.
    let r = matchup(&[(K, 4)], &[(M, 20)], Plains, seeds);
    assert!(r.attacker_wins <= 4, "4K vs 20M: {} wins", r.attacker_wins);
    assert!(
        r.attacker_losses > 20.0,
        "knights untouched: {:.0}",
        r.attacker_losses
    );
    let r = matchup(&[(M, 20)], &[(K, 4)], Plains, seeds);
    assert!(r.attacker_wins >= 2, "20M vs 4K: {} wins", r.attacker_wins);
    // Unprotected militia cannot cross an archer line.
    let r = matchup(&[(M, 12)], &[(L, 12)], Plains, seeds);
    assert!(r.attacker_wins <= 1, "12M vs 12L: {} wins", r.attacker_wins);
    // Quality foot beats militia without annihilating it for free.
    let r = matchup(&[(A, 8)], &[(M, 12)], Plains, seeds);
    assert!(
        (2..=6).contains(&r.attacker_wins),
        "8A vs 12M: {} wins",
        r.attacker_wins
    );
    assert!(
        r.attacker_losses > 40.0,
        "men-at-arms untouched: {:.0}",
        r.attacker_losses
    );
    // Horse archers kite militia but pay in men.
    let r = matchup(&[(MA, 10)], &[(M, 10)], Plains, seeds);
    assert!(
        r.attacker_wins >= 3,
        "10MA vs 10M: {} wins",
        r.attacker_wins
    );
    assert!(
        r.attacker_losses > 5.0,
        "horse archers untouched: {:.0}",
        r.attacker_losses
    );
}
