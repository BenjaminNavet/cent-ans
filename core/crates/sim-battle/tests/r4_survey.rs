//! R4 surveys (ignored): arrow losses and outcomes of the English position
//! (archers behind a hedge on a crest) and of a line on a reverse slope.
//! Only the pre-R4 public API is used, so the same file measures the code
//! before R4.
//!
//! `cargo test --release -p sim-battle --test r4_survey -- --ignored --nocapture`
//! (`R4_SEEDS=0..32` by default; `R4_JITTER=1` moves the crest and its
//! hedges by up to 16 m per seed, so the seeds sample several geometries
//! instead of replaying almost the same battle).

mod common;

use common::*;
use data_model::GameData;
use sim_battle::{BattleSim, Obstacle, ObstacleKind, SideId, UnitState};

fn seeds() -> std::ops::Range<u64> {
    std::env::var("R4_SEEDS")
        .ok()
        .and_then(|s| {
            let (a, b) = s.split_once("..")?;
            Some(a.parse().ok()?..b.parse().ok()?)
        })
        .unwrap_or(0..32)
}

fn shape(sim: &mut BattleSim, height: impl Fn(f64) -> f64) {
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
        *h = height((k / nx) as f64 * res);
    }
}

fn ridge(crest_z: f64) -> impl Fn(f64) -> f64 {
    move |z| 20.0 * (-((z - crest_z) / 45.0).powi(2)).exp()
}

/// Totals over a sample of battles.
#[derive(Default)]
struct Tally {
    battles: u32,
    defender_wins: u32,
    /// Men of the watched side killed by missiles.
    arrow_losses: f64,
    duration: f64,
}

impl Tally {
    fn line(&self, label: &str) -> String {
        let n = f64::from(self.battles.max(1));
        format!(
            "{label:<28} défenseur {:>2}/{:<2}  pertes au trait (camp observé) {:>6.1}/bataille  durée {:>4.0} s",
            self.defender_wins,
            self.battles,
            self.arrow_losses / n,
            self.duration / n
        )
    }
}

/// Runs to the end, adding the missile losses of the units `watched` picks.
fn run_and_count(
    mut sim: BattleSim,
    tally: &mut Tally,
    watched: impl Fn(&sim_battle::Unit) -> bool,
) {
    let ids: Vec<u32> = sim
        .units()
        .iter()
        .filter(|u| watched(u))
        .map(|u| u.id)
        .collect();
    while !sim.is_finished() && sim.elapsed() < 1800.0 {
        sim.step();
        for shot in sim.take_shots() {
            if shot.target.is_some_and(|t| ids.contains(&t)) {
                tally.arrow_losses += shot.kills;
            }
        }
    }
    tally.battles += 1;
    tally.defender_wins += u32::from(sim.winner() == Some(SideId::Defender));
    tally.duration += sim.elapsed();
}

/// Crest shift of a seed under `R4_JITTER=1` (−16 to +16 m), else 0.
fn jitter(seed: u64) -> f64 {
    if std::env::var("R4_JITTER").is_ok_and(|v| v == "1") {
        (seed * 7 % 9) as f64 * 4.0 - 16.0
    } else {
        0.0
    }
}

fn hedge(z: f64) -> Obstacle {
    Obstacle {
        a: (460.0, z),
        b: (740.0, z),
        kind: ObstacleKind::Hedge,
    }
}

/// English (defender) against French knights, on a crest at z 505 (45 m in
/// front of the English deployment line: the English get there before the
/// French foot).
fn english_position(data: &GameData, seed: u64, crest: bool, hedges: &[Obstacle]) -> BattleSim {
    // `R4_FRENCH=heavy`: 3 knights + 1 man-at-arms (a close fight).
    let french: &[&str] = if std::env::var("R4_FRENCH").is_ok_and(|v| v == "heavy") {
        &[
            "unit_knights",
            "unit_knights",
            "unit_knights",
            "unit_men_at_arms_foot",
            "unit_crossbowmen",
        ]
    } else {
        &[
            "unit_knights",
            "unit_knights",
            "unit_men_at_arms_foot",
            "unit_crossbowmen",
        ]
    };
    let english = [
        "unit_longbowmen",
        "unit_longbowmen",
        "unit_longbowmen",
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
    ];
    let mut battle = setup(units(data, french), units(data, &english), None);
    battle.village = Some(false);
    let mut sim = BattleSim::new(battle, seed).unwrap();
    sim.set_weather(sim_battle::Weather::Clear);
    let shift = jitter(seed);
    if crest {
        shape(&mut sim, ridge(505.0 + shift));
    } else {
        shape(&mut sim, |_| 0.0);
    }
    sim.field_mut()
        .obstacles
        .extend(hedges.iter().map(|&h| Obstacle {
            a: (h.a.0, h.a.1 + shift),
            b: (h.b.0, h.b.1 + shift),
            ..h
        }));
    sim
}

