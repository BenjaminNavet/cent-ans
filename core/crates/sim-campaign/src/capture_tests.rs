//! Fate of a captured place (occupy, ransom, sack, raze), on the
//! real data of 1337.

use data_model::{GameData, ProvinceId, SettlementId};

use crate::capture::{ai_choice, preview, CaptureError, CaptureOutcome};
use crate::orders::{Order, OrderError};
use crate::state::{ArmyId, ArmyPosition, CampaignState, Stance, Unit};

use data_model::test_support::{fac, game_data};

fn set(id: &str) -> SettlementId {
    SettlementId::new(id).unwrap()
}

fn city(state: &CampaignState, province: &str) -> SettlementId {
    state
        .province_city_id(&ProvinceId::new(province).unwrap())
        .unwrap()
        .clone()
}

fn at_war(state: &mut CampaignState, a: &str, b: &str) {
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

/// An army of `faction` standing on `settlement`, one veteran-less unit.
fn army_on(
    state: &mut CampaignState,
    data: &GameData,
    faction: &str,
    place: &SettlementId,
) -> ArmyId {
    let unit_type = data.unit_types.values().next().unwrap();
    let id = ArmyId::from_index(9_000 + state.armies.len() as u32);
    let mut army = state
        .armies
        .values()
        .next()
        .cloned()
        .expect("an army exists");
    army.faction = fac(faction);
    army.general = None;
    army.position = ArmyPosition::Settlement(place.clone());
    army.stance = Stance::Normal;
    army.units = vec![Unit::fresh(unit_type)];
    army.units[0].experience = 0;
    state.armies.insert(id.clone(), army);
    id
}

/// France (player) takes `place` from England through the common path.
fn french_capture(state: &mut CampaignState, data: &GameData, place: &SettlementId) -> u32 {
    let mut events = Vec::new();
    crate::siege::capture(state, data, place, &fac("fac_france"), &mut events);
    state
        .captures
        .pending
        .iter()
        .find(|p| &p.settlement == place)
        .expect("a pending capture")
        .id
}

fn english_setup(data: &GameData, place: &SettlementId) -> CampaignState {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 7).unwrap();
    at_war(&mut state, "fac_france", "fac_england");
    let s = state.settlements.get_mut(place).unwrap();
    s.controller = fac("fac_england");
    s.garrison.clear();
    state
}

#[test]
fn player_capture_opens_a_decision_with_six_priced_choices() {
    let data = game_data();
    let guyenne = city(
        &CampaignState::new_1337(data, fac("fac_france"), 7).unwrap(),
        "prov_guyenne",
    );
    let mut state = english_setup(data, &guyenne);
    let id = french_capture(&mut state, data, &guyenne);
    assert_eq!(state.settlements[&guyenne].controller, fac("fac_france"));
    let views = state.capture_decision_views(data, &fac("fac_france"));
    assert_eq!(views.len(), 1);
    let view = &views[0];
    assert_eq!(view.id, id);
    let outcomes: Vec<CaptureOutcome> = view.options.iter().map(|o| o.outcome).collect();
    assert_eq!(outcomes, CaptureOutcome::ALL.to_vec());
    // A province city cannot be razed; the preview says why.
    let raze = &view.options[3];
    assert!(!raze.allowed && raze.reason.is_some());
    // Ransom pays, sack pays more; both are written out.
    let ransom = &view.options[1].effects;
    let sack = &view.options[2].effects;
    assert!(
        ransom.gold > 0 && sack.gold > ransom.gold,
        "{ransom:?} {sack:?}"
    );
    assert!(view.options[2].effects_text.contains("Trésor"));
    // An AI capture decides at once: nothing pending for England.
    assert!(state
        .capture_decision_views(data, &fac("fac_england"))
        .is_empty());
}

