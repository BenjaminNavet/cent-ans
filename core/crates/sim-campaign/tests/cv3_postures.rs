//! Lot CV3-1: army stances (spec `docs/design/2026-09-27-campagne-vivante.md`
//! § 1): ambush, forced march, entrenched camp.
//!
//! The rules run on the real game data over an all-plain synthetic grid and
//! a synthetic cover map (forest where the tests say), so that the geometry
//! is known.

use std::path::PathBuf;

use data_model::{CoverMap, FactionId, GameData, MapRasters, NavGrid, SettlementId, PLAIN_COST};
use sim_battle::{BattleOpening, SideId};
use sim_campaign::agents::Agent;
use sim_campaign::posture;
use sim_campaign::{
    ArmyId, ArmyPosition, CampaignRng, CampaignState, EventKind, MoveOrderTarget, Order,
    OrderError, Stance, StopReason,
};

fn real_data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn px_per_km(data: &GameData) -> f32 {
    data.navgrid().px_per_km() as f32
}

fn east(data: &GameData, point: [f32; 2], km: f32) -> [f32; 2] {
    [point[0] + km * px_per_km(data), point[1]]
}

fn distance_km(data: &GameData, a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt() / px_per_km(data)
}

/// A point of central France with no settlement within `radius_km`.
fn empty_spot(data: &GameData, radius_km: f32) -> [f32; 2] {
    let settlements: Vec<[f32; 2]> = data
        .settlements
        .keys()
        .filter_map(|id| data.settlement_point(id))
        .collect();
    for y in (2000..2600).step_by(8) {
        for x in (1900..2500).step_by(8) {
            let p = [x as f32, y as f32];
            if settlements
                .iter()
                .all(|s| distance_km(data, *s, p) > radius_km)
            {
                return p;
            }
        }
    }
    panic!("no empty spot");
}

/// Real data on an all-plain grid, with forest on the cells within one cell
/// of each point of `forests`.
fn data_with_forest(forests: &[[f32; 2]]) -> GameData {
    let mut data = real_data();
    data.set_map_rasters(MapRasters {
        navgrid: NavGrid::uniform(2048, 2048, 2, 1.438, PLAIN_COST),
        provinces: None,
    });
    let mut cover = CoverMap::open(2048, 2048, 2);
    for point in forests {
        for dy in -1..=1 {
            for dx in -1..=1 {
                cover.set(
                    [point[0] + dx as f32 * 2.0, point[1] + dy as f32 * 2.0],
                    255,
                    0,
                );
            }
        }
    }
    data.set_cover_map(Some(cover));
    data
}

fn main_army(state: &CampaignState, faction: &str) -> ArmyId {
    let faction = fac(faction);
    state
        .armies
        .iter()
        .filter(|(_, a)| a.faction == faction)
        .max_by_key(|(id, a)| (a.units.len(), std::cmp::Reverse((*id).clone())))
        .map(|(id, _)| id.clone())
        .expect("faction has an army")
}

fn refill(state: &mut CampaignState, data: &GameData, id: &ArmyId) {
    let allowance = state.army_grid_allowance(data, &state.armies[id]);
    state.armies.get_mut(id).unwrap().movement_left = allowance;
}

/// France (player) and England at war, one field army each.
fn duel(
    data: &GameData,
    french_at: [f32; 2],
    english_at: [f32; 2],
) -> (CampaignState, ArmyId, ArmyId) {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 5).unwrap();
    state.interactive_battles = false;
    let french = main_army(&state, "fac_france");
    let english = main_army(&state, "fac_england");
    state.armies.retain(|id, _| *id == french || *id == english);
    state.armies.get_mut(&french).unwrap().position = ArmyPosition::field(french_at);
    state.armies.get_mut(&english).unwrap().position = ArmyPosition::field(english_at);
    for id in [&french, &english] {
        refill(&mut state, data, id);
    }
    assert!(state.is_at_war(&fac("fac_france"), &fac("fac_england")));
    (state, french, english)
}

fn set_stance(state: &mut CampaignState, data: &GameData, army: &ArmyId, stance: Stance) {
    let faction = state.armies[army].faction.clone();
    state
        .apply_order(
            data,
            &faction,
            Order::SetStance {
                army: army.clone(),
                stance,
            },
        )
        .unwrap_or_else(|e| panic!("{stance:?} refused: {e}"));
}

