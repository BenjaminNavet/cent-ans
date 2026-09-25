//! Lot EQ1: the opening economy of each playable faction when a human plays
//! it and gives no order (no disbanding, no recruitment): treasury, income
//! and every upkeep line for the first turns, plus the garrison bill by
//! settlement kind at turn 0.
//!
//! Usage: `cargo run --release -p ai --example start_economy_probe -- [turns] [faction...]`
//! (default: 8 turns; England, France, Burgundy, Scotland).
use data_model::{FactionId, GameData};
use sim_campaign::CampaignState;

fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _) = GameData::load(&root).expect("data");
    let args: Vec<String> = std::env::args().skip(1).collect();
    let turns: u32 = args.first().and_then(|a| a.parse().ok()).unwrap_or(8);
    let mut factions: Vec<String> = args.iter().skip(1).cloned().collect();
    if factions.is_empty() {
        factions = ["fac_england", "fac_france", "fac_burgundy", "fac_scotland"]
            .map(String::from)
            .to_vec();
    }
    for name in factions {
        let player = FactionId::new(&name).expect("faction id");
        let mut state = CampaignState::new_1337(&data, player.clone(), 1).expect("setup");
        state.interactive_battles = false;
        println!("== {name} (joueur inactif)");
        let mut by_kind: std::collections::BTreeMap<String, (usize, usize, i64)> =
            Default::default();
        for s in state
            .settlements
            .values()
            .filter(|s| s.controller == player)
        {
            let e = by_kind.entry(format!("{:?}", s.kind)).or_default();
            e.0 += 1;
            e.1 += s.garrison.len();
            e.2 += s
                .garrison
                .iter()
                .map(|u| sim_campaign::economy::unit_upkeep(&data, u))
                .sum::<i64>()
                * sim_campaign::economy::garrison_upkeep_percent(&data, s.kind)
                / 100;
        }
        println!("  garrisons by kind (places, units, upkeep): {by_kind:?}");
        let field: usize = state
            .armies
            .values()
            .filter(|a| a.faction == player)
            .map(|a| a.units.len())
            .sum();
        println!("  field units: {field}");
        // Tax and garrison bill per province (turn 0), largest tax first.
        let tax_rate = state.factions[&player].tax_rate;
        let tech = sim_campaign::research::faction_province_tech_effects(&state, &data, &player);
        let mut rows: Vec<(String, f64, i64)> = state
            .controlled_provinces(&player)
            .iter()
            .map(|p| {
                let tax: f64 = state
                    .settlements_of(p)
                    .filter(|(_, s)| s.controller == player)
                    .map(|(sid, _)| state.settlement_tax(&data, sid, tax_rate, &tech))
                    .sum();
                let garrison: i64 = state
                    .settlements_of(p)
                    .filter(|(_, s)| s.controller == player)
                    .map(|(_, s)| {
                        s.garrison
                            .iter()
                            .map(|u| sim_campaign::economy::unit_upkeep(&data, u))
                            .sum::<i64>()
                            * sim_campaign::economy::garrison_upkeep_percent(&data, s.kind)
                            / 100
                    })
                    .sum();
                (p.as_str().to_string(), tax, garrison)
            })
            .collect();
        rows.sort_by(|a, b| b.1.total_cmp(&a.1));
        println!("  provinces: {}", rows.len());
        for (p, tax, garrison) in &rows {
            println!("    {p:<28} tax {tax:>7.0} garrison {garrison:>5}");
        }
        for turn in 0..=turns {
            let e = state.faction_economy(&data, &player).expect("economy");
            println!(
                "  t{turn} treasury {} income {} trade {} army {} buildings {} admin {} table {} net {} tax {:?}",
                e.treasury,
                e.projected_income,
                e.trade_income,
                e.army_upkeep,
                e.building_upkeep,
                e.administration_upkeep,
                e.table_upkeep,
                e.projected_income + e.trade_income
                    - e.army_upkeep
                    - e.building_upkeep
                    - e.administration_upkeep
                    - e.table_upkeep,
                e.tax_rate,
            );
            if turn < turns {
                let events = state.end_turn_with(&data, ai::plan_turn);
                for ev in events.iter().filter(|ev| {
                    ev.kind == sim_campaign::EventKind::Bankruptcy
                        && ev.faction.as_ref() == Some(&player)
                }) {
                    println!("    ! {}", ev.text_fr);
                }
            }
        }
    }
}
