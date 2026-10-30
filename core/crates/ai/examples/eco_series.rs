//! Economic series probe (ECO-R0): the AI plays every faction (France included, unless
//! `--passive`), one JSON line per turn with the budget, army, politics of the watched factions.
//!
//! `cargo run --release -p ai --example eco_series -- [--seeds 1,2,3] [--turns 464]
//!   [--passive] [--factions fac_france,fac_england]`
//!
//! Each line carries `seed`. `--passive`: France gives no order and refuses nothing (the AI
//! still plays the others). Default factions: the ten major houses.
use std::collections::BTreeMap;
use std::path::Path;

use data_model::{FactionId, GameData};
use serde_json::json;
use sim_campaign::{CampaignState, EventKind, Order};

const DEFAULT_WATCH: [&str; 10] = [
    "fac_france",
    "fac_england",
    "fac_burgundy",
    "fac_brittany",
    "fac_flanders",
    "fac_navarre",
    "fac_scotland",
    "fac_castile",
    "fac_blois",
    "fac_armagnac",
];

fn fid(s: &str) -> FactionId {
    FactionId::new(s).expect("id")
}

struct Options {
    seeds: Vec<u64>,
    turns: u32,
    passive: bool,
    factions: Vec<String>,
}

fn parse_options(args: &[String]) -> Options {
    let mut options = Options {
        seeds: vec![1],
        turns: 464,
        passive: false,
        factions: DEFAULT_WATCH.iter().map(|s| (*s).to_owned()).collect(),
    };
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--seeds" => {
                if let Some(v) = iter.next() {
                    options.seeds = v.split(',').filter_map(|s| s.parse().ok()).collect();
                }
            }
            "--turns" => {
                if let Some(v) = iter.next().and_then(|v| v.parse().ok()) {
                    options.turns = v;
                }
            }
            "--passive" => options.passive = true,
            "--factions" => {
                if let Some(v) = iter.next() {
                    options.factions = v.split(',').map(str::to_owned).collect();
                }
            }
            other => eprintln!("unknown option {other}"),
        }
    }
    options
}

