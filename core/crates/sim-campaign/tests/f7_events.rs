//! F7b (« Chronique enrichie ») integration tests: the new historical events
//! and their chains fire in a simulated 1337-1453 campaign (fixed seed,
//! built-in AI for every faction, the player's decisions taken by the AI
//! weights). See `docs/design/m10-events.md` § F7b.

use std::collections::BTreeMap;
use std::path::PathBuf;

use data_model::{EventCategory, EventId, FactionId, GameData};
use sim_campaign::{ai_minimal, chronicle, CampaignState, Order};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

/// Runs a campaign from 1337 to the end of 1453 and returns the year each
/// event first fired.
fn simulate(data: &GameData, player: &str, seed: u64) -> BTreeMap<EventId, i32> {
    let player = fac(player);
    let mut state = CampaignState::new_1337(data, player.clone(), seed).expect("1337 start");
    let mut fired: BTreeMap<EventId, i32> = BTreeMap::new();
    while state.year <= 1453 {
        // The player plays like an AI faction: same planner, AI choices.
        let pending: Vec<(u32, EventId)> = state
            .chronicle
            .pending_decisions
            .iter()
            .filter(|d| d.faction == player)
            .map(|d| (d.id, d.event.clone()))
            .collect();
        for (decision, event) in pending {
            let option = chronicle::ai_choice(&mut state, &data.events[&event]);
            let _ = state.submit_order(data, Order::ChooseEventOption { decision, option });
        }
        if state.factions[&player].alive {
            for order in ai_minimal::plan_turn(&state, data, &player) {
                let _ = state.submit_order(data, order);
            }
        }
        state.end_turn(data);
        if std::env::var("F7_TRACE").is_ok() && state.season == sim_campaign::Season::Spring {
            let wars: Vec<String> = state
                .factions
                .iter()
                .filter(|(_, f)| f.alive)
                .flat_map(|(id, f)| {
                    f.at_war_with
                        .iter()
                        .filter(move |e| id.as_str() < e.as_str())
                        .map(move |e| format!("{}-{}", &id.as_str()[4..], &e.as_str()[4..]))
                })
                .collect();
            let dead: Vec<&str> = state
                .factions
                .iter()
                .filter(|(_, f)| !f.alive)
                .map(|(id, _)| &id.as_str()[4..])
                .collect();
            let rulers: Vec<String> = ["fac_france", "fac_england", "fac_burgundy", "fac_scotland"]
                .iter()
                .map(|f| format!("{:?}", state.factions[&fac(f)].ruler))
                .collect();
            println!("{} wars {wars:?} dead {dead:?} rulers {rulers:?}", state.year);
        }
        for event in &state.chronicle.fired_events {
            fired.entry(event.clone()).or_insert(state.year);
        }
    }
    fired
}

/// Events added by F7b (listed in `docs/design/m10-events.md` § F7b).
const F7_HISTORICAL: &[&str] = &[];

#[test]
fn f7_events_load_and_are_historical_or_chained() {
    let data = data();
    for id in F7_HISTORICAL {
        let event = &data.events[&EventId::new(*id).unwrap()];
        assert_eq!(event.kind, EventCategory::Historical, "{id}");
    }
}

#[test]
#[ignore = "exploration: cargo test --release -- --ignored --nocapture"]
fn print_campaign_chronicle() {
    let data = data();
    let fired = simulate(&data, "fac_france", 1337);
    let mut by_year: Vec<(i32, &EventId)> = fired.iter().map(|(e, y)| (*y, e)).collect();
    by_year.sort();
    for (year, event) in by_year {
        println!("{year} {event}");
    }
    let missing: Vec<&EventId> = data
        .events
        .iter()
        .filter(|(id, e)| e.kind != EventCategory::Random && !fired.contains_key(*id))
        .map(|(id, _)| id)
        .collect();
    println!("not fired: {missing:?}");
}
