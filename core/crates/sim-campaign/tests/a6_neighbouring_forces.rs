//! A6-L1: nearly equal forces give an uncertain auto-resolution (35-65 %),
//! the odds the pre-battle forecast announces (ADR 0181).

use std::path::PathBuf;

use data_model::{GameData, Terrain, UnitCategory, UnitTypeId};
use sim_campaign::{
    resolve_with, BattleContext, BattleUnit, CampaignRng, FieldConditions, Season, Side,
    UnitProfile, Winner,
};

fn army(data: &GameData, comp: &[(&str, usize)], scale: f64) -> (Side, Vec<UnitProfile>) {
    let mut side = Side::default();
    let mut profiles = Vec::new();
    for (id, count) in comp {
        let t = &data.unit_types[&UnitTypeId::new(*id).expect("unit id")];
        for _ in 0..*count {
            let strength = (f64::from(t.soldiers) * scale).round() as u32;
            side.units.push(BattleUnit {
                strength,
                max_strength: strength,
                experience: 0,
                morale: t.stats.morale,
                melee: t.stats.melee,
                ranged: t.stats.ranged,
                armor: t.stats.armor,
                is_ranged: matches!(t.category, UnitCategory::Ranged | UnitCategory::Siege),
            });
            profiles.push(UnitProfile::of(t));
        }
    }
    (side, profiles)
}

fn win_rate(data: &GameData, comp: &[(&str, usize)], attacker_scale: f64) -> f64 {
    let (attacker, ap) = army(data, comp, attacker_scale);
    let (defender, dp) = army(data, comp, 1.0);
    let conditions = FieldConditions {
        terrain: Some(Terrain::Plains),
        season: Some(Season::Summer),
        weather: None,
    };
    let seeds = 400;
    let wins = (0..seeds)
        .filter(|seed| {
            resolve_with(
                &attacker,
                &ap,
                &defender,
                &dp,
                &BattleContext::default(),
                &conditions,
                &data.auto_resolve,
                &mut CampaignRng::from_seed(*seed),
            )
            .winner
                == Winner::Attacker
        })
        .count();
    wins as f64 / seeds as f64
}

#[test]
fn nearly_equal_forces_give_an_uncertain_battle() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let data = GameData::load(&root).expect("data").0;
    let compositions: [&[(&str, usize)]; 3] = [
        &[("unit_men_at_arms_foot", 4), ("unit_longbowmen", 4)],
        &[
            ("unit_knights", 3),
            ("unit_crossbowmen", 3),
            ("unit_urban_militia", 4),
        ],
        &[("unit_flemish_pikemen", 5), ("unit_genoese_crossbowmen", 3)],
    ];
    let mut misses = Vec::new();
    for comp in compositions {
        for (scale, low, high) in [(0.85, 0.1, 0.5), (1.0, 0.35, 0.65), (1.15, 0.5, 0.9)] {
            let rate = win_rate(&data, comp, scale);
            eprintln!("{comp:?} x{scale}: {:.0} %", rate * 100.0);
            if !(low..=high).contains(&rate) {
                misses.push(format!("{comp:?} x{scale}: {:.0} %", rate * 100.0));
            }
        }
    }
    assert!(misses.is_empty(), "{misses:#?}");
}
