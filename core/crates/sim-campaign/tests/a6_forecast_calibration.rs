//! A6-L1: the pre-battle forecast agrees with the auto-resolution it
//! announces (ADR 0177). Armies of the 1337 setup are staged against each
//! other (with the defender's regiments scaled to vary the odds); the
//! forecast is compared with the frequency of attacker victories over 200
//! auto-resolutions on fresh seeds.

use std::path::PathBuf;

use data_model::{FactionId, GameData};
use sim_campaign::{ArmyId, CampaignRng, CampaignState};

const SEEDS: u64 = 200;

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn armies_of(state: &CampaignState, faction: &str) -> Vec<ArmyId> {
    state
        .armies
        .iter()
        .filter(|(_, a)| a.faction == fac(faction) && !a.units.is_empty())
        .map(|(id, _)| id.clone())
        .collect()
}

/// Whether the attacker army won (winners gain experience or morale, losers
/// lose 20 morale).
fn attacker_won(before: &CampaignState, after: &CampaignState, army: &ArmyId) -> bool {
    let Some(now) = after.armies.get(army) else {
        return false;
    };
    let xp = |s: &sim_campaign::Army| s.units.iter().map(|u| u32::from(u.experience)).sum::<u32>();
    xp(now) > xp(&before.armies[army])
        || now.units.iter().map(|u| u32::from(u.morale)).sum::<u32>()
            > before.armies[army]
                .units
                .iter()
                .map(|u| u32::from(u.morale))
                .sum::<u32>()
}

#[test]
fn forecast_matches_auto_resolution_frequency() {
    let data = data();
    let mut base = CampaignState::new_1337(&data, fac("fac_france"), 7).expect("1337 start");
    base.chronicle.disabled = true;
    let factions = [
        "fac_france",
        "fac_england",
        "fac_burgundy",
        "fac_scotland",
        "fac_flanders",
    ];
    for a in factions {
        for b in factions {
            if a != b {
                base.factions
                    .get_mut(&fac(a))
                    .unwrap()
                    .at_war_with
                    .insert(fac(b));
            }
        }
    }
    let mut cases = Vec::new();
    let scales = [0.55, 0.7, 0.85, 1.0, 1.15, 1.35, 1.7];
    for (i, a) in factions.iter().enumerate() {
        for mirror in [false, true] {
            let b = factions[(i + 1) % factions.len()];
            let (attackers, defenders) = (armies_of(&base, a), armies_of(&base, b));
            let (Some(attacker), Some(defender)) = (attackers.first(), defenders.first()) else {
                continue;
            };
            for scale in scales {
                let mut state = base.clone();
                // Odd cases: a mirror match (same regiments), headcount scaled.
                if mirror {
                    let copy = state.armies[attacker].units.clone();
                    state.armies.get_mut(defender).unwrap().units = copy;
                }
                for unit in &mut state.armies.get_mut(defender).unwrap().units {
                    unit.strength = ((f64::from(unit.strength) * scale) as u32).max(1);
                    unit.max_strength = unit.strength;
                }
                if state.debug_stage_battle(attacker, defender).is_err() {
                    continue;
                }
                cases.push((
                    state,
                    attacker.clone(),
                    format!("{a}/{b} x{scale} mirror={mirror}"),
                ));
            }
        }
    }
    assert!(cases.len() >= 20, "{} cases", cases.len());
    let mut total_gap = 0.0;
    for (state, attacker, name) in &cases {
        let forecast = state.battle_forecast(&data, 0).expect("forecast");
        let wins = (0..SEEDS)
            .filter(|seed| {
                let mut trial = state.clone();
                trial.rng = CampaignRng::from_seed(9_000 + seed);
                trial.auto_resolve_pending(&data, 0).unwrap();
                attacker_won(state, &trial, attacker)
            })
            .count();
        let frequency = wins as f64 / SEEDS as f64;
        let gap = (forecast.attacker_win_chance - frequency).abs();
        total_gap += gap;
        let ratio = forecast.attacker_power / forecast.defender_power;
        eprintln!(
            "{name}: ratio {ratio:.2} forecast {:.0} % observed {:.0} %",
            forecast.attacker_win_chance * 100.0,
            frequency * 100.0
        );
        assert!(
            (forecast.attacker_share - forecast.attacker_win_chance).abs() < 1e-12,
            "bar and verdict share one probability"
        );
    }
    let mean_gap = total_gap / cases.len() as f64;
    eprintln!("mean gap {:.1} points", mean_gap * 100.0);
    assert!(mean_gap <= 0.10, "mean gap {:.1} points", mean_gap * 100.0);
}
