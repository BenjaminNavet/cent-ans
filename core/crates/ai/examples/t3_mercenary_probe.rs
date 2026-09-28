//! Lot TW2-T3 (ADR 0103): companies the AI hires over N turns, their cost
//! and the arrears. Usage: `t3_mercenary_probe [seed] [turns] [on|off]
//! [start_year]` — `off` empties the bands (no company for hire); the
//! campaign starts in 1337, `start_year` moves its calendar (bands of a later
//! period: Great Companies from 1360, Scots in France from 1419...).
use std::path::Path;

use data_model::{FactionId, GameData};
use sim_campaign::mercenaries::is_mercenary;
use sim_campaign::{CampaignState, EventKind};

fn main() {
    let mut args = std::env::args().skip(1);
    let seed: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(7);
    let turns: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(20);
    let on = args.next().is_none_or(|s| s != "off");
    let start_year: Option<i32> = args.next().and_then(|s| s.parse().ok());
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (mut data, _) = GameData::load(&root).expect("data");
    if !on {
        data.mercenary_rules.bands.clear();
    }
    // Tuning: `MERC_RICH_PERCENT`, `MERC_THREAT_PERCENT` override the AI rules.
    let env = |key: &str| std::env::var(key).ok().and_then(|v| v.parse::<i64>().ok());
    if let Some(seasons) = env("MERC_RICH_PERCENT") {
        data.mercenary_rules.ai.rich_income_percent = seasons;
    }
    if let Some(percent) = env("MERC_THREAT_PERCENT") {
        data.mercenary_rules.ai.threat_ratio_percent = percent as u32;
    }
    let majors =
        ["fac_france", "fac_england", "fac_burgundy"].map(|id| FactionId::new(id).expect("id"));
    // Every major is played by the AI: the Papacy watches.
    let mut state =
        CampaignState::new_1337(&data, FactionId::new("fac_papacy").unwrap(), seed).unwrap();
    if let Some(year) = start_year {
        state.year = year;
    }
    println!(
        "seed {seed}, {turns} turns from {}, T3 {}",
        state.year,
        if on { "on" } else { "off" }
    );
    let mut hires: std::collections::BTreeMap<FactionId, u32> = Default::default();
    let mut premium: std::collections::BTreeMap<FactionId, i64> = Default::default();
    let mut arrears = 0u32;
    let mut totals = [0u64; 3];
    for turn in 0..=turns {
        if turn > 0 {
            let events = state.end_turn_with(&data, ai::plan_turn);
            arrears += events
                .iter()
                .filter(|e| {
                    matches!(e.kind, EventKind::Attrition | EventKind::Raid)
                        && e.text_fr.starts_with("Solde impayée")
                })
                .count() as u32;
            for (faction, paid) in &state.mercenaries.premium_last_turn {
                *premium.entry(faction.clone()).or_default() += paid;
            }
        }
        // Hires of the AI turn just played (counters of the resolved turn).
        for (faction, count) in &state.mercenaries.faction_hires {
            if state.mercenaries.hires_turn + 1 == state.turn {
                *hires.entry(faction.clone()).or_default() += count;
            }
        }
        let mut line = format!("{:>3} {:>16}", turn, state.date_label());
        for (index, faction) in majors.iter().enumerate() {
            let armies: Vec<_> = state
                .armies
                .values()
                .filter(|a| &a.faction == faction)
                .collect();
            let men: u32 = armies.iter().map(|a| a.total_strength()).sum();
            let mercs = armies
                .iter()
                .flat_map(|a| &a.units)
                .filter(|u| is_mercenary(&data, &u.unit_type))
                .count();
            let treasury = state.factions.get(faction).map_or(0, |f| f.treasury);
            // Treasury in seasons of gross income (the AI's « rich » test).
            let income = state.faction_income_effective(&data, faction).max(1);
            totals[index] += u64::from(men);
            line.push_str(&format!(
                " | {} {:>6}h {:>2}m {:>7}£ {:>4.1}s",
                &faction.as_str()[4..7],
                men,
                mercs,
                treasury,
                treasury as f64 / income as f64
            ));
        }
        if turn % 4 == 0 || turn == turns {
            println!("{line}");
        }
    }
    let n = u64::from(turns + 1);
    println!(
        "mean field men: FRA {} ENG {} BUR {}",
        totals[0] / n,
        totals[1] / n,
        totals[2] / n
    );
    let total: u32 = hires.values().sum();
    println!("companies hired: {total} {hires:?}");
    println!("premium paid: {premium:?}");
    println!("arrears events: {arrears}");
}