#[test]
#[ignore = "survey: run in release with --nocapture"]
fn survey_english_position_against_knights() {
    let data = data();
    let cases: [(&str, bool, Vec<Obstacle>); 4] = [
        ("crête + haie", true, vec![hedge(500.0)]),
        ("crête nue", true, vec![]),
        ("haie en creux + crête", true, vec![hedge(580.0)]),
        ("rase campagne", false, vec![]),
    ];
    for (label, crest, hedges) in cases {
        let mut tally = Tally::default();
        for seed in seeds() {
            let sim = english_position(&data, seed, crest, &hedges);
            // Watched: the French (arrows they take).
            run_and_count(sim, &mut tally, |u| u.side == SideId::Attacker);
        }
        println!("{}", tally.line(label));
    }
}

/// A weaker defender holding a crest (z 470) with one regiment of shooters
/// against an attacker with three regiments of longbows.
fn reverse_slope_battle(data: &GameData, seed: u64, ridged: bool) -> BattleSim {
    let attacker = [
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_urban_militia",
        "unit_longbowmen",
        "unit_longbowmen",
        "unit_longbowmen",
    ];
    let defender = [
        "unit_men_at_arms_foot",
        "unit_men_at_arms_foot",
        "unit_longbowmen",
    ];
    let mut battle = setup(units(data, &attacker), units(data, &defender), None);
    battle.village = Some(false);
    let mut sim = BattleSim::new(battle, seed).unwrap();
    sim.set_weather(sim_battle::Weather::Clear);
    if ridged {
        shape(&mut sim, ridge(470.0));
    } else {
        shape(&mut sim, |_| 0.0);
    }
    sim
}

#[test]
#[ignore = "survey: run in release with --nocapture"]
fn survey_reverse_slope_against_longbows() {
    let data = data();
    for (label, ridged) in [("crête (contre-pente)", true), ("rase campagne", false)] {
        let mut tally = Tally::default();
        let mut before_contact = 0.0;
        for seed in seeds() {
            let mut sim = reverse_slope_battle(&data, seed, ridged);
            let line: Vec<u32> = sim
                .units()
                .iter()
                .filter(|u| u.side == SideId::Defender && !u.can_shoot())
                .map(|u| u.id)
                .collect();
            // Arrow losses of the defender's foot before the first melee.
            while !sim.is_finished() && !sim.units().iter().any(|u| u.state == UnitState::Melee) {
                sim.step();
                for shot in sim.take_shots() {
                    if shot.target.is_some_and(|t| line.contains(&t)) {
                        before_contact += shot.kills;
                    }
                }
            }
            run_and_count(sim, &mut tally, |u| {
                u.side == SideId::Defender && !u.can_shoot()
            });
        }
        let n = f64::from(tally.battles.max(1));
        println!("{}", tally.line(&format!("{label}, après le contact")));
        println!(
            "{label} : pertes au trait de la ligne avant le contact {:.1}/bataille",
            before_contact / n
        );
    }
}

#[test]
#[ignore = "debug trace: R4_TRACE=case,seed (case: crest_hedge, bare, hollow, open)"]
fn trace_english_position() {
    let data = data();
    let spec = std::env::var("R4_TRACE").unwrap_or_else(|_| "crest_hedge,0".into());
    let (case, seed) = spec.split_once(',').unwrap();
    let seed: u64 = seed.parse().unwrap();
    let (crest, hedges) = match case {
        "bare" => (true, vec![]),
        "hollow" => (true, vec![hedge(580.0)]),
        "open" => (false, vec![]),
        _ => (true, vec![hedge(500.0)]),
    };
    let mut sim = english_position(&data, seed, crest, &hedges);
    let mut t = 0.0;
    while !sim.is_finished() && sim.elapsed() < 1800.0 {
        if sim.elapsed() >= t {
            println!("--- t={:.0}", sim.elapsed());
            for u in sim.units().iter().filter(|u| u.present()) {
                println!(
                    "{:>2} {:<3} {:<22} x{:>5.0} z{:>5.0} h{:>5.1} hp{:>4.0} m{:>3.0} {:?} amm{} stk{} tgt{:?}",
                    u.id,
                    &u.side.key()[..3],
                    u.unit_type,
                    u.x,
                    u.z,
                    sim.field().height(u.x, u.z),
                    u.hp,
                    u.morale,
                    u.state,
                    u.ammo,
                    u8::from(u.stakes_planted),
                    u.target
                );
            }
            t += std::env::var("R4_STEP")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(20.0);
        }
        sim.step();
    }
    println!("winner {:?} at {:.0}", sim.winner(), sim.elapsed());
    for e in sim.events() {
        println!("{:>5.0} {}", e.time, e.text_fr);
    }
}
