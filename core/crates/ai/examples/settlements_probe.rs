//! Lot C7a: 50 turns of AI against AI on several seeds, measured on the
//! settlements (`docs/archive/chantiers.md`).
//!
//! Per seed: treasury of the great crowns (every 10 turns, minimum, end),
//! bankruptcies, settlements and provinces held by the majors (start / end),
//! sieges under way (cities and secondary places), captures by settlement
//! kind, armies stuck in front of a place, long sieges of level-4
//! fortresses, factions that disappeared.
//!
//! Usage: `settlements_probe [turns] [seed...]` (default 50 turns, seeds
//! 1-8). `VERBOSE=1` prints one line per major every 10 turns.
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use data_model::{FactionId, GameData, SettlementId, SettlementKind};
use sim_campaign::{ArmyId, CampaignState, EventKind};

const MAJORS: &[&str] = &["fac_france", "fac_england", "fac_burgundy", "fac_scotland"];
/// Turns without moving, outside friendly places and not besieging, after
/// which an army counts as stuck.
const STUCK_TURNS: u32 = 4;
/// A siege of a level-4 fortress lasting this long counts as a dead end.
const LONG_SIEGE_TURNS: u32 = 8;

fn id(s: &str) -> FactionId {
    FactionId::new(s).expect("well-formed id")
}

fn kind_label(kind: SettlementKind) -> &'static str {
    match kind {
        SettlementKind::City => "cité",
        SettlementKind::Town => "ville",
        SettlementKind::Castle => "château",
        SettlementKind::Abbey => "abbaye",
        SettlementKind::Village => "village",
    }
}

const KINDS: &[SettlementKind] = &[
    SettlementKind::City,
    SettlementKind::Town,
    SettlementKind::Castle,
    SettlementKind::Abbey,
    SettlementKind::Village,
];

#[derive(Default, Clone)]
struct MajorReport {
    treasury: Vec<i64>,
    min_treasury: i64,
    bankruptcies: u32,
    settlements: (usize, usize),
    provinces: (usize, usize),
    income_end: i64,
    upkeep_end: i64,
    alive_end: bool,
    /// Walled secondary places (town, castle, abbey) of border provinces
    /// held at the end: (with an empty garrison, total).
    bare_border: (usize, usize),
}

#[derive(Default)]
struct Report {
    seed: u64,
    majors: Vec<MajorReport>,
    /// Sum over turns of sieges under way (cities / other places).
    siege_turns_city: u32,
    siege_turns_other: u32,
    max_sieges: usize,
    /// Sieges started, by settlement kind.
    sieges_started: BTreeMap<&'static str, u32>,
    /// Changes of controller, by settlement kind.
    captures: BTreeMap<&'static str, u32>,
    /// Sum over turns of stuck armies, and the worst turn.
    stuck_turns: u32,
    max_stuck: u32,
    /// Of which next to (or on) an enemy level-4 fortress.
    stuck_l4_turns: u32,
    long_l4_sieges: u32,
    routs: u32,
    dispersed: u32,
    battles: u32,
    /// Landings on a hostile shore (all factions / England).
    landings: u32,
    landings_england: u32,
    destroyed: Vec<String>,
    turns: u32,
    seconds: f64,
}

