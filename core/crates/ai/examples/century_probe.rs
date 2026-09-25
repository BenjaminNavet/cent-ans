//! Plays whole campaigns (default 5 seeds × 464 turns, 1337-1453) with the
//! strategic AI for every faction and prints the F4 balance table
//! (`docs/design/v2-finalisation.md` § 2.2): France-England war share and
//! phases, historical alliances, calls to arms, idle treasuries,
//! bankruptcies, marriages, battles, map changes and survivals.
//!
//! Usage: `century_probe [turns] [seed...]`; `VERBOSE=1` prints the
//! France-England wars and peaces.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Instant;

use data_model::{CharacterId, FactionId, GameData};
use sim_campaign::{CampaignState, EventKind};

/// First turn of 1350 (the treasury rule applies from then on).
const TURN_1350: u32 = 13 * 4;
/// First turn of 1400 (survival check).
const TURN_1400: u32 = 63 * 4;
/// Treasury limit in seasons of income.
const IDLE_SEASONS: f64 = 8.0;

const PAIRS: &[(&str, &str, &str)] = &[
    ("Auld Alliance", "fac_scotland", "fac_france"),
    ("Angl.-Flandre", "fac_england", "fac_flanders"),
    ("Angl.-Hainaut", "fac_england", "fac_hainaut"),
    ("Angl.-Gueldre", "fac_england", "fac_guelders"),
    ("Angl.-Brabant", "fac_england", "fac_brabant"),
    ("Fr.-Bohême", "fac_france", "fac_bohemia"),
    ("Fr.-Naples", "fac_france", "fac_naples"),
    ("Bourg.-France", "fac_burgundy", "fac_france"),
    ("Bourg.-Angl.", "fac_burgundy", "fac_england"),
];
const MAJORS: &[&str] = &["fac_england", "fac_france", "fac_burgundy", "fac_scotland"];
/// UR2: the 15th-century unit types (lot UR1, `available_from` 1380-1448), checked so that a
/// probe over the whole 1337-1453 span can confirm the AI actually recruits them.
const CENTURY_15: &[(&str, &str)] = &[
    ("ordonnance", "unit_ordonnance_gendarmes"),
    ("francs-archers", "unit_francs_archers"),
    ("coutiliers", "unit_coutiliers"),
    ("couleuvriniers", "unit_culveriners"),
];

fn id(s: &str) -> FactionId {
    FactionId::new(s).expect("well-formed id")
}

#[derive(Default)]
struct Report {
    seed: u64,
    turns: u32,
    war_turns: u32,
    war_phases: u32,
    /// Turns England dominates the French realm (G2).
    dominance_turns: u32,
    /// Most French provinces (of 1337) England held at once.
    max_held: usize,
    alliance_turns: Vec<u32>,
    /// Alliance turns while France and England are at war (G2).
    alliance_war_turns: Vec<u32>,
    /// Turn each alliance was first seen (G4).
    alliance_first: Vec<Option<u32>>,
    calls_honoured: u32,
    calls_refused: u32,
    /// Worst treasury / income ratio after 1350 and its faction.
    idle_ratio: f64,
    idle_faction: String,
    /// Factions above the limit for more than 4 turns after 1350.
    idle_count: usize,
    bankruptcies: BTreeMap<FactionId, u32>,
    factions: usize,
    marriages_cross: u32,
    marriages_same: u32,
    battles_fr_en: u32,
    captures: u32,
    majors_1400: Vec<bool>,
    /// Turn each major first fell (G5).
    majors_fall: Vec<Option<u32>>,
    destroyed: u32,
    issued: u32,
    refused: u32,
    /// UR2: whether each of `CENTURY_15` was ever seen in an army or a garrison (any faction).
    recruited_15th: Vec<bool>,
    seconds: f64,
}

