//! Lot TW2-T3 (ADR 0103): mercenary companies hired by an army from its
//! region's reserve (spec `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T3).

use std::path::PathBuf;

use data_model::{FactionId, GameData, SettlementId, SettlementKind, UnitTypeId};
use sim_campaign::mercenaries::{is_mercenary, resolve_mercenaries};
use sim_campaign::{ArmyId, ArmyPosition, CampaignState, Order, OrderError, Season, Stance, Unit};

fn real_data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn unit(id: &str) -> UnitTypeId {
    UnitTypeId::new(id).unwrap()
}

fn capital_city(state: &CampaignState, faction: &str) -> SettlementId {
    let capital = state.factions[&fac(faction)].capital.clone();
    state
        .province_city_id(&capital)
        .cloned()
        .expect("capital city")
}

fn armies_of(state: &CampaignState, faction: &str) -> Vec<ArmyId> {
    let faction = fac(faction);
    state
        .armies
        .iter()
        .filter(|(_, a)| a.faction == faction)
        .map(|(id, _)| id.clone())
        .collect()
}

fn hire(
    state: &mut CampaignState,
    data: &GameData,
    army: &ArmyId,
    id: &str,
) -> Result<(), OrderError> {
    state.submit_order(
        data,
        Order::HireMercenary {
            army: army.clone(),
            unit: unit(id),
        },
    )
}

/// France played by the player, two armies in Paris (region `france_nord`),
/// a full treasury, spring of `year`.
fn setup(year: i32) -> (GameData, CampaignState, Vec<ArmyId>) {
    let data = real_data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 3).unwrap();
    state.season = Season::Spring;
    state.year = year;
    let paris = capital_city(&state, "fac_france");
    let first = armies_of(&state, "fac_france")[0].clone();
    state.armies.get_mut(&first).unwrap().position = ArmyPosition::Settlement(paris.clone());
    // A second army split off the first, in Paris too.
    state
        .submit_order(
            &data,
            Order::SplitArmy {
                army: first.clone(),
                unit_indices: vec![0],
            },
        )
        .unwrap();
    let mut armies: Vec<ArmyId> = armies_of(&state, "fac_france")
        .into_iter()
        .filter(|id| state.armies[id].is_at(&paris))
        .collect();
    armies.sort_by_key(|id| std::cmp::Reverse(state.armies[id].units.len()));
    armies.truncate(2);
    assert_eq!(armies.len(), 2);
    for id in &armies {
        let army = state.armies.get_mut(id).unwrap();
        army.position = ArmyPosition::Settlement(paris.clone());
        army.stance = Stance::Normal;
    }
    state.factions.get_mut(&fac("fac_france")).unwrap().treasury = 100_000;
    (data, state, armies)
}

fn mercenaries_in(state: &CampaignState, data: &GameData, army: &ArmyId) -> usize {
    state.armies.get(army).map_or(0, |a| {
        a.units
            .iter()
            .filter(|u| is_mercenary(data, &u.unit_type))
            .count()
    })
}

#[test]
fn an_army_hires_a_company_of_its_region_at_once() {
    let (data, mut state, armies) = setup(1340);
    let army = &armies[0];
    let market = state.mercenary_market(&data, army).unwrap();
    assert_eq!(market.region.as_deref(), Some("france_nord"));
    assert_eq!(market.region_name, "France du Nord");
    assert!(market.blocked.is_none(), "{:?}", market.blocked);
    // Genoese of the royal galleys (1337-1347), no routiers before Brétigny.
    let genoese = market
        .options
        .iter()
        .find(|o| o.unit_type.as_str() == "unit_genoese_crossbowmen")
        .expect("Genoese offered in the north of France in 1340");
    assert!(market
        .options
        .iter()
        .all(|o| o.unit_type.as_str() != "unit_routiers"));
    let base = data.unit_types[&unit("unit_genoese_crossbowmen")].clone();
    let rules = &data.mercenary_rules;
    assert_eq!(
        genoese.cost,
        base.cost.money * rules.hire_cost_percent / 100
    );
    assert_eq!(genoese.upkeep, base.upkeep * rules.upkeep_percent / 100);
    assert!(genoese.available && genoese.pool.available == genoese.pool.cap);

    let units_before = state.armies[army].units.len();
    let treasury = state.factions[&fac("fac_france")].treasury;
    let movement = state.armies[army].movement_left;
    hire(&mut state, &data, army, "unit_genoese_crossbowmen").unwrap();
    // No recruitment time: the company is in the army now, full strength.
    let joined = state.armies[army].units.last().unwrap().clone();
    assert_eq!(state.armies[army].units.len(), units_before + 1);
    assert_eq!(joined.unit_type.as_str(), "unit_genoese_crossbowmen");
    assert_eq!(joined.strength, base.soldiers);
    assert_eq!(state.armies[army].movement_left, movement);
    assert_eq!(
        state.factions[&fac("fac_france")].treasury,
        treasury - i64::from(genoese.cost)
    );
    let after = state.mercenary_market(&data, army).unwrap();
    let pool = after
        .options
        .iter()
        .find(|o| o.unit_type.as_str() == "unit_genoese_crossbowmen")
        .unwrap()
        .pool;
    assert_eq!(pool.available, genoese.pool.cap - 1);
    assert_eq!(after.hires_left, rules.hires_per_army_per_turn - 1);
}

