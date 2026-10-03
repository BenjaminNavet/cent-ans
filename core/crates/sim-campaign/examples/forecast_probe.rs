//! LR-13 probe: pre-battle forecast vs auto-resolver vs 3D battle.
//!
//! For a sample of compositions, prints the attacker's win chance as:
//! - `old`: the pre-N1 forecast (`side_power` + one ±10 % roll);
//! - `forecast`: [`sim_campaign::battle_forecast::forecast_sides`] (the
//!   pre-battle screen's number since LR-13);
//! - `auto`: the frequency of attacker wins of the N1 auto-resolver over
//!   `AUTO` seeds (default 400, seeds disjoint from the forecast's);
//! - `3d`: the frequency of attacker wins of the 3D battle, AI against AI,
//!   over `SIM3D` seeds (default 0: skipped, it is slow).
//!
//! `cargo run --release -p sim-campaign --example forecast_probe`

use std::path::PathBuf;

use data_model::{GameData, Terrain, UnitCategory, UnitTypeId};
use sim_campaign::battle_auto::{effective_armor, side_power};
use sim_campaign::battle_forecast::{forecast_sides, win_chance};
use sim_campaign::rng::CampaignRng;
use sim_campaign::state::Season;
use sim_campaign::{
    resolve_with_crossings, BattleContext, BattleUnit, FieldConditions, Side, UnitProfile, Winner,
};

struct Army {
    side: Side,
    profiles: Vec<UnitProfile>,
    ids: Vec<String>,
}

fn army(data: &GameData, spec: &[(&str, usize)]) -> Army {
    let mut army = Army {
        side: Side::default(),
        profiles: Vec::new(),
        ids: Vec::new(),
    };
    for (id, n) in spec {
        let t = &data.unit_types[&UnitTypeId::new(*id).unwrap()];
        for _ in 0..*n {
            army.side.units.push(BattleUnit {
                strength: t.soldiers,
                max_strength: t.soldiers,
                experience: 0,
                morale: t.stats.morale,
                melee: t.stats.melee,
                ranged: t.stats.ranged,
                armor: t.stats.armor,
                is_ranged: matches!(t.category, UnitCategory::Ranged | UnitCategory::Siege),
            });
            army.profiles.push(UnitProfile::of(t));
            army.ids.push((*id).to_owned());
        }
    }
    army
}

type Spec = &'static [(&'static str, usize)];

const MAA: &str = "unit_men_at_arms_foot";
const LB: &str = "unit_longbowmen";
const KN: &str = "unit_knights";
const MIL: &str = "unit_urban_militia";
const XB: &str = "unit_crossbowmen";
const PK: &str = "unit_flemish_pikemen";
const SP: &str = "unit_welsh_spearmen";
const HB: &str = "unit_hobelars";

const SCENARIOS: &[(&str, Spec, Spec, Terrain)] = &[
    (
        "mirror",
        &[(MAA, 3), (LB, 3), (KN, 2)],
        &[(MAA, 3), (LB, 3), (KN, 2)],
        Terrain::Plains,
    ),
    (
        "English vs French",
        &[(MAA, 4), (LB, 6)],
        &[(KN, 4), (XB, 4), (MIL, 2)],
        Terrain::Plains,
    ),
    (
        "French charge vs English",
        &[(KN, 6), (XB, 3), (MIL, 3)],
        &[(MAA, 4), (LB, 6)],
        Terrain::Plains,
    ),
    (
        "longbows vs militia horde",
        &[(LB, 6), (MAA, 2)],
        &[(MIL, 12)],
        Terrain::Plains,
    ),
    (
        "militia horde vs longbows",
        &[(MIL, 12)],
        &[(LB, 6), (MAA, 2)],
        Terrain::Plains,
    ),
    (
        "knights vs pikes",
        &[(KN, 6)],
        &[(PK, 6), (XB, 2)],
        Terrain::Plains,
    ),
    (
        "small elite vs levy",
        &[(KN, 3), (MAA, 3)],
        &[(MIL, 6), (SP, 4), (XB, 2)],
        Terrain::Plains,
    ),
    (
        "levy vs small elite",
        &[(MIL, 6), (SP, 4), (XB, 2)],
        &[(KN, 3), (MAA, 3)],
        Terrain::Plains,
    ),
    (
        "outnumbered 8 v 12",
        &[(MAA, 3), (LB, 3), (KN, 2)],
        &[(MAA, 4), (XB, 5), (KN, 3)],
        Terrain::Plains,
    ),
    (
        "outnumbering 12 v 8",
        &[(MAA, 4), (XB, 5), (KN, 3)],
        &[(MAA, 3), (LB, 3), (KN, 2)],
        Terrain::Plains,
    ),
    (
        "hill defence",
        &[(KN, 4), (MAA, 4)],
        &[(LB, 5), (MAA, 3)],
        Terrain::Hills,
    ),
    (
        "light horse raid",
        &[(HB, 6), (LB, 2)],
        &[(MIL, 6), (XB, 2)],
        Terrain::Forest,
    ),
];

