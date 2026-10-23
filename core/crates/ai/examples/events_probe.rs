//! EVENTS (ADR 0779): which share of the conditioned history events a whole
//! campaign played by the strategic AI really draws.
//!
//! `cargo run --release -p ai --example events_probe -- [--seeds 1,2,3] [--players fac_france,fac_england]`
//!
//! The AI plays every faction, the player's included (its event decisions
//! expire onto the AI weights). Prints, per run, the history events fired
//! with the year and the England-France land-battle balance, then the share
//! of the list drawn over all runs.

use std::collections::BTreeMap;

use data_model::{FactionId, GameData};
use sim_campaign::CampaignState;

/// The events this lot made conditional or added (see `docs/wip/events-histoire-conditionnee.md`).
const HISTORY: &[&str] = &[
    "evt_bretigny",
    "evt_bretigny_angleterre",
    "evt_bretigny_paix_blanche",
    "evt_bretigny_paix_blanche_anglaise",
    "evt_bretigny_angleterre_a_bout",
    "evt_bretigny_roi_sans_argent",
    "evt_poitiers",
    "evt_poitiers_victoire_francaise",
    "evt_maupertuis_sans_bataille",
    "evt_crecy",
    "evt_crecy_triomphe_anglais",
    "evt_crecy_revers_anglais",
    "evt_crecy_victoire_francaise",
    "evt_crecy_chevauchee_sans_bataille",
    "evt_jeanne_d_arc",
    "evt_sursaut_national",
    "evt_mariage_de_jean_de_gand",
    "evt_expedition_de_galice",
    "evt_traite_de_bayonne",
    "evt_castille_menacee",
    "evt_hommage_de_guerande",
    "evt_confiscation_de_la_bretagne",
    "evt_fin_d_etienne_marcel",
    "evt_ordonnances_de_charles_v",
];

fn list_arg(name: &str, default: &str) -> Vec<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map_or(default, String::as_str)
        .split(',')
        .map(str::to_owned)
        .collect()
}

fn main() {
    let seeds: Vec<u64> = list_arg("--seeds", "1,2,3")
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect();
    let players = list_arg("--players", "fac_france,fac_england");
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _) = GameData::load(&root).expect("data");
    let mut drawn: BTreeMap<&str, u32> = BTreeMap::new();
    let mut runs = 0u32;
    for player in &players {
        for seed in &seeds {
            runs += 1;
            let hits = play(&data, player, *seed);
            for id in hits {
                *drawn.entry(id).or_default() += 1;
            }
        }
    }
    println!("--- share drawn over {runs} runs");
    let mut total = 0;
    for id in HISTORY {
        let n = drawn.get(id).copied().unwrap_or(0);
        total += n;
        println!("{id}: {n}/{runs}");
    }
    println!(
        "mean events per run: {:.1} of {}",
        f64::from(total) / f64::from(runs),
        HISTORY.len()
    );
}

fn play(data: &GameData, player: &str, seed: u64) -> Vec<&'static str> {
    let player = FactionId::new(player).expect("faction");
    let mut state = CampaignState::new_1337(data, player.clone(), seed).expect("state");
    let mut first_year: BTreeMap<&'static str, i32> = BTreeMap::new();
    while state.year <= 1453 {
        for offer in state.factions[&player].offers.clone() {
            let accept = sim_campaign::negotiation::evaluate_treaty(
                &state,
                data,
                &offer.from,
                &player,
                &offer.proposal.articles,
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
        if state.factions[&player].alive {
            for order in ai::plan_turn(&state, data, &player) {
                let _ = state.submit_order(data, order);
            }
        }
        state.end_turn_with(data, ai::plan_turn);
        for id in HISTORY {
            if state
                .chronicle
                .fired_events
                .contains(&data_model::EventId::new(*id).expect("id"))
            {
                first_year.entry(id).or_insert(state.year);
            }
        }
    }
    let france = FactionId::new("fac_france").expect("id");
    let england = FactionId::new("fac_england").expect("id");
    let line: Vec<String> = first_year
        .iter()
        .map(|(id, y)| format!("{}@{y}", &id[4..]))
        .collect();
    println!(
        "{player} seed {seed}: {}/{} | battles E>F {} F>E {} | {}",
        first_year.len(),
        HISTORY.len(),
        state.chronicle.victories(&england, &france),
        state.chronicle.victories(&france, &england),
        line.join(" ")
    );
    first_year.keys().copied().collect()
}