#[test]
fn occupy_adds_nothing_to_the_occupation() {
    let data = game_data();
    let guyenne = city(
        &CampaignState::new_1337(data, fac("fac_france"), 7).unwrap(),
        "prov_guyenne",
    );
    let mut state = english_setup(data, &guyenne);
    let province = state.settlements[&guyenne].province.clone();
    let unrest_before = state.provinces[&province].unrest;
    let treasury = state.factions[&fac("fac_france")].treasury;
    let id = french_capture(&mut state, data, &guyenne);
    let occupied_unrest = state.provinces[&province].unrest;
    assert_eq!(
        occupied_unrest,
        unrest_before
            .saturating_add(crate::capture::occupation_unrest(&state, data, &guyenne))
            .min(100)
    );
    state
        .submit_order(
            data,
            Order::ChooseCaptureOutcome {
                decision: id,
                outcome: CaptureOutcome::Occupy,
                recipient: None,
            },
        )
        .unwrap();
    assert!(state.captures.pending.is_empty());
    assert_eq!(state.provinces[&province].unrest, occupied_unrest);
    assert_eq!(state.factions[&fac("fac_france")].treasury, treasury);
}

#[test]
fn ransom_pays_the_previewed_gold_and_stirs_unrest() {
    let data = game_data();
    let guyenne = city(
        &CampaignState::new_1337(data, fac("fac_france"), 7).unwrap(),
        "prov_guyenne",
    );
    let mut state = english_setup(data, &guyenne);
    let province = state.settlements[&guyenne].province.clone();
    let id = french_capture(&mut state, data, &guyenne);
    let expected = preview(&state, data, &guyenne, CaptureOutcome::Ransom);
    let treasury = state.factions[&fac("fac_france")].treasury;
    let unrest = state.provinces[&province].unrest;
    let buildings = state.settlements[&guyenne].buildings.len();
    state
        .choose_capture_outcome(data, &fac("fac_france"), id, CaptureOutcome::Ransom, None)
        .unwrap();
    assert_eq!(
        state.factions[&fac("fac_france")].treasury,
        treasury + expected.gold
    );
    assert_eq!(
        state.provinces[&province].unrest,
        unrest.saturating_add(expected.extra_unrest).min(100)
    );
    // No destruction.
    assert_eq!(state.settlements[&guyenne].buildings.len(), buildings);
}

#[test]
fn sack_takes_gold_people_and_a_building_and_hardens_the_troops() {
    let data = game_data();
    let guyenne = city(
        &CampaignState::new_1337(data, fac("fac_france"), 7).unwrap(),
        "prov_guyenne",
    );
    let mut state = english_setup(data, &guyenne);
    let army = army_on(&mut state, data, "fac_france", &guyenne);
    let province = state.settlements[&guyenne].province.clone();
    let people = |s: &CampaignState| -> u64 {
        s.provinces[&province]
            .population
            .iter()
            .map(|(_, c)| c.count)
            .sum()
    };
    let id = french_capture(&mut state, data, &guyenne);
    let expected = preview(&state, data, &guyenne, CaptureOutcome::Sack);
    let before_people = people(&state);
    let buildings = state.settlements[&guyenne].buildings.clone();
    assert!(!buildings.is_empty(), "Bordeaux has buildings");
    let favor = state.factions[&fac("fac_france")].papal_favor;
    let treasury = state.factions[&fac("fac_france")].treasury;
    state
        .choose_capture_outcome(data, &fac("fac_france"), id, CaptureOutcome::Sack, None)
        .unwrap();
    assert_eq!(
        state.factions[&fac("fac_france")].treasury,
        treasury + expected.gold
    );
    assert!(people(&state) < before_people);
    assert_eq!(
        state.settlements[&guyenne].buildings.len(),
        buildings.len() - 1
    );
    assert_eq!(
        expected.buildings_lost,
        vec![buildings.last().unwrap().clone()]
    );
    assert_eq!(state.armies[&army].units[0].experience, 1);
    assert!(state.factions[&fac("fac_france")].papal_favor < favor);
    // England remembers.
    assert!(state.factions[&fac("fac_england")]
        .modifiers
        .iter()
        .any(|m| m.with == fac("fac_france") && m.value < 0));
}