/// `TRACE=1`: Burgundy's standing every 5 years (G4).
fn trace_burgundy(state: &CampaignState, data: &GameData, seed: u64) {
    let (france, england, burgundy) = (id("fac_france"), id("fac_england"), id("fac_burgundy"));
    let b = &state.factions[&burgundy];
    let own = |f: &FactionId| {
        state
            .provinces
            .keys()
            .filter(|id| state.province_owner(id) == Some(f))
            .count()
    };
    let score = if state.is_at_war(&france, &england) {
        state.war_score(data, &france, &england)
    } else {
        0
    };
    println!(
        "  [{seed}] {:>16} alive:{} suz:{:?} loy:{} att(fr):{} att(en):{} war(fr/en):{}/{} ally(fr/en):{}/{} en-held:{} prov fr/en/bu:{}/{}/{} fr-en:{} ws(fr,en):{score} claims(en>bu):{:?} grief:{:?}",
        state.date_label(),
        b.alive,
        b.suzerain.as_ref().map(|s| s.as_str()),
        b.loyalty,
        state.attitude(data, &burgundy, &france).0,
        state.attitude(data, &burgundy, &england).0,
        state.is_at_war(&burgundy, &france),
        state.is_at_war(&burgundy, &england),
        state.is_allied(&burgundy, &france),
        state.is_allied(&burgundy, &england),
        ai::alignment::realm_held_by(state, data, &france, &england),
        own(&france),
        own(&england),
        own(&burgundy),
        state.is_at_war(&france, &england),
        sim_campaign::diplomacy::claim_stakes(state, &england, &burgundy).provinces,
        ai::alignment::grievance_change(state, data, &burgundy),
    );
    if std::env::var("TRACE_EVAL").is_ok() {
        let e = sim_campaign::diplomacy::evaluate(
            state,
            data,
            &burgundy,
            &england,
            &sim_campaign::diplomacy::Proposal::Alliance,
        );
        println!(
            "  [{seed}]     side_change {:?} grudge {} alliance en {} {:?}",
            ai::alignment::side_change(state, data, &burgundy),
            ai::alignment::grievance(state, &burgundy, &france),
            e.accept,
            e.reasons
        );
    }
    for low in ["fac_brabant", "fac_hainaut"] {
        let low = id(low);
        let Some(f) = state.factions.get(&low).filter(|f| f.alive) else {
            continue;
        };
        println!(
            "  [{seed}] {:>16}   {} suz:{:?} att(en/fr):{}/{} ally(en/fr):{}/{} war(en/fr):{}/{} allies:{:?}",
            state.date_label(),
            low.as_str(),
            f.suzerain.as_ref().map(|s| s.as_str()),
            state.attitude(data, &low, &england).0,
            state.attitude(data, &low, &france).0,
            state.is_allied(&low, &england),
            state.is_allied(&low, &france),
            state.is_at_war(&low, &england),
            state.is_at_war(&low, &france),
            f.allies.iter().map(|a| a.as_str()).collect::<Vec<_>>(),
        );
    }
}

