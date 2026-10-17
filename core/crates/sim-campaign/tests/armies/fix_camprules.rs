//! Post-playtest fixes (lot camprules): the loser of a played battle falls back.
use data_model::test_support::{fac, game_data};
use sim_campaign::test_support::{capital_city, main_army};
use sim_campaign::{ArmyPosition, CampaignState};

/// The French attacker, beaten in Paris by the English, does not stay with
/// the victors: it falls back (or is destroyed).
#[test]
fn a_beaten_attacker_leaves_the_battlefield() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 3).unwrap();
    let french = main_army(&state, "fac_france");
    let english = main_army(&state, "fac_england");
    let paris = capital_city(&state, "fac_france");
    state.armies.get_mut(&french).unwrap().position = ArmyPosition::Settlement(paris.clone());
    let index = state.debug_stage_battle(&french, &english).unwrap();
    let setup = state.battle_setup(data, index).unwrap();
    let side = |units: usize, losses: u32, routed: bool, delta: i32| {
        serde_json::json!({
            "losses": vec![losses; units],
            "total_losses": losses * units as u32,
            "morale_delta": delta,
            "routed": routed,
        })
    };
    let outcome: sim_battle::BattleOutcome = serde_json::from_value(serde_json::json!({
        "winner": "defender",
        "attacker": side(setup.attacker.units.len(), 5, false, -5),
        "defender": side(setup.defender.units.len(), 1, false, 5),
    }))
    .unwrap();
    let events = state.resolve_pending_battle(data, index, &outcome).unwrap();
    let army = state
        .armies
        .get(&french)
        .expect("army survives a light defeat");
    let place = army.settlement().expect("falls back on a place").clone();
    assert_ne!(place, paris, "the beaten army must leave Paris");
    let battle_province = state.settlement_province(&paris).unwrap();
    assert_ne!(
        state.settlement_province(&place).unwrap(),
        battle_province,
        "it leaves the province of the battle, not a hamlet next door"
    );
    assert!(
        events.iter().any(|e| e.text_fr.contains("se replie")),
        "the retreat is told in the journal"
    );
}
