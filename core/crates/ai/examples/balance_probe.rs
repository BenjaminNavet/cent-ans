//! Balance probe (audit A2, lot O1): AI-vs-AI campaigns and auto-resolve
//! checks, with JSON and Markdown output.
//!
//! Usage (from `core/`, release strongly advised):
//!
//! - `cargo run --release -p ai --example balance_probe -- campaign [turns] [seed...]`
//!   plays `turns` (default 200) turns for each seed (default 1-8) with the
//!   strategic AI driving every faction (France answers offers and picks
//!   option 0 of its decisions, as in `century_probe`); battles are
//!   auto-resolved.
//! - `... -- matrix`: auto-resolve matrix at equal budget, equal upkeep and
//!   equal count, plus the scenarios of the calibration fixture.
//! - `... -- rt [runs]`: every scenario of the calibration fixture
//!   (`crates/ai/tests/fixtures/auto_resolve_scenarios.json`) in the 3D
//!   battle simulation (`runs` seeds, default 6) against the auto-resolve;
//!   prints the winner agreement. `RT_WRITE=1` rewrites the fixture's
//!   `reference_3d` blocks with the 3D results.
//!
//! Output: `OUT_DIR` (default `target/balance_probe/`) receives
//! `<mode>.json` (one object per seed or scenario) and `<mode>.md`; the
//! Markdown is also printed on stdout.
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use data_model::{FactionId, GameData, Terrain, UnitCategory, UnitTypeId};
use serde_json::{json, Value};
use sim_battle::{BattleSeason, BattleSetup, BattleSim, SideId, SideSetup, UnitSetup};
use sim_campaign::{
    resolve_auto, BattleContext, BattleUnit, CampaignRng, CampaignState, EventKind, Order, Side,
    TaxRate, Winner,
};

/// First turn of 1400 (survival of the majors).
const TURN_1400: u32 = 63 * 4;
const MAJORS: &[&str] = &["fac_england", "fac_france", "fac_burgundy", "fac_scotland"];
/// Regiments of a unit type in one matrix cell.
type CountFn = dyn Fn(&data_model::UnitType) -> usize;
/// Draws per auto-resolve matchup.
const AUTO_DRAWS: u64 = 200;
/// Step cap of a 3D battle (10 minutes at 60 Hz).
const MAX_STEPS: u32 = 36_100;

fn fid(s: &str) -> FactionId {
    FactionId::new(s).expect("well-formed id")
}

fn data_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data")
}

fn out_dir() -> PathBuf {
    std::env::var("OUT_DIR").map_or_else(
        |_| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/balance_probe"),
        PathBuf::from,
    )
}

fn fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/auto_resolve_scenarios.json")
}

fn write_outputs(mode: &str, json_value: &Value, markdown: &str) {
    let dir = out_dir();
    std::fs::create_dir_all(&dir).expect("output dir");
    std::fs::write(
        dir.join(format!("{mode}.json")),
        serde_json::to_string_pretty(json_value).expect("json"),
    )
    .expect("write json");
    std::fs::write(dir.join(format!("{mode}.md")), markdown).expect("write md");
    println!("{markdown}");
    eprintln!("écrit : {}/{mode}.{{json,md}}", dir.display());
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mode = if args.is_empty() {
        "campaign".to_owned()
    } else {
        args.remove(0)
    };
    let (data, _) = GameData::load(&data_root()).expect("data");
    match mode.as_str() {
        "campaign" => campaign_mode(&data, &args),
        "matrix" => matrix_mode(&data),
        "rt" => rt_mode(&data, &args),
        other => {
            eprintln!("mode inconnu {other:?} : campaign | matrix | rt");
            std::process::exit(2);
        }
    }
}

// ---------------------------------------------------------------------------
// Campaign mode
// ---------------------------------------------------------------------------

#[derive(Default)]
struct CampaignRun {
    seed: u64,
    turns: u32,
    seconds: f64,
    /// Recruit orders per faction and unit type.
    recruits: BTreeMap<String, BTreeMap<String, u32>>,
    event_kinds: BTreeMap<String, u32>,
    war_fr_en_turns: u32,
    owner_changes: u32,
    majors_1400: Vec<bool>,
    factions_alive: usize,
    buildings_built: BTreeMap<String, u32>,
    avg_unrest: f64,
    high_tax_share: f64,
    techs: BTreeMap<String, usize>,
    provinces: BTreeMap<String, usize>,
    field_men: BTreeMap<String, u32>,
    france_orders: (u32, u32),
    /// Attacker wins / battles.
    battles: (u32, u32),
}