#[test]
fn hires_are_limited_per_army_and_per_faction() {
    let (data, mut state, armies) = setup(1362);
    let rules = data.mercenary_rules.clone();
    assert_eq!(rules.hires_per_army_per_turn, 2);
    assert_eq!(rules.hires_per_faction_per_turn, 3);
    hire(&mut state, &data, &armies[0], "unit_routiers").unwrap();
    hire(&mut state, &data, &armies[0], "unit_routiers").unwrap();
    let refused = hire(&mut state, &data, &armies[0], "unit_brabancons").unwrap_err();
    assert!(
        matches!(&refused, OrderError::MercenaryUnavailable(r) if r.contains("cette armée")),
        "{refused}"
    );
    // The second army hires the third company of the realm, not a fourth.
    hire(&mut state, &data, &armies[1], "unit_routiers").unwrap();
    let refused = hire(&mut state, &data, &armies[1], "unit_brabancons").unwrap_err();
    assert!(
        matches!(&refused, OrderError::MercenaryUnavailable(r) if r.contains("royaume")),
        "{refused}"
    );
    // The reserve of routiers of the region is now empty (cap 3).
    let market = state.mercenary_market(&data, &armies[1]).unwrap();
    let routiers = market
        .options
        .iter()
        .find(|o| o.unit_type.as_str() == "unit_routiers")
        .unwrap();
    assert_eq!(routiers.pool.available, 0);
    // A new turn lifts the limits; the reserve refills slowly.
    state.end_turn_with(&data, |_, _, _| Vec::new());
    let market = state.mercenary_market(&data, &armies[1]).unwrap();
    assert!(market.blocked.is_none(), "{:?}", market.blocked);
    let routiers = market
        .options
        .iter()
        .find(|o| o.unit_type.as_str() == "unit_routiers")
        .unwrap();
    assert_eq!(routiers.pool.milli, 500, "one season of refill");
    assert!(!routiers.available);
    assert!(
        routiers.reason.as_deref() == Some("réserve épuisée (+1 dans 1 saison)"),
        "{:?}",
        routiers.reason
    );
}

#[test]
fn companies_come_and_go_with_their_period() {
    let (data, mut state, armies) = setup(1359);
    let offered = |state: &CampaignState| -> Vec<String> {
        state
            .mercenary_market(&data, &armies[0])
            .unwrap()
            .options
            .iter()
            .map(|o| o.unit_type.to_string())
            .collect()
    };
    assert!(!offered(&state).contains(&"unit_routiers".to_owned()));
    state.year = 1360;
    assert!(offered(&state).contains(&"unit_routiers".to_owned()));
    state.year = 1400;
    assert!(!offered(&state).contains(&"unit_routiers".to_owned()));
    assert!(!offered(&state).contains(&"unit_genoese_crossbowmen".to_owned()));
    state.year = 1440;
    assert!(offered(&state).contains(&"unit_ecorcheurs".to_owned()));
    assert!(offered(&state).contains(&"unit_scots_archers".to_owned()));
    assert!(offered(&state).contains(&"unit_brabancons".to_owned()));
    // Unknown here: a band of another region.
    let refused = hire(&mut state, &data, &armies[0], "unit_routiers").unwrap_err();
    assert!(
        matches!(refused, OrderError::MercenaryUnavailable(_)),
        "{refused}"
    );
}

