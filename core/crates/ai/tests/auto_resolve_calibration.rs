//! Lot N1: the phased auto-resolve agrees with the 3D battle simulation.
//!
//! `tests/fixtures/auto_resolve_scenarios.json` holds 20 matchups and the
//! result of each in `sim-battle` (AI against AI, 6 seeds), produced by
//! `RT_WRITE=1 cargo run --release -p ai --example balance_probe -- rt`.
//! The auto-resolve must name the same winner (majority of its draws) in at
//! least 80 % of them.

use std::path::Path;

use data_model::{GameData, Terrain, UnitCategory, UnitTypeId};
use serde::Deserialize;
use sim_battle::BattleSeason;
use sim_campaign::{
    resolve_with, BattleContext, BattleUnit, CampaignRng, FieldConditions, Season, Side,
    UnitProfile, Winner,
};

#[derive(Deserialize)]
struct Reference3d {
    runs: u32,
    attacker_wins: u32,
}

#[derive(Deserialize)]
struct Scenario {
    name: String,
    attacker: Vec<(String, usize)>,
    defender: Vec<(String, usize)>,
    terrain: Terrain,
    #[serde(default)]
    season: BattleSeason,
    #[serde(default)]
    river: bool,
    reference_3d: Reference3d,
}

#[derive(Deserialize)]
struct Fixture {
    scenarios: Vec<Scenario>,
}

fn army(data: &GameData, comp: &[(String, usize)]) -> (Side, Vec<UnitProfile>) {
    let mut side = Side::default();
    let mut profiles = Vec::new();
    for (id, count) in comp {
        let t = &data.unit_types[&UnitTypeId::new(id).expect("unit id")];
        for _ in 0..*count {
            side.units.push(BattleUnit {
                strength: t.soldiers,
                max_strength: t.soldiers,
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

#[test]
fn auto_resolve_agrees_with_3d_battles() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let (data, _) = GameData::load(&root.join("../../../data")).expect("data");
    let text = std::fs::read_to_string(root.join("tests/fixtures/auto_resolve_scenarios.json"))
        .expect("fixture");
    let fixture: Fixture = serde_json::from_str(&text).expect("fixture json");
    assert!(fixture.scenarios.len() >= 20);
    let mut disagreements = Vec::new();
    for scenario in &fixture.scenarios {
        let (attacker, attacker_profiles) = army(&data, &scenario.attacker);
        let (defender, defender_profiles) = army(&data, &scenario.defender);
        let context = BattleContext {
            defender_terrain_bonus: false,
            river_crossing: scenario.river,
            walls: false,
            assault_bonus_percent: 0,
            crossing: None,
        };
        let conditions = FieldConditions {
            terrain: Some(scenario.terrain),
            season: Some(match scenario.season {
                BattleSeason::Spring => Season::Spring,
                BattleSeason::Summer => Season::Summer,
                BattleSeason::Autumn => Season::Autumn,
                BattleSeason::Winter => Season::Winter,
            }),
            weather: None,
        };
        let draws = 60;
        let wins = (0..draws)
            .filter(|seed| {
                resolve_with(
                    &attacker,
                    &attacker_profiles,
                    &defender,
                    &defender_profiles,
                    &context,
                    &conditions,
                    &data.auto_resolve,
                    &mut CampaignRng::from_seed(*seed),
                )
                .winner
                    == Winner::Attacker
            })
            .count();
        let auto_attacker = 2 * wins > draws as usize;
        let rt = &scenario.reference_3d;
        let rt_attacker = 2 * rt.attacker_wins > rt.runs;
        if auto_attacker != rt_attacker {
            disagreements.push(format!(
                "{}: auto {wins}/{draws}, 3D {}/{}",
                scenario.name, rt.attacker_wins, rt.runs
            ));
        }
    }
    let agreement = 1.0 - disagreements.len() as f64 / fixture.scenarios.len() as f64;
    assert!(
        agreement >= 0.8,
        "agreement {:.0} % < 80 %: {disagreements:#?}",
        agreement * 100.0
    );
}
