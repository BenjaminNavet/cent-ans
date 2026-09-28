//! Integration tests of the campaign model on the real `data/` directory.

use std::path::PathBuf;

use data_model::{FactionId, GameData, ProvinceId, SettlementId, UnitTypeId};
use sim_campaign::{ArmyId, CampaignState, EventKind, Order, OrderError, Stance, Unit, START_YEAR};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn prov(id: &str) -> ProvinceId {
    ProvinceId::new(id).unwrap()
}

fn set(id: &str) -> SettlementId {
    SettlementId::new(id).unwrap()
}

/// The city of a province.
fn city(state: &CampaignState, province: &str) -> SettlementId {
    state.province_city_id(&prov(province)).unwrap().clone()
}

fn unit(id: &str) -> UnitTypeId {
    UnitTypeId::new(id).unwrap()
}

fn france(data: &GameData, seed: u64) -> CampaignState {
    CampaignState::new_1337(data, fac("fac_france"), seed).expect("1337 start")
}

fn main_army(state: &CampaignState, faction: &str) -> ArmyId {
    let faction = fac(faction);
    state
        .armies()
        .iter()
        .filter(|(_, a)| a.faction == faction)
        .max_by_key(|(id, a)| (a.units.len(), std::cmp::Reverse((*id).clone())))
        .map(|(id, _)| id.clone())
        .expect("faction has an army")
}

/// A planner that does nothing: the world stands still except for the player.
fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

#[test]
fn new_1337_matches_game_data() {
    let data = data();
    let state = france(&data, 1);
    assert_eq!(state.factions.len(), 61, "including the virtual fac_rebels");
    assert_eq!(state.provinces.len(), 153);
    assert_eq!(state.date_label(), "Printemps 1337");
    assert_eq!(state.turn(), 0);
    assert_eq!(state.year, START_YEAR);
    assert_eq!(state.player_faction(), &fac("fac_france"));

    let france_state = state.faction_state(&fac("fac_france")).unwrap();
    assert_eq!(france_state.treasury, 60_000);
    assert!(france_state.at_war_with.contains(&fac("fac_england")));
    assert!(france_state.allies.contains(&fac("fac_scotland")));
    assert!(france_state.allies.contains(&fac("fac_burgundy")));
    assert!(!france_state.at_war_with.contains(&fac("fac_scotland")));

    let army_id = main_army(&state, "fac_france");
    let army = state.army(&army_id).unwrap();
    assert!(army.is_at(&set("set_paris")));
    assert_eq!(army.units.len(), 8);
    assert_eq!(
        army.general.as_ref().map(|c| c.as_str()),
        Some("chr_philippe_vi")
    );
    assert_eq!(
        state
            .character(&army.general.clone().unwrap())
            .unwrap()
            .army,
        Some(army_id)
    );

    let england = main_army(&state, "fac_england");
    assert_eq!(state.army(&england).unwrap().units.len(), 6);
    assert_eq!(
        state.army(&england).unwrap().settlement().cloned().unwrap(),
        set("set_londres")
    );
    let burgundy = main_army(&state, "fac_burgundy");
    assert_eq!(state.army(&burgundy).unwrap().units.len(), 4);

    let paris = state.city_state(&prov("prov_ile_de_france")).unwrap();
    assert_eq!(paris.garrison.len(), 4, "capital garrison");
    assert!(state
        .provinces
        .keys()
        .all(|p| !state.city_state(p).unwrap().garrison.is_empty()));
    assert_eq!(state.armies().len(), 60, "one main army per faction");
}

#[test]
fn new_1337_rejects_unknown_faction() {
    let data = data();
    let err = CampaignState::new_1337(&data, fac("fac_atlantis"), 1).unwrap_err();
    assert!(matches!(
        err,
        sim_campaign::CampaignError::UnknownFaction(_)
    ));
}