fn refusal(
    state: &mut CampaignState,
    data: &GameData,
    army: &ArmyId,
    stance: Stance,
) -> OrderError {
    let faction = state.armies[army].faction.clone();
    state
        .apply_order(
            data,
            &faction,
            Order::SetStance {
                army: army.clone(),
                stance,
            },
        )
        .expect_err("stance refused")
}

fn reason(error: &OrderError) -> String {
    match error {
        OrderError::StanceRefused(reason) => reason.clone(),
        other => panic!("expected a stance refusal, got {other:?}"),
    }
}

/// Forces the ambush success chance to `chance` (both bounds).
fn force_chance(data: &mut GameData, chance: f64) {
    let ambush = &mut data.posture_rules.ambush;
    ambush.chance_min = chance;
    ambush.chance_max = chance;
}

// ------------------------------------------------------------------ ambush

#[test]
fn ambush_needs_cover_and_movement_and_spends_it() {
    let probe = real_data();
    let spot = empty_spot(&probe, 25.0);
    let data = data_with_forest(&[spot]);
    let (mut state, french, _) = duel(&data, east(&data, spot, 20.0), [10.0, 10.0]);
    // In the open: refused, the reason names the cover.
    let error = refusal(&mut state, &data, &french, Stance::Ambush);
    assert!(reason(&error).contains("couvert"), "{error}");
    // The options tell the same.
    let options = posture::stance_options(&state, &data, &french);
    let ambush = options.iter().find(|(s, _)| *s == Stance::Ambush).unwrap();
    assert!(ambush.1.is_err());
    assert!(options
        .iter()
        .any(|(s, r)| *s == Stance::Normal && r.is_ok()));
    // In the forest with its movement: accepted, movement spent.
    state.armies.get_mut(&french).unwrap().position = ArmyPosition::field(spot);
    set_stance(&mut state, &data, &french, Stance::Ambush);
    assert_eq!(state.armies[&french].stance, Stance::Ambush);
    assert_eq!(state.armies[&french].movement_left, 0);
    // Too little movement left: refused.
    state.armies.get_mut(&french).unwrap().stance = Stance::Normal;
    let base = state.army_base_grid_allowance(&data, &state.armies[&french]);
    state.armies.get_mut(&french).unwrap().movement_left = base / 10;
    let error = refusal(&mut state, &data, &french, Stance::Ambush);
    assert!(reason(&error).contains("mouvement"), "{error}");
    // Not in a settlement.
    let settlement = state.settlements.keys().next().unwrap().clone();
    refill(&mut state, &data, &french);
    state.armies.get_mut(&french).unwrap().position = ArmyPosition::Settlement(settlement);
    let error = refusal(&mut state, &data, &french, Stance::Ambush);
    assert!(reason(&error).contains("rase campagne"), "{error}");
}

#[test]
fn moving_leaves_the_ambush() {
    let probe = real_data();
    let spot = empty_spot(&probe, 25.0);
    let data = data_with_forest(&[spot]);
    let (mut state, french, _) = duel(&data, spot, [10.0, 10.0]);
    set_stance(&mut state, &data, &french, Stance::Ambush);
    // Next turn: movement back, the ambush holds until the army moves.
    state.end_turn_with(&data, |_, _, _| Vec::new());
    assert_eq!(state.armies[&french].stance, Stance::Ambush);
    assert!(state.armies[&french].movement_left > 0);
    state
        .submit_order(
            &data,
            Order::move_to_point(french.clone(), east(&data, spot, 5.0)),
        )
        .unwrap();
    assert_eq!(state.armies[&french].stance, Stance::Normal);
}