fn campaign_mode(data: &GameData, args: &[String]) {
    let turns: u32 = args.first().and_then(|s| s.parse().ok()).unwrap_or(200);
    let mut seeds: Vec<u64> = args.iter().skip(1).filter_map(|s| s.parse().ok()).collect();
    if seeds.is_empty() {
        seeds = (1..=8).collect();
    }
    let runs: Vec<CampaignRun> = std::thread::scope(|scope| {
        let handles: Vec<_> = seeds
            .iter()
            .map(|seed| scope.spawn(move || run_campaign(data, *seed, turns)))
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("campaign thread"))
            .collect()
    });
    let json_value = Value::Array(runs.iter().map(run_json).collect());
    write_outputs(
        "campaign",
        &json_value,
        &campaign_markdown(data, &runs, turns),
    );
}

fn run_campaign(data: &GameData, seed: u64, turns: u32) -> CampaignRun {
    let started = Instant::now();
    let france = fid("fac_france");
    let england = fid("fac_england");
    let mut state = CampaignState::new_1337(data, france.clone(), seed).expect("state");
    state.interactive_battles = false;
    let mut run = CampaignRun {
        seed,
        turns,
        majors_1400: vec![true; MAJORS.len()],
        ..CampaignRun::default()
    };
    let recruits = std::cell::RefCell::new(BTreeMap::<String, BTreeMap<String, u32>>::new());
    let log = |faction: &FactionId, orders: &[Order]| {
        for order in orders {
            if let Order::Recruit { unit_type, .. } = order {
                *recruits
                    .borrow_mut()
                    .entry(faction.as_str().to_owned())
                    .or_default()
                    .entry(unit_type.as_str().to_owned())
                    .or_default() += 1;
            }
        }
    };
    let planner = |s: &CampaignState, d: &GameData, f: &FactionId| {
        let orders = ai::plan_turn(s, d, f);
        log(f, &orders);
        orders
    };
    let mut prev_buildings = building_set(&state);
    let mut prev_owner = owners(&state);
    let mut high_tax_samples = (0u32, 0u32);
    for turn in 0..turns {
        for offer in state.factions[&france].offers.clone() {
            let accept = sim_campaign::diplomacy::evaluate(
                &state,
                data,
                &offer.from,
                &france,
                &offer.proposal,
            )
            .accept;
            let _ = state.submit_order(
                data,
                Order::AnswerOffer {
                    offer: offer.id,
                    accept,
                },
            );
        }
        let orders = ai::plan_turn(&state, data, &france);
        log(&france, &orders);
        for order in orders {
            run.france_orders.0 += 1;
            if state.submit_order(data, order).is_err() {
                run.france_orders.1 += 1;
            }
        }
        let decisions: Vec<u32> = state
            .chronicle
            .pending_decisions
            .iter()
            .filter(|d| d.faction == france)
            .map(|d| d.id)
            .collect();
        for decision in decisions {
            let _ = state.submit_order(
                data,
                Order::ChooseEventOption {
                    decision,
                    option: 0,
                },
            );
        }
        let events = state.end_turn_with(data, planner);
        for event in &events {
            *run.event_kinds
                .entry(format!("{:?}", event.kind))
                .or_default() += 1;
            if event.kind == EventKind::Battle {
                run.battles.1 += 1;
                // "... A attaque B. Vainqueur : W. ..." : the attacker won
                // when its name follows "Vainqueur".
                let text = &event.text_fr;
                if let (Some((head, _)), Some((_, tail))) = (
                    text.split_once(" attaque "),
                    text.split_once("Vainqueur : "),
                ) {
                    let attacker = head.rsplit(" : ").next().unwrap_or("");
                    let attacker = attacker.split(" (+").next().unwrap_or(attacker);
                    if !attacker.is_empty() && tail.starts_with(attacker) {
                        run.battles.0 += 1;
                    }
                }
            }
        }
        if state.is_at_war(&france, &england) {
            run.war_fr_en_turns += 1;
        }
        let current = building_set(&state);
        for (_, building) in current.difference(&prev_buildings) {
            *run.buildings_built.entry(building.clone()).or_default() += 1;
        }
        prev_buildings = current;
        let now = owners(&state);
        run.owner_changes += now
            .iter()
            .filter(|(p, o)| prev_owner.get(*p) != Some(*o))
            .count() as u32;
        prev_owner = now;
        if turn + 1 == TURN_1400 {
            for (index, major) in MAJORS.iter().enumerate() {
                run.majors_1400[index] = state.factions[&fid(major)].alive;
            }
        }
        if (turn + 1) % 20 == 0 {
            for f in state.factions.values().filter(|f| f.alive) {
                high_tax_samples.1 += 1;
                if f.tax_rate == TaxRate::High {
                    high_tax_samples.0 += 1;
                }
            }
        }
    }
    run.recruits = recruits.into_inner();
    run.factions_alive = state.factions.values().filter(|f| f.alive).count();
    run.avg_unrest = state
        .provinces
        .values()
        .map(|p| f64::from(p.unrest))
        .sum::<f64>()
        / state.provinces.len().max(1) as f64;
    run.high_tax_share = f64::from(high_tax_samples.0) / f64::from(high_tax_samples.1.max(1));
    for major in MAJORS {
        let id = fid(major);
        let Some(faction) = state.factions.get(&id) else {
            continue;
        };
        run.techs
            .insert((*major).to_owned(), faction.technologies.len());
        run.provinces
            .insert((*major).to_owned(), state.controlled_provinces(&id).len());
        run.field_men.insert(
            (*major).to_owned(),
            state
                .armies
                .values()
                .filter(|a| a.faction == id)
                .map(|a| a.total_strength())
                .sum(),
        );
    }
    run.seconds = started.elapsed().as_secs_f64();
    run
}