#[test]
fn invalid_orders_are_rejected_without_side_effects() {
    let data = data();
    let mut state = france(&data, 1);
    let before = state.save_json();
    let own = main_army(&state, "fac_france");
    let foreign = main_army(&state, "fac_england");

    assert!(matches!(
        state.submit_order(
            &data,
            Order::MoveArmy {
                army: foreign,
                target: sim_campaign::MoveOrderTarget::Path(vec![prov("prov_kent").into()])
            }
        ),
        Err(OrderError::NotYourArmy(_))
    ));
    assert!(matches!(
        state.submit_order(
            &data,
            Order::MoveArmy {
                army: own.clone(),
                target: sim_campaign::MoveOrderTarget::Path(vec![])
            }
        ),
        Err(OrderError::EmptyPath)
    ));
    // Lot M2: over the sea, only port-to-port crossings (`embark`).
    assert!(matches!(
        state.submit_order(
            &data,
            Order::move_along(own.clone(), vec![set("set_cantorbery")])
        ),
        Err(OrderError::NoPath)
    ));
    assert!(matches!(
        state.submit_order(
            &data,
            Order::MoveArmy {
                army: ArmyId::parse("army_9999").unwrap(),
                target: sim_campaign::MoveOrderTarget::Path(vec![prov("prov_kent").into()])
            }
        ),
        Err(OrderError::UnknownArmy(_))
    ));
    assert!(matches!(
        state.submit_order(
            &data,
            Order::Recruit {
                settlement: prov("prov_kent").into(),
                unit_type: unit("unit_urban_militia")
            }
        ),
        Err(OrderError::RecruitUnavailable(_))
    ));
    assert!(matches!(
        state.submit_order(
            &data,
            Order::Recruit {
                settlement: set("set_paris").into(),
                unit_type: unit("unit_longbowmen")
            }
        ),
        Err(OrderError::RecruitUnavailable(_))
    ));
    assert!(matches!(
        state.submit_order(
            &data,
            Order::CreateArmy {
                settlement: prov("prov_ile_de_france").into(),
                units_from_garrison: vec![9],
                general: None
            }
        ),
        Err(OrderError::InvalidUnitIndex(9))
    ));
    assert!(matches!(
        state.submit_order(
            &data,
            Order::SplitArmy {
                army: own.clone(),
                unit_indices: (0..8).collect()
            }
        ),
        Err(OrderError::WouldEmptyArmy)
    ));
    assert!(matches!(
        state.submit_order(
            &data,
            Order::AssignGeneral {
                army: own,
                character: data_model::CharacterId::new("chr_edward_iii").unwrap()
            }
        ),
        Err(OrderError::CharacterUnavailable)
    ));
    assert_eq!(
        state.save_json(),
        before,
        "rejected orders leave the state untouched"
    );
}

#[test]
fn valid_orders_apply_immediately() {
    let data = data();
    let mut state = france(&data, 1);
    let paris = set("set_paris");
    let treasury_before = state.faction_state(&fac("fac_france")).unwrap().treasury;

    let options = state.recruitable(&data, &paris);
    let militia = options
        .iter()
        .find(|o| o.unit_type == unit("unit_urban_militia"))
        .unwrap();
    assert!(militia.available, "{:?}", militia.reason);
    assert_eq!(militia.cost, 300);
    let longbow = options
        .iter()
        .find(|o| o.unit_type == unit("unit_longbowmen"))
        .unwrap();
    assert!(!longbow.available);
    assert!(longbow.reason.as_deref().unwrap().contains("technologie"));
    let genoese = options
        .iter()
        .find(|o| o.unit_type == unit("unit_genoese_crossbowmen"))
        .unwrap();
    // TW2-T3 (ADR 0103): Genoese companies are hired by an army from the
    // regional reserve, never levied in a town.
    assert!(
        !genoese.available,
        "towns no longer levy Genoese: {:?}",
        genoese.reason
    );

    state
        .submit_order(
            &data,
            Order::Recruit {
                settlement: paris.clone().into(),
                unit_type: unit("unit_urban_militia"),
            },
        )
        .unwrap();
    assert_eq!(
        state.faction_state(&fac("fac_france")).unwrap().treasury,
        treasury_before - 300
    );
    assert_eq!(
        state.settlement_state(&paris).unwrap().garrison.len(),
        4,
        "recruit lands next turn"
    );
    state.end_turn_with(&data, idle);
    assert_eq!(state.settlement_state(&paris).unwrap().garrison.len(), 5);

    // Form an army from the garrison, split it, merge it back, change stance.
    let commander = data_model::CharacterId::new("chr_raoul_de_brienne").unwrap();
    state
        .submit_order(
            &data,
            Order::CreateArmy {
                settlement: paris.clone().into(),
                units_from_garrison: vec![0, 1, 4],
                general: Some(commander.clone()),
            },
        )
        .unwrap();
    let new_army = state.armies().keys().last().cloned().unwrap();
    assert_eq!(state.army(&new_army).unwrap().units.len(), 3);
    assert_eq!(state.army(&new_army).unwrap().general, Some(commander));
    assert_eq!(state.settlement_state(&paris).unwrap().garrison.len(), 2);

    state
        .submit_order(
            &data,
            Order::SplitArmy {
                army: new_army.clone(),
                unit_indices: vec![2],
            },
        )
        .unwrap();
    let detached = state.armies().keys().last().cloned().unwrap();
    assert_ne!(detached, new_army);
    assert_eq!(state.army(&new_army).unwrap().units.len(), 2);
    assert_eq!(state.army(&detached).unwrap().units.len(), 1);

    state
        .submit_order(
            &data,
            Order::MergeArmies {
                source: detached.clone(),
                target: new_army.clone(),
            },
        )
        .unwrap();
    assert!(state.army(&detached).is_none());
    assert_eq!(state.army(&new_army).unwrap().units.len(), 3);

    state
        .submit_order(
            &data,
            Order::SetStance {
                army: new_army.clone(),
                stance: Stance::Raid,
            },
        )
        .unwrap();
    assert_eq!(state.army(&new_army).unwrap().stance, Stance::Raid);

    state
        .submit_order(
            &data,
            Order::DisbandUnit {
                army: Some(new_army.clone()),
                settlement: None,
                unit_index: 0,
            },
        )
        .unwrap();
    assert_eq!(state.army(&new_army).unwrap().units.len(), 2);
}

