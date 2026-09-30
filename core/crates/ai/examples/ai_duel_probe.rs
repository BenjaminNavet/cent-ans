//! IA night: head-to-head A/B of campaign AI trial rules (`ai::experiment`).
//!
//! For each seed: one game where everybody plays the current rules (A),
//! then, for each faction F, one game where only F plays the trial rules
//! (B). F's fate at the end of B is compared with its fate at the end of A
//! (same seed): settlements held (city 3, town 2, other 1), provinces, army
//! power, treasury. A better rule makes the faction that plays it do better
//! against opponents that do not.
//!
//! Usage: `ai_duel_probe <rules> <turns> <factions> <seed...>`, e.g.
//! `ai_duel_probe defend,guard 60 fac_france,fac_england 1 2 3`.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use data_model::{FactionId, GameData, SettlementKind};
use sim_campaign::CampaignState;

#[derive(Clone, Copy, Default, Debug)]
struct Fate {
    settlements: f64,
    provinces: f64,
    power: f64,
    treasury: f64,
}

fn fate(state: &CampaignState, data: &GameData, faction: &FactionId) -> Fate {
    let settlements = state
        .settlements
        .values()
        .filter(|s| &s.controller == faction)
        .map(|s| match s.kind {
            SettlementKind::City => 3.0,
            SettlementKind::Town => 2.0,
            _ => 1.0,
        })
        .sum();
    let power = state
        .armies
        .iter()
        .filter(|(_, a)| &a.faction == faction)
        .map(|(id, _)| state.army_power(data, id))
        .sum();
    Fate {
        settlements,
        provinces: state.controlled_provinces(faction).len() as f64,
        power,
        treasury: state
            .factions
            .get(faction)
            .map_or(0.0, |f| f.treasury as f64),
    }
}

/// Plays `turns` turns of `seed` and returns every faction's fate.
fn play(data: &GameData, seed: u64, turns: u32) -> BTreeMap<FactionId, Fate> {
    let france = FactionId::new("fac_france").expect("id");
    let mut state = CampaignState::new_1337(data, france.clone(), seed).expect("state");
    state.interactive_battles = false;
    for _ in 0..turns {
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
        state.end_turn_with(data, ai::plan_turn);
    }
    state
        .factions
        .keys()
        .map(|f| (f.clone(), fate(&state, data, f)))
        .collect()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let rules: BTreeSet<String> = args[0].split(',').map(str::to_owned).collect();
    let turns: u32 = args[1].parse().expect("turns");
    let factions: Vec<FactionId> = args[2]
        .split(',')
        .map(|f| FactionId::new(f).expect("faction id"))
        .collect();
    let seeds: Vec<u64> = args[3..].iter().filter_map(|a| a.parse().ok()).collect();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let data = GameData::load(&root).expect("game data loads").0;
    println!("| seed | faction | settl. A → B | prov. A → B | power A → B | treasury A → B |");
    println!("|---|---|---|---|---|---|");
    let mut better = 0;
    let mut worse = 0;
    let mut total = Fate::default();
    for &seed in &seeds {
        ai::experiment::set(None);
        let base = play(&data, seed, turns);
        for faction in &factions {
            ai::experiment::set(Some((
                std::iter::once(faction.as_str().to_owned()).collect(),
                rules.clone(),
            )));
            let trial = play(&data, seed, turns);
            let (a, b) = (base[faction], trial[faction]);
            println!(
                "| {seed} | {faction} | {:.0} → {:.0} | {:.0} → {:.0} | {:.0} → {:.0} | {:.0} → {:.0} |",
                a.settlements, b.settlements, a.provinces, b.provinces, a.power, b.power, a.treasury, b.treasury
            );
            if b.settlements > a.settlements {
                better += 1;
            } else if b.settlements < a.settlements {
                worse += 1;
            }
            total.settlements += b.settlements - a.settlements;
            total.provinces += b.provinces - a.provinces;
            total.power += b.power - a.power;
            total.treasury += b.treasury - a.treasury;
        }
    }
    ai::experiment::set(None);
    let n = (seeds.len() * factions.len()).max(1) as f64;
    println!(
        "\nrules {rules:?}: settlements better {better} / worse {worse} of {n}; mean delta settlements {:+.1}, provinces {:+.2}, power {:+.0}, treasury {:+.0}",
        total.settlements / n,
        total.provinces / n,
        total.power / n,
        total.treasury / n
    );
}
