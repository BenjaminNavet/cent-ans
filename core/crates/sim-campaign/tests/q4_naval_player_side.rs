//! Q4 (Q3 playtest, P1): the naval pre-battle screen showed the wrong side or
//! chances for a French player. The pending view must put the player on the
//! side of his own fleet, and `win_chance` must be the player's share of the
//! auto-resolves — whichever side (interceptor or convoy) he is on.

use data_model::{GameData, SettlementId};
use sim_battle::naval::auto_resolve;
use sim_battle::SideId;
use sim_campaign::{ArmyId, CampaignState};

use data_model::test_support::{fac, game_data};

fn main_army(state: &CampaignState, faction: &str) -> ArmyId {
    state
        .armies()
        .iter()
        .filter(|(_, a)| a.faction == fac(faction))
        .max_by_key(|(id, a)| (a.units.len(), std::cmp::Reverse((*id).clone())))
        .map(|(id, _)| id.clone())
        .unwrap()
}

/// Share of the five forecast auto-resolves won by `side` (same seeds as the view).
fn share(state: &CampaignState, data: &GameData, side: SideId) -> f64 {
    let setup = state.naval_battle_setup(data, 0).unwrap();
    let seed = state.naval.pending[0].seed;
    let wins = (0..5u64)
        .filter(|k| auto_resolve(&setup, seed.wrapping_add(k * 7919)).winner == Some(side))
        .count();
    wins as f64 / 5.0
}

#[test]
fn french_convoy_intercepted_by_england_is_the_players_side() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 7).unwrap();
    let army = main_army(&state, "fac_france");
    let port = SettlementId::new("set_portsmouth").unwrap();
    let index = state
        .debug_stage_naval(data, &army, &port, &fac("fac_england"))
        .expect("staged");
    assert_eq!(index, 0);
    let view = &state.pending_naval_views(data)[0];
    assert_eq!(view.faction, fac("fac_france"));
    assert_eq!(view.interceptor, fac("fac_england"));
    assert_eq!(
        view.player_side,
        SideId::Defender,
        "the French convoy is the defender"
    );
    let setup = state.naval_battle_setup(data, 0).unwrap();
    assert_eq!(setup.defender.faction, "fac_france");
    assert_eq!(setup.player_side, Some(SideId::Defender));
    assert_eq!(view.win_chance, share(&state, data, SideId::Defender));
    // Complementary: the view never reports the interceptor's chances.
    let english = share(&state, data, SideId::Attacker);
    assert!(view.win_chance + english <= 1.0 + 1e-9);
    // The auto-resolve applied by « Résolution automatique » agrees with a sure forecast.
    let outcome = auto_resolve(&setup, state.naval.pending[0].seed);
    if view.win_chance >= 1.0 {
        assert_eq!(outcome.winner, Some(SideId::Defender));
    } else if view.win_chance <= 0.0 {
        assert_ne!(outcome.winner, Some(SideId::Defender));
    }
}

#[test]
fn french_squadron_intercepting_an_english_convoy_is_the_attacker() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 11).unwrap();
    let army = main_army(&state, "fac_england");
    let port = SettlementId::new("set_calais").unwrap();
    state
        .debug_stage_naval(data, &army, &port, &fac("fac_france"))
        .expect("staged");
    let view = &state.pending_naval_views(data)[0];
    assert_eq!(view.interceptor, fac("fac_france"));
    assert_eq!(view.player_side, SideId::Attacker);
    assert_eq!(view.win_chance, share(&state, data, SideId::Attacker));
}
