//! Lot JR4b: starting budget of every faction (receipts, upkeep, balance),
//! sorted by relative balance. `cargo test -p sim-campaign --test
//! jr4b_budget_probe -- --ignored --nocapture`.

use std::path::PathBuf;

use data_model::{FactionId, GameData};
use sim_campaign::CampaignState;

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

#[test]
#[ignore = "diagnostic table of the starting budgets"]
fn starting_budgets() {
    let data = data();
    let state =
        CampaignState::new_1337(&data, FactionId::new("fac_france").unwrap(), 1).expect("start");
    let mut rows = Vec::new();
    for (id, f) in state.factions.iter().filter(|(_, f)| f.alive) {
        let Some(e) = state.faction_economy(&data, id) else {
            continue;
        };
        let receipts = e.projected_income + e.trade_income;
        let field: i64 = state
            .armies
            .values()
            .filter(|a| &a.faction == id)
            .map(|a| sim_campaign::economy::units_upkeep(&data, &a.units))
            .sum();
        let garrisons = e.army_upkeep - field;
        let places = state
            .settlements
            .values()
            .filter(|s| &s.controller == id)
            .count();
        let provinces = state
            .provinces
            .keys()
            .filter(|p| state.controls_province(id, p))
            .count();
        // The idle hoard's share of the administration is no structural
        // cost: it melts with the treasury.
        let rules = &data.economy_rules;
        let income = state.faction_income_effective(&data, id);
        let opulence = (f.treasury - rules.opulence_seasons * income.max(0)).max(0)
            * rules.opulence_percent
            / 100;
        let net = e.net_income() + opulence;
        let rel = net as f64 / receipts.max(1) as f64;
        rows.push((
            rel,
            id.clone(),
            receipts,
            field,
            garrisons,
            e.building_upkeep,
            e.administration_upkeep,
            net,
            places,
            provinces,
            f.treasury,
        ));
    }
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    println!("faction | receipts | field | garrisons | buildings | admin (with hoard) | structural net | rel | places | provinces | treasury");
    for (rel, id, r, fi, g, b, a, n, pl, pr, t) in rows {
        println!(
            "{id} | {r} | {fi} | {g} | {b} | {a} | {n} | {:.0}% | {pl} | {pr} | {t}",
            rel * 100.0
        );
    }
}