fn building_set(state: &CampaignState) -> BTreeSet<(String, String)> {
    state
        .settlements
        .iter()
        .flat_map(|(sid, s)| {
            s.buildings
                .iter()
                .map(move |b| (sid.as_str().to_owned(), b.as_str().to_owned()))
        })
        .collect()
}

fn owners(state: &CampaignState) -> BTreeMap<String, String> {
    state
        .provinces
        .keys()
        .filter_map(|p| {
            state
                .province_owner(p)
                .map(|o| (p.as_str().to_owned(), o.as_str().to_owned()))
        })
        .collect()
}

fn run_json(run: &CampaignRun) -> Value {
    json!({
        "seed": run.seed,
        "turns": run.turns,
        "seconds": run.seconds,
        "recruits": run.recruits,
        "event_kinds": run.event_kinds,
        "war_fr_en_share": f64::from(run.war_fr_en_turns) / f64::from(run.turns.max(1)),
        "owner_changes": run.owner_changes,
        "majors_1400": run.majors_1400,
        "factions_alive": run.factions_alive,
        "buildings_built": run.buildings_built,
        "avg_unrest": run.avg_unrest,
        "high_tax_share": run.high_tax_share,
        "techs": run.techs,
        "provinces": run.provinces,
        "field_men": run.field_men,
        "france_orders": [run.france_orders.0, run.france_orders.1],
        "battles": {"attacker_wins": run.battles.0, "total": run.battles.1},
    })
}

/// Recruit orders of every faction, by unit type.
fn total_recruits(run: &CampaignRun) -> BTreeMap<String, u32> {
    let mut total = BTreeMap::new();
    for per_type in run.recruits.values() {
        for (unit, n) in per_type {
            *total.entry(unit.clone()).or_default() += n;
        }
    }
    total
}

fn share(part: u32, whole: u32) -> f64 {
    100.0 * f64::from(part) / f64::from(whole.max(1))
}