fn run(data: &GameData, seed: u64, turns: u32) -> Report {
    let started = Instant::now();
    let france = id("fac_france");
    let mut state = CampaignState::new_1337(data, france.clone(), seed).expect("state");
    state.interactive_battles = false;
    let majors: Vec<FactionId> = MAJORS.iter().map(|m| id(m)).collect();
    let count_settlements = |state: &CampaignState, f: &FactionId| {
        state
            .settlements
            .values()
            .filter(|s| &s.controller == f)
            .count()
    };
    let mut report = Report {
        seed,
        turns,
        majors: majors
            .iter()
            .map(|f| MajorReport {
                min_treasury: i64::MAX,
                settlements: (count_settlements(&state, f), 0),
                provinces: (state.controlled_provinces(f).len(), 0),
                ..MajorReport::default()
            })
            .collect(),
        ..Report::default()
    };
    let alive_start: Vec<FactionId> = state
        .factions
        .iter()
        .filter(|(_, f)| f.alive)
        .map(|(id, _)| id.clone())
        .collect();
    let mut controllers: BTreeMap<SettlementId, FactionId> = state
        .settlements
        .iter()
        .map(|(id, s)| (id.clone(), s.controller.clone()))
        .collect();
    let mut still: BTreeMap<ArmyId, (SettlementId, u32)> = BTreeMap::new();
    let mut long_sieges_seen: std::collections::BTreeSet<SettlementId> = Default::default();

    for turn in 1..=turns {
        // France is the "player": it answers offers as the AI would.
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
                sim_campaign::Order::AnswerOffer {
                    offer: offer.id,
                    accept,
                },
            );
        }
        for order in ai::plan_turn(&state, data, &france) {
            let _ = state.submit_order(data, order);
        }
        let events = state.end_turn_with(data, ai::plan_turn);
        for event in &events {
            match event.kind {
                EventKind::Bankruptcy => {
                    if let Some(i) = event
                        .faction
                        .as_ref()
                        .and_then(|f| majors.iter().position(|m| m == f))
                    {
                        report.majors[i].bankruptcies += 1;
                    }
                }
                EventKind::Battle if event.text_fr.contains("Vainqueur") => report.battles += 1,
                EventKind::Attrition if event.text_fr.starts_with("Débarquement") => {
                    report.landings += 1;
                    if event
                        .faction
                        .as_ref()
                        .is_some_and(|f| f.as_str() == "fac_england")
                    {
                        report.landings_england += 1;
                    }
                }
                EventKind::Attrition if event.text_fr.starts_with("Débandade") => {
                    report.routs += 1
                }
                EventKind::ArmyDestroyed if event.text_fr.starts_with("Débandade") => {
                    report.dispersed += 1
                }
                _ => {}
            }
        }
        // Captures and sieges by kind.
        for (sid, s) in &state.settlements {
            let before = controllers.get(sid);
            if before != Some(&s.controller) {
                *report.captures.entry(kind_label(s.kind)).or_default() += 1;
                controllers.insert(sid.clone(), s.controller.clone());
            }
            if let Some(siege) = &s.siege {
                if siege.turns_elapsed <= 1 {
                    *report.sieges_started.entry(kind_label(s.kind)).or_default() += 1;
                }
                if s.fortification_level >= 4
                    && siege.turns_elapsed >= LONG_SIEGE_TURNS
                    && long_sieges_seen.insert(sid.clone())
                {
                    report.long_l4_sieges += 1;
                }
            }
        }
        let sieges: Vec<SettlementKind> = state
            .settlements
            .values()
            .filter(|s| s.siege.is_some())
            .map(|s| s.kind)
            .collect();
        let cities = sieges
            .iter()
            .filter(|k| **k == SettlementKind::City)
            .count();
        report.siege_turns_city += cities as u32;
        report.siege_turns_other += (sieges.len() - cities) as u32;
        report.max_sieges = report.max_sieges.max(sieges.len());

        // Stuck armies: same place for STUCK_TURNS turns, not on a friendly
        // place, not besieging it.
        let mut stuck: u32 = 0;
        let mut next_still = BTreeMap::new();
        for (army_id, army) in &state.armies {
            // Lot M2: the army's settlement, or the nearest one in the field.
            let Some(location) = state.army_anchor(data, army) else {
                continue;
            };
            let count = match still.get(army_id) {
                Some((loc, n)) if loc == &location => n + 1,
                _ => 1,
            };
            next_still.insert(army_id.clone(), (location.clone(), count));
            let besieging = state
                .settlements
                .get(&location)
                .and_then(|s| s.siege.as_ref())
                .is_some_and(|s| s.attacker == army.faction);
            if count >= STUCK_TURNS
                && !besieging
                && !state.is_friendly_settlement(&army.faction, &location)
            {
                stuck += 1;
                if std::env::var("STUCK").is_ok() && turn % 10 == 0 {
                    let place = state.settlements.get(&location);
                    println!(
                        "  [{seed}] t{turn} bloquée {army_id} {} à {} ({:?}, tenue par {}, fort. {}, posture {:?}, {} h., chemin {})",
                        army.faction.as_str(),
                        location.as_str(),
                        place.map(|p| p.kind),
                        place.map_or("?", |p| p.controller.as_str()),
                        state.fortification_level(data, &location),
                        army.stance,
                        army.total_strength(),
                        army.planned_path.len()
                    );
                }
                let near_l4 = std::iter::once(location.clone())
                    .chain(
                        sim_campaign::movement::edges(data, &location)
                            .into_iter()
                            .map(|(s, _)| s),
                    )
                    .any(|s| {
                        state.is_hostile_settlement(&army.faction, &s)
                            && state.fortification_level(data, &s) >= 4
                    });
                if near_l4 {
                    report.stuck_l4_turns += 1;
                }
            }
        }
        still = next_still;
        report.stuck_turns += stuck;
        report.max_stuck = report.max_stuck.max(stuck);

        for (i, f) in majors.iter().enumerate() {
            let treasury = state.factions[f].treasury;
            let m = &mut report.majors[i];
            m.min_treasury = m.min_treasury.min(treasury);
            if turn % 10 == 0 {
                m.treasury.push(treasury);
            }
        }
        if std::env::var("VERBOSE").is_ok() && turn % 10 == 0 {
            for f in &majors {
                let fs = &state.factions[f];
                println!(
                    "  [{seed}] {:>14} {:<14} trésor {:>7} revenu {:>6} entretien {:>6} colonies {:>3} provinces {:>2}",
                    state.date_label(),
                    f.as_str(),
                    fs.treasury,
                    fs.income_last_turn,
                    fs.upkeep_last_turn,
                    count_settlements(&state, f),
                    state.controlled_provinces(f).len()
                );
            }
        }
    }
    if std::env::var("VERBOSE").is_ok() {
        for f in &majors {
            let mut holders: BTreeMap<&str, usize> = BTreeMap::new();
            for s in state.settlements.values() {
                if &s.owner == f && &s.controller != f {
                    *holders.entry(s.controller.as_str()).or_default() += 1;
                }
            }
            let mut taken: BTreeMap<&str, usize> = BTreeMap::new();
            for s in state.settlements.values() {
                if &s.controller == f && &s.owner != f {
                    *taken.entry(s.owner.as_str()).or_default() += 1;
                }
            }
            println!(
                "  [{seed}] {} perdues chez {holders:?} ; prises à {taken:?}",
                f.as_str()
            );
        }
    }
    for (i, f) in majors.iter().enumerate() {
        let fs = &state.factions[f];
        let m = &mut report.majors[i];
        m.settlements.1 = count_settlements(&state, f);
        m.provinces.1 = state.controlled_provinces(f).len();
        m.income_end = fs.income_last_turn;
        m.upkeep_end = fs.upkeep_last_turn;
        m.alive_end = fs.alive;
        for s in state.settlements.values() {
            if &s.controller == f
                && matches!(
                    s.kind,
                    SettlementKind::Town | SettlementKind::Castle | SettlementKind::Abbey
                )
                && state.is_frontier(data, f, &s.province)
            {
                m.bare_border.1 += 1;
                if s.garrison.is_empty() {
                    m.bare_border.0 += 1;
                }
            }
        }
    }
    report.destroyed = alive_start
        .iter()
        .filter(|f| !state.factions[*f].alive)
        .map(|f| f.as_str().trim_start_matches("fac_").to_owned())
        .collect();
    report.seconds = started.elapsed().as_secs_f64();
    report
}

