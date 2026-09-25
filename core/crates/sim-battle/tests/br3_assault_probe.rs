//! BR3 probe: siege assaults with both sides under AI, generic town and two
//! landmark towns (Paris, Rouen), over several seeds. Prints the attacker's
//! win rate, the median battle length, the losses and the houses burnt, to
//! compare the town layouts before and after densification.
//!
//! `cargo test -p sim-battle --release --test br3_assault_probe -- --ignored --nocapture`
//! (`SEEDS=n`, `LIMIT_S=s` override the defaults).

mod common;

use common::*;
use data_model::GameData;
use sim_battle::{BattleSim, SideId, SiegeLayout, SiegeSetup, UnitSetup, UnitState, DT};

const BESIEGERS: [&str; 11] = [
    "unit_men_at_arms_foot",
    "unit_men_at_arms_foot",
    "unit_urban_militia",
    "unit_urban_militia",
    "unit_urban_militia",
    "unit_longbowmen",
    "unit_longbowmen",
    "unit_knights",
    "unit_trebuchet",
    "unit_mangonel",
    "unit_siege_tower",
];
const GARRISON: [&str; 6] = [
    "unit_urban_militia",
    "unit_urban_militia",
    "unit_crossbowmen",
    "unit_crossbowmen",
    "unit_men_at_arms_foot",
    "unit_welsh_spearmen",
];

#[derive(Default)]
struct Outcome {
    attacker_won: bool,
    ended: f64,
    attacker_lost: f64,
    defender_lost: f64,
    burnt: usize,
    houses: usize,
    /// BR3b diagnostics (`BR3_TRACE`): seconds from the start.
    trace: Trace,
}

/// BR3b: what happened to the attackers (`BR3_TRACE=1` prints it).
#[derive(Default, Clone)]
struct Trace {
    /// First opening (breach or broken gate).
    opening_at: Option<f64>,
    /// First able attacker regiment inside the walls, on the square.
    inside_at: Option<f64>,
    square_at: Option<f64>,
    /// Longest hold of the square (seconds, before the decay).
    best_hold: f64,
    /// Attacker regiment-seconds within the heat of a burning house, and
    /// the morale and soldiers the heat cost them (same rule as the sim).
    heat_s: f64,
    heat_morale: f64,
    heat_loss: f64,
    /// Same for the garrison.
    def_heat_morale: f64,
    /// Attacker foot regiment-seconds inside the walls, marching, with a
    /// destination, that did not move (pressed against a façade, a prop).
    stuck_s: f64,
    /// Attacker foot regiment-seconds inside the walls.
    inside_s: f64,
    /// Attacker regiments that routed, and where (inside the walls, within
    /// heat of a fire).
    routs: usize,
    routs_inside: usize,
    routs_heated: usize,
    first_rout: Option<f64>,
    /// Houses burning / burnt when the first opening appeared.
    burnt_at_opening: usize,
    /// Mean morale of the able attacker foot inside the walls, sampled
    /// every second.
    inside_morale: Vec<f64>,
}

fn heat_intensity(sim: &BattleSim, u: &sim_battle::Unit) -> f64 {
    let (Some(works), Some(rules)) = (sim.siege(), sim.fire_rules()) else {
        return 0.0;
    };
    works
        .houses
        .iter()
        .filter(|h| h.fire.burning())
        .filter(|h| u.distance_to_rect(h.x, h.z) - h.radius <= rules.heat.radius_m)
        .map(|h| h.fire.intensity)
        .fold(0.0, f64::max)
}