fn campaign_markdown(data: &GameData, runs: &[CampaignRun], turns: u32) -> String {
    let n = runs.len().max(1) as f64;
    let mut md = String::new();
    let _ = writeln!(
        md,
        "# Sonde d'équilibrage — campagne ({} graines × {turns} tours)\n",
        runs.len()
    );
    let _ = writeln!(md, "## Cibles\n");
    let _ = writeln!(md, "| Mesure | Valeur | Cible |\n|---|---|---|");
    let mut all = BTreeMap::<String, u32>::new();
    let mut types_per_run = Vec::new();
    for run in runs {
        let totals = total_recruits(run);
        types_per_run.push(totals.values().filter(|n| **n > 0).count());
        for (unit, count) in totals {
            *all.entry(unit).or_default() += count;
        }
    }
    let all_total: u32 = all.values().sum();
    let militia = all.get("unit_urban_militia").copied().unwrap_or(0);
    let mut england = BTreeMap::<String, u32>::new();
    for run in runs {
        for (unit, count) in run.recruits.get("fac_england").into_iter().flatten() {
            *england.entry(unit.clone()).or_default() += count;
        }
    }
    let england_total: u32 = england.values().sum();
    let longbow = england.get("unit_longbowmen").copied().unwrap_or(0);
    let min_types = types_per_run.iter().min().copied().unwrap_or(0);
    let war: f64 = runs
        .iter()
        .map(|r| share(r.war_fr_en_turns, r.turns))
        .sum::<f64>()
        / n;
    let _ = writeln!(
        md,
        "| Milice / recrutements | {:.1} % | < 40 % |",
        share(militia, all_total)
    );
    let _ = writeln!(
        md,
        "| Archers longs / recrutements anglais | {:.1} % ({longbow} / {england_total}) | ≥ 25 % |",
        share(longbow, england_total)
    );
    let _ = writeln!(
        md,
        "| Types d'unités recrutés par partie (min) | {min_types} | ≥ 6 |"
    );
    let _ = writeln!(
        md,
        "| Guerre France-Angleterre (part des tours) | {war:.0} % | 55-75 % (siècle) |"
    );
    if turns >= TURN_1400 {
        let alive: f64 = runs
            .iter()
            .map(|r| r.majors_1400.iter().filter(|a| **a).count() as f64)
            .sum::<f64>()
            / n;
        let _ = writeln!(md, "| Majeures vivantes en 1400 | {alive:.2} / 4 | 4 / 4 |");
    }
    let unrest: f64 = runs.iter().map(|r| r.avg_unrest).sum::<f64>() / n;
    let high: f64 = runs.iter().map(|r| r.high_tax_share).sum::<f64>() / n * 100.0;
    let _ = writeln!(
        md,
        "| Mécontentement moyen final | {unrest:.1} | 15-35 (E2) |"
    );
    let _ = writeln!(
        md,
        "| Impôt « Haut » (échantillons) | {high:.0} % | < 40 % (E2) |"
    );
    let revolts: f64 = runs
        .iter()
        .map(|r| f64::from(r.event_kinds.get("Revolt").copied().unwrap_or(0)))
        .sum::<f64>()
        / n;
    let _ = writeln!(md, "| Révoltes par partie | {revolts:.1} | — |");
    let changes: f64 = runs.iter().map(|r| f64::from(r.owner_changes)).sum::<f64>() / n;
    let _ = writeln!(md, "| Changements de propriétaire | {changes:.1} | — |");
    let (att_wins, battles) = runs
        .iter()
        .fold((0, 0), |(a, b), r| (a + r.battles.0, b + r.battles.1));
    let _ = writeln!(
        md,
        "| Batailles par partie (l'attaquant gagne) | {:.0} ({:.0} %) | — |",
        f64::from(battles) / n,
        share(att_wins, battles)
    );

    let _ = writeln!(md, "\n## Recrutements (ordres par partie)\n");
    let _ = writeln!(
        md,
        "| Unité | Toutes factions | Part | Angleterre | France |\n|---|---|---|---|---|"
    );
    let mut france = BTreeMap::<String, u32>::new();
    for run in runs {
        for (unit, count) in run.recruits.get("fac_france").into_iter().flatten() {
            *france.entry(unit.clone()).or_default() += count;
        }
    }
    let mut units: Vec<_> = all.iter().collect();
    units.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
    for (unit, count) in units {
        let _ = writeln!(
            md,
            "| {} | {:.1} | {:.1} % | {:.1} | {:.1} |",
            unit.trim_start_matches("unit_"),
            f64::from(*count) / n,
            share(*count, all_total),
            f64::from(england.get(unit).copied().unwrap_or(0)) / n,
            f64::from(france.get(unit).copied().unwrap_or(0)) / n,
        );
    }

    let _ = writeln!(md, "\n## Par graine\n");
    let _ = writeln!(
        md,
        "| Graine | Guerre FR-EN | Types recrutés | Milice | Chang. propr. | Fac. vivantes | Techs FR/EN | Prov. FR/EN | Hommes en campagne FR/EN | Refus FR | s |\n|---|---|---|---|---|---|---|---|---|---|---|"
    );
    for (run, types) in runs.iter().zip(&types_per_run) {
        let totals = total_recruits(run);
        let sum: u32 = totals.values().sum();
        let get = |m: &BTreeMap<String, usize>, k: &str| m.get(k).copied().unwrap_or(0);
        let men = |k: &str| run.field_men.get(k).copied().unwrap_or(0);
        let _ = writeln!(
            md,
            "| {} | {:.0} % | {} | {:.0} % | {} | {} | {}/{} | {}/{} | {}/{} | {:.1} % | {:.0} |",
            run.seed,
            share(run.war_fr_en_turns, run.turns),
            types,
            share(totals.get("unit_urban_militia").copied().unwrap_or(0), sum),
            run.owner_changes,
            run.factions_alive,
            get(&run.techs, "fac_france"),
            get(&run.techs, "fac_england"),
            get(&run.provinces, "fac_france"),
            get(&run.provinces, "fac_england"),
            men("fac_france"),
            men("fac_england"),
            share(run.france_orders.1, run.france_orders.0),
            run.seconds,
        );
    }

    let never: Vec<&str> = data
        .buildings
        .keys()
        .map(|b| b.as_str())
        .filter(|b| runs.iter().all(|r| !r.buildings_built.contains_key(*b)))
        .collect();
    let _ = writeln!(
        md,
        "\n## Bâtiments jamais construits ({} sur {})\n\n{}",
        never.len(),
        data.buildings.len(),
        if never.is_empty() {
            "aucun".to_owned()
        } else {
            never.join(", ")
        }
    );
    let _ = writeln!(md, "\n## Événements (moyenne par partie)\n");
    let mut kinds = BTreeMap::<String, u32>::new();
    for run in runs {
        for (kind, count) in &run.event_kinds {
            *kinds.entry(kind.clone()).or_default() += count;
        }
    }
    let _ = writeln!(md, "| Type | Par partie |\n|---|---|");
    for (kind, count) in kinds {
        let _ = writeln!(md, "| {kind} | {:.1} |", f64::from(count) / n);
    }
    md
}

