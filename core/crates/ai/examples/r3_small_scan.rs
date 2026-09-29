//! Lot OMR R3 (ADR 0117): opening structural margin of every faction holding
//! at most 2 provinces — income minus building upkeep minus the crown's share
//! of the cheapest capital garrison unit (the one the AI never dismisses).
//!
//! Usage: `cargo run --release -p ai --example r3_small_scan`.
use data_model::{FactionId, GameData};
use sim_campaign::economy::{garrison_share, unit_upkeep};
use sim_campaign::CampaignState;

fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _) = GameData::load(&root).expect("data");
    let idle = FactionId::new("fac_papacy").expect("papacy");
    let state = CampaignState::new_1337(&data, idle, 1).expect("setup");
    let mut rows = Vec::new();
    for id in state.factions.keys() {
        if id.as_str() == "fac_rebels" {
            continue;
        }
        let provinces = state.controlled_provinces(id).len();
        if provinces > 2 {
            continue;
        }
        let income = state.faction_income_effective(&data, id);
        let buildings = state.faction_building_upkeep(&data, id);
        let guard = state
            .faction_capital_city(id)
            .and_then(|city| state.settlements.get(city))
            .and_then(|s| {
                let cheapest = s.garrison.iter().map(|u| unit_upkeep(&data, u)).min()?;
                Some(garrison_share(&data, s.kind, true, [cheapest]))
            })
            .unwrap_or(0);
        rows.push((
            income - buildings - guard,
            id.clone(),
            provinces,
            income,
            buildings,
            guard,
        ));
    }
    rows.sort();
    for (margin, id, provinces, income, buildings, guard) in &rows {
        println!(
            "{margin:>6} {:<28} prov {provinces} income {income:>5} buildings {buildings:>4} guard {guard:>3}",
            id.as_str()
        );
    }
    let negative = rows.iter().filter(|r| r.0 < 0).count();
    println!("factions {} ; negative margin {negative}", rows.len());
}
