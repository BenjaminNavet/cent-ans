//! Balance bench of the 3D battle simulation (lot BANC-1, ADR 0798).
//!
//! Replays the matrix of `data/rules/battle_bench.json` (schema
//! `data/schemas/battle_bench.schema.json`): 1-against-1 duels of the main
//! unit types, typical campaign armies, the historical maps, sieges; each line
//! under one or several driving modes and over a number of seeds. It only
//! measures (no rule of the game lives here) and writes `out/bench/<date>.json`
//! plus a text summary.
//!
//! ```text
//! tools/heavy.sh cargo run --release -p sim-battle --example battle_bench
//! tools/heavy.sh cargo run --release -p sim-battle --example battle_bench -- --smoke
//! ... -- --filter crecy --threads 2 --out /tmp/crecy.json
//! ```
//!
//! The JSON is stable: same matrix, same data, same code give the same bytes
//! (no clock inside; the date only names the default output file).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use data_model::test_support::game_data;
use data_model::{GameData, Terrain, UnitCategory, UnitTypeId};
use serde::{Deserialize, Serialize};
use sim_battle::{
    BattleSeason, BattleSetup, BattleSim, Command, HistoricalMap, SideId, SideSetup, SiegeSetup,
    UnitFate, UnitSetup, UnitState,
};

// ---------------------------------------------------------------- matrix