// ---------------------------------------------------------------------------
// Auto-resolve helpers (matrix and rt modes)
// ---------------------------------------------------------------------------

/// A calibration matchup, shared with `tests/auto_resolve_calibration.rs`.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Scenario {
    name: String,
    attacker: Vec<(String, usize)>,
    defender: Vec<(String, usize)>,
    terrain: Terrain,
    #[serde(default)]
    season: BattleSeason,
    #[serde(default)]
    river: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reference_3d: Option<Reference3d>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Reference3d {
    runs: u32,
    attacker_wins: u32,
    attacker_losses: u32,
    defender_losses: u32,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Fixture {
    #[serde(default)]
    comment: String,
    scenarios: Vec<Scenario>,
}

fn unit_type<'a>(data: &'a GameData, id: &str) -> &'a data_model::UnitType {
    &data.unit_types[&UnitTypeId::new(id).expect("unit id")]
}

fn battle_unit(data: &GameData, id: &str) -> BattleUnit {
    let t = unit_type(data, id);
    BattleUnit {
        strength: t.soldiers,
        max_strength: t.soldiers,
        experience: 0,
        morale: t.stats.morale,
        melee: t.stats.melee,
        ranged: t.stats.ranged,
        armor: t.stats.armor,
        is_ranged: matches!(t.category, UnitCategory::Ranged | UnitCategory::Siege),
    }
}

fn auto_side(data: &GameData, comp: &[(String, usize)]) -> Side {
    Side {
        units: comp
            .iter()
            .flat_map(|(id, n)| std::iter::repeat_n(battle_unit(data, id), *n))
            .collect(),
        ..Default::default()
    }
}

fn scenario_context(scenario: &Scenario) -> BattleContext {
    BattleContext {
        defender_terrain_bonus: matches!(
            scenario.terrain,
            Terrain::Hills | Terrain::Forest | Terrain::Mountains
        ),
        river_crossing: scenario.river,
        walls: false,
    }
}