#[test]
fn orders_round_trip_through_snake_case_json() {
    let order: Order = serde_json::from_str(
        r#"{"type":"move_army","army":"army_0001","path":["prov_normandie"]}"#,
    )
    .unwrap();
    assert_eq!(
        order,
        Order::MoveArmy {
            army: ArmyId::parse("army_0001").unwrap(),
            target: sim_campaign::MoveOrderTarget::Path(vec![prov("prov_normandie").into()])
        }
    );
    // Lot M2: a point or a settlement as the target.
    let order: Order = serde_json::from_str(
        r#"{"type":"move_army","army":"army_0001","target":{"x":1024.5,"y":2000.0}}"#,
    )
    .unwrap();
    assert_eq!(
        order,
        Order::move_to_point(ArmyId::parse("army_0001").unwrap(), [1024.5, 2000.0])
    );
    let order: Order =
        serde_json::from_str(r#"{"type":"move_army","army":"army_0001","target":"set_rouen"}"#)
            .unwrap();
    assert_eq!(
        order,
        Order::move_to(ArmyId::parse("army_0001").unwrap(), set("set_rouen"))
    );
    let attack: Order =
        serde_json::from_str(r#"{"type":"attack","army":"army_0001","target_army":"army_0002"}"#)
            .unwrap();
    assert!(matches!(attack, Order::Attack { .. }));
    // Lot C4: settlements, and the v1 `province` field as an alias.
    let recruit: Order = serde_json::from_str(
        r#"{"type":"recruit","province":"prov_kent","unit_type":"unit_urban_militia"}"#,
    )
    .unwrap();
    assert!(matches!(
        recruit,
        Order::Recruit {
            settlement: sim_campaign::Place::Province(_),
            ..
        }
    ));
    let build: Order = serde_json::from_str(
        r#"{"type":"build","settlement":"set_dover","building":"bld_market"}"#,
    )
    .unwrap();
    assert!(matches!(
        build,
        Order::Build {
            settlement: sim_campaign::Place::Settlement(_),
            ..
        }
    ));
    let json = serde_json::to_string(&Order::SetStance {
        army: ArmyId::parse("army_0002").unwrap(),
        stance: Stance::Siege,
    })
    .unwrap();
    assert_eq!(
        json,
        r#"{"type":"set_stance","army":"army_0002","stance":"siege"}"#
    );
    let create: Order = serde_json::from_str(
        r#"{"type":"create_army","province":"prov_kent","units_from_garrison":[0,1]}"#,
    )
    .unwrap();
    assert!(matches!(create, Order::CreateArmy { general: None, .. }));
}

#[test]
fn movement_spans_turns_and_reachable_is_bounded() {
    let data = data();
    let mut state = france(&data, 1);
    let army = main_army(&state, "fac_france");
    let allowance = state.army(&army).unwrap().movement_left;
    // Lot C7a: 3 steps of 70 km (lot DC1, ADR 0082) times the season scale
    // (0.5) in spring, converted to grid costs (lot M2: 10 per plain cell of
    // ~1.44 km).
    assert_eq!(state.season_movement_points(&data), 105);
    assert_eq!(
        allowance,
        sim_campaign::march::km_to_grid_points(&data, 105.0)
    );
    assert!((700..=760).contains(&allowance), "{allowance}");
    let reachable = state.reachable(&data, &army);
    assert!(reachable.contains_key(&set("set_saint_denis")));
    assert!(reachable
        .values()
        .all(|&cost| cost >= 1 && cost <= allowance));
    assert!(!reachable.contains_key(&set("set_paris")));
    let provinces = state.reachable_provinces(&data, &army);
    assert!(provinces.contains_key(&prov("prov_normandie")));
    assert!(!provinces.contains_key(&prov("prov_ile_de_france")));

    // Toulouse is several turns away.
    let toulouse = city(&state, "prov_toulousain");
    let point = data.settlement_point(&toulouse).unwrap();
    let path = state
        .find_path(&data, &army, point)
        .expect("path to Toulouse");
    assert!(path.cost > 2 * allowance, "cost {}", path.cost);
    assert_eq!(path.waypoints.last(), path.cells.last());
    let outcome = state
        .submit_order_outcome(&data, Order::move_to(army.clone(), toulouse.clone()))
        .unwrap();
    let sim_campaign::OrderOutcome::Moved(report) = outcome else {
        panic!("a march report");
    };
    assert_eq!(report.stop, sim_campaign::StopReason::OutOfMovement);
    assert!(!report.walked.is_empty());
    assert!(report.cost <= allowance);
    let after_one = state.army(&army).unwrap();
    assert!(after_one.settlement().is_none(), "in the field");
    assert!(!after_one.planned_path.is_empty(), "the rest waits");
    assert!(after_one.movement_left < allowance);
    for _ in 0..6 {
        if state.army(&army).unwrap().is_at(&toulouse) {
            break;
        }
        assert!(
            !state.army(&army).unwrap().planned_path.is_empty(),
            "leftover path continues next turn"
        );
        state.end_turn_with(&data, idle);
    }
    assert!(state.army(&army).unwrap().is_at(&toulouse));
    assert!(state.army(&army).unwrap().planned_path.is_empty());
    assert_eq!(
        state
            .character(&data_model::CharacterId::new("chr_philippe_vi").unwrap())
            .unwrap()
            .location,
        Some(prov("prov_toulousain"))
    );
}

#[test]
fn v1_province_paths_head_for_the_city() {
    let data = data();
    let mut state = france(&data, 1);
    let army = main_army(&state, "fac_france");
    state
        .submit_order(
            &data,
            Order::MoveArmy {
                army: army.clone(),
                target: sim_campaign::MoveOrderTarget::Path(vec![prov("prov_normandie").into()]),
            },
        )
        .unwrap();
    let rouen = set("set_rouen");
    let army = state.army(&army).unwrap();
    assert!(
        army.is_at(&rouen)
            || army.destination == Some(sim_campaign::MoveTarget::Settlement(rouen.clone())),
        "{:?}",
        army.position
    );
}

#[test]
fn sea_crossings_go_port_to_port_and_take_the_turn() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_england"), 2).unwrap();
    let sea_edges: Vec<_> = data
        .movement_graph
        .adjacency
        .iter()
        .flat_map(|(from, edges)| edges.iter().map(move |e| (from, e)))
        .filter(|(_, e)| e.sea)
        .collect();
    assert!(!sea_edges.is_empty());
    for (from, edge) in &sea_edges {
        assert!(data.settlements[*from].port, "{from} is a port");
        assert!(data.settlements[&edge.to].port, "{} is a port", edge.to);
    }
    let army = main_army(&state, "fac_england");
    // An English port with a crossing to the Boulonnais.
    let (from, to) = sea_edges
        .iter()
        .find(|(from, edge)| {
            state.is_friendly_settlement(&fac("fac_england"), from)
                && state.settlement_province(&edge.to) == Some(&prov("prov_boulonnais"))
        })
        .map(|(from, edge)| ((*from).clone(), edge.to.clone()))
        .expect("a Channel crossing to the Boulonnais");
    // Not in a port: refused.
    let embark = |army: &ArmyId, to: &SettlementId| Order::Embark {
        army: army.clone(),
        to_port: to.clone(),
    };
    state.armies.get_mut(&army).unwrap().position =
        sim_campaign::ArmyPosition::field(data.settlement_point(&from).unwrap());
    assert!(matches!(
        state.submit_order(&data, embark(&army, &to)),
        Err(OrderError::NotInPort)
    ));
    state.armies.get_mut(&army).unwrap().position = sim_campaign::ArmyPosition::Settlement(from);
    // Not a full turn of movement left: refused.
    state.armies.get_mut(&army).unwrap().movement_left -= 1;
    assert!(matches!(
        state.submit_order(&data, embark(&army, &to)),
        Err(OrderError::NoMovementLeft)
    ));
    state.armies.get_mut(&army).unwrap().movement_left += 1;
    let before = state.army(&army).unwrap().total_strength();
    // Lot NV1: no French warship to intercept this crossing.
    state.naval.initialised = true;
    state.naval.fleets.clear();
    state.submit_order(&data, embark(&army, &to)).unwrap();
    let landed = state.army(&army).unwrap();
    assert!(landed.is_at(&to), "landed in {to}: {:?}", landed.position);
    assert_eq!(landed.movement_left, 0, "the crossing takes the turn");
    assert!(
        landed.total_strength() < before,
        "landing on a hostile shore costs men"
    );
}

#[test]
fn attacking_an_enemy_army_triggers_a_battle() {
    let data = data();
    let mut state = france(&data, 3);
    // Auto-resolved battle (interactive battles are covered by tests/m7.rs).
    state.interactive_battles = false;
    let french = main_army(&state, "fac_france");
    let english = main_army(&state, "fac_england");
    // Teleport the English army next door for the test.
    let saint_denis = set("set_saint_denis");
    state.armies.get_mut(&english).unwrap().position =
        sim_campaign::ArmyPosition::Settlement(saint_denis.clone());
    let strength_before: u32 = state.army(&french).unwrap().total_strength();
    state
        .submit_order(
            &data,
            Order::Attack {
                army: french.clone(),
                target_army: english.clone(),
            },
        )
        .unwrap();
    let battle = state
        .pending_events
        .iter()
        .find(|e| e.kind == EventKind::Battle)
        .expect("a battle happened")
        .clone();
    assert_eq!(battle.province, Some(prov("prov_ile_de_france")));
    assert!(battle.text_fr.contains("Bataille"));
    let french_after = state.army(&french).unwrap();
    assert!(french_after.total_strength() < strength_before);
    assert_eq!(french_after.movement_left, 0, "no move after a battle");
    assert!(
        french_after.planned_path.is_empty(),
        "both armies stop after a battle"
    );
    // Out of reach: refused, nothing happens.
    let mut far = france(&data, 3);
    far.interactive_battles = false;
    let before = far.save_json();
    let err = far
        .submit_order(
            &data,
            Order::Attack {
                army: main_army(&far, "fac_france"),
                target_army: main_army(&far, "fac_england"),
            },
        )
        .unwrap_err();
    assert!(
        matches!(err, OrderError::OutOfRange | OrderError::NoPath),
        "{err:?}"
    );
    assert_eq!(far.save_json(), before);
}
#[test]
fn siege_captures_after_fortification_dependent_duration() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_england"), 4).unwrap();
    let english = main_army(&state, "fac_england");
    let boulogne = set("set_boulogne");
    let fortification = state.fortification_level(&data, &boulogne);
    state.armies.get_mut(&english).unwrap().position =
        sim_campaign::ArmyPosition::Settlement(boulogne.clone());
    state
        .submit_order(
            &data,
            Order::SetStance {
                army: english.clone(),
                stance: Stance::Siege,
            },
        )
        .unwrap();

    let events = state.end_turn_with(&data, idle);
    assert!(events.iter().any(|e| e.kind == EventKind::SiegeStarted));
    let siege = state
        .settlement_state(&boulogne)
        .unwrap()
        .siege
        .clone()
        .expect("siege on");
    assert_eq!(siege.attacker, fac("fac_england"));
    assert_eq!(siege.turns_left, 2 + fortification);

    let mut captured_turn = None;
    for turn in 1..=(2 + fortification + 1) {
        let events = state.end_turn_with(&data, idle);
        if events.iter().any(|e| e.kind == EventKind::ProvinceCaptured) {
            captured_turn = Some(turn);
            break;
        }
    }
    assert_eq!(
        captured_turn,
        Some(2 + fortification),
        "capture after 2 + fortification turns of siege"
    );
    let after = state.settlement_state(&boulogne).unwrap();
    assert_eq!(after.controller, fac("fac_england"));
    assert_eq!(after.owner, fac("fac_france"), "de jure owner unchanged");
    assert!(after.garrison.is_empty());
    assert!(after.siege.is_none());
    // Lot C4: taking the city hands over the province (derived control).
    assert_eq!(
        state.province_controller(&prov("prov_boulonnais")),
        Some(&fac("fac_england"))
    );
    assert_eq!(
        state.province_owner(&prov("prov_boulonnais")),
        Some(&fac("fac_france"))
    );
}

