//! The phased auto-resolve agrees with the 3D battle simulation.
//!
//! `tests/fixtures/auto_resolve_scenarios.json` holds the matchups and the
//! result of each in `sim-battle` (AI against AI, odd number of seeds), produced by
//! `tools/heavy.sh cargo run --release -p sim-battle --example battle_bench -- --emit-auto-fixture`
//! (BANC-2, ADR 0800). The fixture carries a `data_hash` of the unit and battle
//! data it was played with: the test fails when the data changed since, so the
//! references can never silently go stale again. The auto-resolve must name the
//! same winner (majority of its draws) in at least 80 % of the scenarios.

use std::path::{Path, PathBuf};

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
    data_hash: String,
    scenarios: Vec<Scenario>,
}

/// FNV-1a 64 of `unit_types/*.json` and `rules/battle_*.json` (names sorted, each
/// hashed as `name 0x00 length bytes`). Same algorithm as `battle_bench::data_hash`.
fn data_hash(data_dir: &Path) -> String {
    let mut files: Vec<PathBuf> = Vec::new();
    for (sub, prefix) in [("unit_types", ""), ("rules", "battle_")] {
        for entry in std::fs::read_dir(data_dir.join(sub)).expect("data dir") {
            let path = entry.expect("dir entry").path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with(prefix) && name.ends_with(".json") {
                files.push(path);
            }
        }
    }
    files.sort();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |bytes: &[u8]| {
        for b in bytes {
            hash ^= u64::from(*b);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    };
    for path in &files {
        let rel = path
            .strip_prefix(data_dir)
            .expect("under data dir")
            .to_string_lossy()
            .replace('\\', "/");
        let bytes = std::fs::read(path).expect("data file");
        eat(rel.as_bytes());
        eat(&[0]);
        eat(&(bytes.len() as u64).to_le_bytes());
        eat(&bytes);
    }
    format!("{hash:016x}")
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
    let current = data_hash(&root.join("../../../data"));
    assert_eq!(
        fixture.data_hash, current,
        "the 3D references of auto_resolve_scenarios.json are stale (unit or battle data changed). \
         Regenerate them: cd core && ../tools/heavy.sh cargo run --release -p sim-battle \
         --example battle_bench -- --emit-auto-fixture"
    );
    let mut disagreements = Vec::new();
    for scenario in &fixture.scenarios {
        let (attacker, attacker_profiles) = army(&data, &scenario.attacker);
        let (defender, defender_profiles) = army(&data, &scenario.defender);
        let context = BattleContext {
            defender_terrain_bonus: false,
            river_crossing: scenario.river,
            walls: false,
            assault_bonus_percent: 0,
            wall: Default::default(),
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
    eprintln!(
        "auto-resolve vs 3D agreement: {:.1} % ({} disagreements out of {}): {disagreements:#?}",
        agreement * 100.0,
        disagreements.len(),
        fixture.scenarios.len()
    );
    assert!(
        agreement >= 0.80,
        "agreement {:.0} % < 80 %: {disagreements:#?}",
        agreement * 100.0
    );
}