fn run(data: &GameData, seed: u64, turns: u32, verbose: bool) -> Report {
    // `TRACE=1` follows Burgundy; any other value filters events by that word.
    let trace_word =
        std::env::var("TRACE")
            .ok()
            .map(|w| if w == "1" { "Bourgogne".to_owned() } else { w });
    let trace = trace_word.is_some();
    let started = Instant::now();
    let france = id("fac_france");
    let england = id("fac_england");
    let mut state = CampaignState::new_1337(data, france.clone(), seed).expect("state");
    let mut report = Report {
        seed,
        turns,
        alliance_turns: vec![0; PAIRS.len()],
        alliance_war_turns: vec![0; PAIRS.len()],
        alliance_first: vec![None; PAIRS.len()],
        majors_1400: vec![true; MAJORS.len()],
        majors_fall: vec![None; MAJORS.len()],
        recruited_15th: vec![false; CENTURY_15.len()],
        ..Report::default()
    };
    let names: Vec<String> = [&france, &england]
        .iter()
        .map(|f| data.factions[*f].short_or_display_name().to_owned())
        .collect();
    let married = |state: &CampaignState| -> BTreeSet<(CharacterId, CharacterId, bool)> {
        state
            .characters
            .iter()
            .filter(|(_, c)| c.alive)
            .filter_map(|(id, c)| {
                let spouse = c.spouse.as_ref()?;
                let s = state.characters.get(spouse)?;
                (id < spouse).then(|| (id.clone(), spouse.clone(), s.faction != c.faction))
            })
            .collect()
    };
    let mut seen = married(&state);
    let mut over_limit: BTreeMap<FactionId, u32> = BTreeMap::new();
    // Rolling income (last 8 seasons): a season under siege is not the
    // measure of a realm's revenue.
    let mut incomes: BTreeMap<FactionId, Vec<i64>> = BTreeMap::new();
    let mut was_at_war = false;
    for _ in 0..turns {
        // France is the "player": it answers offers as the AI would judge
        // them (otherwise no peace offered to France is ever signed).
        let offers = state.factions[&france].offers.clone();
        for offer in offers {
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
            report.issued += 1;
            if state.submit_order(data, order).is_err() {
                report.refused += 1;
            }
        }
        let events = state.end_turn_with(data, ai::plan_turn);
        for event in &events {
            match event.kind {
                EventKind::Bankruptcy => {
                    if let Some(f) = &event.faction {
                        *report.bankruptcies.entry(f.clone()).or_default() += 1;
                    }
                }
                EventKind::Battle
                    if !event.text_fr.starts_with("Bataille en vue")
                        && !event.text_fr.contains("se prépare")
                        && names.iter().any(|n| event.text_fr.contains(n.as_str())) =>
                {
                    report.battles_fr_en += 1;
                }
                EventKind::ProvinceCaptured => report.captures += 1,
                EventKind::FactionDestroyed => report.destroyed += 1,
                EventKind::WarDeclared if event.text_fr.contains("répond à l'appel") => {
                    report.calls_honoured += 1;
                }
                EventKind::AllianceBroken if event.text_fr.contains("refuse de soutenir") => {
                    report.calls_refused += 1;
                }
                _ => {}
            }
            if trace
                && trace_word
                    .as_ref()
                    .is_some_and(|w| event.text_fr.contains(w.as_str()))
                && !matches!(event.kind, EventKind::Battle | EventKind::Marriage)
            {
                println!("  [{seed}] {:>16} * {}", state.date_label(), event.text_fr);
            }
            if verbose
                && matches!(event.kind, EventKind::WarDeclared | EventKind::PeaceSigned)
                && names.iter().all(|n| event.text_fr.contains(n.as_str()))
            {
                println!("  [{seed}] {:>16} {}", state.date_label(), event.text_fr);
            }
        }
        let at_war = state.is_at_war(&france, &england);
        if at_war {
            report.war_turns += 1;
            if !was_at_war {
                report.war_phases += 1;
            }
        }
        was_at_war = at_war;
        report.max_held = report.max_held.max(ai::alignment::realm_held_by(
            &state, data, &france, &england,
        ));
        if ai::alignment::dominates_realm(&state, data, &england, &france) {
            report.dominance_turns += 1;
        }
        for (index, (_, a, b)) in PAIRS.iter().enumerate() {
            if state.is_allied(&id(a), &id(b)) {
                report.alliance_turns[index] += 1;
                report.alliance_first[index].get_or_insert(state.turn);
                if at_war {
                    report.alliance_war_turns[index] += 1;
                }
            }
        }
        let now = married(&state);
        for (_, _, cross) in now.difference(&seen) {
            if *cross {
                report.marriages_cross += 1;
            } else {
                report.marriages_same += 1;
            }
        }
        seen = now;
        for (fid, f) in &state.factions {
            let history = incomes.entry(fid.clone()).or_default();
            history.push(f.income_last_turn);
            if history.len() > 8 {
                history.remove(0);
            }
        }
        if state.turn >= TURN_1350 {
            for (fid, f) in state.factions.iter().filter(|(_, f)| f.alive) {
                if fid.as_str() == "fac_rebels" {
                    continue;
                }
                let history = &incomes[fid];
                let income = history.iter().sum::<i64>() / history.len().max(1) as i64;
                let ratio = f.treasury as f64 / income.max(1) as f64;
                if ratio > IDLE_SEASONS && f.treasury > 10_000 {
                    *over_limit.entry(fid.clone()).or_default() += 1;
                }
                if f.treasury > 10_000 && ratio > report.idle_ratio {
                    report.idle_ratio = ratio;
                    report.idle_faction = fid.to_string();
                }
            }
        }
        if trace && (state.turn.is_multiple_of(20) || (328..372).contains(&state.turn)) {
            trace_burgundy(&state, data, seed);
        }
        for (index, major) in MAJORS.iter().enumerate() {
            if !state.factions[&id(major)].alive {
                report.majors_fall[index].get_or_insert(state.turn);
            }
        }
        if state.turn == TURN_1400 {
            for (index, major) in MAJORS.iter().enumerate() {
                report.majors_1400[index] = state.factions[&id(major)].alive;
            }
        }
        if report.recruited_15th.iter().any(|seen| !seen) {
            for (index, (_, unit_type)) in CENTURY_15.iter().enumerate() {
                if !report.recruited_15th[index]
                    && state
                        .armies
                        .values()
                        .flat_map(|army| army.units.iter())
                        .chain(
                            state
                                .settlements
                                .values()
                                .flat_map(|settlement| settlement.garrison.iter()),
                        )
                        .any(|unit| unit.unit_type.as_str() == *unit_type)
                {
                    report.recruited_15th[index] = true;
                }
            }
        }
    }
    report.idle_count = over_limit.values().filter(|n| **n > 4).count();
    report.factions = state
        .factions
        .keys()
        .filter(|f| f.as_str() != "fac_rebels")
        .count();
    report.seconds = started.elapsed().as_secs_f64();
    if verbose {
        let mut worst: Vec<_> = report.bankruptcies.iter().collect();
        worst.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
        println!("  [{seed}] bankruptcies {worst:?}");
        println!("  [{seed}] idle treasuries (turns over limit) {over_limit:?}");
    }
    report
}