#[test]
fn raze_is_refused_on_cities_and_emblematic_places() {
    let data = game_data();
    let guyenne = city(
        &CampaignState::new_1337(data, fac("fac_france"), 7).unwrap(),
        "prov_guyenne",
    );
    let mut state = english_setup(data, &guyenne);
    let id = french_capture(&mut state, data, &guyenne);
    let refused =
        state.choose_capture_outcome(data, &fac("fac_france"), id, CaptureOutcome::Raze, None);
    assert!(matches!(refused, Err(CaptureError::RazeForbidden(_))));
    // The question is still open.
    assert_eq!(state.captures.pending.len(), 1);

    let saint_denis = set("set_saint_denis");
    let mut state = english_setup(data, &saint_denis);
    let id = french_capture(&mut state, data, &saint_denis);
    let view = &state.capture_decision_views(data, &fac("fac_france"))[0];
    assert!(!view.options[3].allowed);
    assert!(matches!(
        state.choose_capture_outcome(data, &fac("fac_france"), id, CaptureOutcome::Raze, None),
        Err(CaptureError::RazeForbidden(_))
    ));
}

#[test]
fn raze_costs_a_level_or_leaves_a_ruin() {
    let data = game_data();
    // A strong castle loses one level.
    let montlhery = set("set_peyrepertuse");
    let mut state = english_setup(data, &montlhery);
    let level = state.settlements[&montlhery].fortification_level;
    assert!(level >= 2);
    let id = french_capture(&mut state, data, &montlhery);
    state
        .choose_capture_outcome(data, &fac("fac_france"), id, CaptureOutcome::Raze, None)
        .unwrap();
    let place = &state.settlements[&montlhery];
    assert_eq!(place.fortification_level, level - 1);
    assert!(place.buildings.is_empty());
    assert!(!crate::capture::is_ruined(&state, &montlhery));
    // Others frown upon it.
    assert!(state.factions[&fac("fac_castile")]
        .modifiers
        .iter()
        .any(|m| m.with == fac("fac_france") && m.value < 0));

    // A small place (level 1) is left a ruin: no recruitment there.
    // CC C3: every level-1 abbey left is on the no-raze list, so a level-1 town stands in.
    let royaumont = set("set_sluis");
    let mut state = english_setup(data, &royaumont);
    let id = french_capture(&mut state, data, &royaumont);
    state
        .choose_capture_outcome(data, &fac("fac_france"), id, CaptureOutcome::Raze, None)
        .unwrap();
    assert!(crate::capture::is_ruined(&state, &royaumont));
    assert_eq!(state.settlements[&royaumont].fortification_level, 0);
    let listed = state.recruitable(data, &royaumont);
    assert!(!listed.is_empty());
    assert!(listed
        .iter()
        .all(|o| !o.available && o.reason.as_deref() == Some("la colonie est en ruines")));
    let unit_type = data.unit_types.keys().next().unwrap().clone();
    let refused = state.submit_order(
        data,
        Order::Recruit {
            settlement: royaumont.clone().into(),
            unit_type,
        },
    );
    assert_eq!(refused, Err(OrderError::SettlementRuined));
    // The ruin passes after its time.
    state.turn += data.capture_rules.raze.ruin_turns;
    assert!(!crate::capture::is_ruined(&state, &royaumont));
}

#[test]
fn unanswered_captures_are_simply_occupied() {
    let data = game_data();
    let guyenne = city(
        &CampaignState::new_1337(data, fac("fac_france"), 7).unwrap(),
        "prov_guyenne",
    );
    let mut state = english_setup(data, &guyenne);
    let buildings = state.settlements[&guyenne].buildings.len();
    french_capture(&mut state, data, &guyenne);
    crate::capture::resolve_unanswered(&mut state);
    assert!(state.captures.pending.is_empty());
    assert_eq!(state.settlements[&guyenne].buildings.len(), buildings);
    // A lost place cannot be sacked afterwards.
    let id = french_capture(&mut state, data, &guyenne);
    state.settlements.get_mut(&guyenne).unwrap().controller = fac("fac_england");
    assert_eq!(
        state.choose_capture_outcome(data, &fac("fac_france"), id, CaptureOutcome::Sack, None),
        Err(CaptureError::PlaceLost)
    );
}