#[test]
fn empty_garrison_is_captured_instantly() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_england"), 5).unwrap();
    let english = main_army(&state, "fac_england");
    let boulogne = set("set_boulogne");
    state
        .settlements
        .get_mut(&boulogne)
        .unwrap()
        .garrison
        .clear();
    state.armies.get_mut(&english).unwrap().position =
        sim_campaign::ArmyPosition::Settlement(boulogne.clone());
    state
        .submit_order(
            &data,
            Order::SetStance {
                army: english.clone(),
                stance: Stance::Siege,
            },
        )
        .unwrap();
    let events = state.end_turn_with(&data, idle);
    assert!(events.iter().any(|e| e.kind == EventKind::ProvinceCaptured));
    assert_eq!(
        state.settlement_state(&boulogne).unwrap().controller,
        fac("fac_england")
    );
}

#[test]
fn raid_devastates_and_loots() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_england"), 6).unwrap();
    let english = main_army(&state, "fac_england");
    let boulogne = prov("prov_boulonnais");
    state.armies.get_mut(&english).unwrap().position =
        sim_campaign::ArmyPosition::Settlement(city(&state, "prov_boulonnais"));
    state
        .submit_order(
            &data,
            Order::SetStance {
                army: english.clone(),
                stance: Stance::Raid,
            },
        )
        .unwrap();
    let events = state.end_turn_with(&data, idle);
    assert!(events.iter().any(|e| e.kind == EventKind::Raid));
    let province = state.province_state(&boulogne).unwrap();
    assert!(
        province.devastation >= 20,
        "devastation {}",
        province.devastation
    );
}

