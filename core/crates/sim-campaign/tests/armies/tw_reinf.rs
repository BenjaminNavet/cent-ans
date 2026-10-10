//! TW reinf (ADR 0330): a far allied army joins the 3D battle with a timed
//! arrival, by the flank it comes from.
use data_model::test_support::{fac, game_data};
use data_model::GameData;
use sim_battle::{BattleSim, EntryEdge, SideId};
use sim_campaign::march::px_per_km;
use sim_campaign::test_support::capital_city;
use sim_campaign::{Army, ArmyId, ArmyPosition, CampaignState, Season, Unit};

fn unit(data: &GameData, id: &str) -> Unit {
    Unit::fresh(&data.unit_types[&data_model::UnitTypeId::new(id).unwrap()])
}

fn field_army(state: &mut CampaignState, index: u32, faction: &str, point: [f32; 2]) -> ArmyId {
    let data = game_data();
    let id = ArmyId::from_index(index);
    let mut army = Army::new(
        fac(faction),
        ArmyPosition::Field {
            x: point[0],
            y: point[1],
        },
        vec![unit(data, "unit_knights"), unit(data, "unit_men_at_arms_foot")],
    );
    army.movement_left = 10_000;
    state.armies.insert(id.clone(), army);
    id
}

/// France (lead 900 + ally 902, `distance_km` to the west) against England
/// (901), as a pending field battle.
fn battle(distance_km: f32) -> (CampaignState, sim_battle::BattleSetup) {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 5).unwrap();
    state.chronicle.disabled = true;
    state.season = Season::Summer;
    for (a, b) in [("fac_france", "fac_england"), ("fac_england", "fac_france")] {
        state
            .factions
            .get_mut(&fac(a))
            .unwrap()
            .at_war_with
            .insert(fac(b));
    }
    state
        .armies
        .retain(|_, a| a.faction != fac("fac_france") && a.faction != fac("fac_england"));
    let ppk = px_per_km(data);
    let lead = field_army(&mut state, 900, "fac_france", [1000.0, 1000.0]);
    let enemy = field_army(&mut state, 901, "fac_england", [1000.0 + 2.0 * ppk, 1000.0]);
    field_army(
        &mut state,
        902,
        "fac_france",
        [1000.0 - distance_km * ppk, 1000.0],
    );
    let city = capital_city(&state, "fac_france");
    state.pending_battles.push(sim_campaign::BattleRequest {
        attacker: lead,
        defender: enemy,
        province: state.settlement_province(&city).unwrap().clone(),
        location: city,
        siege: false,
        sortie: false,
        opening: Default::default(),
    });
    let setup = state.battle_setup(data, 0).unwrap();
    (state, setup)
}

#[test]
fn a_far_ally_arrives_later_from_its_flank() {
    let data = game_data();
    let rules = data.free_movement_rules();
    let km = rules.reinforce_radius_km as f32 - 5.0;
    let (_, setup) = battle(km);
    let units = &setup.attacker.units;
    assert_eq!(units.len(), 4);
    assert!(units[..2].iter().all(|u| u.arrival_s.is_none()), "lead is there");
    let expected = rules.reinforce_base_delay_s
        + (f64::from(km) - rules.engage_radius_km) * rules.reinforce_seconds_per_km;
    for late in &units[2..] {
        let t = late.arrival_s.expect("timed arrival");
        assert!((t - expected).abs() < 5.0, "{t} vs {expected}");
        assert_eq!(late.entry_edge, EntryEdge::West);
    }
    assert!(setup.defender.units.iter().all(|u| u.arrival_s.is_none()));
    // Neighbours within the engagement radius are on the field from the start.
    let (_, near) = battle(rules.engage_radius_km as f32 - 0.5);
    assert!(near.attacker.units.iter().all(|u| u.arrival_s.is_none()));
}

#[test]
fn the_ally_enters_the_3d_battle_on_time_and_deterministically() {
    let (_, setup) = battle(20.0);
    let arrival = setup.attacker.units[2].arrival_s.unwrap();
    let run = |seed| {
        let mut sim = BattleSim::new(setup.clone(), seed).unwrap();
        sim.set_ai(SideId::Attacker, false);
        sim.set_ai(SideId::Defender, false);
        sim.set_end_conditions(false);
        assert!(sim.units()[2].reserve);
        let eta = sim.next_arrival_in(SideId::Attacker).unwrap();
        assert!((eta - arrival).abs() < 1.0);
        while sim.elapsed() < arrival - 1.0 {
            sim.step();
        }
        assert!(sim.units()[2].reserve, "not before its hour");
        while sim.elapsed() < arrival + 1.0 {
            sim.step();
        }
        assert!(!sim.units()[2].reserve);
        sim.units()
            .iter()
            .map(|u| (u.x.to_bits(), u.z.to_bits()))
            .collect::<Vec<_>>()
    };
    assert_eq!(run(3), run(3));
}
