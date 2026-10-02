//! Lot JR1 (ADR 0165): the crusader faction led by the campaign AI.

use std::path::PathBuf;

use data_model::{FactionId, GameData};
use sim_campaign::{CampaignState, EventKind, Order};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    ai::feudal::install();
    GameData::load(&root).expect("game data loads").0
}

#[test]
fn the_planner_preaches_as_soon_as_it_can() {
    let data = data();
    let faction = data.crusade_rules.as_ref().expect("rules").faction.clone();
    let state = CampaignState::new_1337(&data, FactionId::new("fac_france").unwrap(), 1)
        .expect("1337 start");
    let orders = ai::plan_turn(&state, &data, &faction);
    assert!(orders.contains(&Order::PreachPassage), "{orders:?}");
    // No other faction ever does.
    let cyprus = FactionId::new("fac_cyprus").unwrap();
    assert!(!ai::plan_turn(&state, &data, &cyprus).contains(&Order::PreachPassage));
}

#[test]
fn forty_turns_of_the_real_ai_keep_the_crusade_alive() {
    let data = data();
    let rules = data.crusade_rules.as_ref().expect("rules");
    let faction = rules.faction.clone();
    let mut state = CampaignState::new_1337(&data, FactionId::new("fac_france").unwrap(), 11)
        .expect("1337 start");
    state.interactive_battles = false;
    let mut landings = 0;
    for turn in 0..40 {
        let events = state.end_turn_with(&data, ai::plan_turn);
        landings += events
            .iter()
            .filter(|e| e.kind == EventKind::Crusade && e.text_fr.contains("débarque"))
            .count();
        if turn % 10 == 9 {
            let crusade = state.crusade.as_ref().expect("crusade kept");
            let f = &state.factions[&faction];
            let units: usize = state
                .armies
                .values()
                .filter(|a| a.faction == faction)
                .map(|a| a.units.len())
                .sum();
            println!(
                "turn {}: fervour {}, treasury {}, income {}, upkeep {}, field units {units}, \
                 target taken {}",
                turn + 1,
                crusade.fervor,
                f.treasury,
                f.income_last_turn,
                f.upkeep_last_turn,
                crusade.target_taken
            );
        }
    }
    assert!(
        state.factions[&faction].alive,
        "the crusaders are still there"
    );
    assert!(landings >= 2, "contingents landed: {landings}");
    // Landed volunteers do not rot in the base's garrison: the host musters
    // them (a cityless faction raises armies from the places it holds).
    let base = &state.settlements[&rules.base_settlement];
    if base.controller == faction {
        assert!(base.garrison.len() <= 6, "garrison {}", base.garrison.len());
    }
}