fn main() {
    let mut args = std::env::args().skip(1);
    let turns: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(464);
    let mut seeds: Vec<u64> = args.filter_map(|s| s.parse().ok()).collect();
    if seeds.is_empty() {
        seeds = vec![1, 2, 3, 4, 5];
    }
    let verbose = std::env::var("VERBOSE").is_ok();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _) = GameData::load(&root).expect("data");
    // Seeds run in parallel (each campaign is deterministic on its own).
    let reports: Vec<Report> = std::thread::scope(|scope| {
        let handles: Vec<_> = seeds
            .iter()
            .map(|seed| {
                let data = &data;
                scope.spawn(move || run(data, *seed, turns, verbose))
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().expect("campaign thread"))
            .collect()
    });

    let decades = f64::from(turns) / 40.0;
    println!(
        "\n| graine | guerre FR-EN | phases | batailles FR/EN / déc. | prises | trésor max (saisons) | trésors > 8 | banqueroutes / fac. / déc. | pire faction | mariages (inter / intra) | appels (oui / non) | majeures en 1400 | détruites | refus FR | s |"
    );
    println!("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for r in &reports {
        let total: u32 = r.bankruptcies.values().sum();
        let (worst, worst_n) = r
            .bankruptcies
            .iter()
            .max_by_key(|(f, n)| (**n, std::cmp::Reverse((*f).clone())))
            .map_or(("-".to_owned(), 0), |(f, n)| (f.to_string(), *n));
        let majors: usize = r.majors_1400.iter().filter(|a| **a).count();
        println!(
            "| {} | {:.0} % | {} | {:.1} | {} | {:.1} ({}) | {} | {:.2} | {} ({:.1}/déc.) | {} / {} | {} / {} | {}/4 | {} | {:.1} % | {:.1} |",
            r.seed,
            100.0 * f64::from(r.war_turns) / f64::from(r.turns),
            r.war_phases,
            f64::from(r.battles_fr_en) / decades,
            r.captures,
            r.idle_ratio,
            r.idle_faction.trim_start_matches("fac_"),
            r.idle_count,
            f64::from(total) / r.factions as f64 / decades,
            worst.trim_start_matches("fac_"),
            f64::from(worst_n) / decades,
            r.marriages_cross,
            r.marriages_same,
            r.calls_honoured,
            r.calls_refused,
            majors,
            r.destroyed,
            100.0 * f64::from(r.refused) / f64::from(r.issued.max(1)),
            r.seconds,
        );
    }
    println!("\nAlliances (part des tours) :");
    for (index, (label, _, _)) in PAIRS.iter().enumerate() {
        let shares: Vec<String> = reports
            .iter()
            .map(|r| {
                format!(
                    "{:.0} %",
                    100.0 * f64::from(r.alliance_turns[index]) / f64::from(r.turns)
                )
            })
            .collect();
        let war_shares: Vec<String> = reports
            .iter()
            .map(|r| {
                format!(
                    "{:.0} %",
                    100.0 * f64::from(r.alliance_war_turns[index]) / f64::from(r.war_turns.max(1))
                )
            })
            .collect();
        let first: Vec<String> = reports
            .iter()
            .map(|r| {
                r.alliance_first[index].map_or("-".to_owned(), |turn| {
                    format!("{}", 1337 + (turn.saturating_sub(1)) / 4)
                })
            })
            .collect();
        println!(
            "  {label:14} {}   (en guerre FR-EN : {} ; dès {})",
            shares.join(" | "),
            war_shares.join(" | "),
            first.join(" | ")
        );
    }
    let scots: Vec<String> = reports
        .iter()
        .map(|r| {
            let n = r
                .bankruptcies
                .get(&id("fac_scotland"))
                .copied()
                .unwrap_or(0);
            format!("{:.1}", f64::from(n) / decades)
        })
        .collect();
    let dominance: Vec<String> = reports
        .iter()
        .map(|r| {
            format!(
                "{:.0} % (max {} prov.)",
                100.0 * f64::from(r.dominance_turns) / f64::from(r.turns),
                r.max_held
            )
        })
        .collect();
    println!("Angleterre dominant le royaume : {}", dominance.join(" | "));
    println!("Banqueroutes de l'Écosse / déc. : {}", scots.join(" | "));
    let survivors: Vec<String> = MAJORS
        .iter()
        .enumerate()
        .map(|(index, m)| {
            let n = reports.iter().filter(|r| r.majors_1400[index]).count();
            format!("{} {n}/{}", m.trim_start_matches("fac_"), reports.len())
        })
        .collect();
    println!("Survie en 1400 : {}", survivors.join(", "));
    let century_15: Vec<String> = CENTURY_15
        .iter()
        .enumerate()
        .map(|(index, (label, _))| {
            let n = reports.iter().filter(|r| r.recruited_15th[index]).count();
            format!("{label} {n}/{}", reports.len())
        })
        .collect();
    println!("Types du XVe s. recrutés (UR2) : {}", century_15.join(", "));
    print_summary(&reports, decades);
}