/// Attacker win share (%) and mean losses over [`AUTO_DRAWS`] draws.
fn auto_duel(attacker: &Side, defender: &Side, context: &BattleContext) -> (f64, f64, f64) {
    let (mut wins, mut la, mut ld) = (0u32, 0.0, 0.0);
    for seed in 0..AUTO_DRAWS {
        let result = resolve_auto(
            attacker,
            defender,
            context,
            &mut CampaignRng::from_seed(seed),
        );
        if result.winner == Winner::Attacker {
            wins += 1;
        }
        la += f64::from(result.attacker.total_losses);
        ld += f64::from(result.defender.total_losses);
    }
    let draws = AUTO_DRAWS as f64;
    (100.0 * f64::from(wins) / draws, la / draws, ld / draws)
}

fn load_fixture() -> Fixture {
    let text = std::fs::read_to_string(fixture_path()).expect("calibration fixture");
    serde_json::from_str(&text).expect("fixture json")
}

fn matrix_mode(data: &GameData) {
    let types: Vec<&data_model::UnitType> = data
        .unit_types
        .values()
        .filter(|t| t.category != UnitCategory::Siege)
        .collect();
    let mut md = String::from("# Sonde d'équilibrage — matrice d'auto-résolution\n");
    let mut json_rows = Vec::new();
    let header = |md: &mut String| {
        let _ = write!(md, "\n| Attaquant \\ défenseur |");
        for t in &types {
            let _ = write!(md, " {} |", t.id.as_str().trim_start_matches("unit_"));
        }
        let _ = write!(md, "\n|---|");
        for _ in &types {
            let _ = write!(md, "---|");
        }
        md.push('\n');
    };
    let tables: [(&str, &CountFn); 3] = [
        (
            "Budget égal (9 000 livres) : % de victoires de l'attaquant",
            &|t| (9000 / t.cost.money.max(1)).max(1) as usize,
        ),
        (
            "Entretien égal (600 livres par saison) : % de victoires de l'attaquant",
            &|t| (600 / t.upkeep.max(1)).max(1) as usize,
        ),
        (
            "Effectif égal (8 contre 8) : % de victoires de l'attaquant",
            &|_| 8,
        ),
    ];
    let mut extremes = Vec::new();
    for (title, count) in tables {
        let _ = writeln!(md, "\n## {title}");
        header(&mut md);
        for a in &types {
            let _ = write!(md, "| {} |", a.id.as_str().trim_start_matches("unit_"));
            let mut row_wins = Vec::new();
            for d in &types {
                let sa = auto_side(data, &[(a.id.as_str().to_owned(), count(a))]);
                let sd = auto_side(data, &[(d.id.as_str().to_owned(), count(d))]);
                let (wins, _, _) = auto_duel(&sa, &sd, &BattleContext::default());
                let _ = write!(md, " {wins:.0} |");
                row_wins.push(wins);
                json_rows.push(json!({"table": title, "attacker": a.id.as_str(), "defender": d.id.as_str(), "attacker_win_pct": wins}));
            }
            md.push('\n');
            if title.starts_with("Budget") {
                let mean = row_wins.iter().sum::<f64>() / row_wins.len() as f64;
                extremes.push((a.id.as_str().to_owned(), mean));
            }
        }
    }
    let _ = writeln!(
        md,
        "\n## Victoires moyennes à budget égal (cible E8 : entre 20 et 80 %)\n\n| Unité | Moyenne |\n|---|---|"
    );
    for (unit, mean) in &extremes {
        let _ = writeln!(md, "| {} | {mean:.0} % |", unit.trim_start_matches("unit_"));
    }
    let fixture = load_fixture();
    let _ = writeln!(
        md,
        "\n## Scénarios de calibration (auto-résolution)\n\n| Scénario | Attaquant gagne | Pertes A / D |\n|---|---|---|"
    );
    for s in &fixture.scenarios {
        let (w, la, ld) = auto_duel(
            &auto_side(data, &s.attacker),
            &auto_side(data, &s.defender),
            &scenario_context(s),
        );
        let _ = writeln!(md, "| {} | {w:.0} % | {la:.0} / {ld:.0} |", s.name);
    }
    write_outputs("matrix", &Value::Array(json_rows), &md);
}

fn battle_side(data: &GameData, name: &str, comp: &[(String, usize)]) -> SideSetup {
    SideSetup {
        faction: name.into(),
        faction_name: name.into(),
        army: String::new(),
        units: comp
            .iter()
            .flat_map(|(id, n)| {
                let t = unit_type(data, id);
                std::iter::repeat_n(
                    UnitSetup::from_unit_type(t, t.soldiers, t.stats.morale, 0),
                    *n,
                )
            })
            .collect(),
        general: None,
    }
}

