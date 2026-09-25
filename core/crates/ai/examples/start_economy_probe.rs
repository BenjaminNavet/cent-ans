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
        // `AI=<seed>`: the faction is played by the AI (the Papacy is the
        // idle player) and every `STEP` turns are printed.
        let ai_seed: Option<u64> = std::env::var("AI").ok().and_then(|s| s.parse().ok());
        let step: u32 = std::env::var("STEP")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        let human = if ai_seed.is_some() {
            FactionId::new("fac_papacy").expect("papacy")
        } else {
            player.clone()
        };
        let mut state = CampaignState::new_1337(&data, human, ai_seed.unwrap_or(1)).expect("setup");
        state.interactive_battles = false;
        println!(
            "== {name} ({})",
            if ai_seed.is_some() {
                "IA"
            } else {
                "joueur inactif"
            }
        );
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
            if turn % step != 0 {
                // Orders of the watched faction, printed when its treasury
                // turns negative (`AI` mode).
                let before = state.factions[&player].treasury;
                let logged = std::cell::RefCell::new(Vec::new());
                let planner = |s: &CampaignState, d: &GameData, f: &FactionId| {
                    let orders = ai::plan_turn(s, d, f);
                    if f == &player {
                        logged.borrow_mut().extend(orders.iter().cloned());
                    }
                    orders
                };
                let events = state.end_turn_with(&data, planner);
                let after = state.factions[&player].treasury;
                if after < 0 && before >= 0 {
                    let f = &state.factions[&player];
                    let tributes: Vec<_> = f
                        .ledger
                        .tributes
                        .iter()
                        .map(|t| (t.to.as_str().to_owned(), t.per_season))
                        .collect();
                    println!(
                        "  t{turn} {before} -> {after} (income {} upkeep {} trade {}, other {}) tributes {tributes:?} suzerain {:?}",
                        f.income_last_turn,
                        f.upkeep_last_turn,
                        f.trade_income_last_turn,
                        after - before - f.income_last_turn + f.upkeep_last_turn
                            - f.trade_income_last_turn,
                        f.suzerain
                    );
                    for order in logged.borrow().iter().filter(|o| {
                        !matches!(
                            o,
                            sim_campaign::Order::MoveArmy { .. }
                                | sim_campaign::Order::SetStance { .. }
                                | sim_campaign::Order::MoveAgent { .. }
                        )
                    }) {
                        println!("    order {order:?}");
                    }
                    for ev in events
                        .iter()
                        .filter(|ev| ev.faction.as_ref() == Some(&player))
                    {
                        println!("    event {:?} {}", ev.kind, ev.text_fr);
                    }
                }
                continue;
            }
            let units: usize = state
                .armies
                .values()
                .filter(|a| a.faction == player)
                .map(|a| a.units.len())
                .sum::<usize>();
            let garrison_units: usize = state
                .settlements
                .values()
                .filter(|s| s.controller == player)
                .map(|s| s.garrison.len())
                .sum();
            println!(
                "  t{turn} field units {units} garrison units {garrison_units} provinces {} war {}",
                state.controlled_provinces(&player).len(),
                state.factions[&player].at_war_with.len()
            );
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