#[test]
fn france_income_is_positive_and_in_target_range() {
    let data = data();
    let mut state = france(&data, 7);
    let france_id = fac("fac_france");
    let income = state.faction_income(&data, &france_id);
    let upkeep = state.faction_upkeep(&data, &france_id);
    let army = main_army(&state, "fac_france");
    let army_upkeep = sim_campaign::economy::units_upkeep(&data, &state.army(&army).unwrap().units);
    println!("France income {income}, upkeep {upkeep}, main army upkeep {army_upkeep}");
    assert!(
        (20_000..=30_000).contains(&income),
        "France income {income}"
    );
    let share = army_upkeep as f64 / income as f64;
    assert!((0.08..=0.18).contains(&share), "8-unit army share {share}");
    assert!(income > upkeep, "France must run a surplus at the start");

    // The actually-collected income (spec § 1.4: tax rate + building
    // effects) stays in the same target range with the default Normal rate.
    let effective_income = state.faction_income_effective(&data, &france_id);
    println!("France effective income (Normal tax) {effective_income}");
    assert!(
        (20_000..=30_000).contains(&effective_income),
        "France effective income {effective_income}"
    );

    let treasury_before = state.faction_state(&france_id).unwrap().treasury;
    let effective_upkeep = state.faction_army_upkeep(&data, &france_id)
        + state.faction_building_upkeep(&data, &france_id)
        + state.faction_administration_upkeep(&data, &france_id);
    let events = state.end_turn_with(&data, idle);
    assert!(events.iter().any(|e| e.kind == EventKind::Income));
    let summary = state.faction_summary(&france_id).unwrap();
    assert_eq!(summary.income, effective_income);
    // M5: vassals pay 10 % of their income as tribute.
    let tribute: i64 = state
        .factions
        .values()
        .filter(|f| f.suzerain.as_ref() == Some(&france_id))
        .map(|f| f.income_last_turn * data.feudal_rules.vassal_tribute_percent / 100)
        .sum();
    assert!(tribute > 0, "Burgundy, Brittany and Flanders pay tribute");
    // C5: trade routes touching a French hub (Troyes, Provins) settle after
    // the tax income, in their own treasury line.
    let trade_income = state.factions[&france_id].trade_income_last_turn;
    assert_eq!(
        summary.treasury,
        treasury_before + effective_income - effective_upkeep + tribute + trade_income
    );
    // FE4a: prov_bourbonnais and prov_bearn moved to fac_bourbon/fac_foix_bearn,
    // prov_valois (new) stays with the crown: 27 - 2 + 1 = 26.
    assert_eq!(summary.provinces_count, 26);
}