#[test]
fn the_ai_chooses_by_doctrine_treasury_and_culture() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_scotland"), 7).unwrap();
    at_war(&mut state, "fac_france", "fac_england");
    at_war(&mut state, "fac_france", "fac_scotland");
    let paris = city(&state, "prov_ile_de_france");
    let montlhery = set("set_peyrepertuse");
    let rich = 100_000;
    state
        .factions
        .get_mut(&fac("fac_england"))
        .unwrap()
        .treasury = rich;
    state
        .factions
        .get_mut(&fac("fac_scotland"))
        .unwrap()
        .treasury = rich;
    state.factions.get_mut(&fac("fac_france")).unwrap().treasury = rich;
    // A rich England in a French city it can hold: occupies.
    assert_eq!(
        ai_choice(&state, data, &paris, &fac("fac_england")),
        CaptureOutcome::Occupy
    );
    // Poor, it sets the city to ransom.
    state
        .factions
        .get_mut(&fac("fac_england"))
        .unwrap()
        .treasury = 0;
    assert_eq!(
        ai_choice(&state, data, &paris, &fac("fac_england")),
        CaptureOutcome::Ransom
    );
    // France retaking its own castle: occupies, even poor.
    state.factions.get_mut(&fac("fac_france")).unwrap().treasury = 0;
    assert_eq!(
        ai_choice(&state, data, &montlhery, &fac("fac_france")),
        CaptureOutcome::Occupy
    );
    // Scotland in a castle it cannot hold (Paris stays French): razes it.
    assert_eq!(
        ai_choice(&state, data, &montlhery, &fac("fac_scotland")),
        CaptureOutcome::Raze
    );
    // …but never an emblematic abbey.
    assert_ne!(
        ai_choice(&state, data, &set("set_saint_denis"), &fac("fac_scotland")),
        CaptureOutcome::Raze
    );
    // Rebels only occupy.
    assert_eq!(
        ai_choice(&state, data, &paris, &fac("fac_rebels")),
        CaptureOutcome::Occupy
    );
}

#[test]
fn an_ai_capture_is_applied_at_once() {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 7).unwrap();
    at_war(&mut state, "fac_france", "fac_scotland");
    let montlhery = set("set_peyrepertuse");
    state
        .settlements
        .get_mut(&montlhery)
        .unwrap()
        .garrison
        .clear();
    let level = state.settlements[&montlhery].fortification_level;
    let mut events = Vec::new();
    crate::siege::capture(
        &mut state,
        data,
        &montlhery,
        &fac("fac_scotland"),
        &mut events,
    );
    assert!(state.captures.pending.is_empty());
    assert_eq!(state.settlements[&montlhery].fortification_level, level - 1);
    assert!(events.iter().any(|e| e.text_fr.contains("rase")));
}

#[test]
fn defaults_mirror_the_data_file() {
    let data = game_data();
    assert!(
        data.capture_rules.description.is_some(),
        "capture.json not read"
    );
    assert_eq!(data.capture_rules, data_model::CaptureRules::default());
}

#[test]
fn tx_m1_losing_a_place_to_the_enemy_leaves_a_grudge_to_the_player_only() {
    use crate::grudges::GrudgeKind;
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 7).unwrap();
    at_war(&mut state, "fac_france", "fac_england");
    let place = city(&state, "prov_normandie");
    let mut events = Vec::new();
    crate::siege::capture(&mut state, data, &place, &fac("fac_england"), &mut events);
    let grudges = &state.missions.grudges;
    assert_eq!(grudges.len(), 1);
    assert_eq!(grudges[0].kind, GrudgeKind::LostSettlement);
    assert_eq!(grudges[0].culprit, fac("fac_england"));
    assert_eq!(grudges[0].settlement.as_ref(), Some(&place));
    // An AI's loss is not remembered.
    let guyenne = city(&state, "prov_guyenne");
    state.settlements.get_mut(&guyenne).unwrap().controller = fac("fac_england");
    crate::siege::capture(&mut state, data, &guyenne, &fac("fac_france"), &mut events);
    assert_eq!(state.missions.grudges.len(), 1);
}

fn ally(state: &mut CampaignState, a: &str, b: &str) {
    state
        .factions
        .get_mut(&fac(a))
        .unwrap()
        .allies
        .insert(fac(b));
    state
        .factions
        .get_mut(&fac(b))
        .unwrap()
        .allies
        .insert(fac(a));
}

fn guyenne_taken() -> (CampaignState, SettlementId, u32) {
    let data = game_data();
    let probe = CampaignState::new_1337(data, fac("fac_france"), 7).unwrap();
    let guyenne = city(&probe, "prov_guyenne");
    let mut state = english_setup(data, &guyenne);
    let id = french_capture(&mut state, data, &guyenne);
    (state, guyenne, id)
}