/// Mean, minimum and maximum of `values`.
fn spread(values: &[f64]) -> (f64, f64, f64) {
    let n = values.len().max(1) as f64;
    let mean = values.iter().sum::<f64>() / n;
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (mean, min, max)
}

/// G5 synthesis: the indicators of `docs/status.md` (G2, G4) over all seeds.
fn print_summary(reports: &[Report], decades: f64) {
    let pair = |label: &str| {
        PAIRS
            .iter()
            .position(|(l, _, _)| *l == label)
            .expect("pair")
    };
    let war_share = |r: &Report, index: usize| {
        100.0 * f64::from(r.alliance_war_turns[index]) / f64::from(r.war_turns.max(1))
    };
    let war: Vec<f64> = reports
        .iter()
        .map(|r| 100.0 * f64::from(r.war_turns) / f64::from(r.turns))
        .collect();
    let (mean, min, max) = spread(&war);
    let in_band = war.iter().filter(|w| (55.0..=75.0).contains(*w)).count();
    println!("\nSynthèse ({} graines) :", reports.len());
    println!(
        "  Guerre FR-EN : moy. {mean:.0} % [{min:.0}-{max:.0}], {in_band}/{} graines dans 55-75 %",
        reports.len()
    );
    for label in ["Auld Alliance", "Angl.-Brabant", "Bourg.-France"] {
        let index = pair(label);
        let shares: Vec<f64> = reports.iter().map(|r| war_share(r, index)).collect();
        let (mean, min, max) = spread(&shares);
        println!("  {label} (tours de guerre FR-EN) : moy. {mean:.0} % [{min:.0}-{max:.0}]");
    }
    let burgundy = pair("Bourg.-Angl.");
    // Turn 253 is the first of 1400.
    let fifteenth = reports
        .iter()
        .filter(|r| r.alliance_first[burgundy].is_some_and(|t| t > TURN_1400))
        .count();
    let any = reports
        .iter()
        .filter(|r| r.alliance_first[burgundy].is_some())
        .count();
    println!(
        "  Bourg.-Angl. : {any}/{n} graines, dont {fifteenth}/{n} au XVe s.",
        n = reports.len()
    );
    let bankruptcies: Vec<f64> = reports
        .iter()
        .map(|r| {
            let total: u32 = r.bankruptcies.values().sum();
            f64::from(total) / r.factions as f64 / decades
        })
        .collect();
    let (mean, min, max) = spread(&bankruptcies);
    println!("  Banqueroutes / fac. / déc. : moy. {mean:.2} [{min:.2}-{max:.2}]");
    let calls: Vec<f64> = reports
        .iter()
        .map(|r| f64::from(r.calls_honoured))
        .collect();
    let (mean, min, max) = spread(&calls);
    println!("  Appels aux armes honorés : moy. {mean:.0} [{min:.0}-{max:.0}]");
    let all_alive = reports
        .iter()
        .filter(|r| r.majors_1400.iter().all(|a| *a))
        .count();
    println!(
        "  4 majeures en vie en 1400 : {all_alive}/{}",
        reports.len()
    );
    for r in reports {
        for (index, major) in MAJORS.iter().enumerate() {
            if let Some(turn) = r.majors_fall[index] {
                println!(
                    "    graine {} : {} tombe en {}{}",
                    r.seed,
                    major.trim_start_matches("fac_"),
                    1337 + turn.saturating_sub(1) / 4,
                    if r.majors_1400[index] {
                        ""
                    } else {
                        " (avant 1400)"
                    }
                );
            }
        }
    }
}