#[test]
fn attrition_abroad_and_recovery_at_home() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_england"), 8).unwrap();
    let english = main_army(&state, "fac_england");
    state.armies.get_mut(&english).unwrap().position =
        sim_campaign::ArmyPosition::Settlement(set("set_boulogne"));
    let strength_before = state.army(&english).unwrap().total_strength();
    let mut supplies = Vec::new();
    for _ in 0..7 {
        state.end_turn_with(&data, idle);
        supplies.push(state.army(&english).unwrap().supply);
    }
    assert!(supplies[0] < 100);
    assert_eq!(*supplies.last().unwrap(), 0);
    assert!(
        state.army(&english).unwrap().total_strength() < strength_before,
        "starvation costs men"
    );

    state.armies.get_mut(&english).unwrap().position =
        sim_campaign::ArmyPosition::Settlement(set("set_cantorbery"));
    state.end_turn_with(&data, idle);
    assert!(state.army(&english).unwrap().supply >= 40);
}

#[test]
fn save_load_round_trip() {
    let data = data();
    let mut state = france(&data, 9);
    for _ in 0..3 {
        state.end_turn(&data);
    }
    let json = state.save_json();
    assert!(json.contains(&format!(
        "\"state_version\":{}",
        sim_campaign::STATE_VERSION
    )));
    assert!(json.contains("\"tax_rate\""), "new field round-trips");
    let loaded = CampaignState::load_json(&json).unwrap();
    assert_eq!(loaded, state);
    assert_eq!(loaded.save_json(), json);
    assert_eq!(loaded.date_label(), "Hiver 1337");

    // A loaded game continues exactly like the original.
    let mut a = state.clone();
    let mut b = loaded;
    a.end_turn(&data);
    b.end_turn(&data);
    assert_eq!(a.save_json(), b.save_json());

    let err = CampaignState::load_json(&json.replace(
        &format!("\"state_version\":{}", sim_campaign::STATE_VERSION),
        "\"state_version\":1",
    ))
    .unwrap_err();
    assert!(matches!(
        err,
        sim_campaign::CampaignError::PreSettlementSave { found: 1, .. }
    ));
    // Lot M2: a v5 save (armies on settlements) is refused too.
    let err = CampaignState::load_json(&json.replace(
        &format!("\"state_version\":{}", sim_campaign::STATE_VERSION),
        "\"state_version\":5",
    ))
    .unwrap_err();
    assert!(matches!(
        err,
        sim_campaign::CampaignError::PreFreeMovementSave { found: 5, .. }
    ));
    assert!(err.to_string().contains("mouvement libre"), "{err}");
    // Lot C4: a v4 save is refused with an explicit message.

    let err = CampaignState::load_json(&json.replace(
        &format!("\"state_version\":{}", sim_campaign::STATE_VERSION),
        "\"state_version\":4",
    ))
    .unwrap_err();
    assert!(err
        .to_string()
        .contains("antérieure à la refonte des colonies"));
    let err = CampaignState::load_json(&json.replace(
        &format!("\"state_version\":{}", sim_campaign::STATE_VERSION),
        "\"state_version\":99",
    ))
    .unwrap_err();
    assert!(matches!(
        err,
        sim_campaign::CampaignError::VersionMismatch { found: 99, .. }
    ));
    assert!(CampaignState::load_json("not json").is_err());
}

