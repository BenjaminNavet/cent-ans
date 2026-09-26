//! Regression tests for the fixes of the 2026-09-26 code review
//! (`docs/wip/revue-code.md`, section « Corrections sim-campaign »).

use std::path::PathBuf;

use data_model::{FactionId, GameData, ProvinceId, SettlementId};

use crate::battle_auto::{BattleResult, SideOutcome, Winner};
use crate::state::{ArmyId, ArmyPosition, CampaignState, Stance};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn prov(id: &str) -> ProvinceId {
    ProvinceId::new(id).unwrap()
}

fn city(state: &CampaignState, province: &str) -> SettlementId {
    state.province_city_id(&prov(province)).unwrap().clone()
}

/// The first army of `faction` led by a general.
fn led_army(state: &CampaignState, faction: &str) -> ArmyId {
    state
        .armies
        .iter()
        .find(|(_, a)| a.faction == fac(faction) && a.general.is_some())
        .map(|(id, _)| id.clone())
        .expect("a led army")
}

/// `army` besieges `place` (siege stance, siege state begun).
fn besiege(state: &mut CampaignState, data: &GameData, army: &ArmyId, place: &SettlementId) {
    let a = state.armies.get_mut(army).unwrap();
    a.position = ArmyPosition::Settlement(place.clone());
    a.stance = Stance::Siege;
    a.clear_plan();
    let faction = a.faction.clone();
    let mut events = Vec::new();
    crate::siege::begin_siege(state, data, place, &faction, army, &mut events);
}

fn outcome(units: usize, general_captured: bool) -> SideOutcome {
    SideOutcome {
        power: 1.0,
        losses: vec![0; units],
        total_losses: 0,
        morale_delta: 0,
        routed: false,
        general_captured,
    }
}

/// Fix 1: a general taken in a failed assault is held by the defender.
#[test]
fn general_captured_in_assault_has_a_captor() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    let army = led_army(&state, "fac_france");
    let general = state.armies[&army].general.clone().unwrap();
    let guyenne = city(&state, "prov_guyenne");
    besiege(&mut state, &data, &army, &guyenne);
    let controller = state.settlements[&guyenne].controller.clone();
    let units = state.armies[&army].units.len();
    let garrison = state.settlements[&guyenne].garrison.len();
    let result = BattleResult {
        winner: Winner::Defender,
        attacker: outcome(units, true),
        defender: outcome(garrison, false),
    };
    let mut events = Vec::new();
    crate::siege::apply_assault_result(
        &mut state,
        &data,
        std::slice::from_ref(&army),
        &guyenne,
        &result,
        true,
        &mut events,
    );
    let c = &state.characters[&general];
    assert!(c.captive);
    assert_eq!(c.captor.as_ref(), Some(&controller));
}

/// Fix 2: a besieged garrison cannot form an army (it would lift the siege).
#[test]
fn besieged_garrison_cannot_create_an_army() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    let army = led_army(&state, "fac_france");
    let guyenne = city(&state, "prov_guyenne");
    besiege(&mut state, &data, &army, &guyenne);
    let controller = state.settlements[&guyenne].controller.clone();
    assert!(!state.settlements[&guyenne].garrison.is_empty());
    let order = crate::orders::Order::CreateArmy {
        settlement: crate::orders::Place::Settlement(guyenne.clone()),
        units_from_garrison: vec![0],
        general: None,
    };
    assert_eq!(
        state.apply_order(&data, &controller, order),
        Err(crate::orders::OrderError::SettlementBesieged)
    );
}

/// Makes `a` and `b` allies at war with `enemy`.
fn ally_against(state: &mut CampaignState, a: &str, b: &str, enemy: &str) {
    for (x, y) in [(a, b), (b, a)] {
        let f = state.factions.get_mut(&fac(x)).unwrap();
        f.allies.insert(fac(y));
        f.at_war_with.remove(&fac(y));
    }
    for x in [a, b] {
        state
            .factions
            .get_mut(&fac(x))
            .unwrap()
            .at_war_with
            .insert(fac(enemy));
        state
            .factions
            .get_mut(&fac(enemy))
            .unwrap()
            .at_war_with
            .insert(fac(x));
    }
}