#[test]
fn no_hiring_on_enemy_lands() {
    let (data, mut state, armies) = setup(1362);
    let english = state
        .settlements
        .iter()
        .find(|(_, s)| s.controller == fac("fac_england") && s.kind == SettlementKind::City)
        .map(|(id, _)| id.clone())
        .expect("an English city");
    assert!(state.is_at_war(&fac("fac_france"), &fac("fac_england")));
    state.armies.get_mut(&armies[0]).unwrap().position = ArmyPosition::Settlement(english);
    let market = state.mercenary_market(&data, &armies[0]).unwrap();
    assert!(
        market
            .blocked
            .as_deref()
            .is_some_and(|r| r.contains("terre ennemie")),
        "{:?}",
        market.blocked
    );
    assert!(market.options.iter().all(|o| !o.available));
    let refused = hire(&mut state, &data, &armies[0], "unit_brabancons");
    assert!(refused.is_err());
}

#[test]
fn towns_no_longer_levy_companies() {
    let (data, state, _) = setup(1362);
    let paris = capital_city(&state, "fac_france");
    let option = state
        .recruit_option(&data, &fac("fac_france"), &paris, &unit("unit_routiers"))
        .unwrap();
    assert!(!option.available);
    assert_eq!(
        option.reason.as_deref(),
        Some("compagnie de mercenaires : à engager depuis une armée")
    );
}

#[test]
fn companies_cost_a_premium_and_unpaid_ones_desert_or_pillage() {
    let (data, mut state, armies) = setup(1362);
    let army = armies[0].clone();
    for _ in 0..2 {
        hire(&mut state, &data, &army, "unit_routiers").unwrap();
    }
    let routier = &data.unit_types[&unit("unit_routiers")];
    let premium = state.mercenary_premium(&data, &fac("fac_france"));
    let extra = i64::from(data.mercenary_rules.upkeep_percent - 100);
    let mercs = mercenaries_in(&state, &data, &army) as i64;
    assert!(mercs >= 2);
    // Player at normal prices: the premium above the ordinary upkeep.
    let expected = mercs * i64::from(routier.upkeep) * 4 * extra / 100;
    assert!(premium >= expected, "{premium} < {expected}");

    // Paid: the treasury loses the premium, nobody leaves.
    let mut events = Vec::new();
    let treasury = state.factions[&fac("fac_france")].treasury;
    resolve_mercenaries(&mut state, &data, &mut events);
    assert_eq!(
        state.factions[&fac("fac_france")].treasury,
        treasury - premium
    );
    assert_eq!(mercenaries_in(&state, &data, &army) as i64, mercs);
    assert_eq!(
        state.mercenaries.premium_last_turn.get(&fac("fac_france")),
        Some(&premium)
    );

    // Unpaid: each company deserts or pillages the province.
    for _ in 0..6 {
        state
            .armies
            .get_mut(&army)
            .unwrap()
            .units
            .push(Unit::fresh(routier));
    }
    let all = mercenaries_in(&state, &data, &army);
    state.factions.get_mut(&fac("fac_france")).unwrap().treasury = 0;
    let province = state
        .army_province(&data, &state.armies[&army])
        .expect("Paris");
    let unrest = state.provinces[&province].unrest;
    let devastation = state.provinces[&province].devastation;
    let mut events = Vec::new();
    resolve_mercenaries(&mut state, &data, &mut events);
    assert!(state.factions[&fac("fac_france")].treasury < 0);
    let left = mercenaries_in(&state, &data, &army);
    assert!(left < all, "some companies desert ({left}/{all})");
    assert!(left > 0, "some companies stay and pillage ({left}/{all})");
    let p = &state.provinces[&province];
    assert!(p.unrest > unrest || unrest == 100);
    assert!(p.devastation > devastation);
    assert!(events.iter().any(|e| e.text_fr.contains("désertent")));
    assert!(events
        .iter()
        .any(|e| e.text_fr.contains("se paient sur le pays")));
}

#[test]
fn mercenary_state_survives_a_save() {
    let (data, mut state, armies) = setup(1362);
    hire(&mut state, &data, &armies[0], "unit_routiers").unwrap();
    let json = serde_json::to_string(&state).unwrap();
    let loaded: CampaignState = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.mercenaries, state.mercenaries);
    // An older save without the field starts with full reserves.
    let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
    value.as_object_mut().unwrap().remove("mercenaries");
    let old: CampaignState = serde_json::from_value(value).unwrap();
    assert!(old.mercenaries.pools.is_empty());
}