#[test]
fn twenty_turns_with_ai_are_deterministic() {
    let data = data();
    let run = |seed: u64| {
        let mut state = france(&data, seed);
        let army = main_army(&state, "fac_france");
        state
            .submit_order(
                &data,
                Order::MoveArmy {
                    army,
                    target: sim_campaign::MoveOrderTarget::Path(
                        vec![prov("prov_normandie").into()],
                    ),
                },
            )
            .unwrap();
        for _ in 0..20 {
            state.end_turn(&data);
        }
        state.save_json()
    };
    assert_eq!(run(1337), run(1337));
    assert_ne!(run(1337), run(1338), "the seed matters");
}

#[test]
fn forty_turns_all_ai_change_the_map() {
    let data = data();
    let mut state = france(&data, 21);
    let mut changed = 0;
    let mut battles = 0;
    let mut captures = 0;
    for _ in 0..40 {
        let events = state.end_turn(&data);
        battles += events
            .iter()
            .filter(|e| e.kind == EventKind::Battle)
            .count();
        captures += events
            .iter()
            .filter(|e| e.kind == EventKind::ProvinceCaptured)
            .count();
    }
    for id in state.provinces.keys() {
        if state.province_controller(id) != state.province_owner(id) {
            changed += 1;
        }
    }
    println!(
        "40 turns: {battles} battles, {captures} captures, {changed} provinces occupied, {} armies",
        state.armies().len()
    );
    for (id, faction) in &state.factions {
        println!(
            "  {id}: treasury {} income {} upkeep {} alive {}",
            faction.treasury, faction.income_last_turn, faction.upkeep_last_turn, faction.alive
        );
    }
    assert_eq!(state.date_label(), "Printemps 1347");
    assert!(
        captures >= 1,
        "at least one province must change hands in 40 turns"
    );
    assert!(state.factions.values().filter(|f| f.alive).count() >= 10);
}

