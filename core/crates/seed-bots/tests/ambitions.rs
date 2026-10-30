//! R1-2 acceptance: every ambition of a written house changes the campaign. For each ambition
//! of the eight written houses, 40 turns from the opening of its act (the strategic AI at
//! the player's seat, the minimal AI elsewhere) move at least one probe figure by more than 10 %
//! against the same campaign without any ambition. Renown is left out: it is what an ambition
//! earns, not what it changes.

use std::collections::BTreeMap;

use seed_bots::timeline::{self, ProbeConfig, Profile};

const TURNS: u32 = 40;
const HOUSES: [&str; 8] = [
    "fac_france",
    "fac_england",
    "fac_burgundy",
    "fac_brittany",
    "fac_navarre",
    "fac_scotland",
    "fac_flanders",
    "fac_castile",
];
const FIGURES: [&str; 6] = [
    "treasury_end",
    "treasury_mean",
    "prestige_end",
    "prestige_mean",
    "field_men_mean",
    "provinces_end",
];

fn config(faction: &str, start_year: Option<i32>, ambition: Option<&str>) -> ProbeConfig {
    let mut config = ProbeConfig::new(faction, 1, Profile::Ai);
    config.turns = TURNS;
    config.start_year = start_year;
    config.minimal_others = true;
    config.auto_ambitions = false;
    config.ambitions = ambition.map(|a| vec![a.to_owned()]).unwrap_or_default();
    config
}

fn figures(config: &ProbeConfig) -> BTreeMap<&'static str, f64> {
    let data = data_model::test_support::game_data();
    timeline::run(data, config).summary()
}

fn moved(base: f64, with: f64) -> f64 {
    (with - base).abs() / base.abs().max(1.0)
}

#[test]
fn every_ambition_changes_the_campaign() {
    let data = data_model::test_support::game_data();
    let rules = &data.acts_rules;
    let starts = [None, Some(1361), Some(1416)];
    let mut runs: Vec<(String, String, ProbeConfig)> = Vec::new();
    for house in HOUSES {
        let key = house.to_owned();
        for (index, act) in rules.acts.iter().enumerate() {
            // Flanders (one province) is wiped out when the probe jumps its clock to 1416: no
            // figure can move, so its third act cannot be probed this way.
            if house == "fac_flanders" && index == 2 {
                continue;
            }
            let set = rules.ambition_sets.get(&key).and_then(|s| s.get(&act.id));
            for ambition in set.into_iter().flatten() {
                runs.push((
                    house.to_owned(),
                    ambition.id.clone(),
                    config(house, starts[index], Some(&ambition.id)),
                ));
            }
            runs.push((
                house.to_owned(),
                String::new(),
                config(house, starts[index], None),
            ));
        }
    }
    let results: Vec<BTreeMap<&'static str, f64>> = std::thread::scope(|scope| {
        let handles: Vec<_> = runs
            .iter()
            .map(|(_, _, config)| scope.spawn(move || figures(config)))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let mut failures = Vec::new();
    let mut base: Option<&BTreeMap<&'static str, f64>> = None;
    // Baselines come after their act's ambitions: walk backwards.
    for ((house, id, _), result) in runs.iter().zip(&results).rev() {
        if id.is_empty() {
            base = Some(result);
            continue;
        }
        let base = base.expect("baseline");
        let best = FIGURES
            .iter()
            .map(|f| (*f, moved(base[f], result[f])))
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap();
        eprintln!("{house} {id}: {} {:.1} %", best.0, best.1 * 100.0);
        if best.1 <= 0.10 {
            failures.push(format!("{house} {id} ({} {:.1} %)", best.0, best.1 * 100.0));
        }
    }
    assert!(
        failures.is_empty(),
        "ambitions without effect: {failures:?}"
    );
}