#[test]
fn tx_m2_foreign_culture_adds_unrest_to_every_outcome() {
    let data = game_data();
    let (mut state, guyenne, _) = guyenne_taken();
    let rules = &data.capture_rules;
    assert!(crate::capture::is_foreign_culture(&state, data, &guyenne));
    assert_eq!(
        crate::capture::occupation_unrest(&state, data, &guyenne),
        rules.occupy.unrest_city + rules.foreign_culture_extra.occupy
    );
    let foreign_sack = preview(&state, data, &guyenne, CaptureOutcome::Sack).unwrap_unrest();
    // The same place held by a native lord stirs less.
    let owner_culture = data.provinces[&state.settlements[&guyenne].province]
        .culture
        .clone();
    let native = data
        .factions
        .iter()
        .find(|(id, f)| f.culture == owner_culture && state.factions.contains_key(*id))
        .map(|(id, _)| id.clone())
        .expect("a faction of the culture");
    state.settlements.get_mut(&guyenne).unwrap().controller = native;
    assert!(!crate::capture::is_foreign_culture(&state, data, &guyenne));
    let native_sack = preview(&state, data, &guyenne, CaptureOutcome::Sack).unwrap_unrest();
    assert_eq!(foreign_sack - native_sack, rules.foreign_culture_extra.sack);
}

trait UnrestOf {
    fn unwrap_unrest(&self) -> u8;
}
impl UnrestOf for crate::capture::CaptureEffects {
    fn unwrap_unrest(&self) -> u8 {
        self.unrest
    }
}

#[test]
fn tx_m2_sack_leaves_a_banditry_that_ends() {
    let data = game_data();
    let (mut state, guyenne, id) = guyenne_taken();
    let province = state.settlements[&guyenne].province.clone();
    let before = state.provinces[&province].devastation;
    let effects = preview(&state, data, &guyenne, CaptureOutcome::Sack);
    assert!(effects.banditry_devastation > 0 && effects.banditry_turns > 0);
    state
        .choose_capture_outcome(data, &fac("fac_france"), id, CaptureOutcome::Sack, None)
        .unwrap();
    let after = state.provinces[&province].devastation;
    assert_eq!(after, before.saturating_add(effects.devastation).min(100));
    let banditry = state.captures.banditry[&province];
    assert_eq!(banditry.until, state.turn + effects.banditry_turns);
    // Not over yet.
    crate::capture::resolve_unanswered(&mut state);
    assert!(state.captures.banditry.contains_key(&province));
    state.turn = banditry.until;
    crate::capture::resolve_unanswered(&mut state);
    assert!(state.captures.banditry.is_empty());
    assert_eq!(
        state.provinces[&province].devastation,
        after.saturating_sub(banditry.points)
    );
}

#[test]
fn tx_m2_gift_hands_the_place_to_an_ally_without_unrest() {
    let data = game_data();
    let (mut state, guyenne, id) = guyenne_taken();
    let province = state.settlements[&guyenne].province.clone();
    let unrest_before = state.provinces[&province].unrest;
    let occupation = crate::capture::occupation_unrest(&state, data, &guyenne);
    ally(&mut state, "fac_france", "fac_aragon");
    // An enemy is no recipient.
    let enemy = state.choose_capture_outcome(
        data,
        &fac("fac_france"),
        id,
        CaptureOutcome::Gift,
        Some(fac("fac_england")),
    );
    assert!(matches!(enemy, Err(CaptureError::CessionForbidden(_))));
    let view = &state.capture_decision_views(data, &fac("fac_france"))[0];
    let gift = view
        .options
        .iter()
        .find(|o| o.outcome == CaptureOutcome::Gift)
        .unwrap();
    assert!(gift.allowed);
    assert!(gift.recipients.contains(&fac("fac_aragon")));
    state
        .choose_capture_outcome(
            data,
            &fac("fac_france"),
            id,
            CaptureOutcome::Gift,
            Some(fac("fac_aragon")),
        )
        .unwrap();
    let place = &state.settlements[&guyenne];
    assert_eq!(place.owner, fac("fac_aragon"));
    assert_eq!(place.controller, fac("fac_aragon"));
    assert_eq!(
        state.provinces[&province].unrest,
        unrest_before.saturating_sub(occupation)
    );
    assert!(state.factions[&fac("fac_aragon")]
        .modifiers
        .iter()
        .any(
            |m| m.with == fac("fac_france") && m.value == data.capture_rules.gift.recipient_opinion
        ));
    assert!(state.captures.pending.is_empty());
}