fn auto_rate(
    attacker: &Army,
    defender: &Army,
    conditions: &FieldConditions,
    data: &GameData,
    runs: u64,
) -> f64 {
    let wins = (0..runs)
        .filter(|seed| {
            // Seeds far from the forecast's (which starts at 0).
            let mut rng = CampaignRng::from_seed(1_000_003 + *seed);
            resolve_with_crossings(
                &attacker.side,
                &attacker.profiles,
                &defender.side,
                &defender.profiles,
                &BattleContext::default(),
                conditions,
                &data.auto_resolve,
                &data.river_crossing_rules,
                &mut rng,
            )
            .winner
                == Winner::Attacker
        })
        .count();
    wins as f64 / runs as f64
}

fn sim3d_rate(
    attacker: &Army,
    defender: &Army,
    terrain: Terrain,
    data: &GameData,
    runs: u64,
) -> Option<f64> {
    use sim_battle::{BattleSeason, BattleSetup, BattleSim, SideId, SideSetup, UnitSetup};
    if runs == 0 {
        return None;
    }
    let setup_side = |ids: &[String]| SideSetup {
        faction: "f".into(),
        faction_name: "F".into(),
        army: String::new(),
        units: ids
            .iter()
            .map(|id| {
                let t = &data.unit_types[&UnitTypeId::new(id).unwrap()];
                UnitSetup::from_unit_type(t, t.soldiers, t.stats.morale, 0)
            })
            .collect(),
        general: None,
        forced_march: false,
        entrenched: false,
        start_fatigue: 0.0,
    };
    let mut wins = 0;
    for seed in 0..runs {
        let setup = BattleSetup {
            crossing: None,
            province: String::new(),
            province_name: String::new(),
            terrain,
            river: false,
            season: BattleSeason::Summer,
            coastal: false,
            village: None,
            attacker: setup_side(&attacker.ids),
            defender: setup_side(&defender.ids),
            player_side: None,
            siege: None,
            siege_layout: None,
            orders: Vec::new(),
            abilities: Vec::new(),
            standards: None,
            decor_plan: None,
            opening: Default::default(),
        };
        let mut sim = BattleSim::new(setup, seed).ok()?;
        let mut steps = 0;
        while !sim.is_finished() && steps < 36_100 {
            sim.step();
            steps += 1;
        }
        if sim.outcome().is_some_and(|o| o.winner == SideId::Attacker) {
            wins += 1;
        }
    }
    Some(wins as f64 / runs as f64)
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let data = GameData::load(&root).unwrap().0;
    let env = |key: &str, default: u64| {
        std::env::var(key)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(default)
    };
    let runs = env("AUTO", 400);
    let runs_3d = env("SIM3D", 0);
    println!(
        "{:<28} {:>6} {:>9} {:>6} {:>6}",
        "scenario", "old", "forecast", "auto", "3d"
    );
    let (mut worst_old, mut worst_new): (f64, f64) = (0.0, 0.0);
    for (name, a, d, terrain) in SCENARIOS {
        let attacker = army(&data, a);
        let defender = army(&data, d);
        let rough = matches!(
            terrain,
            Terrain::Hills | Terrain::Forest | Terrain::Mountains
        );
        let old = win_chance(
            side_power(&attacker.side, effective_armor(&defender.side), 1.0),
            side_power(
                &defender.side,
                effective_armor(&attacker.side),
                if rough { 1.15 } else { 1.0 },
            ),
        );
        let conditions = FieldConditions {
            terrain: Some(*terrain),
            season: Some(Season::Summer),
            weather: None,
        };
        let forecast = forecast_sides(
            (&attacker.side, &attacker.profiles),
            (&defender.side, &defender.profiles),
            &BattleContext::default(),
            &conditions,
            &data.auto_resolve,
            &data.river_crossing_rules,
            0,
        )
        .win_chance;
        let auto = auto_rate(&attacker, &defender, &conditions, &data, runs);
        let sim3d = sim3d_rate(&attacker, &defender, *terrain, &data, runs_3d);
        worst_old = worst_old.max((old - auto).abs());
        worst_new = worst_new.max((forecast - auto).abs());
        println!(
            "{name:<28} {:>5.0}% {:>8.0}% {:>5.0}% {:>6}",
            old * 100.0,
            forecast * 100.0,
            auto * 100.0,
            sim3d.map_or_else(|| "-".to_owned(), |r| format!("{:.0}%", r * 100.0)),
        );
    }
    println!(
        "worst gap to the auto-resolver: old {:.0} points, forecast {:.0} points",
        worst_old * 100.0,
        worst_new * 100.0
    );
}
