//! Lot M1 (ADR 0720): dated grudges feeding the mission offer.
use data_model::test_support::{fac, game_data};
use data_model::{GameData, SettlementId};
use sim_campaign::grudges::{candidates, Grudge, GrudgeKind};
use sim_campaign::missions::{resolve_missions, NoticeKind};
use sim_campaign::test_support::start;
use sim_campaign::CampaignState;

fn prestige(state: &CampaignState) -> i32 {
    let ruler = state.factions[&state.player_faction].ruler.clone().unwrap();
    state.characters[&ruler].prestige
}

fn make_war(state: &mut CampaignState, a: &str, b: &str) {
    state
        .factions
        .get_mut(&fac(a))
        .unwrap()
        .at_war_with
        .insert(fac(b));
    state
        .factions
        .get_mut(&fac(b))
        .unwrap()
        .at_war_with
        .insert(fac(a));
}

/// A French settlement that is not emblematic.
fn plain_place(data: &GameData) -> String {
    let state = start(data, "fac_france", 5);
    let emblematic = &data.mission_rules.grudge.emblematic_settlements;
    state
        .settlements
        .iter()
        .find(|(id, s)| s.controller == fac("fac_france") && !emblematic.contains(id))
        .map(|(id, _)| id.to_string())
        .unwrap()
}

/// A French state at war with England, a grudge of `age` seasons about `place`.
fn with_grudge(data: &GameData, kind: GrudgeKind, place: Option<&str>, age: u32) -> CampaignState {
    let mut state = start(data, "fac_france", 5);
    state.turn = 40;
    make_war(&mut state, "fac_france", "fac_england");
    let settlement = place.map(|p| SettlementId::new(p).unwrap());
    let province = settlement
        .as_ref()
        .and_then(|s| state.settlement_province(s).cloned());
    if let Some(s) = &settlement {
        state.settlements.get_mut(s).unwrap().controller = fac("fac_england");
    }
    state.missions.faction = Some(fac("fac_france"));
    state.missions.last_closed_turn = Some(state.turn);
    state.missions.grudges.push(Grudge {
        kind,
        culprit: fac("fac_england"),
        settlement,
        province,
        turn: 40 - age,
    });
    state
}

#[test]
fn tx_m1_age_multiplier_follows_thresholds() {
    let rules = &game_data().mission_rules.grudge;
    let at = |age| rules.age_multiplier(age);
    assert_eq!([at(0), at(7)], [1.0, 1.0]);
    assert_eq!([at(8), at(15), at(16), at(23)], [1.5; 4]);
    assert_eq!([at(24), at(31), at(32), at(90)], [2.0; 4]);
}

#[test]
fn tx_m1_reward_grows_with_age() {
    let data = game_data();
    let reward = |age| {
        let state = with_grudge(data, GrudgeKind::DefeatedArmy, None, age);
        let mut found = candidates(&state, data);
        assert_eq!(found.len(), 1);
        found.remove(0).1.reward
    };
    let (young, old) = (reward(2), reward(30));
    let base = &data.mission_rules.grudge.defeated_army.reward;
    assert_eq!(young.gold, base.gold);
    assert_eq!(old.gold, base.gold * 2);
    assert_eq!(old.prestige, base.prestige * 2);
}

#[test]
fn tx_m1_emblematic_loss_doubles_prestige() {
    let data = game_data();
    let prestige_of = |place: &str| {
        let state = with_grudge(data, GrudgeKind::LostSettlement, Some(place), 2);
        candidates(&state, data).remove(0).1.reward.prestige
    };
    let plain = prestige_of(&plain_place(data));
    let emblematic = prestige_of("set_calais");
    assert_eq!(emblematic, plain * 2);
}

#[test]
fn tx_m1_offer_includes_grudges_deterministically() {
    let data = game_data();
    let mut seen = false;
    for seed in 1..40 {
        let mut state = with_grudge(data, GrudgeKind::DefeatedArmy, None, 10);
        state.seed = seed;
        state.missions.last_closed_turn = None;
        let mut events = Vec::new();
        resolve_missions(&mut state, data, &mut events);
        let offer = state.missions.offer.clone().expect("an offer");
        seen |= offer.candidates.iter().any(|m| m.grudge.is_some());
    }
    assert!(seen, "a grudge is offered for some seeds");
}

