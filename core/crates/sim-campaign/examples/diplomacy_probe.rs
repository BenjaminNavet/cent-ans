//! Plays up to 170 turns (spring 1337 → 1379) and prints the diplomatic and
//! religious journal (M5 probe). Usage: `diplomacy_probe [seed] [turns]`.
use data_model::{FactionId, GameData};
use sim_campaign::{CampaignState, EventKind};
use std::path::Path;

fn main() {
    let mut args = std::env::args().skip(1);
    let seed: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(7);
    let turns: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(170);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _) = GameData::load(&root).expect("data");
    let france = FactionId::new("fac_france").unwrap();
    let mut state = CampaignState::new_1337(&data, france.clone(), seed).expect("state");
    let mut counts = std::collections::BTreeMap::<String, u32>::new();
    for _ in 0..turns {
        let date = state.date_label();
        // The player (France) accepts every offer, to exercise the flow.
        let offers: Vec<u32> = state.factions[&france]
            .offers
            .iter()
            .map(|o| o.id)
            .collect();
        for offer in offers {
            let _ = state.submit_order(
                &data,
                sim_campaign::Order::AnswerOffer {
                    offer,
                    accept: true,
                },
            );
        }
        for event in state.end_turn(&data) {
            let shown = matches!(
                event.kind,
                EventKind::WarDeclared
                    | EventKind::PeaceSigned
                    | EventKind::AllianceFormed
                    | EventKind::AllianceBroken
                    | EventKind::Vassalage
                    | EventKind::VassalRebellion
                    | EventKind::Embargo
                    | EventKind::DiplomaticOffer
                    | EventKind::Excommunication
                    | EventKind::Schism
                    | EventKind::Heresy
                    | EventKind::Succession
                    | EventKind::FactionDestroyed
            );
            if shown {
                *counts.entry(format!("{:?}", event.kind)).or_default() += 1;
                println!("{date:>16} {:?}: {}", event.kind, event.text_fr);
            }
        }
    }
    println!("\n{} — events: {counts:?}", state.date_label());
    for (id, f) in &state.factions {
        if f.alive {
            println!(
                "{id}: war {:?} allies {:?} religion {:?} favor {} provinces {}",
                f.at_war_with,
                f.allies,
                f.religion,
                f.papal_favor,
                state
                    .provinces
                    .values()
                    .filter(|p| &p.controller == id)
                    .count()
            );
        }
    }
}