fn assault(data: &GameData, town: Option<&str>, seed: u64, limit_s: f64) -> Outcome {
    let attackers: Vec<UnitSetup> = units(data, &BESIEGERS);
    let env = |k: &str, d: u32| {
        std::env::var(k)
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(d)
    };
    let mut setup = setup(
        attackers,
        units(data, &GARRISON),
        Some(SiegeSetup {
            fortification: env("FORT", 2),
            breach: env("BREACH", 40) as u8,
        }),
    );
    setup.siege_layout =
        town.map(|id| SiegeLayout::from_landmark(&data.landmarks[id]).expect("siege.battle"));
    let mut sim = BattleSim::new(setup, seed).unwrap();
    tune_fire(&mut sim);
    sim.set_ai(SideId::Attacker, true);
    sim.set_ai(SideId::Defender, true);
    if sim.is_deploying() {
        sim.start_battle().expect("start");
    }
    let tracing = std::env::var("BR3_TRACE").is_ok();
    let mut t = Trace::default();
    let heat = sim.fire_rules().map(|r| r.heat.clone());
    let mut last: Vec<(f64, f64, UnitState)> =
        sim.units().iter().map(|u| (u.x, u.z, u.state)).collect();
    while !sim.is_finished() && sim.elapsed() < limit_s {
        sim.step();
        if !tracing {
            continue;
        }
        let now = sim.elapsed();
        let works = sim.siege().unwrap();
        if t.opening_at.is_none() && !works.openings().is_empty() {
            t.opening_at = Some(now);
            t.burnt_at_opening = works.burnt_houses() + works.burning_houses();
        }
        t.best_hold = t.best_hold.max(works.hold_time);
        let second = (now / DT).round() as u64 % (1.0 / DT).round() as u64 == 0;
        let mut morale_sum = (0.0, 0usize);
        for (i, u) in sim.units().iter().enumerate() {
            let (px, pz, pstate) = last[i];
            last[i] = (u.x, u.z, u.state);
            if !u.present() || u.synthetic {
                continue;
            }
            let hi = heat_intensity(&sim, u);
            if let Some(h) = heat.as_ref().filter(|_| hi > 0.0) {
                if u.side == SideId::Attacker {
                    t.heat_s += DT;
                    t.heat_morale += h.morale_per_s * hi * DT;
                    t.heat_loss += u.hp * h.loss_per_s * hi * DT;
                } else {
                    t.def_heat_morale += h.morale_per_s * hi * DT;
                }
            }
            if u.side != SideId::Attacker {
                continue;
            }
            let inside = works.inside(u.x, u.z);
            if u.state == UnitState::Routing && pstate != UnitState::Routing {
                t.routs += 1;
                t.routs_inside += usize::from(inside);
                t.routs_heated += usize::from(hi > 0.0);
                t.first_rout.get_or_insert(now);
                if std::env::var("BR3_VERBOSE").is_ok() {
                    println!(
                        "T   rout {} at {now:.0} s ({:.0}, {:.0}) inside {inside} heat {hi:.2} hp {:.0} morale {:.0}",
                        u.unit_type, u.x, u.z, u.hp, u.morale
                    );
                }
            }
            if !u.able() || u.category == data_model::UnitCategory::Siege {
                continue;
            }
            if inside && !u.on_wall {
                t.inside_at.get_or_insert(now);
                t.inside_s += DT;
                if u.state == UnitState::Marching
                    && u.destination.is_some()
                    && (u.x - px).hypot(u.z - pz) < 0.02
                {
                    t.stuck_s += DT;
                }
                morale_sum.0 += u.morale;
                morale_sum.1 += 1;
            }
            if works.in_square(u.x, u.z) {
                t.square_at.get_or_insert(now);
            }
        }
        if second && morale_sum.1 > 0 {
            t.inside_morale.push(morale_sum.0 / morale_sum.1 as f64);
        }
    }
    let lost = |side: SideId| {
        sim.units()
            .iter()
            .filter(|u| u.side == side && !u.synthetic)
            .map(|u| f64::from(u.initial_soldiers) - u.hp.max(0.0))
            .sum::<f64>()
    };
    let works = sim.siege().unwrap();
    if std::env::var("BR3_VERBOSE").is_ok() {
        println!(
            "{town:?} seed {seed}: winner {:?} at {:.0} s, openings {}, hold {:.0}, opening {:?}, inside {:?}, square {:?}, best hold {:.0}, routs {} (inside {}, heated {}), heat morale {:.0}, stuck {:.0}/{:.0} s",
            sim.winner(),
            sim.elapsed(),
            works.openings().len(),
            works.hold_time,
            t.opening_at.map(|v| v.round()),
            t.inside_at.map(|v| v.round()),
            t.square_at.map(|v| v.round()),
            t.best_hold,
            t.routs,
            t.routs_inside,
            t.routs_heated,
            t.heat_morale,
            t.stuck_s,
            t.inside_s,
        );
    }
    Outcome {
        attacker_won: sim.winner() == Some(SideId::Attacker),
        ended: sim.elapsed(),
        attacker_lost: lost(SideId::Attacker),
        defender_lost: lost(SideId::Defender),
        burnt: works.houses.iter().filter(|h| h.fire.burnt()).count(),
        houses: works.houses.iter().filter(|h| !h.suburb).count(),
        trace: t,
    }
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(f64::total_cmp);
    v[v.len() / 2]
}

/// Towns of the probe: `TOWNS=generic,paris,rouen` (default), any landmark
/// with a `siege.battle` block.
fn towns() -> Vec<Option<String>> {
    std::env::var("TOWNS")
        .unwrap_or_else(|_| "generic,paris,rouen".to_owned())
        .split(',')
        .map(|t| (t != "generic").then(|| t.to_owned()))
        .collect()
}

