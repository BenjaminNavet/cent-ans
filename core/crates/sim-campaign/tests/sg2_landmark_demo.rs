//! SG2: siege demos of the landmark towns at peace in 1337 (Avignon,
//! Bruges): the demo army declares war on the holder and assaults the town
//! drawn from its plan (`debug_stage_landmark_siege`).

use std::path::PathBuf;

use data_model::{FactionId, GameData};
use sim_campaign::{ArmyId, CampaignState};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn largest_army(state: &CampaignState, faction: &str) -> ArmyId {
    let faction = FactionId::new(faction).unwrap();
    state
        .armies
        .iter()
        .filter(|(_, a)| a.faction == faction)
        .max_by_key(|(_, a)| a.units.len())
        .map(|(id, _)| id.clone())
        .unwrap()
}

#[test]
fn avignon_and_bruges_can_be_assaulted_in_their_plan() {
    let data = data();
    for (landmark, attacker, settlement) in [
        ("avignon", "fac_england", "set_avignon"),
        ("bruges", "fac_france", "set_bruges"),
    ] {
        let mut state = CampaignState::new_1337(&data, FactionId::new("fac_france").unwrap(), 5)
            .expect("1337 start");
        state.chronicle.disabled = true;
        let army = largest_army(&state, attacker);
        let index = state
            .debug_stage_landmark_siege(&data, &army, landmark)
            .unwrap_or_else(|e| panic!("{landmark}: {e}"));
        let request = state.pending_battles[index].clone();
        assert!(request.siege);
        assert_eq!(request.location.as_str(), settlement);
        let holder = state.settlements[&request.location].controller.clone();
        assert!(state.is_at_war(&FactionId::new(attacker).unwrap(), &holder));
        let setup = state.battle_setup(&data, index).expect("setup");
        let layout = setup.siege_layout.expect("the town is drawn from its plan");
        assert_eq!(layout.id, landmark);
        assert!(!setup.defender.units.is_empty(), "{landmark}: a garrison");
    }
}

#[test]
fn unknown_landmark_or_own_town_is_refused() {
    let data = data();
    let mut state =
        CampaignState::new_1337(&data, FactionId::new("fac_france").unwrap(), 5).unwrap();
    let army = largest_army(&state, "fac_france");
    assert!(state
        .debug_stage_landmark_siege(&data, &army, "atlantis")
        .is_err());
    assert!(state
        .debug_stage_landmark_siege(&data, &army, "paris")
        .is_err());
    assert!(state.pending_battles.is_empty());
}

/// The assault paths of the demo towns work: with both sides under AI, the
/// ram reaches the gate or ladders are raised against the walls of the plan.
#[test]
fn avignon_and_bruges_assault_paths_reach_the_walls() {
    use sim_battle::{BattleSim, SideId, SiegeFxKind};
    let data = data();
    for (landmark, attacker) in [("avignon", "fac_england"), ("bruges", "fac_france")] {
        let mut state = CampaignState::new_1337(&data, FactionId::new("fac_france").unwrap(), 5)
            .expect("1337 start");
        state.chronicle.disabled = true;
        let army = largest_army(&state, attacker);
        let index = state
            .debug_stage_landmark_siege(&data, &army, landmark)
            .unwrap();
        let setup = state.battle_setup(&data, index).expect("setup");
        let mut sim = BattleSim::new(setup, 11).expect("battle");
        sim.set_ai(SideId::Attacker, true);
        sim.set_ai(SideId::Defender, true);
        if sim.is_deploying() {
            sim.start_battle().expect("start");
        }
        let mut steps = 0;
        while !sim.is_finished() && steps < 6000 {
            sim.step();
            steps += 1;
        }
        let count = |pred: fn(&SiegeFxKind) -> bool| {
            sim.siege_fx().iter().filter(|f| pred(&f.kind)).count()
        };
        let strikes = count(|k| matches!(k, SiegeFxKind::RamStrike { .. }));
        let ladders = count(|k| matches!(k, SiegeFxKind::LaddersRaised { .. }));
        let on_wall = count(|k| matches!(k, SiegeFxKind::OnWall { .. }));
        let gate = count(|k| matches!(k, SiegeFxKind::GateBroken { .. }));
        println!(
            "{landmark}: {strikes} ram blows, gate broken {gate}, {ladders} ladders, {on_wall} on the wall, {steps} steps"
        );
        assert!(
            strikes > 0 || ladders > 0,
            "{landmark}: the assault never reached the walls"
        );
    }
}
