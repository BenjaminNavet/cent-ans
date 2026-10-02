//! Lot JR4: balance probe of the AI-led crusader faction (5 seeds × 50
//! turns with the real AI for every faction). Prints one table per seed:
//! fervour, treasury, units, Holy Land places held, landings, status.
//!
//! `cargo test -p ai --test jr_crusade_probe -- --ignored --nocapture`;
//! `JR_SEEDS=3,4` picks the seeds, `JR_TURNS=60` the length, `JR_TRACE=1`
//! prints the crusaders' armies and orders every turn.

use std::collections::BTreeSet;
use std::path::PathBuf;

use data_model::{FactionId, GameData, ProvinceId, SettlementId};
use sim_campaign::{CampaignState, EventKind, Order};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    ai::feudal::install();
    GameData::load(&root).expect("game data loads").0
}

/// What one seed did.
#[derive(Debug, Default, Clone)]
pub struct SeedReport {
    pub seed: u64,
    /// Fervour at turns 10, 20, 30... (255: faction dead).
    pub fervor: Vec<u8>,
    pub fervor_min: u8,
    pub fervor_max: u8,
    /// Turns spent at 95 or more.
    pub saturated_turns: u32,
    pub treasury: Vec<i64>,
    /// Turns with a negative treasury, and the longest such streak.
    pub negative_turns: u32,
    pub negative_streak: u32,
    pub units: Vec<usize>,
    /// Holy Land places held at the checkpoints.
    pub places: Vec<usize>,
    /// Times an army of the faction arrived in the Holy Land from outside.
    pub landings: u32,
    pub first_landing: Option<u32>,
    /// Holy Land places the faction besieged at least once.
    pub sieges: BTreeSet<SettlementId>,
    /// Holy Land places the faction held at least once.
    pub taken: BTreeSet<SettlementId>,
    pub target_turn: Option<u32>,
    pub dead_turn: Option<u32>,
    /// Provinces held by the holder of the target at the start, at the
    /// checkpoints.
    pub holder_provinces: Vec<usize>,
    pub contingents: u32,
}

fn holy(state: &CampaignState, holy_land: &[ProvinceId], settlement: &SettlementId) -> bool {
    state
        .settlement_province(settlement)
        .is_some_and(|p| holy_land.contains(p))
}