#[test]
fn unit_fresh_uses_data_stats() {
    let data = data();
    let knights = Unit::fresh(&data.unit_types[&unit("unit_knights")]);
    assert_eq!(knights.strength, 60);
    assert_eq!(knights.morale, 80);
}

#[test]
fn succession_follows_heir_then_house_then_none() {
    let data = data();
    let mut state = france(&data, 10);
    let philippe = data_model::CharacterId::new("chr_philippe_vi").unwrap();
    let jean = data_model::CharacterId::new("chr_jean_de_normandie").unwrap();
    let mut events = Vec::new();
    sim_campaign::characters::kill(&mut state, &data, &philippe, &mut events);
    assert!(events.iter().any(|e| e.kind == EventKind::Death));
    assert!(events.iter().any(|e| e.kind == EventKind::Succession));
    let france_state = state.faction_state(&fac("fac_france")).unwrap();
    assert_eq!(france_state.ruler, Some(jean.clone()));
    assert!(!state.character(&philippe).unwrap().alive);
    let army = main_army(&state, "fac_france");
    assert_eq!(
        state.army(&army).unwrap().general,
        None,
        "dead general leaves the army"
    );

    // Jean dies too: the eldest living Valois male takes over.
    let mut events = Vec::new();
    sim_campaign::characters::kill(&mut state, &data, &jean, &mut events);
    let ruler = state
        .faction_state(&fac("fac_france"))
        .unwrap()
        .ruler
        .clone()
        .expect("house heir");
    assert_eq!(state.character(&ruler).unwrap().house, "Valois");

    // Navarre (cognatic): the queen's son inherits although he belongs to
    // his father's house (Évreux).
    let jeanne = data_model::CharacterId::new("chr_jeanne_ii_de_navarre").unwrap();
    let charles = data_model::CharacterId::new("chr_charles_ii_de_navarre").unwrap();
    let mut events = Vec::new();
    sim_campaign::characters::kill(&mut state, &data, &jeanne, &mut events);
    assert_eq!(
        state.faction_state(&fac("fac_navarre")).unwrap().ruler,
        Some(charles)
    );

    // Kill every ruler in turn: once the house is gone, a new lord takes
    // over (M5: a realm with land never simply vanishes).
    let mut new_house = false;
    for _ in 0..10 {
        let ruler = state
            .faction_state(&fac("fac_navarre"))
            .unwrap()
            .ruler
            .clone()
            .unwrap();
        let mut events = Vec::new();
        sim_campaign::characters::kill(&mut state, &data, &ruler, &mut events);
        if events.iter().any(|e| {
            e.kind == EventKind::Succession
                && (e.text_fr.contains("nouvelle maison") || e.text_fr.contains("s'empare"))
        }) {
            new_house = true;
            break;
        }
    }
    assert!(new_house, "a new ruler once the line is extinct");
    let navarre = state.faction_state(&fac("fac_navarre")).unwrap();
    assert!(navarre.alive && navarre.ruler.is_some());
}

#[test]
fn natural_death_probability_by_age() {
    use sim_campaign::characters::death_permille;
    assert_eq!(death_permille(39), 0);
    assert_eq!(death_permille(40), 5);
    assert_eq!(death_permille(60), 20);
    assert_eq!(death_permille(75), 80);
}