/// Plays `turns` turns of one seed, printing one JSON line per turn.
fn run_seed(data: &GameData, seed: u64, options: &Options, out: &mut impl FnMut(String)) {
    let france = fid("fac_france");
    let mut state = CampaignState::new_1337(data, france.clone(), seed).expect("state");
    let turns = options.turns;
    let passive = options.passive;
    let watch: Vec<&str> = options.factions.iter().map(String::as_str).collect();
    for _ in 0..turns {
        if !passive {
            for offer in state.factions[&france].offers.clone() {
                let accept = sim_campaign::negotiation::evaluate_treaty(
                    &state,
                    data,
                    &offer.from,
                    &france,
                    &offer.proposal.articles,
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
            for order in ai::plan_turn(&state, data, &france) {
                let _ = state.submit_order(data, order);
            }
        }
        let events = state.end_turn_with(data, ai::plan_turn);
        let mut kinds: BTreeMap<String, u32> = BTreeMap::new();
        let mut notable: Vec<serde_json::Value> = Vec::new();
        for e in &events {
            *kinds.entry(format!("{:?}", e.kind)).or_default() += 1;
            let watched = e
                .faction
                .as_ref()
                .is_some_and(|f| watch.contains(&f.as_str()));
            if watched
                && matches!(
                    e.kind,
                    EventKind::Revolt
                        | EventKind::Plague
                        | EventKind::Bankruptcy
                        | EventKind::PeaceSigned
                        | EventKind::WarDeclared
                        | EventKind::Vassalage
                        | EventKind::VassalRebellion
                        | EventKind::Succession
                        | EventKind::Regency
                        | EventKind::Coinage
                        | EventKind::Ransom
                        | EventKind::Marriage
                        | EventKind::AllianceFormed
                        | EventKind::Loyalty
                        | EventKind::Excommunication
                        | EventKind::Schism
                        | EventKind::Famine
                        | EventKind::Death
                        | EventKind::Edict
                        | EventKind::RoyalAct
                        | EventKind::Assembly
                        | EventKind::Diplomacy
                )
            {
                notable.push(json!({"k": format!("{:?}", e.kind), "f": e.faction.as_ref().map(|f| f.as_str()), "t": e.text_fr}));
            }
        }
        let mut fac = serde_json::Map::new();
        for w in watch.iter().copied() {
            let id = fid(w);
            let Some(f) = state.factions.get(&id) else {
                continue;
            };
            let b = &f.last_budget;
            let provs = state.owned_provinces(&id);
            let (mut pop, mut unrest_w) = (0u64, 0.0f64);
            for p in &provs {
                if let Some(ps) = state.provinces.get(p) {
                    let c = &ps.population;
                    let n = c.peasants.count + c.burghers.count + c.clergy.count + c.nobility.count;
                    pop += n;
                    unrest_w += sim_campaign::population::weighted_unrest(c) * n as f64;
                }
            }
            let units: usize = state
                .armies
                .values()
                .filter(|a| a.faction == id)
                .map(|a| a.units.len())
                .sum();
            // ECO-R2a: the men under arms count the contingents of the vassals' host.
            let host_men: u64 = state
                .armies
                .values()
                .filter(|a| {
                    a.faction != id && a.host_service.as_ref().is_some_and(|s| s.liege == id)
                })
                .map(|a| u64::from(a.total_strength()))
                .sum();
            let own_men: u64 = state
                .armies
                .values()
                .filter(|a| a.faction == id)
                .map(|a| u64::from(a.total_strength()))
                .sum();
            let men = own_men + host_men;
            fac.insert(w.to_string(), json!({
                "alive": f.alive, "tr": f.treasury, "inc": b.income, "up": b.upkeep(), "army": b.army_upkeep,
                "bld": b.building_upkeep, "adm": b.administration, "hh": b.household, "wag": b.wages, "sie": b.siege,
                "obl": b.obligations, "int": b.interest, "levy": b.levy_income, "bor": b.borrowed, "rep": b.repaid,
                "trade": b.trade_income, "seign": b.seigniorage,
                "debt": f.finance.debt(), "defaults": f.finance.defaults, "levies": f.finance.levies_raised,
                "rating": f.finance.rating, "coin": format!("{:?}", f.coinage), "price": f.price_level,
                "prov": provs.len(), "pop": pop, "unrest": if pop > 0 { unrest_w / pop as f64 } else { 0.0 },
                "units": units, "men": men, "host_men": host_men,
                "wars": f.at_war_with.iter().map(|x| x.as_str().to_string()).collect::<Vec<_>>(),
                "allies": f.allies.len(), "suz": f.suzerain.as_ref().map(|s| s.as_str().to_string()), "loy": f.loyalty,
                "vassals": state.factions.iter().filter(|(_, o)| o.alive && o.suzerain.as_ref() == Some(&id)).count(),
                "prestige": f.ruler.as_ref().and_then(|r| state.characters.get(r)).map(|c| c.prestige),
                "regency": f.regency, "ransom_debts": f.ransom_debts.len(),
                "deficit_seasons": f.deficit_seasons,
                "asm": assembly_summary(&state, data, &id),
            }));
        }
        let line = json!({"seed": seed, "turn": state.turn, "year": state.year, "kinds": kinds, "fac": fac, "notable": notable,
            "outcome": state.outcome.as_ref().map(|o| format!("{:?}", o.kind))});
        out(line.to_string());
    }
}

/// ECO-R1: the assemblies of a house (sessions convened, granted, refused, heavy terms, moods).
fn assembly_summary(state: &CampaignState, data: &GameData, id: &FactionId) -> serde_json::Value {
    let (mut convened, mut granted, mut refused, mut heavy, mut running) =
        (0u32, 0u32, 0u32, 0u32, 0i64);
    let mut moods = serde_json::Map::new();
    for assembly in data
        .assembly_rules
        .assemblies
        .iter()
        .filter(|a| a.house == id.as_str())
    {
        if let Some(rt) = state.assemblies.houses.get(&assembly.id) {
            convened += rt.convened;
            granted += rt.granted;
            refused += rt.refused;
            heavy += rt
                .sessions
                .iter()
                .filter(|s| s.concessions.len() >= 2)
                .count() as u32;
            running += rt
                .subsidies
                .iter()
                .map(|s| s.shares.iter().map(|x| x.amount).sum::<i64>())
                .sum::<i64>();
            moods.insert(assembly.id.clone(), json!(rt.mood));
        }
    }
    let permanent = state
        .assemblies
        .permanent
        .get(id)
        .cloned()
        .unwrap_or_default();
    json!({"convened": convened, "granted": granted, "refused": refused, "heavy": heavy, "running": running, "mood": moods, "permanent": permanent})
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let options = parse_options(&args);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _) = GameData::load(&root).expect("data");
    for &seed in &options.seeds {
        run_seed(&data, seed, &options, &mut |line| println!("{line}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "plays 4 turns of the real campaign, slow without --release"]
    fn probe_runs_four_turns() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
        let (data, _) = GameData::load(&root).expect("data");
        let options = Options {
            seeds: vec![1],
            turns: 4,
            passive: false,
            factions: vec!["fac_france".to_owned()],
        };
        let mut lines = Vec::new();
        run_seed(&data, 1, &options, &mut |line| lines.push(line));
        assert_eq!(lines.len(), 4);
        assert!(lines[3].contains("\"fac_france\""));
    }
}