pub fn run_seed(data: &GameData, seed: u64, turns: u32, trace: bool) -> SeedReport {
    let rules = data.crusade_rules.as_ref().expect("rules");
    let faction = rules.faction.clone();
    let mut state = CampaignState::new_1337(data, FactionId::new("fac_france").unwrap(), seed)
        .expect("1337 start");
    state.interactive_battles = false;
    let holder = state
        .province_controller(&rules.target_province)
        .cloned()
        .expect("the target has a master");
    let holder_count = |state: &CampaignState| {
        state
            .provinces
            .keys()
            .filter(|p| state.controls_province(&holder, p))
            .count()
    };
    let mut report = SeedReport {
        seed,
        fervor_min: 100,
        ..SeedReport::default()
    };
    let mut ashore: BTreeSet<sim_campaign::ArmyId> = BTreeSet::new();
    let mut streak = 0;
    for turn in 1..=turns {
        let planner = |s: &CampaignState, d: &GameData, f: &FactionId| -> Vec<Order> {
            let orders = ai::plan_turn(s, d, f);
            if trace && f == &faction {
                for (id, army) in s.armies.iter().filter(|(_, a)| a.faction == faction) {
                    println!(
                        "  [s{seed} t{turn}] {id} at {:?} units {} power {:.0} stance {:?}",
                        army.position,
                        army.units.len(),
                        s.army_power(d, id),
                        army.stance
                    );
                }
                println!("  [s{seed} t{turn}] orders {orders:?}");
            }
            orders
        };
        let events = state.end_turn_with(data, planner);
        if trace {
            for e in events.iter().filter(|e| {
                e.faction.as_ref() == Some(&faction)
                    || e.kind == EventKind::Crusade
                    || e.text_fr.contains("Crois")
                    || e.text_fr.contains("crois")
            }) {
                println!("  [s{seed} t{turn}] {:?}: {}", e.kind, e.text_fr);
            }
        }
        report.contingents += events
            .iter()
            .filter(|e| e.kind == EventKind::Crusade && e.text_fr.contains("débarque à"))
            .count() as u32;
        let alive = state.factions[&faction].alive;
        if !alive && report.dead_turn.is_none() {
            report.dead_turn = Some(turn);
        }
        if alive {
            let crusade = state.crusade.as_ref().expect("crusade kept");
            report.fervor_min = report.fervor_min.min(crusade.fervor);
            report.fervor_max = report.fervor_max.max(crusade.fervor);
            if crusade.fervor >= 95 {
                report.saturated_turns += 1;
            }
            if crusade.target_taken && report.target_turn.is_none() {
                report.target_turn = Some(turn);
            }
            if state.factions[&faction].treasury < 0 {
                report.negative_turns += 1;
                streak += 1;
                report.negative_streak = report.negative_streak.max(streak);
            } else {
                streak = 0;
            }
            // Armies standing in the Holy Land that were not there before.
            let now: BTreeSet<sim_campaign::ArmyId> = state
                .armies
                .iter()
                .filter(|(_, a)| a.faction == faction)
                .filter(|(_, a)| {
                    state
                        .army_province(data, a)
                        .is_some_and(|p| rules.holy_land.contains(&p))
                })
                .map(|(id, _)| id.clone())
                .collect();
            let fresh = now.difference(&ashore).count() as u32;
            if fresh > 0 {
                report.landings += fresh;
                report.first_landing.get_or_insert(turn);
            }
            ashore = now;
            for (id, s) in &state.settlements {
                if !holy(&state, &rules.holy_land, id) {
                    continue;
                }
                if s.siege.as_ref().is_some_and(|g| g.attacker == faction) {
                    report.sieges.insert(id.clone());
                }
                if s.controller == faction {
                    report.taken.insert(id.clone());
                }
            }
        }
        if turn % 10 == 0 {
            let f = &state.factions[&faction];
            report.fervor.push(if alive {
                state.crusade.as_ref().map_or(0, |c| c.fervor)
            } else {
                255
            });
            report.treasury.push(f.treasury);
            report.units.push(
                state
                    .armies
                    .values()
                    .filter(|a| a.faction == faction)
                    .map(|a| a.units.len())
                    .sum(),
            );
            report.places.push(
                state
                    .settlements
                    .iter()
                    .filter(|(id, s)| s.controller == faction && holy(&state, &rules.holy_land, id))
                    .count(),
            );
            report.holder_provinces.push(holder_count(&state));
        }
    }
    report
}

pub fn print_report(r: &SeedReport) {
    println!(
        "seed {:>2} | fervour {:?} (min {}, max {}, {} turns >= 95) | treasury {:?} (negative {} \
         turns, streak {}) | units {:?} | places {:?} | landings {} (first {:?}), contingents {} \
         | besieged {} | held {} | target {:?} | dead {:?} | holder provinces {:?}",
        r.seed,
        r.fervor,
        r.fervor_min,
        r.fervor_max,
        r.saturated_turns,
        r.treasury,
        r.negative_turns,
        r.negative_streak,
        r.units,
        r.places,
        r.landings,
        r.first_landing,
        r.contingents,
        r.sieges.len(),
        r.taken.len(),
        r.target_turn,
        r.dead_turn,
        r.holder_provinces,
    );
}

#[test]
#[ignore = "balance probe: 5 seeds x 50 turns of the whole AI (minutes)"]
fn crusade_probe() {
    let data = data();
    let seeds: Vec<u64> = std::env::var("JR_SEEDS")
        .ok()
        .map(|s| s.split(',').filter_map(|x| x.trim().parse().ok()).collect())
        .unwrap_or_else(|| vec![1, 2, 3, 4, 5]);
    let turns: u32 = std::env::var("JR_TURNS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(50);
    let trace = std::env::var("JR_TRACE").is_ok();
    let reports: Vec<SeedReport> = std::thread::scope(|scope| {
        let handles: Vec<_> = seeds
            .iter()
            .map(|&seed| {
                let data = &data;
                scope.spawn(move || run_seed(data, seed, turns, trace))
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("seed thread"))
            .collect()
    });
    for report in &reports {
        print_report(report);
    }
}