/// Fix 3: an ally joining (or taking over) a siege keeps its progress.
#[test]
fn allied_besieger_keeps_the_siege_progress() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    ally_against(&mut state, "fac_france", "fac_scotland", "fac_england");
    let french = led_army(&state, "fac_france");
    let scots = state
        .armies
        .iter()
        .find(|(_, a)| a.faction == fac("fac_scotland"))
        .map(|(id, _)| id.clone())
        .unwrap();
    let guyenne = city(&state, "prov_guyenne");
    besiege(&mut state, &data, &french, &guyenne);
    {
        let siege = state.settlements.get_mut(&guyenne).unwrap().siege.as_mut();
        let siege = siege.unwrap();
        siege.breach = 40;
        siege.supplies = 50;
        siege.started_turn = 0;
    }
    besiege(&mut state, &data, &scots, &guyenne);
    let siege = state.settlements[&guyenne].siege.clone().unwrap();
    assert_eq!(siege.attacker, fac("fac_france"));
    assert_eq!(siege.breach, 40);
    // The French leave: the Scots carry on the same siege.
    let kent = city(&state, "prov_kent");
    state.armies.get_mut(&french).unwrap().position = ArmyPosition::Settlement(kent);
    let mut events = Vec::new();
    crate::siege::resolve_sieges(&mut state, &data, &mut events);
    if let Some(siege) = state.settlements[&guyenne].siege.clone() {
        assert_eq!(siege.attacker, fac("fac_scotland"));
        assert!(siege.breach >= 40 && siege.supplies <= 50);
    } else {
        assert_eq!(state.settlements[&guyenne].controller, fac("fac_scotland"));
    }
}

/// Fix 5: a captured settlement loses its building site (the occupant
/// cannot cancel it for the refund).
#[test]
fn capture_drops_the_construction() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    let guyenne = city(&state, "prov_guyenne");
    let building = data.buildings.keys().next().unwrap().clone();
    state.settlements.get_mut(&guyenne).unwrap().construction = Some(crate::state::Construction {
        building,
        turns_left: 3,
        paid: 100,
        drawn: Default::default(),
    });
    let mut events = Vec::new();
    crate::siege::capture(&mut state, &data, &guyenne, &fac("fac_france"), &mut events);
    let s = &state.settlements[&guyenne];
    assert_eq!(s.controller, fac("fac_france"));
    assert!(s.construction.is_none());
    assert!(s.recruit_queue.is_empty());
}

/// Fix 7: a player who still holds a castle or a town is not defeated.
#[test]
fn holding_a_lesser_settlement_is_no_defeat() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 4).unwrap();
    let cities: Vec<SettlementId> = state
        .provinces
        .keys()
        .filter_map(|p| state.province_city_id(p).cloned())
        .collect();
    let kept = state
        .settlements
        .iter()
        .find(|(id, s)| s.controller == fac("fac_france") && !cities.contains(id))
        .map(|(id, _)| id.clone())
        .expect("France holds a lesser settlement");
    for (id, s) in state.settlements.iter_mut() {
        if s.controller == fac("fac_france") && *id != kept {
            s.controller = fac("fac_england");
        }
    }
    let mut events = Vec::new();
    crate::victory::resolve_victory(&mut state, &data, &mut events);
    assert!(state.outcome.is_none());
    state.settlements.get_mut(&kept).unwrap().controller = fac("fac_england");
    crate::victory::resolve_victory(&mut state, &data, &mut events);
    assert!(state.outcome.is_some());
}