type Roster = Vec<(String, usize)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Mode {
    /// Both sides under the AI.
    AiVsAi,
    /// The attacker does nothing (an idle player).
    PassiveAttacker,
    /// The defender does nothing.
    PassiveDefender,
    /// One "everyone attack the nearest" order for the attacker, then nothing.
    Blob,
    /// Attacker AI, but its cavalry swings wide and hits a shooter on the flank.
    Flank,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Duels {
    seeds: u64,
    terrain: Terrain,
    per_side: usize,
    modes: Vec<Mode>,
    types: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ArmyLine {
    id: String,
    label: String,
    terrain: Terrain,
    attacker: Roster,
    defender: Roster,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Armies {
    seeds: u64,
    modes: Vec<Mode>,
    lines: Vec<ArmyLine>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Historical {
    seeds: u64,
    modes: Vec<Mode>,
    maps: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SiegeLine {
    id: String,
    label: String,
    fortification: u32,
    attacker: Roster,
    defender: Roster,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sieges {
    seeds: u64,
    #[serde(default)]
    cap_s: Option<f64>,
    modes: Vec<Mode>,
    lines: Vec<SiegeLine>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Matrix {
    #[serde(default)]
    description: String,
    cap_s: f64,
    duels: Duels,
    armies: Armies,
    historical: Historical,
    sieges: Sieges,
}

/// What a line plays.
#[derive(Debug, Clone)]
enum Scenario {
    Field {
        terrain: Terrain,
        attacker: Roster,
        defender: Roster,
    },
    Siege {
        fortification: u32,
        attacker: Roster,
        defender: Roster,
    },
    Historical {
        map: String,
    },
}

/// One row of the bench: a scenario under one mode over `seeds` seeds.
#[derive(Debug, Clone)]
struct Line {
    id: String,
    kind: &'static str,
    label: String,
    mode: Mode,
    seeds: u64,
    cap_s: f64,
    scenario: Scenario,
}

fn roster_of(list: &[(&str, usize)]) -> Roster {
    list.iter().map(|(id, n)| ((*id).to_owned(), *n)).collect()
}

/// Expands the matrix into its lines, in a fixed order.
fn lines_of(matrix: &Matrix) -> Vec<Line> {
    let mut lines = Vec::new();
    let d = &matrix.duels;
    for (i, a) in d.types.iter().enumerate() {
        for b in &d.types[i + 1..] {
            for &mode in &d.modes {
                lines.push(Line {
                    id: format!("duel:{a}:{b}"),
                    kind: "duel",
                    label: format!("{} {a} contre {} {b}", d.per_side, d.per_side),
                    mode,
                    seeds: d.seeds,
                    cap_s: matrix.cap_s,
                    scenario: Scenario::Field {
                        terrain: d.terrain,
                        attacker: roster_of(&[(a, d.per_side)]),
                        defender: roster_of(&[(b, d.per_side)]),
                    },
                });
            }
        }
    }
    for l in &matrix.armies.lines {
        for &mode in &matrix.armies.modes {
            lines.push(Line {
                id: format!("army:{}", l.id),
                kind: "army",
                label: l.label.clone(),
                mode,
                seeds: matrix.armies.seeds,
                cap_s: matrix.cap_s,
                scenario: Scenario::Field {
                    terrain: l.terrain,
                    attacker: l.attacker.clone(),
                    defender: l.defender.clone(),
                },
            });
        }
    }
    for map in &matrix.historical.maps {
        for &mode in &matrix.historical.modes {
            lines.push(Line {
                id: format!("historical:{map}"),
                kind: "historical",
                label: format!("Carte historique {map}"),
                mode,
                seeds: matrix.historical.seeds,
                cap_s: matrix.cap_s,
                scenario: Scenario::Historical { map: map.clone() },
            });
        }
    }
    for l in &matrix.sieges.lines {
        for &mode in &matrix.sieges.modes {
            lines.push(Line {
                id: format!("siege:{}", l.id),
                kind: "siege",
                label: l.label.clone(),
                mode,
                seeds: matrix.sieges.seeds,
                cap_s: matrix.sieges.cap_s.unwrap_or(matrix.cap_s),
                scenario: Scenario::Siege {
                    fortification: l.fortification,
                    attacker: l.attacker.clone(),
                    defender: l.defender.clone(),
                },
            });
        }
    }
    lines
}

// ---------------------------------------------------------------- one run

/// Measures of one battle.
#[derive(Debug, Clone)]
struct Run {
    winner: Option<SideId>,
    end_s: f64,
    contact_s: Option<f64>,
    first_rout_s: Option<f64>,
    /// Losses (fraction of the regiment's start) of the first regiment to rout.
    first_rout_loss: Option<f64>,
    loss_attacker: f64,
    loss_defender: f64,
    /// Share of the ammunition shot (all regiments with a quiver).
    ammo_spent: Option<f64>,
    routed_attacker: usize,
    routed_defender: usize,
    destroyed_attacker: usize,
    destroyed_defender: usize,
}

fn side_setup(name: &str, units: Vec<UnitSetup>) -> SideSetup {
    SideSetup {
        faction: format!("fac_{}", name.to_lowercase()),
        faction_name: name.to_owned(),
        army: String::new(),
        units,
        general: None,
        forced_march: false,
        entrenched: false,
        start_fatigue: 0.0,
    }
}

fn expand(data: &GameData, roster: &Roster) -> Vec<UnitSetup> {
    let mut units = Vec::new();
    for (id, count) in roster {
        let key = UnitTypeId::new(format!("unit_{id}")).expect("unit id");
        let t = data
            .unit_types
            .get(&key)
            .unwrap_or_else(|| panic!("unknown unit type {id}"));
        for _ in 0..*count {
            units.push(UnitSetup::from_unit_type(t, t.soldiers, t.stats.morale, 0));
        }
    }
    units
}

fn field_setup(
    data: &GameData,
    terrain: Terrain,
    attacker: &Roster,
    defender: &Roster,
    siege: Option<SiegeSetup>,
) -> BattleSetup {
    BattleSetup {
        crossing: None,
        province: "prov_test".into(),
        province_name: "Test".into(),
        terrain,
        river: false,
        season: BattleSeason::Summer,
        coastal: false,
        bare_field: false,
        attacker: side_setup("France", expand(data, attacker)),
        defender: side_setup("Angleterre", expand(data, defender)),
        player_side: None,
        siege,
        siege_layout: None,
        orders: Vec::new(),
        abilities: Vec::new(),
        standards: None,
        decor_plan: None,
        opening: Default::default(),
    }
}

fn load_map(data_dir: &Path, id: &str) -> HistoricalMap {
    let path = data_dir.join("battle_maps").join(format!("{id}.json"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    HistoricalMap::from_json(&text).unwrap_or_else(|e| panic!("{id}: {e}"))
}

fn build(data: &GameData, data_dir: &Path, line: &Line, seed: u64) -> BattleSim {
    match &line.scenario {
        Scenario::Field {
            terrain,
            attacker,
            defender,
        } => BattleSim::new(field_setup(data, *terrain, attacker, defender, None), seed)
            .expect("battle starts"),
        Scenario::Siege {
            fortification,
            attacker,
            defender,
        } => {
            let siege = SiegeSetup {
                fortification: *fortification,
                breach: 0,
                ..Default::default()
            };
            BattleSim::new(
                field_setup(data, Terrain::Plains, attacker, defender, Some(siege)),
                seed,
            )
            .expect("siege starts")
        }
        Scenario::Historical { map } => {
            let map = load_map(data_dir, map);
            let mut setup = map
                .battle_setup(
                    &data.unit_types,
                    data.battle_orders.values().cloned().collect(),
                    Some(data.battle_standard_rules.clone()),
                    None,
                )
                .expect("historical setup");
            setup.abilities = data.battle_abilities.values().cloned().collect();
            map.start(setup, seed).expect("historical battle starts")
        }
    }
}

/// Every attacker regiment attacks its nearest enemy (the "blob" order).
fn send_blob(sim: &mut BattleSim) {
    let ids: Vec<u32> = sim
        .units()
        .iter()
        .filter(|u| u.side == SideId::Attacker && !u.synthetic)
        .map(|u| u.id)
        .collect();
    for id in ids {
        let me = &sim.units()[id as usize];
        let target = sim
            .units()
            .iter()
            .filter(|e| e.side == SideId::Defender && e.able() && !e.synthetic)
            .min_by(|p, q| {
                let dp = (p.x - me.x).hypot(p.z - me.z);
                let dq = (q.x - me.x).hypot(q.z - me.z);
                dp.total_cmp(&dq)
            })
            .map(|e| e.id);
        if let Some(target) = target {
            let _ = sim.apply_command(
                Command::Attack {
                    units: vec![id],
                    target,
                    run: false,
                    queue: false,
                },
                Some(SideId::Attacker),
            );
        }
    }
}

/// State of the flank manoeuvre of the attacker's cavalry.
#[derive(Default)]
struct Flank {
    phase: u8,
}

impl Flank {
    /// Called every simulated second: swing wide, then hit the nearest shooter.
    fn tick(&mut self, sim: &mut BattleSim) {
        let horse: Vec<u32> = sim
            .units()
            .iter()
            .filter(|u| u.side == SideId::Attacker && u.mounted && u.able() && !u.synthetic)
            .map(|u| u.id)
            .collect();
        let enemies: Vec<(u32, f64, f64, bool)> = sim
            .units()
            .iter()
            .filter(|u| u.side == SideId::Defender && u.able() && !u.synthetic)
            .map(|u| (u.id, u.x, u.z, u.can_shoot()))
            .collect();
        if horse.is_empty() || enemies.is_empty() {
            return;
        }
        let ex = enemies.iter().map(|e| e.1).fold(f64::MIN, f64::max);
        let ez = enemies.iter().map(|e| e.2).sum::<f64>() / enemies.len() as f64;
        let n = horse.len() as f64;
        let hx = horse
            .iter()
            .map(|&h| sim.units()[h as usize].x)
            .sum::<f64>()
            / n;
        let hz = horse
            .iter()
            .map(|&h| sim.units()[h as usize].z)
            .sum::<f64>()
            / n;
        let wide = (ex + 150.0, ez);
        if self.phase == 0 {
            let _ = sim.apply_command(
                Command::Move {
                    units: horse,
                    x: wide.0,
                    z: wide.1,
                    run: false,
                    facing: None,
                    queue: false,
                    width: None,
                    match_speed: false,
                    group_tag: None,
                },
                Some(SideId::Attacker),
            );
            self.phase = 1;
        } else if self.phase == 1 && (hx - wide.0).hypot(hz - wide.1) < 80.0 {
            let target = enemies
                .iter()
                .filter(|e| e.3)
                .min_by(|p, q| {
                    (p.1 - hx)
                        .hypot(p.2 - hz)
                        .total_cmp(&(q.1 - hx).hypot(q.2 - hz))
                })
                .or(enemies.first())
                .map(|e| e.0)
                .expect("an enemy");
            let _ = sim.apply_command(
                Command::Attack {
                    units: horse,
                    target,
                    run: true,
                    queue: false,
                },
                Some(SideId::Attacker),
            );
            self.phase = 2;
        }
    }
}

fn run_one(data: &GameData, data_dir: &Path, line: &Line, seed: u64) -> Run {
    let mut sim = build(data, data_dir, line, seed);
    match line.mode {
        Mode::AiVsAi | Mode::Flank => {}
        Mode::PassiveAttacker | Mode::Blob => sim.set_ai(SideId::Attacker, false),
        Mode::PassiveDefender => sim.set_ai(SideId::Defender, false),
    }
    let start: Vec<u32> = sim.units().iter().map(|u| u.initial_soldiers).collect();
    let (mut contact_s, mut first_rout_s, mut first_rout_loss) = (None, None, None);
    let mut blob_sent = false;
    let mut flank = Flank::default();
    let mut next_flank = 0.0;
    while !sim.is_finished() && sim.elapsed() < line.cap_s {
        if line.mode == Mode::Blob && !blob_sent {
            blob_sent = true;
            send_blob(&mut sim);
        }
        if line.mode == Mode::Flank && sim.elapsed() >= next_flank {
            next_flank = sim.elapsed() + 2.0;
            flank.tick(&mut sim);
        }
        sim.step();
        let units = sim.units();
        if contact_s.is_none() && units.iter().any(|u| u.state == UnitState::Melee) {
            contact_s = Some(sim.elapsed());
        }
        if first_rout_s.is_none() {
            if let Some(u) = units
                .iter()
                .find(|u| u.state == UnitState::Routing && !u.synthetic)
            {
                first_rout_s = Some(sim.elapsed());
                first_rout_loss =
                    Some(1.0 - f64::from(u.soldiers()) / f64::from(start[u.id as usize].max(1)));
            }
        }
    }
    let loss = |side: SideId| {
        let (mut lost, mut total) = (0.0, 0.0);
        for u in sim
            .units()
            .iter()
            .filter(|u| u.side == side && !u.synthetic)
        {
            lost += f64::from(u.initial_soldiers.saturating_sub(u.soldiers()));
            total += f64::from(u.initial_soldiers);
        }
        lost / total.max(1.0)
    };
    let count = |side: SideId, fate: UnitFate| {
        sim.units()
            .iter()
            .filter(|u| u.side == side && !u.synthetic && u.fate() == fate)
            .count()
    };
    let (mut quiver, mut left) = (0.0, 0.0);
    for u in sim
        .units()
        .iter()
        .filter(|u| !u.synthetic && u.stats.ammo > 0)
    {
        quiver += f64::from(u.stats.ammo);
        left += f64::from(u.ammo.min(u.stats.ammo));
    }
    let _ = UnitCategory::Ranged;
    Run {
        winner: if sim.is_finished() {
            sim.winner()
        } else {
            None
        },
        end_s: sim.elapsed(),
        contact_s,
        first_rout_s,
        first_rout_loss,
        loss_attacker: loss(SideId::Attacker),
        loss_defender: loss(SideId::Defender),
        ammo_spent: (quiver > 0.0).then(|| 1.0 - left / quiver),
        routed_attacker: count(SideId::Attacker, UnitFate::Routed),
        routed_defender: count(SideId::Defender, UnitFate::Routed),
        destroyed_attacker: count(SideId::Attacker, UnitFate::Destroyed),
        destroyed_defender: count(SideId::Defender, UnitFate::Destroyed),
    }
}

// ---------------------------------------------------------------- report

/// One row of `out/bench/<date>.json`.
#[derive(Debug, Clone, PartialEq, Serialize)]
struct Row {
    id: String,
    kind: &'static str,
    label: String,
    mode: Mode,
    seeds: u64,
    attacker_wins: usize,
    defender_wins: usize,
    /// Battles that hit the time cap without a winner.
    undecided: usize,
    attacker_win_rate: f64,
    median_loss_attacker: Option<f64>,
    median_loss_defender: Option<f64>,
    median_contact_s: Option<f64>,
    median_first_rout_s: Option<f64>,
    median_first_rout_loss: Option<f64>,
    median_duration_s: Option<f64>,
    median_ammo_spent: Option<f64>,
    median_routed_attacker: Option<f64>,
    median_routed_defender: Option<f64>,
    median_destroyed_attacker: Option<f64>,
    median_destroyed_defender: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
struct Report {
    description: String,
    cap_s: f64,
    rows: Vec<Row>,
}

fn round(x: f64, digits: i32) -> f64 {
    let k = 10f64.powi(digits);
    (x * k).round() / k
}

fn median(mut values: Vec<f64>, digits: i32) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let n = values.len();
    let m = if n % 2 == 1 {
        values[n / 2]
    } else {
        (values[n / 2 - 1] + values[n / 2]) / 2.0
    };
    Some(round(m, digits))
}

fn row_of(line: &Line, runs: &[Run]) -> Row {
    let wins = |side| runs.iter().filter(|r| r.winner == Some(side)).count();
    let attacker_wins = wins(SideId::Attacker);
    let defender_wins = wins(SideId::Defender);
    let pick = |f: &dyn Fn(&Run) -> Option<f64>, digits| {
        median(runs.iter().filter_map(f).collect(), digits)
    };
    Row {
        id: line.id.clone(),
        kind: line.kind,
        label: line.label.clone(),
        mode: line.mode,
        seeds: line.seeds,
        attacker_wins,
        defender_wins,
        undecided: runs.len() - attacker_wins - defender_wins,
        attacker_win_rate: round(attacker_wins as f64 / runs.len().max(1) as f64, 3),
        median_loss_attacker: pick(&|r| Some(r.loss_attacker), 3),
        median_loss_defender: pick(&|r| Some(r.loss_defender), 3),
        median_contact_s: pick(&|r| r.contact_s, 1),
        median_first_rout_s: pick(&|r| r.first_rout_s, 1),
        median_first_rout_loss: pick(&|r| r.first_rout_loss, 3),
        median_duration_s: pick(&|r| Some(r.end_s), 1),
        median_ammo_spent: pick(&|r| r.ammo_spent, 3),
        median_routed_attacker: pick(&|r| Some(r.routed_attacker as f64), 1),
        median_routed_defender: pick(&|r| Some(r.routed_defender as f64), 1),
        median_destroyed_attacker: pick(&|r| Some(r.destroyed_attacker as f64), 1),
        median_destroyed_defender: pick(&|r| Some(r.destroyed_defender as f64), 1),
    }
}

/// Plays every (line, seed) on `threads` threads; the result does not depend on
/// the thread count (each battle is deterministic and rows keep their order).
fn play(data: &GameData, data_dir: &Path, lines: &[Line], threads: usize) -> Vec<Row> {
    let tasks: Vec<(usize, u64)> = lines
        .iter()
        .enumerate()
        .flat_map(|(i, l)| (0..l.seeds).map(move |s| (i, s)))
        .collect();
    let results: Mutex<Vec<Option<Run>>> = Mutex::new(vec![None; tasks.len()]);
    let next = AtomicUsize::new(0);
    std::thread::scope(|scope| {
        for _ in 0..threads.max(1) {
            scope.spawn(|| loop {
                let t = next.fetch_add(1, Ordering::Relaxed);
                let Some(&(line, seed)) = tasks.get(t) else {
                    break;
                };
                let run = run_one(data, data_dir, &lines[line], seed);
                results.lock().expect("results")[t] = Some(run);
            });
        }
    });
    let results = results.into_inner().expect("results");
    let mut rows = Vec::new();
    let mut cursor = 0;
    for line in lines {
        let n = line.seeds as usize;
        let runs: Vec<Run> = results[cursor..cursor + n]
            .iter()
            .map(|r| r.clone().expect("run"))
            .collect();
        cursor += n;
        rows.push(row_of(line, &runs));
    }
    rows
}

fn summary(report: &Report) -> String {
    let f =
        |x: Option<f64>, scale: f64| x.map_or("   -".to_owned(), |v| format!("{:4.0}", v * scale));
    let mut out = String::new();
    for row in &report.rows {
        out += &format!(
            "{:<34} {:<16} A {}/{} D {} nul {} | pertes A{}% D{}% | contact{}s rout1{}s ({}%) fin{}s | munit.{}% | fuite A{} D{} det. A{} D{}\n",
            row.id,
            format!("{:?}", row.mode),
            row.attacker_wins,
            row.seeds,
            row.defender_wins,
            row.undecided,
            f(row.median_loss_attacker, 100.0),
            f(row.median_loss_defender, 100.0),
            f(row.median_contact_s, 1.0),
            f(row.median_first_rout_s, 1.0),
            f(row.median_first_rout_loss, 100.0),
            f(row.median_duration_s, 1.0),
            f(row.median_ammo_spent, 100.0),
            f(row.median_routed_attacker, 1.0),
            f(row.median_routed_defender, 1.0),
            f(row.median_destroyed_attacker, 1.0),
            f(row.median_destroyed_defender, 1.0),
        );
    }
    out
}

/// The smoke selection: the first duel, the first army line, the first siege,
/// one seed each.
fn smoke(lines: Vec<Line>) -> Vec<Line> {
    let mut picked: Vec<Line> = Vec::new();
    for kind in ["duel", "army", "siege"] {
        if let Some(l) = lines.iter().find(|l| l.kind == kind) {
            picked.push(Line {
                seeds: 1,
                ..l.clone()
            });
        }
    }
    picked
}

fn report_of(
    matrix: &Matrix,
    data: &GameData,
    data_dir: &Path,
    lines: &[Line],
    threads: usize,
) -> Report {
    Report {
        description: matrix.description.clone(),
        cap_s: matrix.cap_s,
        rows: play(data, data_dir, lines, threads),
    }
}

/// `YYYY-MM-DD` (UTC) of today, without a date crate.
fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data")
}

fn load_matrix(path: &Path) -> Matrix {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (mut smoke_only, mut out, mut filter, mut matrix_path) = (false, None, None, None);
    let mut threads = std::thread::available_parallelism()
        .map_or(2, |n| n.get())
        .min(6);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--smoke" => smoke_only = true,
            "--out" => out = args.next().map(PathBuf::from),
            "--filter" => filter = args.next(),
            "--threads" => threads = args.next().and_then(|v| v.parse().ok()).unwrap_or(threads),
            "--matrix" => matrix_path = args.next().map(PathBuf::from),
            other => {
                eprintln!("unknown argument {other}");
                eprintln!("usage: battle_bench [--smoke] [--filter SUBSTR] [--threads N] [--matrix FILE] [--out FILE]");
                std::process::exit(2);
            }
        }
    }
    let dir = data_dir();
    let matrix = load_matrix(&matrix_path.unwrap_or_else(|| dir.join("rules/battle_bench.json")));
    let mut lines = lines_of(&matrix);
    if let Some(f) = &filter {
        lines.retain(|l| l.id.contains(f.as_str()));
    }
    if smoke_only {
        lines = smoke(lines);
    }
    let started = std::time::Instant::now();
    let report = report_of(&matrix, game_data(), &dir, &lines, threads);
    print!("{}", summary(&report));
    let target = out.unwrap_or_else(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../out/bench");
        root.join(format!("{}.json", today()))
    });
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).expect("output dir");
    }
    let json = serde_json::to_string_pretty(&report).expect("serialise") + "\n";
    std::fs::write(&target, json).expect("write report");
    eprintln!(
        "{} rows, {:.0} s, written to {}",
        report.rows.len(),
        started.elapsed().as_secs_f64(),
        target.display()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix() -> Matrix {
        load_matrix(&data_dir().join("rules/battle_bench.json"))
    }

    /// 3 lines, 1 seed: runs in a few seconds in release.
    #[test]
    fn bench_smoke() {
        let m = matrix();
        let lines = smoke(lines_of(&m));
        assert_eq!(lines.len(), 3);
        let report = report_of(&m, game_data(), &data_dir(), &lines, 3);
        assert_eq!(report.rows.len(), 3);
        for row in &report.rows {
            assert_eq!(row.seeds, 1);
            assert!(row.median_duration_s.is_some_and(|d| d > 0.0), "{}", row.id);
        }
    }

    /// Two passes give the same JSON, whatever the thread count.
    #[test]
    fn bench_is_deterministic() {
        let m = matrix();
        let lines = smoke(lines_of(&m));
        let a = report_of(&m, game_data(), &data_dir(), &lines, 1);
        let b = report_of(&m, game_data(), &data_dir(), &lines, 3);
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
    }

    /// The matrix names only existing unit types and loadable maps, among them
    /// the six historical ones.
    #[test]
    fn matrix_is_consistent() {
        let m = matrix();
        let data = game_data();
        let known = |id: &String| {
            data.unit_types
                .contains_key(&UnitTypeId::new(format!("unit_{id}")).unwrap())
        };
        assert_eq!(m.duels.types.len(), 14);
        assert!(m.duels.types.iter().all(known));
        for l in &m.armies.lines {
            assert!(
                l.attacker
                    .iter()
                    .chain(&l.defender)
                    .all(|(id, _)| known(id)),
                "{}",
                l.id
            );
        }
        for l in &m.sieges.lines {
            assert!(
                l.attacker
                    .iter()
                    .chain(&l.defender)
                    .all(|(id, _)| known(id)),
                "{}",
                l.id
            );
        }
        for id in [
            "crecy",
            "poitiers",
            "azincourt",
            "formigny",
            "castillon",
            "auray",
        ] {
            assert!(m.historical.maps.iter().any(|m| m == id), "{id} missing");
        }
        for id in &m.historical.maps {
            let _ = load_map(&data_dir(), id);
        }
    }
}