#[test]
fn an_ambush_is_hidden_until_an_army_or_a_spy_comes_close() {
    let probe = real_data();
    let spot = empty_spot(&probe, 25.0);
    let data = data_with_forest(&[spot]);
    let (mut state, french, english) = duel(&data, spot, east(&data, spot, 20.0));
    let england = fac("fac_england");
    let sees = |state: &CampaignState, viewer: &FactionId| {
        state
            .vision(&data, viewer)
            .sees_army(state, &data, &state.armies[&french])
    };
    assert!(sees(&state, &england), "20 km away, in plain sight");
    set_stance(&mut state, &data, &french, Stance::Ambush);
    assert!(!sees(&state, &england), "hidden in the forest");
    assert!(sees(&state, &fac("fac_france")), "the owner sees it");
    assert!(posture::is_hidden_from(
        &state,
        &data,
        &state.armies[&french],
        &england
    ));
    // An English army 2 km away finds it.
    state.armies.get_mut(&english).unwrap().position = ArmyPosition::field(east(&data, spot, 2.0));
    assert!(sees(&state, &england), "an army 2 km away sees it");
    // Far again, but an English spy in a settlement 6 km away.
    state.armies.get_mut(&english).unwrap().position = ArmyPosition::field(east(&data, spot, 20.0));
    assert!(!sees(&state, &england));
    let (settlement, point) = data
        .settlements
        .keys()
        .filter_map(|id| Some((id.clone(), data.settlement_point(id)?)))
        .find(|(_, p)| distance_km(&data, *p, spot) > 30.0)
        .unwrap();
    let hideout = east(&data, point, 6.0);
    state.armies.get_mut(&french).unwrap().position = ArmyPosition::field(hideout);
    let data = data_with_forest(&[hideout]);
    assert!(posture::is_hidden_from(
        &state,
        &data,
        &state.armies[&french],
        &england
    ));
    spy(&mut state, "fac_england", &settlement, 1);
    assert!(!posture::is_hidden_from(
        &state,
        &data,
        &state.armies[&french],
        &england
    ));
    // A spy 6 km away does not see an ambush 10 km away.
    state.armies.get_mut(&french).unwrap().position = ArmyPosition::field(east(&data, point, 10.0));
    let data = data_with_forest(&[east(&data, point, 10.0)]);
    assert!(posture::is_hidden_from(
        &state,
        &data,
        &state.armies[&french],
        &england
    ));
}

fn spy(state: &mut CampaignState, faction: &str, at: &SettlementId, index: u32) {
    state.agents.agents.insert(
        sim_campaign::AgentId::from_index(index),
        Agent {
            faction: fac(faction),
            kind: data_model::AgentKind::Spy,
            name: "Espion".to_owned(),
            location: at.clone(),
            movement_points: 0,
            experience: 0,
            level: 1,
            acted: false,
            destination: None,
            recruited_turn: 0,
            last_report: None,
        },
    );
}

/// French ambush in a forest `spot`; the English march west through it.
fn ambush_setup(chance: f64, interactive: bool) -> (GameData, CampaignState, ArmyId, ArmyId) {
    let probe = real_data();
    let spot = empty_spot(&probe, 25.0);
    let mut data = data_with_forest(&[spot]);
    force_chance(&mut data, chance);
    let (mut state, french, english) = duel(&data, spot, east(&data, spot, 30.0));
    state.interactive_battles = interactive;
    set_stance(&mut state, &data, &french, Stance::Ambush);
    (data, state, french, english)
}