/// Aragon hands a courtier as a hostage to Castile (treaty article).
fn aragonese_hostage(state: &mut CampaignState, data: &GameData) -> data_model::CharacterId {
    let (ar, ca) = (fac("fac_aragon"), fac("fac_castile"));
    let ruler = state.factions[&ar].ruler.clone();
    let hostage = state
        .characters
        .iter()
        .find(|(id, c)| {
            c.alive
                && c.faction == ar
                && !c.captive
                && c.army.is_none()
                && Some(*id) != ruler.as_ref()
        })
        .map(|(id, _)| id.clone())
        .expect("an Aragonese courtier");
    let article = crate::negotiation::Article::Hostage {
        giver: crate::negotiation::Party::Proposer,
        character: hostage.clone(),
    };
    crate::negotiation::apply_treaty(state, data, &ar, &ca, &[article]).unwrap();
    hostage
}

/// Fix 8: a treaty hostage can be neither bought back nor freed on parole
/// before his term (ADR 0025 § 6).
#[test]
fn treaty_hostage_is_held_for_his_term() {
    use crate::ransom::{self, RansomError, RansomTerms};
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_aragon"), 10).unwrap();
    let hostage = aragonese_hostage(&mut state, &data);
    let (ar, ca) = (fac("fac_aragon"), fac("fac_castile"));
    assert_eq!(
        state.characters[&hostage].ransom_terms,
        Some(RansomTerms::Hold)
    );
    state.factions.get_mut(&ar).unwrap().treasury = 1_000_000;
    assert_eq!(
        ransom::pay_ransom(&mut state, &data, &ar, &hostage, 1),
        Err(RansomError::Held)
    );
    assert_eq!(
        ransom::release_on_parole(&mut state, &data, &ca, &hostage),
        Err(RansomError::Held)
    );
    assert_eq!(
        ransom::set_ransom_terms(&mut state, &data, &ca, &hostage, RansomTerms::Money),
        Err(RansomError::Held)
    );
    assert!(state.characters[&hostage].captive);
}

/// Fix 11: the holder attacking the giver of its hostages does not make the
/// giver a perjurer; the giver attacking the holder does.
#[test]
fn hostage_betrayal_falls_on_the_aggressor_only() {
    let data = data();
    let (ar, ca) = (fac("fac_aragon"), fac("fac_castile"));
    let betrayed = |state: &CampaignState| {
        state.factions[&ca]
            .modifiers
            .iter()
            .any(|m| m.with == ar && m.reason_fr == crate::negotiation::HOSTAGE_BETRAYAL_REASON)
    };
    let idle = |_: &CampaignState, _: &GameData, _: &FactionId| Vec::new();
    // The holder attacks.
    let mut state = CampaignState::new_1337(&data, fac("fac_aragon"), 10).unwrap();
    aragonese_hostage(&mut state, &data);
    state.declare_war(&data, &ca, &ar).unwrap();
    state.end_turn_with(&data, idle);
    assert!(!betrayed(&state));
    // The giver attacks.
    let mut state = CampaignState::new_1337(&data, fac("fac_aragon"), 10).unwrap();
    aragonese_hostage(&mut state, &data);
    state.declare_war(&data, &ar, &ca).unwrap();
    assert!(betrayed(&state));
}

/// Fix 12: a vanished faction pays no more tribute.
#[test]
fn dead_faction_pays_no_tribute() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1).unwrap();
    let (payer, payee) = (fac("fac_navarre"), fac("fac_castile"));
    let turn = state.turn;
    {
        let f = state.factions.get_mut(&payer).unwrap();
        f.alive = false;
        f.ledger.tributes.push(crate::negotiation::TributeDue {
            to: payee.clone(),
            per_season: 100,
            until_turn: turn + 10,
        });
    }
    let before = (
        state.factions[&payer].treasury,
        state.factions[&payee].treasury,
    );
    let mut events = Vec::new();
    crate::negotiation::resolve_negotiation(&mut state, &data, &mut events);
    assert_eq!(state.factions[&payer].treasury, before.0);
    assert_eq!(state.factions[&payee].treasury, before.1);
    assert!(state.factions[&payer].ledger.tributes.is_empty());
}
