//! Plays 60 turns as France and prints the dynastic journal (M4 probe):
//! births, deaths, marriages, successions, regencies, acquired traits.
use data_model::{FactionId, GameData};
use sim_campaign::{CampaignState, EventKind};
use std::path::Path;

fn main() {
    let seed: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(7);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _) = GameData::load(&root).expect("data");
    let france = FactionId::new("fac_france").unwrap();
    let mut state = CampaignState::new_1337(&data, france.clone(), seed).expect("state");
    let mut counts = std::collections::BTreeMap::<String, u32>::new();
    for _ in 0..60 {
        let date = state.date_label();
        for event in state.end_turn(&data) {
            let dynastic = matches!(
                event.kind,
                EventKind::Birth
                    | EventKind::Death
                    | EventKind::Succession
                    | EventKind::Regency
                    | EventKind::NoHeir
                    | EventKind::TraitAcquired
            );
            if dynastic {
                *counts.entry(format!("{:?}", event.kind)).or_default() += 1;
                if event.faction.as_ref() == Some(&france)
                    || matches!(event.kind, EventKind::Succession)
                {
                    println!("{date:>16} {:?}: {}", event.kind, event.text_fr);
                }
            }
        }
    }
    let ruler = state.faction_state(&france).unwrap().ruler.clone();
    println!(
        "\nFrance ruler after 60 turns: {ruler:?} ({})",
        state.date_label()
    );
    println!("event counts (all factions): {counts:?}");
}
