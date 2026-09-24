//! Follows the France-England war (score, occupation, peace offers) under the
//! strategic AI. Usage: `war_probe [seed] [turns]`.
use std::path::Path;

use data_model::{FactionId, GameData};
use sim_campaign::{CampaignState, EventKind};

fn main() {
    let mut args = std::env::args().skip(1);
    let seed: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(7);
    let turns: u32 = args.next().and_then(|s| s.parse().ok()).unwrap_or(80);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _) = GameData::load(&root).expect("data");
    let france = FactionId::new("fac_france").unwrap();
    let england = FactionId::new("fac_england").unwrap();
    // Neither side is the player: Burgundy watches.
    let mut state =
        CampaignState::new_1337(&data, FactionId::new("fac_papacy").unwrap(), seed).unwrap();
    for turn in 0..turns {
        let events = state.end_turn_with(&data, ai::plan_turn);
        if turn % 8 == 0 || events.iter().any(|e| e.kind == EventKind::PeaceSigned) {
            let at_war = state.is_at_war(&france, &england);
            println!(
                "{:>16} war {at_war} score FR {:+} / EN {:+} | FR holds {} EN provinces, EN holds {} FR",
                state.date_label(),
                state.war_score(&data, &france, &england),
                state.war_score(&data, &england, &france),
                state.provinces.keys().filter(|p| state.province_owner(p) == Some(&england) && state.controls_province(&france, p)).count(),
                state.provinces.keys().filter(|p| state.province_owner(p) == Some(&france) && state.controls_province(&england, p)).count(),
            );
        }
        for e in events
            .iter()
            .filter(|e| e.kind == EventKind::PeaceSigned || e.kind == EventKind::WarDeclared)
        {
            println!("    {}", e.text_fr);
        }
    }
}
