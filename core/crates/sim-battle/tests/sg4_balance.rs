//! SG4 survey (ignored): attacker against defender at epic scale with
//! mirrored armies, on flat ground or with a crest in front of the defender,
//! with and without the archers' stakes (R4, ADR 0046).
//!
//! `cargo test --release -p sim-battle --test sg4_balance -- --ignored --nocapture`
//! (`SG4_SEEDS=1..11`, `SG4_REGIMENTS=60` by default).

mod common;

use common::*;
use data_model::Ability;
use sim_battle::{BattleSim, SideId};

const KINDS: [&str; 5] = [
    "unit_men_at_arms_foot",
    "unit_longbowmen",
    "unit_knights",
    "unit_urban_militia",
    "unit_crossbowmen",
];

fn seeds() -> std::ops::Range<u64> {
    std::env::var("SG4_SEEDS")
        .ok()
        .and_then(|s| {
            let (a, b) = s.split_once("..")?;
            Some(a.parse().ok()?..b.parse().ok()?)
        })
        .unwrap_or(1..11)
}

fn regiments() -> usize {
    std::env::var("SG4_REGIMENTS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60)
}

/// `ground`: 0 flat, 1 crest 45 m in front of the defender, 2 the generated
/// plains field without village in clear weather, 3 the `ep1_scale` battle
/// (generated plains, village and weather drawn).
fn battle(seed: u64, ground: u8, stakes: bool) -> BattleSim {
    let data = data();
    let army: Vec<&str> = (0..regiments()).map(|i| KINDS[i % KINDS.len()]).collect();
    let mut setup = setup(units(&data, &army), units(&data, &army), None);
    if ground != 3 {
        setup.village = Some(false);
    }
    for unit in setup
        .attacker
        .units
        .iter_mut()
        .chain(setup.defender.units.iter_mut())
    {
        unit.soldiers = 120;
        unit.max_soldiers = 120;
        if !stakes {
            unit.abilities.retain(|a| *a != Ability::Stakes);
        }
    }
    let mut sim = BattleSim::new(setup, seed).unwrap();
    if ground != 3 {
        sim.set_weather(sim_battle::Weather::Clear);
    }
    sim.set_ai(SideId::Attacker, true);
    sim.set_ai(SideId::Defender, true);
    if ground >= 2 {
        return sim;
    }
    let crest = ground == 1;
    let crest_z = sim.field().defender_line_z() - 45.0;
    let field = sim.field_mut();
    field.forests.clear();
    field.forest_parts.clear();
    field.mud.clear();
    field.mud_parts.clear();
    field.pools.clear();
    field.obstacles.clear();
    field.river = None;
    field.bridges.clear();
    let (nx, res) = (field.nx, field.resolution);
    for (k, h) in field.heights.iter_mut().enumerate() {
        let z = (k / nx) as f64 * res;
        *h = if crest {
            20.0 * (-((z - crest_z) / 45.0).powi(2)).exp()
        } else {
            0.0
        };
    }
    sim
}

#[derive(Default)]
struct Tally {
    attacker: u32,
    defender: u32,
    draws: u32,
    duration: f64,
    /// Share of its soldiers each side lost.
    attacker_loss: f64,
    defender_loss: f64,
}

fn losses(sim: &BattleSim, side: SideId) -> f64 {
    let (left, full) = sim
        .units()
        .iter()
        .filter(|u| u.side == side && !u.synthetic)
        .fold((0.0, 0.0), |(l, f), u| {
            (l + u.hp.max(0.0), f + f64::from(u.initial_soldiers))
        });
    1.0 - left / full.max(1.0)
}

#[test]
#[ignore = "survey: epic attacker/defender matrix"]
fn survey_epic_attacker_defender() {
    println!(
        "| Terrain | Pieux | Attaquant gagne | Défenseur gagne | Nuls | Pertes att. | Pertes déf. | Durée (s) |"
    );
    println!("|---|---|---|---|---|---|---|---|");
    let cases: Vec<(u8, bool)> = match std::env::var("SG4_CASES").as_deref() {
        Ok("flat") => vec![(0, true), (0, false)],
        Ok("generated") => vec![(2, true), (2, false)],
        Ok("flat_bare") => vec![(0, false)],
        Ok("crest") => vec![(1, true), (1, false)],
        Ok("ep1") => vec![(3, true)],
        _ => vec![
            (0, true),
            (0, false),
            (1, true),
            (1, false),
            (2, true),
            (2, false),
            (3, true),
        ],
    };
    for (ground, stakes) in cases {
        let mut t = Tally::default();
        for seed in seeds() {
            let mut sim = battle(seed, ground, stakes);
            let sides: Vec<SideId> = sim.units().iter().map(|u| u.side).collect();
            let mut shot = [0.0f64; 2];
            let mut shots: std::collections::BTreeMap<String, (f64, f64, f64)> =
                Default::default();
            let mut contact_at: Option<f64> = None;
            let trace = std::env::var("SG4_TRACE").is_ok();
            let mut next_trace = 120.0;
            let routs = std::env::var("SG4_ROUTS").is_ok();
            let mut was_routing: Vec<bool> = vec![false; sim.units().len()];
            while !sim.is_finished() && sim.elapsed() < 1800.0 {
                sim.step();
                if routs {
                    for (k, u) in sim.units().iter().enumerate() {
                        let now = u.state == sim_battle::UnitState::Routing;
                        if now && !was_routing[k] && u.side == SideId::Defender {
                            let friends = sim
                                .units()
                                .iter()
                                .filter(|f| {
                                    f.side == u.side
                                        && f.id != u.id
                                        && f.present()
                                        && f.state == sim_battle::UnitState::Routing
                                        && (f.x - u.x).hypot(f.z - u.z) < 120.0
                                })
                                .count();
                            let foe = sim
                                .units()
                                .iter()
                                .filter(|e| e.side != u.side && e.able())
                                .map(|e| (e.x - u.x).hypot(e.z - u.z))
                                .fold(f64::INFINITY, f64::min);
                            eprintln!(
                                "  rout t {:.0} {:<22} ({:.0},{:.0}) hp {:.0} routing friends {} foe {:.0} m",
                                sim.elapsed(), u.unit_type, u.x, u.z, u.hp, friends, foe
                            );
                        }
                        if k < was_routing.len() {
                            was_routing[k] = now;
                        }
                    }
                }
                let horse_window = std::env::var("SG4_HORSE").ok().map(|w| {
                    w.split_once("..")
                        .and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?)))
                        .unwrap_or((180.0, 260.0))
                });
                if let Some((from, to)) = horse_window.filter(|&(from, to): &(f64, f64)| {
                    sim.elapsed() > from && sim.elapsed() < to
                }) {
                    let _ = (from, to);
                }
                if horse_window.is_some_and(|(from, to)| sim.elapsed() > from && sim.elapsed() < to)
                    && sim.ticks().is_multiple_of(25)
                {
                    for u in sim
                        .units()
                        .iter()
                        .filter(|u| u.unit_type == "unit_knights" && u.present())
                        .take(200)
                    {
                        let tgt = u.target.map(|t| {
                            let e = &sim.units()[t as usize];
                            format!("{}:{}", t, &e.unit_type[5..9])
                        });
                        eprintln!(
                            "  t {:.0} {:?} {:>3} {:?} run {} ({:.0},{:.0}) face {:.2} tgt {:?} dest {:?} mor {:.0}",
                            sim.elapsed(), u.side, u.id, u.state, u.running, u.x, u.z, u.facing, tgt,
                            u.destination.map(|(x, z)| (x.round(), z.round())), u.morale
                        );
                    }
                }
                if trace && sim.elapsed() >= next_trace {
                    next_trace += 30.0;
                    eprintln!("  t {:.0}", sim.elapsed());
                    for side in [SideId::Attacker, SideId::Defender] {
                        for k in KINDS {
                            let us: Vec<&sim_battle::Unit> = sim
                                .units()
                                .iter()
                                .filter(|u| u.side == side && u.unit_type == k && u.present())
                                .collect();
                            let n = us.len().max(1) as f64;
                            let count = |st: sim_battle::UnitState| {
                                us.iter().filter(|u| u.state == st).count()
                            };
                            eprintln!(
                                "    {:?} {:<22} n {:>2} hp {:>4.0} mor {:>3.0} fat {:>3.0} z {:>5.0} melee {} chg {} march {} rout {} idle {} shoot {}",
                                side, k, us.len(),
                                us.iter().map(|u| u.hp).sum::<f64>() / n,
                                us.iter().map(|u| u.morale).sum::<f64>() / n,
                                us.iter().map(|u| u.fatigue).sum::<f64>() / n,
                                us.iter().map(|u| u.z).sum::<f64>() / n,
                                count(sim_battle::UnitState::Melee),
                                count(sim_battle::UnitState::Charging),
                                count(sim_battle::UnitState::Marching),
                                count(sim_battle::UnitState::Routing),
                                count(sim_battle::UnitState::Idle),
                                count(sim_battle::UnitState::Shooting),
                            );
                        }
                    }
                }
                for s in sim.take_shots() {
                    if let Some(t) = s.target {
                        shot[sides[t as usize].index()] += s.kills;
                        if std::env::var("SG4_SHOTS").is_ok() {
                            let (a, b) = (&sim.units()[s.shooter as usize], &sim.units()[t as usize]);
                            let key = format!("{:?} {} -> {}", a.side, &a.unit_type[5..9], &b.unit_type[5..9]);
                            let d = (s.from.0 - s.aim.0).hypot(s.from.1 - s.aim.1);
                            let e = shots.entry(key).or_insert((0.0, 0.0, 0.0));
                            e.0 += s.kills;
                            e.1 += d;
                            e.2 += 1.0;
                        }
                    }
                }
                if contact_at.is_none()
                    && sim
                        .units()
                        .iter()
                        .any(|u| u.state == sim_battle::UnitState::Melee)
                {
                    contact_at = Some(sim.elapsed());
                }
            }
            for (k, (kills, d, n)) in &shots {
                eprintln!("   shots {k}: kills {kills:.0}, volleys {n}, mean range {:.0} m", d / n);
            }
            let initial = |side: SideId| -> f64 {
                sim.units()
                    .iter()
                    .filter(|u| u.side == side && !u.synthetic)
                    .map(|u| f64::from(u.initial_soldiers))
                    .sum()
            };
            eprintln!(
                "   missiles: attacker lost {:.0} %, defender {:.0} %; first melee at {:.0} s",
                shot[0] / initial(SideId::Attacker) * 100.0,
                shot[1] / initial(SideId::Defender) * 100.0,
                contact_at.unwrap_or(-1.0)
            );
            let by_kind = |side: SideId| -> String {
                KINDS
                    .iter()
                    .map(|k| {
                        let (l, f) = sim
                            .units()
                            .iter()
                            .filter(|u| u.side == side && u.unit_type == *k)
                            .fold((0.0, 0.0), |(l, f), u| {
                                (l + u.hp.max(0.0), f + f64::from(u.initial_soldiers))
                            });
                        format!("{} {:.0}", &k[5..9], (1.0 - l / f64::max(f, 1.0)) * 100.0)
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            eprintln!(
                "   lost by kind: att [{}] def [{}]",
                by_kind(SideId::Attacker),
                by_kind(SideId::Defender)
            );
            match sim.winner() {
                Some(SideId::Attacker) => t.attacker += 1,
                Some(SideId::Defender) => t.defender += 1,
                None => t.draws += 1,
            }
            t.duration += sim.elapsed();
            t.attacker_loss += losses(&sim, SideId::Attacker);
            t.defender_loss += losses(&sim, SideId::Defender);
            eprintln!(
                "ground {ground} stakes {stakes} seed {seed}: {:?} at {:.0} s, losses {:.0} % / {:.0} %",
                sim.winner(),
                sim.elapsed(),
                losses(&sim, SideId::Attacker) * 100.0,
                losses(&sim, SideId::Defender) * 100.0
            );
        }
        let n = f64::from(t.attacker + t.defender + t.draws);
        println!(
            "| {} | {} | {} | {} | {} | {:.0} % | {:.0} % | {:.0} |",
            ["plat", "crête", "plaine générée", "comme ep1_scale"][usize::from(ground)],
            if stakes { "oui" } else { "non" },
            t.attacker,
            t.defender,
            t.draws,
            t.attacker_loss / n * 100.0,
            t.defender_loss / n * 100.0,
            t.duration / n
        );
    }
}