fn english_march_west(
    state: &mut CampaignState,
    data: &GameData,
    english: &ArmyId,
    french: &ArmyId,
) -> sim_campaign::MoveReport {
    let start = state.army_point(data, &state.armies[english]);
    let target = [start[0] - 60.0 * px_per_km(data), start[1]];
    match state
        .apply_order_outcome(
            data,
            &fac("fac_england"),
            Order::MoveArmy {
                army: english.clone(),
                target: MoveOrderTarget::Point {
                    x: target[0],
                    y: target[1],
                },
            },
        )
        .expect("march accepted")
    {
        sim_campaign::OrderOutcome::Moved(report) => {
            assert_eq!(
                report.stop,
                StopReason::EnemyZoneOfControl {
                    army: french.clone()
                }
            );
            report
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_sprung_ambush_opens_the_battle_in_ambush() {
    let (data, mut state, french, english) = ambush_setup(1.0, true);
    english_march_west(&mut state, &data, &english, &french);
    let texts: Vec<&str> = state
        .pending_events
        .iter()
        .map(|e| e.text_fr.as_str())
        .collect();
    assert!(
        texts.iter().any(|t| t.starts_with("Embuscade !")),
        "{texts:?}"
    );
    // The player's battle waits, the ambusher attacks, the English are the
    // victims.
    assert_eq!(state.pending_battles.len(), 1);
    let request = &state.pending_battles[0];
    assert_eq!(request.attacker, french);
    assert_eq!(request.defender, english);
    assert_eq!(
        request.opening,
        BattleOpening::Ambush {
            victim: SideId::Defender
        }
    );
    let setup = state.battle_setup(&data, 0).expect("setup");
    assert_eq!(setup.opening.ambush_victim(), Some(SideId::Defender));
    // Revealed once sprung.
    assert_eq!(state.armies[&french].stance, Stance::Normal);
    // The request round-trips through a save.
    let saved = state.save_json();
    let restored = CampaignState::load_json(&saved).unwrap();
    assert_eq!(restored.pending_battles[0].opening, request.opening);
}

#[test]
fn a_failed_ambush_is_revealed_and_gives_a_normal_battle() {
    let (data, mut state, french, english) = ambush_setup(0.0, true);
    english_march_west(&mut state, &data, &english, &french);
    assert!(state
        .pending_events
        .iter()
        .any(|e| e.kind == EventKind::Battle && e.text_fr.starts_with("Embuscade éventée")));
    assert_eq!(state.pending_battles.len(), 1);
    let request = &state.pending_battles[0];
    assert_eq!(request.attacker, english, "the marching army falls on it");
    assert_eq!(request.opening, BattleOpening::Standard);
    assert_eq!(state.armies[&french].stance, Stance::Normal);
}

#[test]
fn the_ambush_draw_is_deterministic_with_a_fixed_seed() {
    let run = |seed: u64| {
        let (data, mut state, french, english) = ambush_setup(0.55, false);
        state.rng = CampaignRng::from_seed(seed);
        let mut probe = state.rng.clone();
        let chance = posture::ambush_chance(&state, &data, &french, &english);
        let expected = probe.chance_permille((chance * 1000.0).round() as u32);
        english_march_west(&mut state, &data, &english, &french);
        let sprung = state
            .pending_events
            .iter()
            .any(|e| e.text_fr.starts_with("Embuscade !"));
        assert_eq!(sprung, expected, "seed {seed}");
        // Auto-resolved: the outcome was classified.
        assert!(state.last_battle_outcome.is_some());
        sprung
    };
    let results: Vec<bool> = (1..=12).map(run).collect();
    assert_eq!(results, (1..=12).map(run).collect::<Vec<_>>());
    assert!(
        results.contains(&true) && results.contains(&false),
        "{results:?}"
    );
}

#[test]
fn the_ambush_chance_follows_cover_skill_scouts_and_forced_march() {
    let (data, mut state, french, english) = ambush_setup(0.5, false);
    // Unclamped rules for the formula.
    let mut data = data;
    data.posture_rules.ambush.chance_min = 0.0;
    data.posture_rules.ambush.chance_max = 1.0;
    let rules = data.posture_rules.ambush.clone();
    let skill = posture::ambush_skill(&state, &data, &state.armies[&french]);
    let scouts = posture::scout_share(&data, &state.armies[&english]);
    let expected = rules.base_chance
        + rules.terrain_bonus[&data_model::CoverClass::Forest]
        + rules.per_skill * skill
        - rules.scout_malus * scouts;
    let chance = posture::ambush_chance(&state, &data, &french, &english);
    assert!((chance - expected).abs() < 1e-9, "{chance} vs {expected}");
    // All scouts: lower. Forced march: higher.
    let scout_type = data_model::UnitTypeId::new(&rules.scout_unit_types[0]).unwrap();
    for unit in &mut state.armies.get_mut(&english).unwrap().units {
        unit.unit_type = scout_type.clone();
    }
    let scouted = posture::ambush_chance(&state, &data, &french, &english);
    assert!((chance - scouted - rules.scout_malus * (1.0 - scouts)).abs() < 1e-9);
    state.armies.get_mut(&english).unwrap().stance = Stance::ForcedMarch;
    let tired = posture::ambush_chance(&state, &data, &french, &english);
    assert!((tired - scouted - data.posture_rules.forced_march.ambush_bonus).abs() < 1e-9);
    // Clamped by the data.
    data.posture_rules.ambush.chance_max = 0.3;
    assert_eq!(
        posture::ambush_chance(&state, &data, &french, &english),
        0.3
    );
}

#[test]
fn the_ambusher_charges_harder_in_auto_resolve() {
    let data = real_data();
    let (charge, defense) = posture::auto_resolve_bonus(&data, None, true);
    assert_eq!(charge, data.posture_rules.ambush.auto_attack_percent);
    assert_eq!(defense, 0.0);
    let (charge, _) = posture::auto_resolve_bonus(&data, None, false);
    assert_eq!(charge, 0.0);
}

// ------------------------------------------------------------ forced march

#[test]
fn forced_march_adds_movement_and_forbids_attack_siege_and_ambush() {
    let probe = real_data();
    let spot = empty_spot(&probe, 25.0);
    let data = data_with_forest(&[spot]);
    let (mut state, french, english) = duel(&data, spot, east(&data, spot, 20.0));
    let base = state.army_base_grid_allowance(&data, &state.armies[&french]);
    set_stance(&mut state, &data, &french, Stance::ForcedMarch);
    let bonus = data.posture_rules.forced_march.movement_bonus_percent;
    let boosted = (f64::from(base) * (1.0 + bonus / 100.0)).round() as u32;
    assert_eq!(state.armies[&french].movement_left, boosted);
    assert_eq!(
        state.army_grid_allowance(&data, &state.armies[&french]),
        boosted
    );
    // No attack, no ambush, no siege.
    let attack = state.submit_order(
        &data,
        Order::Attack {
            army: french.clone(),
            target_army: english.clone(),
        },
    );
    assert!(
        matches!(attack, Err(OrderError::ForcedMarchForbids(_))),
        "{attack:?}"
    );
    assert!(reason(&refusal(&mut state, &data, &french, Stance::Ambush)).contains("marche forcée"));
    assert!(reason(&refusal(&mut state, &data, &french, Stance::Siege)).contains("marche forcée"));
    // No entry into a hostile place.
    let hostile = state
        .settlements
        .iter()
        .find(|(_, s)| s.controller == fac("fac_england"))
        .map(|(id, _)| id.clone())
        .expect("an English place");
    let enter = state.submit_order(
        &data,
        Order::MoveArmy {
            army: french.clone(),
            target: MoveOrderTarget::Place(sim_campaign::Place::Settlement(hostile)),
        },
    );
    assert!(
        matches!(enter, Err(OrderError::ForcedMarchForbids(_))),
        "{enter:?}"
    );
    // Giving it up before moving takes the bonus back.
    set_stance(&mut state, &data, &french, Stance::Normal);
    assert_eq!(state.armies[&french].movement_left, base);
}

#[test]
fn forced_march_is_decided_before_moving() {
    let probe = real_data();
    let spot = empty_spot(&probe, 25.0);
    let data = data_with_forest(&[]);
    let (mut state, french, _) = duel(&data, spot, [10.0, 10.0]);
    state
        .submit_order(
            &data,
            Order::move_to_point(french.clone(), east(&data, spot, 5.0)),
        )
        .unwrap();
    let error = refusal(&mut state, &data, &french, Stance::ForcedMarch);
    assert!(reason(&error).contains("avant de bouger"), "{error}");
    let error = refusal(&mut state, &data, &french, Stance::Entrenched);
    assert!(reason(&error).contains("déjà bougé"), "{error}");
}

#[test]
fn forced_march_costs_supply_and_ends_next_turn() {
    let probe = real_data();
    let spot = empty_spot(&probe, 25.0);
    let data = data_with_forest(&[]);
    let (mut state, french, _) = duel(&data, spot, [10.0, 10.0]);
    // A twin army standing at the same place, in normal stance.
    let mut twin = state.armies[&french].clone();
    twin.general = None;
    let twin_id = ArmyId::from_index(900);
    state.armies.insert(twin_id.clone(), twin);
    for id in [&french, &twin_id] {
        state.armies.get_mut(id).unwrap().supply = 50;
    }
    set_stance(&mut state, &data, &french, Stance::ForcedMarch);
    state.end_turn_with(&data, |_, _, _| Vec::new());
    let cost = data.posture_rules.forced_march.supply_cost;
    assert_eq!(
        state.armies[&french].supply + cost,
        state.armies[&twin_id].supply
    );
    assert_eq!(state.armies[&french].stance, Stance::Normal);
    assert_eq!(
        state.armies[&french].movement_left,
        state.army_base_grid_allowance(&data, &state.armies[&french])
    );
}

#[test]
fn forced_march_reaches_the_3d_battle_tired() {
    let probe = real_data();
    let spot = empty_spot(&probe, 25.0);
    let data = data_with_forest(&[]);
    let (mut state, french, english) = duel(&data, spot, east(&data, spot, 3.0));
    state.interactive_battles = true;
    set_stance(&mut state, &data, &french, Stance::ForcedMarch);
    state.armies.get_mut(&english).unwrap().stance = Stance::Entrenched;
    // The English attack the tired French.
    state
        .apply_order(
            &data,
            &fac("fac_england"),
            Order::Attack {
                army: english.clone(),
                target_army: french.clone(),
            },
        )
        .unwrap();
    assert_eq!(state.pending_battles.len(), 1);
    let setup = state.battle_setup(&data, 0).unwrap();
    assert!(setup.defender.forced_march);
    assert_eq!(
        setup.defender.start_fatigue,
        data.posture_rules.forced_march.start_fatigue
    );
    assert!(!setup.attacker.forced_march);
    assert_eq!(setup.attacker.start_fatigue, 0.0);
    // Attacking left the English camp.
    assert!(!setup.attacker.entrenched);
    assert_eq!(setup.opening, BattleOpening::Standard);
}

// --------------------------------------------------------------- entrenched

#[test]
fn entrenched_camp_holds_until_a_move_and_reaches_the_battle() {
    let probe = real_data();
    let spot = empty_spot(&probe, 25.0);
    let data = data_with_forest(&[]);
    let (mut state, french, english) = duel(&data, spot, east(&data, spot, 3.0));
    set_stance(&mut state, &data, &french, Stance::Entrenched);
    assert_eq!(state.armies[&french].movement_left, 0);
    state.end_turn_with(&data, |_, _, _| Vec::new());
    assert_eq!(state.armies[&french].stance, Stance::Entrenched);
    // Attacked in its camp: the 3D battle knows it.
    state.interactive_battles = true;
    refill(&mut state, &data, &english);
    state
        .apply_order(
            &data,
            &fac("fac_england"),
            Order::Attack {
                army: english.clone(),
                target_army: french.clone(),
            },
        )
        .unwrap();
    let setup = state.battle_setup(&data, 0).unwrap();
    assert!(setup.defender.entrenched);
    let (_, defense) = posture::auto_resolve_bonus(&data, Some(&state.armies[&french]), false);
    assert_eq!(defense, data.posture_rules.entrenched.auto_defense_percent);
    // A move order breaks the camp.
    state.pending_battles.clear();
    refill(&mut state, &data, &french);
    state
        .submit_order(
            &data,
            Order::move_to_point(french.clone(), [spot[0], spot[1] - 20.0]),
        )
        .unwrap();
    assert_eq!(state.armies[&french].stance, Stance::Normal);
}

#[test]
fn entrenched_camp_needs_the_open_field_and_saves_supply() {
    let data = real_data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 5).unwrap();
    let french = main_army(&state, "fac_france");
    refill(&mut state, &data, &french);
    let place = state.armies[&french].settlement().cloned();
    if place.is_some() {
        let error = refusal(&mut state, &data, &french, Stance::Entrenched);
        assert!(reason(&error).contains("place"), "{error}");
    }
    let army = state.armies[&french].clone();
    let saved = posture::entrenched_loss(&data, &army, 10);
    assert_eq!(saved, 10, "not entrenched: the whole loss");
    let mut camp = army;
    camp.stance = Stance::Entrenched;
    let saving = data.posture_rules.entrenched.supply_saving_percent;
    assert_eq!(
        posture::entrenched_loss(&data, &camp, 10),
        (10.0 * (1.0 - saving / 100.0)).round() as u8
    );
}

// --------------------------------------------------------------- old saves

#[test]
fn old_saves_without_the_stance_fields_still_load() {
    let (data, mut state, french, _) = ambush_setup(1.0, false);
    state
        .armies
        .get_mut(&french)
        .unwrap()
        .morale_modifiers
        .push(sim_campaign::MoraleModifier { value: 5, turns: 2 });
    let saved = state.save_json();
    assert!(saved.contains("\"ambush\""));
    assert!(saved.contains("\"morale_modifiers\""));
    let restored = CampaignState::load_json(&saved).expect("round trip");
    assert_eq!(restored.armies[&french].stance, Stance::Ambush);
    assert_eq!(restored.armies[&french].morale_modifier(), 5);

    // A pre-CV3 save: no morale modifiers, no outcome, old stances only.
    let mut json: serde_json::Value = serde_json::from_str(&saved).unwrap();
    for army in json["armies"].as_object_mut().unwrap().values_mut() {
        let object = army.as_object_mut().unwrap();
        object.remove("morale_modifiers");
        object.insert("stance".to_owned(), serde_json::json!("raid"));
    }
    json.as_object_mut().unwrap().remove("last_battle_outcome");
    let old = CampaignState::load_json(&json.to_string()).expect("pre-CV3 save loads");
    assert!(old.armies.values().all(|a| a.morale_modifiers.is_empty()));
    assert!(old.armies.values().all(|a| a.stance == Stance::Raid));
    assert!(old.last_battle_outcome.is_none());
    assert_eq!(old.state_version, state.state_version);
    let _ = data;
}