fn main() {
    let mut args = std::env::args().skip(1);
    let turns: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(50);
    let mut seeds: Vec<u64> = args.filter_map(|s| s.parse().ok()).collect();
    if seeds.is_empty() {
        seeds = (1..=8).collect();
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _) = GameData::load(&root).expect("data");
    let reports: Vec<Report> = seeds.iter().map(|s| run(&data, *s, turns)).collect();

    println!("\n## Trésor des grandes couronnes ({turns} tours ; relevés tous les 10 tours, min, banqueroutes)\n");
    println!("| graine | faction | trésor /10 tours | min | banq. | revenu fin | entretien fin |");
    println!("|---|---|---|---|---|---|---|");
    for r in &reports {
        for (i, m) in r.majors.iter().enumerate() {
            let t: Vec<String> = m.treasury.iter().map(|t| t.to_string()).collect();
            println!(
                "| {} | {} | {} | {} | {} | {} | {} |",
                r.seed,
                MAJORS[i].trim_start_matches("fac_"),
                t.join(" / "),
                m.min_treasury,
                m.bankruptcies,
                m.income_end,
                m.upkeep_end
            );
        }
    }
    println!("\n## Colonies et provinces tenues (début → fin)\n");
    println!("| graine | France col. | France prov. | Angl. col. | Angl. prov. | Bourg. col. | Bourg. prov. | Écosse col. | Écosse prov. |");
    println!("|---|---|---|---|---|---|---|---|---|");
    for r in &reports {
        let cells: Vec<String> = r
            .majors
            .iter()
            .flat_map(|m| {
                [
                    format!("{} → {}", m.settlements.0, m.settlements.1),
                    format!("{} → {}", m.provinces.0, m.provinces.1),
                ]
            })
            .collect();
        println!("| {} | {} |", r.seed, cells.join(" | "));
    }
    println!("\n## Sièges, prises, armées bloquées, disparitions\n");
    println!("| graine | sièges/tour (cités / autres) | max | sièges ouverts par type | prises par type | batailles | débandades (ralliées / dispersées) | bloquées/tour (max) | dont devant niv. 4 | sièges niv. 4 > {LONG_SIEGE_TURNS} t. | factions disparues | s |");
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|");
    let by_kind = |m: &BTreeMap<&'static str, u32>| {
        KINDS
            .iter()
            .map(|k| format!("{} {}", kind_label(*k), m.get(kind_label(*k)).unwrap_or(&0)))
            .collect::<Vec<_>>()
            .join(", ")
    };
    for r in &reports {
        let t = f64::from(r.turns.max(1));
        println!(
            "| {} | {:.1} / {:.1} | {} | {} | {} | {} | {} / {} | {:.1} ({}) | {:.1} | {} | {} | {:.1} |",
            r.seed,
            f64::from(r.siege_turns_city) / t,
            f64::from(r.siege_turns_other) / t,
            r.max_sieges,
            by_kind(&r.sieges_started),
            by_kind(&r.captures),
            r.battles,
            r.routs,
            r.dispersed,
            f64::from(r.stuck_turns) / t,
            r.max_stuck,
            f64::from(r.stuck_l4_turns) / t,
            r.long_l4_sieges,
            if r.destroyed.is_empty() {
                "-".to_owned()
            } else {
                r.destroyed.join(", ")
            },
            r.seconds
        );
    }

    // Synthesis.
    let n = reports.len() as f64;
    println!("\n## Synthèse ({} graines)\n", reports.len());
    for (i, major) in MAJORS.iter().enumerate() {
        let end: Vec<i64> = reports
            .iter()
            .map(|r| *r.majors[i].treasury.last().unwrap_or(&0))
            .collect();
        let negative = reports
            .iter()
            .filter(|r| r.majors[i].min_treasury < 0)
            .count();
        let bankrupt: u32 = reports.iter().map(|r| r.majors[i].bankruptcies).sum();
        let prov: f64 = reports
            .iter()
            .map(|r| r.majors[i].provinces.1 as f64 - r.majors[i].provinces.0 as f64)
            .sum::<f64>()
            / n;
        let col: f64 = reports
            .iter()
            .map(|r| r.majors[i].settlements.1 as f64 - r.majors[i].settlements.0 as f64)
            .sum::<f64>()
            / n;
        let dead = reports.iter().filter(|r| !r.majors[i].alive_end).count();
        let bare: usize = reports.iter().map(|r| r.majors[i].bare_border.0).sum();
        let border: usize = reports.iter().map(|r| r.majors[i].bare_border.1).sum();
        println!(
            "- {:<9} trésor final moy. {:>7.0} [{} ; {}], trésor négatif {}/{} graines, banqueroutes {}, Δ provinces {:+.1}, Δ colonies {:+.1}, disparue {}/{}, places frontalières secondaires sans garnison {}/{}",
            major.trim_start_matches("fac_"),
            end.iter().sum::<i64>() as f64 / n,
            end.iter().min().unwrap_or(&0),
            end.iter().max().unwrap_or(&0),
            negative,
            reports.len(),
            bankrupt,
            prov,
            col,
            dead,
            reports.len(),
            bare,
            border
        );
    }
    let sum = |f: &dyn Fn(&Report) -> f64| reports.iter().map(f).sum::<f64>() / n;
    let per_turn = |x: u32, r: &Report| f64::from(x) / f64::from(r.turns.max(1));
    println!(
        "- sièges en cours / tour : cités {:.1}, autres places {:.1} ; prises secondaires / graine {:.1}, cités {:.1}",
        sum(&|r| per_turn(r.siege_turns_city, r)),
        sum(&|r| per_turn(r.siege_turns_other, r)),
        sum(&|r| r
            .captures
            .iter()
            .filter(|(k, _)| **k != "cité")
            .map(|(_, v)| f64::from(*v))
            .sum()),
        sum(&|r| f64::from(*r.captures.get("cité").unwrap_or(&0))),
    );
    println!(
        "- armées bloquées / tour {:.1} (devant niv. 4 : {:.1}) ; sièges de niv. 4 > {LONG_SIEGE_TURNS} tours / graine {:.1}",
        sum(&|r| per_turn(r.stuck_turns, r)),
        sum(&|r| per_turn(r.stuck_l4_turns, r)),
        sum(&|r| f64::from(r.long_l4_sieges)),
    );
    println!(
        "- batailles / graine {:.1} ; débarquements en terre hostile / graine {:.1} (Angleterre {:.1})",
        sum(&|r| f64::from(r.battles)),
        sum(&|r| f64::from(r.landings)),
        sum(&|r| f64::from(r.landings_england)),
    );
    println!(
        "- débandades / graine {:.1} (dispersées {:.1}) ; factions disparues / graine {:.1} ; {:.1} s / graine",
        sum(&|r| f64::from(r.routs + r.dispersed)),
        sum(&|r| f64::from(r.dispersed)),
        sum(&|r| r.destroyed.len() as f64),
        sum(&|r| r.seconds),
    );
}
