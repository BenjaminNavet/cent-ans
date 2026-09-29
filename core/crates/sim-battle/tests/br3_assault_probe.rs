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
    /// T4: how the battle ended (`square_held`, `rout`, `nightfall`...).
    end: &'static str,
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
    sim.heat_intensity(u)
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
    let mut ram_hp: Option<f64> = sim.units().iter().find(|u| u.ram).map(|u| u.hp);
    let mut ram_losses: std::collections::BTreeMap<String, f64> = Default::default();
    let mut shooters_log: Vec<String> = Vec::new();
    while !sim.is_finished() && sim.elapsed() < limit_s {
        sim.step();
        if std::env::var("BR3_RAM").is_ok() {
            if let Some(r) = sim.units().iter().find(|u| u.ram) {
                let before = ram_hp.unwrap_or(r.hp);
                if r.hp < before - 1e-9 {
                    let by = r
                        .loss_by
                        .and_then(|id| sim.units().iter().find(|u| u.id == id))
                        .map(|u| format!("{} on_wall {}", u.unit_type, u.on_wall))
                        .unwrap_or_else(|| "?".to_owned());
                    *ram_losses
                        .entry(format!("{:?} by {by}", r.loss_cause))
                        .or_default() += before - r.hp;
                }
                ram_hp = Some(r.hp);
            }
            if ((sim.elapsed() / sim_battle::DT).round() as u64).is_multiple_of(600) {
                let line: Vec<String> = sim
                    .units()
                    .iter()
                    .filter(|u| u.side == SideId::Defender && u.present())
                    .map(|u| {
                        format!(
                            "{}({:.0},{:.0} {:?} wall {} hp {:.0} m {:.0})",
                            &u.unit_type[5..],
                            u.x,
                            u.z,
                            u.state,
                            u.on_wall,
                            u.hp,
                            u.morale
                        )
                    })
                    .collect();
                shooters_log.push(format!("{:.0}: {}", sim.elapsed(), line.join(" ")));
            }
        }
        // T4: the garrison's morale every 5 s (`BR3_WATCH`).
        if std::env::var("BR3_WATCH").is_ok()
            && ((sim.elapsed() / DT).round() as u64).is_multiple_of((5.0 / DT).round() as u64)
        {
            let c = sim.siege().unwrap().center;
            let line: Vec<String> = sim
                .units()
                .iter()
                .filter(|u| {
                    u.present() && !u.synthetic && u.category != data_model::UnitCategory::Siege
                })
                .map(|u| {
                    format!(
                        "{}{}[{:.0}m {:?} w{} hp{:.0} m{:.0} f{:.0}]",
                        if u.side == SideId::Defender { "D" } else { "A" },
                        &u.unit_type[5..9],
                        (u.x - c.0).hypot(u.z - c.1),
                        u.state,
                        u8::from(u.on_wall),
                        u.hp,
                        u.morale,
                        u.fatigue
                    )
                })
                .collect();
            println!("W {:.0}: {}", sim.elapsed(), line.join(" "));
        }
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
        let second = ((now / DT).round() as u64).is_multiple_of((1.0 / DT).round() as u64);
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
                // T4: when and where the garrison breaks (`BR3_DEF_ROUTS`).
                if u.state == UnitState::Routing
                    && pstate != UnitState::Routing
                    && std::env::var("BR3_DEF_ROUTS").is_ok()
                {
                    let c = works.center;
                    println!(
                        "D   {town:?} seed {seed} rout {} at {now:.0} s, {:.0} m from the square, wall {}, hp {:.0}/{}, openings {}",
                        u.unit_type,
                        (u.x - c.0).hypot(u.z - c.1),
                        u.on_wall,
                        u.hp,
                        u.initial_soldiers,
                        works.openings().len(),
                    );
                }
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
    if std::env::var("BR3_RAM").is_ok() {
        let (mut blows, mut oil, mut first_blow, mut gate_at) = (0, 0, None, None);
        for fx in sim.siege_fx() {
            match fx.kind {
                sim_battle::SiegeFxKind::RamStrike { .. } => {
                    blows += 1;
                    first_blow.get_or_insert(fx.time.round());
                }
                sim_battle::SiegeFxKind::BoilingOil { .. } => oil += 1,
                sim_battle::SiegeFxKind::GateBroken { .. } => {
                    gate_at.get_or_insert(fx.time.round());
                }
                _ => {}
            }
        }
        let ram = sim
            .units()
            .iter()
            .find(|u| u.ram)
            .map(|u| (u.hp.round(), u.state));
        println!("R   ram losses {ram_losses:?}");
        if std::env::var("BR3_DEF").is_ok() {
            for l in &shooters_log {
                println!("R   {l}");
            }
        }
        let w = sim.siege().unwrap();
        println!(
            "R {town:?} seed {seed}: won {:?}, blows {blows} (first {first_blow:?}), oil {oil}, gate at {gate_at:?}, gate hp {:.0}/{:.0}, ram {ram:?}",
            sim.winner(),
            w.pieces[w.gate].hp,
            w.pieces[w.gate].max_hp,
        );
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
        end: sim.outcome().map_or("—", |o| o.end.key()),
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
    println!("| Ville | Maisons | Victoires assaillant | Durée médiane (s) | Pertes assaillant | Pertes garnison | Maisons brûlées (moy.) | Fins |");
    println!("|---|---|---|---|---|---|---|---|");
    let mut traces: Vec<(String, Vec<Trace>, Vec<bool>)> = Vec::new();
    for town in towns() {
        let town = town.as_deref();
        let runs: Vec<Outcome> = (1..=seeds)
            .map(|s| assault(&data, town, s, limit))
            .collect();
        let n = runs.len() as f64;
        let wins = runs.iter().filter(|r| r.attacker_won).count();
        let mut ends: std::collections::BTreeMap<&str, usize> = Default::default();
        for r in &runs {
            *ends.entry(r.end).or_default() += 1;
        }
        let ends: Vec<String> = ends.iter().map(|(k, n)| format!("{k} {n}")).collect();
        println!(
            "| {} | {} | {}/{} | {:.0} | {:.0} | {:.0} | {:.1} | {} |",
            town.unwrap_or("générique"),
            runs[0].houses,
            wins,
            runs.len(),
            median(runs.iter().map(|r| r.ended).collect()),
            runs.iter().map(|r| r.attacker_lost).sum::<f64>() / n,
            runs.iter().map(|r| r.defender_lost).sum::<f64>() / n,
            runs.iter().map(|r| r.burnt as f64).sum::<f64>() / n,
            ends.join(", "),
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
    let var = |k: &str| std::env::var(k).ok().and_then(|s| s.parse::<f64>().ok());
    if let Some(v) = var("HEAT_MORALE") {
        rules.heat.morale_per_s = v;
    }
    if let Some(v) = var("HEAT_RADIUS") {
        rules.heat.radius_m = v;
    }
    if let Some(v) = var("HEAT_LOSS") {
        rules.heat.loss_per_s = v;
    }
    if let Some(v) = var("WALL_HEAT") {
        rules.heat.wall_walk_factor = v;
    }
    if let Some(v) = var("OVERSHOOT") {
        rules.ignition.overshoot_m = v;
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

/// BR3b: an ASCII map of one assault at `SNAP_T` seconds (`TOWN`, `SEED`):
/// walls (`#`, `O` open), blocks (`H`, `F` burning, `,` burnt), heat
/// around the fires (`~`), the square (`*`), regiments (`A`/`D`, `r`
/// routing) and the A* way of the first attacker foot regiment inside the
/// walls to the square (`+`). 8 m per character.
#[test]
#[ignore = "probe: prints an ASCII map of an assault"]
fn probe_assault_map() {
    let data = data();
    let town = std::env::var("TOWN").ok().filter(|t| t != "generic");
    let seed: u64 = std::env::var("SEED")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2);
    let times: Vec<f64> = std::env::var("SNAP_T")
        .unwrap_or_else(|_| "480".to_owned())
        .split(',')
        .filter_map(|s| s.parse().ok())
        .collect();
    let mut setup = setup(
        units(&data, &BESIEGERS),
        units(&data, &GARRISON),
        Some(SiegeSetup {
            fortification: 2,
            breach: 40,
        }),
    );
    setup.siege_layout = town
        .as_deref()
        .map(|id| SiegeLayout::from_landmark(&data.landmarks[id]).expect("siege.battle"));
    let mut sim = BattleSim::new(setup, seed).unwrap();
    tune_fire(&mut sim);
    sim.set_ai(SideId::Attacker, true);
    sim.set_ai(SideId::Defender, true);
    if sim.is_deploying() {
        sim.start_battle().expect("start");
    }
    for t in times {
        while !sim.is_finished() && sim.elapsed() < t {
            sim.step();
        }
        let works = sim.siege().unwrap();
        let heat = sim.fire_rules().unwrap().heat.radius_m;
        let cell = 8.0;
        let (mut x0, mut z0, mut x1, mut z1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
        for v in &works.vertices {
            x0 = x0.min(v.0);
            z0 = z0.min(v.1);
            x1 = x1.max(v.0);
            z1 = z1.max(v.1);
        }
        let (x0, z0, x1, z1) = (x0 - 60.0, z0 - 120.0, x1 + 20.0, z1 + 20.0);
        let (nx, nz) = (((x1 - x0) / cell) as usize, ((z1 - z0) / cell) as usize);
        let mut grid = vec![vec![' '; nx]; nz];
        for (iz, row) in grid.iter_mut().enumerate() {
            for (ix, ch) in row.iter_mut().enumerate() {
                let (x, z) = (x0 + (ix as f64 + 0.5) * cell, z0 + (iz as f64 + 0.5) * cell);
                if let Some(p) = works.pieces.iter().find(|p| p.distance(x, z) < cell * 0.5) {
                    *ch = if p.intact() { '#' } else { 'O' };
                } else if let Some(h) = works.houses.iter().find(|h| h.edge_distance(x, z) < 0.0) {
                    *ch = if h.fire.burning() {
                        'F'
                    } else if h.fire.burnt() {
                        ','
                    } else {
                        'H'
                    };
                } else if works
                    .houses
                    .iter()
                    .any(|h| h.fire.burning() && h.edge_distance(x, z) < heat)
                {
                    *ch = '~';
                } else if works.in_square(x, z) {
                    *ch = '*';
                } else if works.inside(x, z) {
                    *ch = '.';
                }
            }
        }
        let mut put = |x: f64, z: f64, c: char| {
            let (ix, iz) = (((x - x0) / cell) as isize, ((z - z0) / cell) as isize);
            if ix >= 0 && iz >= 0 && (ix as usize) < nx && (iz as usize) < nz {
                grid[iz as usize][ix as usize] = c;
            }
        };
        let walker = sim.units().iter().find(|u| {
            u.side == SideId::Attacker
                && u.able()
                && u.category != data_model::UnitCategory::Siege
                && works.inside(u.x, u.z)
        });
        if let Some(u) = walker {
            if let Some(route) = sim.siege_route(SideId::Attacker, (u.x, u.z), works.center) {
                for (x, z) in route {
                    put(x, z, '+');
                }
            } else {
                println!("no route for {} at ({:.0}, {:.0})", u.unit_type, u.x, u.z);
            }
        }
        for u in sim.units().iter().filter(|u| u.present() && !u.synthetic) {
            let c = match (u.side, u.state == UnitState::Routing) {
                (_, true) => 'r',
                (SideId::Attacker, _) => 'A',
                (SideId::Defender, _) => 'D',
            };
            put(u.x, u.z, c);
        }
        println!(
            "T {town:?} seed {seed} at {:.0} s: burning {}, burnt {}, hold {:.0}",
            sim.elapsed(),
            works.burning_houses(),
            works.burnt_houses(),
            works.hold_time
        );
        for row in grid.iter().rev() {
            println!("T {}", row.iter().collect::<String>());
        }
        for h in works
            .houses
            .iter()
            .filter(|h| h.fire.burning() || h.fire.burnt())
        {
            let f = h.footprint();
            let wall = works
                .pieces
                .iter()
                .map(|p| f.distance_to_segment(p.a, p.b))
                .fold(f64::INFINITY, f64::min);
            println!(
                "T   fire block ({:.0}, {:.0}) {:.0} x {:.0} rows {} suburb {} burning {}: {wall:.1} m from the wall line",
                h.x, h.z, h.length, h.depth, h.rows, h.suburb, h.fire.burning()
            );
        }
        for u in sim.units().iter().filter(|u| u.present() && !u.synthetic) {
            println!(
                "T {:?} {:<22} ({:>4.0}, {:>4.0}) {:?} hp {:>4.0} morale {:>3.0} heat {:.2} dest {:?}",
                u.side,
                u.unit_type,
                u.x,
                u.z,
                u.state,
                u.hp,
                u.morale,
                heat_intensity(&sim, u),
                u.destination.map(|d| (d.0.round(), d.1.round()))
            );
            let (w, d) = u.extent();
            let rect = sim_battle::Footprint::new(u.x, u.z, w, d, -u.facing);
            let gap = works
                .houses
                .iter()
                .filter(|h| h.fire.burning())
                .map(|h| h.footprint().distance_to(&rect))
                .fold(f64::INFINITY, f64::min);
            let wall = works
                .pieces
                .iter()
                .map(|p| p.distance(u.x, u.z))
                .fold(f64::INFINITY, f64::min);
            println!(
                "T     extent {w:.1} x {d:.1}, on wall {}, gap to fire {gap:.1} m, centre to wall line {wall:.1} m",
                u.on_wall
            );
        }
    }
}
