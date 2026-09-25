//! Lot EQ2: diagnostics of the balance points EQ1 left open, on AI-vs-AI
//! campaigns (every faction, France included, planned by the AI).
//!
//! - why factions sit on the « High » tax (war, deficit, debt, hoard);
//! - the province disorder gauge (`ProvinceState::unrest`): share of
//!   province-seasons at 90 or more, longest streak and its causes;
//! - edicts chosen (Feudal aid share);
//! - France-England war share and the France-England peaces.
//!
//! Usage: `cargo run --release -p ai --example eq2_probe -- [turns] [seed...]`
//! (default 200 turns, seeds 1-8). `PROVINCE=prov_lothian` prints that
//! province's disorder every 4 turns.
use std::collections::BTreeMap;

use data_model::{FactionId, GameData, ProvinceId};
use sim_campaign::{CampaignState, TaxRate};

fn fid(s: &str) -> FactionId {
    FactionId::new(s).expect("well-formed id")
}

#[derive(Default)]
struct Totals {
    samples: u32,
    high: u32,
    high_war: u32,
    high_deficit: u32,
    high_debt: u32,
    high_hoard: u32,
    high_war_only_rich: u32,
    disorder_sum: f64,
    disorder_n: u32,
    disorder_90: u32,
    edicts: BTreeMap<String, u32>,
    war_turns: u32,
    turns: u32,
    fr_en_peaces: u32,
    fr_en_wars: u32,
    longest: Vec<(u32, String, u64)>,
    longest_peace: u32,
}

fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _) = GameData::load(&root).expect("data");
    let args: Vec<String> = std::env::args().skip(1).collect();
    let turns: u32 = args.first().and_then(|a| a.parse().ok()).unwrap_or(200);
    let mut seeds: Vec<u64> = args.iter().skip(1).filter_map(|a| a.parse().ok()).collect();
    if seeds.is_empty() {
        seeds = (1..=8).collect();
    }
    let watch = std::env::var("PROVINCE").ok();
    let mut t = Totals::default();
    for seed in seeds {
        run(&data, seed, turns, watch.as_deref(), &mut t);
    }
    let pct = |a: u32, b: u32| 100.0 * f64::from(a) / f64::from(b.max(1));
    println!("| Mesure | Valeur |\n|---|---|");
    println!("| Impôt Haut | {:.0} % |", pct(t.high, t.samples));
    println!(
        "| Haut : en guerre / déficit / dette / trésor au-dessus de la réserve | {:.0} / {:.0} / {:.0} / {:.0} % |",
        pct(t.high_war, t.high),
        pct(t.high_deficit, t.high),
        pct(t.high_debt, t.high),
        pct(t.high_hoard, t.high)
    );
    println!(
        "| Haut en guerre sans déficit ni dette, trésor au-dessus de la réserve | {:.0} % |",
        pct(t.high_war_only_rich, t.high)
    );
    println!(
        "| Troubles de province moyens | {:.1} |",
        t.disorder_sum / f64::from(t.disorder_n.max(1))
    );
    println!(
        "| Province-saisons à 90+ | {:.2} % |",
        pct(t.disorder_90, t.disorder_n)
    );
    let total: u32 = t.edicts.values().sum();
    let edicts: Vec<String> = t
        .edicts
        .iter()
        .map(|(k, v)| format!("{} {:.0} %", k.trim_start_matches("edict_"), pct(*v, total)))
        .collect();
    println!("| Édits | {} |", edicts.join(", "));
    println!("| Guerre FR-EN | {:.0} % |", pct(t.war_turns, t.turns));
    println!(
        "| Paix / guerres FR-EN (total) ; plus longue paix | {} / {} ; {} saisons |",
        t.fr_en_peaces, t.fr_en_wars, t.longest_peace
    );
    t.longest.sort();
    for (len, province, seed) in t.longest.iter().rev().take(5) {
        println!("| Plus longue série à 90+ | {province} (graine {seed}) : {len} saisons |");
    }
}

fn run(data: &GameData, seed: u64, turns: u32, watch: Option<&str>, t: &mut Totals) {
    let france = fid("fac_france");
    let england = fid("fac_england");
    let papacy = fid("fac_papacy");
    let mut state = CampaignState::new_1337(data, papacy, seed).expect("state");
    state.interactive_battles = false;
    let mut streak: BTreeMap<ProvinceId, u32> = BTreeMap::new();
    let mut best: (u32, String) = (0, String::new());
    let mut was_war = state.factions[&france].at_war_with.contains(&england);
    let mut peace_len = 0u32;
    let planner = |s: &CampaignState, d: &GameData, f: &FactionId| ai::plan_turn(s, d, f);
    for turn in 0..turns {
        state.end_turn_with(data, planner);
        t.turns += 1;
        let war = state.factions[&france].at_war_with.contains(&england);
        if war {
            t.war_turns += 1;
            peace_len = 0;
        } else {
            peace_len += 1;
            t.longest_peace = t.longest_peace.max(peace_len);
        }
        if war != was_war {
            if war {
                t.fr_en_wars += 1;
            } else {
                t.fr_en_peaces += 1;
            }
        }
        was_war = war;
        for (id, p) in &state.provinces {
            t.disorder_sum += f64::from(p.unrest);
            t.disorder_n += 1;
            let s = streak.entry(id.clone()).or_default();
            if p.unrest >= 90 {
                t.disorder_90 += 1;
                *s += 1;
                if *s > best.0 {
                    best = (*s, id.to_string());
                }
            } else {
                *s = 0;
            }
        }
        if let Some(w) = watch {
            if turn % 4 == 0 {
                let id = ProvinceId::new(w).expect("province id");
                if let Some(p) = state.provinces.get(&id) {
                    let controller = state.province_controller(&id);
                    let whole = controller
                        .as_ref()
                        .is_some_and(|c| state.holds_whole_province(c, &id));
                    let regency = controller
                        .as_ref()
                        .and_then(|c| state.factions.get(*c))
                        .is_some_and(|f| f.regency);
                    println!(
                        "seed {seed} turn {turn}: disorder {} unrest {:.0} controller {:?} whole {whole} regency {regency}",
                        p.unrest,
                        sim_campaign::population::weighted_unrest(&p.population),
                        controller.map(|c| c.to_string())
                    );
                }
            }
        }
        if (turn + 1) % 10 == 0 {
            for p in state.provinces.values() {
                let edict = p
                    .edict
                    .as_ref()
                    .map_or("edict_none".to_owned(), |e| e.edict.as_str().to_owned());
                *t.edicts.entry(edict).or_default() += 1;
            }
            for (id, f) in state.factions.iter().filter(|(_, f)| f.alive) {
                if id.as_str() == "fac_rebels" {
                    continue;
                }
                t.samples += 1;
                if f.tax_rate != TaxRate::High {
                    continue;
                }
                t.high += 1;
                let war = f.at_war_with.iter().any(|e| e.as_str() != "fac_rebels");
                let deficit = f.income_last_turn < f.upkeep_last_turn;
                let debt = f.treasury < 0;
                let hoard = f.treasury > 3 * f.income_last_turn.max(0);
                t.high_war += u32::from(war);
                t.high_deficit += u32::from(deficit);
                t.high_debt += u32::from(debt);
                t.high_hoard += u32::from(hoard);
                t.high_war_only_rich += u32::from(war && !deficit && !debt && hoard);
            }
        }
    }
    t.longest.push((best.0, best.1, seed));
}