#[test]
#[ignore = "probe: prints assault outcomes of the generic and landmark towns"]
fn probe_town_assaults() {
    let data = data();
    let seeds: u64 = std::env::var("SEEDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    let limit: f64 = std::env::var("LIMIT_S")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1800.0);
    println!("| Ville | Maisons | Victoires assaillant | Durée médiane (s) | Pertes assaillant | Pertes garnison | Maisons brûlées (moy.) |");
    println!("|---|---|---|---|---|---|---|");
    let mut traces: Vec<(String, Vec<Trace>, Vec<bool>)> = Vec::new();
    for town in towns() {
        let town = town.as_deref();
        let runs: Vec<Outcome> = (1..=seeds)
            .map(|s| assault(&data, town, s, limit))
            .collect();
        let n = runs.len() as f64;
        let wins = runs.iter().filter(|r| r.attacker_won).count();
        println!(
            "| {} | {} | {}/{} | {:.0} | {:.0} | {:.0} | {:.1} |",
            town.unwrap_or("générique"),
            runs[0].houses,
            wins,
            runs.len(),
            median(runs.iter().map(|r| r.ended).collect()),
            runs.iter().map(|r| r.attacker_lost).sum::<f64>() / n,
            runs.iter().map(|r| r.defender_lost).sum::<f64>() / n,
            runs.iter().map(|r| r.burnt as f64).sum::<f64>() / n,
        );
        traces.push((
            town.unwrap_or("générique").to_owned(),
            runs.iter().map(|r| r.trace.clone()).collect(),
            runs.iter().map(|r| r.attacker_won).collect(),
        ));
    }
    if std::env::var("BR3_TRACE").is_err() {
        return;
    }
    println!();
    println!("| Ville | Issue | Parties | Ouverture (méd., s) | Maisons en feu à l'ouverture | Dedans (méd., s) | Place (méd., s) | Meilleure tenue (s) | Déroutes (dedans / chaleur) | Moral perdu à la chaleur (assaillant / garnison) | Pertes à la chaleur | Bloqué / dedans (s) | Moral moyen dedans |");
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for (town, runs, won) in &traces {
        for outcome in [true, false] {
            let set: Vec<&Trace> = runs
                .iter()
                .zip(won)
                .filter(|(_, &w)| w == outcome)
                .map(|(t, _)| t)
                .collect();
            if set.is_empty() {
                continue;
            }
            let n = set.len() as f64;
            let med = |f: &dyn Fn(&Trace) -> Option<f64>| {
                let v: Vec<f64> = set.iter().filter_map(|t| f(t)).collect();
                if v.is_empty() {
                    "—".to_owned()
                } else {
                    format!("{:.0} ({}/{})", median(v.clone()), v.len(), set.len())
                }
            };
            let mean = |f: &dyn Fn(&Trace) -> f64| set.iter().map(|t| f(t)).sum::<f64>() / n;
            let morale: Vec<f64> = set
                .iter()
                .flat_map(|t| t.inside_morale.iter().copied())
                .collect();
            println!(
                "| {town} | {} | {} | {} | {:.1} | {} | {} | {:.0} | {:.1} ({:.1} / {:.1}) | {:.0} / {:.0} | {:.1} | {:.0} / {:.0} | {:.0} |",
                if outcome { "victoire" } else { "défaite" },
                set.len(),
                med(&|t| t.opening_at),
                mean(&|t| t.burnt_at_opening as f64),
                med(&|t| t.inside_at),
                med(&|t| t.square_at),
                mean(&|t| t.best_hold),
                mean(&|t| t.routs as f64),
                mean(&|t| t.routs_inside as f64),
                mean(&|t| t.routs_heated as f64),
                mean(&|t| t.heat_morale),
                mean(&|t| t.def_heat_morale),
                mean(&|t| t.heat_loss),
                mean(&|t| t.stuck_s),
                mean(&|t| t.inside_s),
                if morale.is_empty() {
                    0.0
                } else {
                    morale.iter().sum::<f64>() / morale.len() as f64
                },
            );
        }
    }
}

/// Tuning sweeps: `FIRE_CHANCE`, `FIRE_REACH` override the spread.
fn tune_fire(sim: &mut BattleSim) {
    let mut rules = sim.fire_rules().unwrap().clone();
    if let Some(v) = std::env::var("FIRE_CHANCE")
        .ok()
        .and_then(|s| s.parse().ok())
    {
        rules.spread.chance_per_period = v;
    }
    if let Some(v) = std::env::var("FIRE_REACH")
        .ok()
        .and_then(|s| s.parse().ok())
    {
        rules.spread.edge_distance_m = v;
    }
    sim.set_fire_rules(Some(rules));
}

/// Houses caught by the fire 10 minutes after the house nearest the square
/// is set alight (lab: no AI, clear weather).
fn fire_spread(data: &GameData, town: Option<&str>, seed: u64) -> (usize, usize) {
    let mut setup = setup(
        units(data, &["unit_urban_militia"]),
        units(data, &["unit_urban_militia"]),
        Some(SiegeSetup {
            fortification: 2,
            breach: 0,
        }),
    );
    setup.siege_layout =
        town.map(|id| SiegeLayout::from_landmark(&data.landmarks[id]).expect("siege.battle"));
    let mut sim = BattleSim::new(setup, seed).unwrap();
    lab(&mut sim);
    // Tuning sweeps: `FIRE_CHANCE`, `FIRE_REACH` override the spread.
    let mut rules = sim.fire_rules().unwrap().clone();
    if let Some(v) = std::env::var("FIRE_CHANCE")
        .ok()
        .and_then(|s| s.parse().ok())
    {
        rules.spread.chance_per_period = v;
    }
    if let Some(v) = std::env::var("FIRE_REACH")
        .ok()
        .and_then(|s| s.parse().ok())
    {
        rules.spread.edge_distance_m = v;
    }
    sim.set_fire_rules(Some(rules));
    let works = sim.siege().unwrap();
    let (cx, cz) = works.center;
    let first = (0..works.houses.len())
        .filter(|&i| !works.houses[i].suburb)
        .min_by(|&a, &b| {
            let d = |i: usize| (works.houses[i].x - cx).hypot(works.houses[i].z - cz);
            d(a).total_cmp(&d(b))
        })
        .unwrap();
    let total = works.houses.iter().filter(|h| !h.suburb).count();
    sim.ignite_house(first);
    run(&mut sim, 600.0);
    let caught = sim
        .siege()
        .unwrap()
        .houses
        .iter()
        .filter(|h| !h.suburb && (h.fire.burning() || h.fire.burnt()))
        .count();
    (caught, total)
}

#[test]
#[ignore = "probe: prints how far a fire spreads in each town"]
fn probe_fire_spread() {
    let data = data();
    println!("| Ville | Maisons | Touchées après 10 min (moy.) | Part |");
    println!("|---|---|---|---|");
    for town in [None, Some("paris"), Some("rouen")] {
        let runs: Vec<(usize, usize)> = (1..=10).map(|s| fire_spread(&data, town, s)).collect();
        let mean = runs.iter().map(|r| r.0 as f64).sum::<f64>() / runs.len() as f64;
        println!(
            "| {} | {} | {:.1} | {:.0} % |",
            town.unwrap_or("générique"),
            runs[0].1,
            mean,
            100.0 * mean / runs[0].1 as f64
        );
    }
}

/// Cost of the figure poses (BR3 push-out) in a crowded siege at 2.5
/// figures per soldier, against the bare layout of the figures.
#[test]
#[ignore = "probe: prints the cost of the figure poses"]
fn probe_figure_cost() {
    let data = data();
    let mut attackers = Vec::new();
    let mut defenders = Vec::new();
    for _ in 0..4 {
        attackers.extend(units(&data, &BESIEGERS[..8]));
        defenders.extend(units(&data, &GARRISON));
    }
    let s = setup(
        attackers,
        defenders,
        Some(SiegeSetup {
            fortification: 2,
            breach: 60,
        }),
    );
    let mut sim = BattleSim::new(s, 3).unwrap();
    sim.set_ai(SideId::Attacker, true);
    sim.set_ai(SideId::Defender, true);
    if sim.is_deploying() {
        sim.start_battle().expect("start");
    }
    run(&mut sim, 240.0);
    let scale = 2.5;
    let figures: usize = sim
        .units()
        .iter()
        .map(|u| sim.soldier_poses(u, scale).len())
        .sum();
    let frames = 50;
    let t = std::time::Instant::now();
    for _ in 0..frames {
        for u in sim.units() {
            std::hint::black_box(sim.soldier_poses(u, scale));
        }
    }
    let with = t.elapsed().as_secs_f64() * 1e6 / f64::from(frames);
    let t = std::time::Instant::now();
    for _ in 0..frames {
        for u in sim.units() {
            std::hint::black_box(u.figure_positions(scale));
        }
    }
    let raw = t.elapsed().as_secs_f64() * 1e6 / f64::from(frames);
    println!(
        "{} regiments, {figures} figures: poses {with:.0} µs/frame, bare layout {raw:.0} µs/frame",
        sim.units().len()
    );
}