/// Runs a scenario `runs` times in the 3D simulation.
fn run_3d(data: &GameData, scenario: &Scenario, runs: u32) -> Reference3d {
    let mut reference = Reference3d {
        runs,
        attacker_wins: 0,
        attacker_losses: 0,
        defender_losses: 0,
    };
    for seed in 0..u64::from(runs) {
        let setup = BattleSetup {
            province: String::new(),
            province_name: String::new(),
            terrain: scenario.terrain,
            river: scenario.river,
            season: scenario.season,
            coastal: false,
            village: Some(false),
            attacker: battle_side(data, "A", &scenario.attacker),
            defender: battle_side(data, "D", &scenario.defender),
            player_side: None,
            siege: None,
            orders: Vec::new(),
        };
        let mut sim = BattleSim::new(setup, seed).expect("battle setup");
        let mut steps = 0;
        while !sim.is_finished() && steps < MAX_STEPS {
            sim.step();
            steps += 1;
        }
        let Some(outcome) = sim.outcome() else {
            continue;
        };
        if outcome.winner == SideId::Attacker {
            reference.attacker_wins += 1;
        }
        reference.attacker_losses += outcome.attacker.total_losses;
        reference.defender_losses += outcome.defender.total_losses;
    }
    reference.attacker_losses /= runs.max(1);
    reference.defender_losses /= runs.max(1);
    reference
}

fn rt_mode(data: &GameData, args: &[String]) {
    let runs: u32 = args.first().and_then(|s| s.parse().ok()).unwrap_or(6);
    let mut fixture = load_fixture();
    let references: Vec<Reference3d> = std::thread::scope(|scope| {
        let handles: Vec<_> = fixture
            .scenarios
            .iter()
            .map(|s| scope.spawn(move || run_3d(data, s, runs)))
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("3D thread"))
            .collect()
    });
    let mut md = format!(
        "# Sonde d'équilibrage — auto-résolution contre bataille 3D ({runs} graines)\n\n| Scénario | 3D : A gagne | 3D pertes A / D | Auto : A gagne | Auto pertes A / D | Accord |\n|---|---|---|---|---|---|\n"
    );
    let mut rows = Vec::new();
    let mut agree = 0;
    for (scenario, reference) in fixture.scenarios.iter().zip(&references) {
        let (w, la, ld) = auto_duel(
            &auto_side(data, &scenario.attacker),
            &auto_side(data, &scenario.defender),
            &scenario_context(scenario),
        );
        let rt_attacker = 2 * reference.attacker_wins > reference.runs;
        let auto_attacker = w > 50.0;
        let ok = rt_attacker == auto_attacker;
        agree += usize::from(ok);
        let _ = writeln!(
            md,
            "| {} | {}/{} | {} / {} | {w:.0} % | {la:.0} / {ld:.0} | {} |",
            scenario.name,
            reference.attacker_wins,
            reference.runs,
            reference.attacker_losses,
            reference.defender_losses,
            if ok { "oui" } else { "**non**" }
        );
        rows.push(json!({
            "name": scenario.name,
            "rt_attacker_wins": reference.attacker_wins,
            "rt_runs": reference.runs,
            "rt_losses": [reference.attacker_losses, reference.defender_losses],
            "auto_attacker_win_pct": w,
            "auto_losses": [la, ld],
            "agree": ok,
        }));
    }
    let _ = writeln!(
        md,
        "\nAccord sur le vainqueur : **{agree} / {}** ({:.0} %, cible ≥ 80 %).",
        fixture.scenarios.len(),
        100.0 * agree as f64 / fixture.scenarios.len().max(1) as f64
    );
    if std::env::var("RT_WRITE").is_ok() {
        for (scenario, reference) in fixture.scenarios.iter_mut().zip(references) {
            scenario.reference_3d = Some(reference);
        }
        std::fs::write(
            fixture_path(),
            serde_json::to_string_pretty(&fixture).expect("json") + "\n",
        )
        .expect("write fixture");
        eprintln!("fixture réécrite : {}", fixture_path().display());
    }
    write_outputs("rt", &Value::Array(rows), &md);
}