#[test]
fn tx_m1_grudge_cancelled_on_peace_without_penalty() {
    let data = game_data();
    let mut state = with_grudge(data, GrudgeKind::DefeatedArmy, None, 10);
    let mission = candidates(&state, data).remove(0).1;
    state.missions.active = vec![mission];
    let before = prestige(&state);
    state
        .factions
        .get_mut(&fac("fac_france"))
        .unwrap()
        .at_war_with
        .clear();
    state
        .factions
        .get_mut(&fac("fac_england"))
        .unwrap()
        .at_war_with
        .clear();
    let mut events = Vec::new();
    resolve_missions(&mut state, data, &mut events);
    assert!(state.missions.active.is_empty());
    assert!(state.missions.grudges.is_empty());
    assert_eq!(prestige(&state), before);
    assert!(state
        .mission_notices()
        .iter()
        .any(|n| n.kind == NoticeKind::Expired && n.text.contains("Aucune pénalité")));
}

#[test]
fn tx_m1_retaking_the_place_succeeds_and_clears_the_grudge() {
    let data = game_data();
    let place = plain_place(data);
    let mut state = with_grudge(data, GrudgeKind::LostSettlement, Some(&place), 30);
    let mission = candidates(&state, data).remove(0).1;
    let reward = mission.reward.clone();
    let province = mission.province.clone().unwrap();
    state.missions.active = vec![mission];
    let before = prestige(&state);
    // Retaken: both the place and its province.
    let city = state.province_city_id(&province).cloned().unwrap();
    for id in [city, SettlementId::new(&place).unwrap()] {
        state.settlements.get_mut(&id).unwrap().controller = fac("fac_france");
    }
    let mut events = Vec::new();
    resolve_missions(&mut state, data, &mut events);
    assert!(state.missions.active.is_empty());
    assert!(state.missions.grudges.is_empty());
    assert_eq!(prestige(&state), before + reward.prestige);
}

#[test]
fn tx_m1_failure_penalty_is_scaled_and_capped() {
    let data = game_data();
    let mut state = with_grudge(data, GrudgeKind::DefeatedArmy, None, 30);
    let mut mission = candidates(&state, data).remove(0).1;
    mission.deadline_turn = state.turn;
    let expected = (f64::from(data.mission_rules.failure_prestige)
        * data.mission_rules.grudge.failure_multiplier_cap)
        .round() as i32;
    assert_eq!(mission.failure_prestige, Some(expected));
    state.missions.active = vec![mission];
    let before = prestige(&state);
    let mut events = Vec::new();
    resolve_missions(&mut state, data, &mut events);
    assert!(state.missions.active.is_empty());
    assert_eq!(prestige(&state), before + expected);
    assert_eq!(
        state.missions.grudges.len(),
        1,
        "a failure keeps the grudge"
    );
}

#[test]
fn tx_m1_a_raid_on_the_players_province_leaves_a_grudge() {
    use sim_campaign::test_support::idle;
    use sim_campaign::{ArmyPosition, Stance};
    let data = game_data();
    let mut state = start(data, "fac_france", 6);
    make_war(&mut state, "fac_france", "fac_england");
    let province = data_model::ProvinceId::new("prov_normandie").unwrap();
    let city = state.province_city_id(&province).cloned().unwrap();
    state.settlements.get_mut(&city).unwrap().controller = fac("fac_france");
    let english = state
        .armies
        .iter()
        .find(|(_, a)| a.faction == fac("fac_england"))
        .map(|(id, _)| id.clone())
        .unwrap();
    let army = state.armies.get_mut(&english).unwrap();
    army.position = ArmyPosition::Settlement(city);
    army.stance = Stance::Raid;
    state.end_turn_with(data, idle);
    assert!(state
        .missions
        .grudges
        .iter()
        .any(|g| g.kind == GrudgeKind::RaidedProvince && g.culprit == fac("fac_england")));
}