#[test]
fn tx_m2_liberate_returns_the_place_and_forgets_the_grudge() {
    use crate::grudges::{Grudge, GrudgeKind};
    let data = game_data();
    let (mut state, guyenne, id) = guyenne_taken();
    // Held in right by a third party at peace with France; England is at fault.
    state.settlements.get_mut(&guyenne).unwrap().owner = fac("fac_aragon");
    let view = &state.capture_decision_views(data, &fac("fac_france"))[0];
    let liberate = view
        .options
        .iter()
        .find(|o| o.outcome == CaptureOutcome::Liberate)
        .unwrap();
    assert!(liberate.allowed, "{:?}", liberate.reason);
    assert_eq!(liberate.recipient, Some(fac("fac_aragon")));
    state.missions.grudges.push(Grudge {
        kind: GrudgeKind::LostSettlement,
        culprit: fac("fac_england"),
        settlement: Some(guyenne.clone()),
        province: None,
        turn: 0,
    });
    ally(&mut state, "fac_aragon", "fac_portugal");
    let prestige = |s: &CampaignState| {
        let ruler = s.factions[&fac("fac_france")].ruler.clone().unwrap();
        s.characters[&ruler].prestige
    };
    let prestige_before = prestige(&state);
    state
        .choose_capture_outcome(data, &fac("fac_france"), id, CaptureOutcome::Liberate, None)
        .unwrap();
    assert!(state.missions.grudges.is_empty());
    assert_eq!(state.settlements[&guyenne].controller, fac("fac_aragon"));
    assert_eq!(
        prestige(&state) - prestige_before,
        data.capture_rules.liberate.prestige
    );
    let rules = &data.capture_rules.liberate;
    let opinion = |holder: &str| {
        state.factions[&fac(holder)]
            .modifiers
            .iter()
            .find(|m| m.with == fac("fac_france"))
            .map(|m| m.value)
    };
    assert_eq!(opinion("fac_aragon"), Some(rules.former_holder_opinion));
    assert_eq!(opinion("fac_portugal"), Some(rules.allies_opinion));
}

#[test]
fn tx_m2_ai_gives_to_an_ally_beside_it_and_liberates_a_peaceful_owner() {
    let data = game_data();
    let (mut state, guyenne, _) = guyenne_taken();
    // England captured: poor, an allied army stands on the place.
    state.settlements.get_mut(&guyenne).unwrap().controller = fac("fac_england");
    state.settlements.get_mut(&guyenne).unwrap().owner = fac("fac_france");
    for faction in ["fac_england", "fac_scotland"] {
        let f = state.factions.get_mut(&fac(faction)).unwrap();
        f.at_war_with.remove(&fac("fac_england"));
        f.at_war_with.remove(&fac("fac_scotland"));
    }
    ally(&mut state, "fac_england", "fac_scotland");
    assert_ne!(
        ai_choice(&state, data, &guyenne, &fac("fac_england")),
        CaptureOutcome::Gift
    );
    army_on(&mut state, data, "fac_scotland", &guyenne);
    assert_eq!(
        ai_choice(&state, data, &guyenne, &fac("fac_england")),
        CaptureOutcome::Gift
    );
    // Without the ally, a rightful owner at peace gets the place back.
    state.armies.retain(|_, a| a.faction != fac("fac_scotland"));
    state
        .factions
        .get_mut(&fac("fac_england"))
        .unwrap()
        .allies
        .clear();
    state
        .factions
        .get_mut(&fac("fac_scotland"))
        .unwrap()
        .allies
        .clear();
    state.settlements.get_mut(&guyenne).unwrap().owner = fac("fac_aragon");
    assert_eq!(
        ai_choice(&state, data, &guyenne, &fac("fac_england")),
        CaptureOutcome::Liberate
    );
}
