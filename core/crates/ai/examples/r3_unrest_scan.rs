//! Lot OMR R3: plays `turns` AI turns then breaks down the unrest of the
//! provinces whose weighted unrest exceeds `threshold`.
//!
//! Usage: `r3_unrest_scan [turns] [seed] [threshold]`.
use data_model::{FactionId, GameData, SocialClass};
use sim_campaign::CampaignState;

fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _) = GameData::load(&root).expect("data");
    let args: Vec<String> = std::env::args().skip(1).collect();
    let turns: u32 = args.first().and_then(|a| a.parse().ok()).unwrap_or(20);
    let seed: u64 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(1);
    let threshold: f64 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(60.0);
    let idle = FactionId::new("fac_papacy").expect("papacy");
    let mut state = CampaignState::new_1337(&data, idle, seed).expect("setup");
    state.interactive_battles = false;
    // `TRACE_PROV=prov_syrte`: that province's peasants, target and events every turn.
    let traced = std::env::var("TRACE_PROV").ok();
    for _ in 0..turns {
        let incite = std::env::var("INCITE").is_ok();
        let events = state.end_turn_with(&data, |s, d, f| {
            let orders = ai::plan_turn(s, d, f);
            if incite {
                for order in &orders {
                    let text = format!("{order:?}");
                    if text.contains("Incite") {
                        println!("t{} {} {text}", s.turn, f.as_str());
                    }
                }
            }
            orders
        });
        let Some(traced) = &traced else { continue };
        let id = data_model::ProvinceId::new(traced).expect("province id");
        let p = &state.provinces[&id];
        let eq = sim_campaign::population::equilibrium(&state, &data, &id).unwrap_or_default();
        let target = eq
            .iter()
            .find(|(c, _, _)| *c == SocialClass::Peasants)
            .map_or(0.0, |(_, _, t)| t.unrest);
        println!(
            "t{} peasants u{} target {target:.0} dis {} dev {} count {}",
            state.turn,
            p.population.peasants.unrest,
            p.unrest,
            p.devastation,
            p.population.peasants.count
        );
        let name = &data.provinces[&id].name.display;
        for e in events
            .iter()
            .filter(|e| e.province.as_ref() == Some(&id) || e.text_fr.contains(name.as_str()))
        {
            println!("    {:?}: {}", e.kind, e.text_fr);
        }
    }
    let mut hot = 0;
    for (id, p) in &state.provinces {
        let pop = &p.population;
        let weighted = SocialClass::ALL
            .iter()
            .map(|c| f64::from(pop.get(*c).unrest) * pop.get(*c).count as f64)
            .sum::<f64>()
            / pop.total().max(1) as f64;
        if weighted < threshold {
            continue;
        }
        hot += 1;
        let city = &state.settlements[&p.city];
        let ctrl = &city.controller;
        let f = &state.factions[ctrl];
        let eq = sim_campaign::population::equilibrium(&state, &data, id).unwrap_or_default();
        let peasants = eq.iter().find(|(c, _, _)| *c == SocialClass::Peasants);
        println!(
            "{:<22} w{weighted:>3.0} ctrl {:<20} own {:<20} tax {:?} price {} pol {:.0} dis {} dev {} gar {} eq_peasant {:.0} goods {:.0} health {:.0} bld {:?}",
            id.as_str(),
            ctrl.as_str(),
            city.owner.as_str(),
            f.tax_rate,
            f.price_level,
            state.political_unrest(id),
            p.unrest,
            p.devastation,
            state.weighted_garrison_strength(&data, id),
            peasants.map_or(0.0, |(_, _, t)| t.unrest),
            peasants.map_or(0.0, |(_, _, t)| t.goods_satisfaction),
            peasants.map_or(0.0, |(_, _, t)| t.health),
            city.buildings.iter().map(|b| b.as_str()).collect::<Vec<_>>(),
        );
    }
    println!("hot provinces: {hot} / {}", state.provinces.len());
}
