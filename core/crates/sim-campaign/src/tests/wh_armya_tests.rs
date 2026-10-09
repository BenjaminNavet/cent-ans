//! Lot WH armya (ADR 0272): tests reading crate-private items (siege entry,
//! start of a siege).
use data_model::test_support::{fac, game_data};
use data_model::GameData;

use crate::state::{Army, ArmyId, ArmyPosition, CampaignState, Stance, Unit};

fn unit(data: &GameData, id: &str) -> Unit {
    Unit::fresh(&data.unit_types[&data_model::UnitTypeId::new(id).unwrap()])
}

fn start() -> (&'static GameData, CampaignState) {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 5).unwrap();
    state.chronicle.disabled = true;
    for (a, b) in [("fac_france", "fac_england"), ("fac_england", "fac_france")] {
        state
            .factions
            .get_mut(&fac(a))
            .unwrap()
            .at_war_with
            .insert(fac(b));
    }
    (data, state)
}

fn field_army(
    state: &mut CampaignState,
    index: u32,
    faction: &str,
    point: [f32; 2],
    units: Vec<Unit>,
) -> ArmyId {
    let id = ArmyId::from_index(index);
    let mut army = Army::new(
        fac(faction),
        ArmyPosition::Field {
            x: point[0],
            y: point[1],
        },
        units,
    );
    army.movement_left = 10_000;
    state.armies.insert(id.clone(), army);
    id
}

#[test]
fn leaderless_army_halts_at_the_walls() {
    let (data, mut state) = start();
    let england = fac("fac_england");
    let place = state
        .settlements
        .iter()
        .find(|(_, s)| {
            s.controller == england
                && !s.garrison.is_empty()
                && s.kind == data_model::SettlementKind::City
        })
        .map(|(id, _)| id.clone())
        .expect("an English city with a garrison");
    state.armies.retain(|_, a| a.faction != england);
    let point = data.settlement_point(&place).unwrap();
    let id = field_army(
        &mut state,
        900,
        "fac_france",
        point,
        vec![unit(data, "unit_knights")],
    );
    let mut events = Vec::new();
    let stop = crate::march::enter_settlement(&mut state, data, &id, &place, &mut events);
    assert!(
        matches!(stop, crate::march::StopReason::EnemySettlement { .. }),
        "{stop:?}"
    );
    assert!(state.settlements[&place].siege.is_none());
    assert!(matches!(
        state.armies[&id].position,
        ArmyPosition::Field { .. }
    ));
}

#[test]
fn forecast_says_when_the_battle_lifts_a_siege() {
    let (data, mut state) = start();
    let england = fac("fac_england");
    let place = state
        .settlements
        .iter()
        .find(|(_, s)| s.controller == fac("fac_france") && s.siege.is_none())
        .map(|(id, _)| id.clone())
        .unwrap();
    state
        .armies
        .retain(|_, a| a.faction != england && a.faction != fac("fac_france"));
    let point = data.settlement_point(&place).unwrap();
    let besieger = field_army(
        &mut state,
        900,
        "fac_england",
        point,
        vec![unit(data, "unit_knights")],
    );
    let relief = field_army(
        &mut state,
        901,
        "fac_france",
        [point[0] + 20.0, point[1]],
        vec![unit(data, "unit_knights")],
    );
    {
        let a = state.armies.get_mut(&besieger).unwrap();
        a.position = ArmyPosition::Settlement(place.clone());
        a.stance = Stance::Siege;
    }
    crate::siege::begin_siege(
        &mut state,
        data,
        &place,
        &england,
        &besieger,
        &mut Vec::new(),
    );
    let request = |siege_loc: &data_model::SettlementId| crate::state::BattleRequest {
        attacker: relief.clone(),
        defender: besieger.clone(),
        location: siege_loc.clone(),
        province: state.settlement_province(siege_loc).unwrap().clone(),
        siege: false,
        opening: Default::default(),
    };
    state.pending_battles.push(request(&place));
    let forecast = state.battle_forecast(data, 0).unwrap();
    assert!(forecast.lifts_siege, "{forecast:?}");
    assert_eq!(forecast.siege_place, data.settlement_name(&place));
    // Without the siege stance there is nothing to lift.
    state.armies.get_mut(&besieger).unwrap().stance = Stance::Normal;
    assert!(!state.battle_forecast(data, 0).unwrap().lifts_siege);
}
