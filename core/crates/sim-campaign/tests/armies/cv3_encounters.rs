//! Lot CV3-3: map encounters (spawn, expiry, trigger, choices, battle and
//! join outcomes, vision, old saves, data references).

use sim_campaign::test_support::start_quiet;
use std::collections::BTreeSet;

use data_model::{
    Condition, EncounterId, EncounterOutcome, EventEffect, GameData, ProvinceId, UnitTypeId,
    ARMY_EFFECT_KINDS,
};
use sim_campaign::encounter::{self, EncounterError, EncounterSite};
use sim_campaign::state::{ArmyId, ArmyPosition, Unit};
use sim_campaign::{CampaignState, Cell, MoveOrderTarget, Order, OrderError};

use data_model::test_support::{fac, game_data};

fn enc(id: &str) -> EncounterId {
    EncounterId::new(id).unwrap()
}

fn army_of(state: &CampaignState, faction: &str) -> ArmyId {
    state
        .armies
        .iter()
        .find(|(_, a)| a.faction.as_str() == faction)
        .map(|(id, _)| id.clone())
        .expect("faction has an army")
}

/// A reachable point about 6 km from `army`, for a one-step march.
fn nearby_target(state: &CampaignState, data: &GameData, army: &ArmyId) -> [f32; 2] {
    let px_km = sim_campaign::march::px_per_km(data);
    let origin = state.army_point(data, &state.armies[army]);
    for (dx, dy) in [(6.0, 0.0), (-6.0, 0.0), (0.0, 6.0), (0.0, -6.0), (4.0, 4.0)] {
        let point = [origin[0] + dx * px_km, origin[1] + dy * px_km];
        if state.find_path(data, army, point).is_some() {
            return point;
        }
    }
    panic!("no reachable point near the army");
}

/// Puts a site of `encounter` on the cell of `point` and returns its id.
fn put_site(state: &mut CampaignState, data: &GameData, encounter: &str, point: [f32; 2]) -> u32 {
    let grid = data.navgrid();
    let cell = Cell::of_point(grid, point);
    let province = data
        .province_at_point(point[0], point[1])
        .cloned()
        .unwrap_or_else(|| ProvinceId::new("prov_ile_de_france").unwrap());
    let id = state.encounters.next_site_id;
    state.encounters.next_site_id += 1;
    state.encounters.sites.push(EncounterSite {
        id,
        encounter: enc(encounter),
        cell,
        province,
        expires_turn: state.turn + 3,
        claimed_by: None,
    });
    id
}

fn march_to(
    state: &mut CampaignState,
    data: &GameData,
    faction: &str,
    army: &ArmyId,
    to: [f32; 2],
) {
    state.armies.get_mut(army).unwrap().movement_left = 400;
    let order = Order::MoveArmy {
        army: army.clone(),
        target: MoveOrderTarget::Point { x: to[0], y: to[1] },
    };
    state
        .apply_order(data, &fac(faction), order)
        .expect("march accepted");
}

/// The player army meets a site of `encounter`; returns (army, site).
fn meet(state: &mut CampaignState, data: &GameData, encounter: &str) -> (ArmyId, u32) {
    let army = army_of(state, "fac_france");
    let target = nearby_target(state, data, &army);
    let site = put_site(state, data, encounter, target);
    march_to(state, data, "fac_france", &army, target);
    (army, site)
}

fn texts(state: &CampaignState) -> String {
    state
        .pending_events
        .iter()
        .chain(state.events.iter())
        .map(|e| e.text_fr.clone())
        .collect::<Vec<_>>()
        .join("\n")
}

fn fresh(data: &GameData, unit: &str, count: usize) -> Vec<Unit> {
    let unit_type = &data.unit_types[&UnitTypeId::new(unit).unwrap()];
    (0..count).map(|_| Unit::fresh(unit_type)).collect()
}

fn spawn_seasons(state: &mut CampaignState, data: &GameData, seasons: u32) {
    for _ in 0..seasons {
        state.turn += 1;
        encounter::start_season(state, data, &mut Vec::new());
    }
}

#[test]
fn sites_spawn_within_the_rules() {
    let data = game_data();
    let rules = &data.encounter_rules;
    let mut state = start_quiet(data, "fac_france", 11);
    spawn_seasons(&mut state, data, 12);
    let sites = &state.encounters.sites;
    assert!(!sites.is_empty(), "sites appear");
    assert!(sites.len() <= rules.max_active as usize);
    let grid = data.navgrid();
    let px_km = sim_campaign::march::px_per_km(data);
    for site in sites {
        let center = site.cell.center(grid);
        assert!(grid.passable(i64::from(site.cell.x), i64::from(site.cell.y)));
        assert_eq!(
            data.province_at_point(center[0], center[1]),
            Some(&site.province)
        );
        assert!(
            sim_campaign::march::settlements_near(data, center, rules.min_settlement_distance_km)
                .is_empty(),
            "site {} stands on a settlement",
            site.id
        );
        for other in sites.iter().filter(|o| o.id != site.id) {
            let o = other.cell.center(grid);
            let km = ((o[0] - center[0]).powi(2) + (o[1] - center[1]).powi(2)).sqrt() / px_km;
            assert!(km as f64 >= rules.min_site_distance_km - 1.5);
        }
        assert!(site.expires_turn > state.turn);
    }
    // Encounters respect their spawn rules (a single province per site).
    let provinces: BTreeSet<_> = sites.iter().map(|s| &s.province).collect();
    assert_eq!(provinces.len(), sites.len());
}

#[test]
fn spawn_respects_max_active_and_per_season() {
    let mut data = game_data().clone();
    data.encounter_rules.max_active = 3;
    data.encounter_rules.spawn_per_season = 5;
    let mut state = start_quiet(&data, "fac_france", 3);
    spawn_seasons(&mut state, &data, 1);
    assert!(state.encounters.sites.len() <= 3);
    spawn_seasons(&mut state, &data, 4);
    assert!(state.encounters.sites.len() <= 3);
    data.encounter_rules.spawn_per_season = 0;
    let mut empty = start_quiet(&data, "fac_france", 3);
    spawn_seasons(&mut empty, &data, 4);
    assert!(empty.encounters.sites.is_empty());
}

#[test]
fn sites_expire() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 5);
    spawn_seasons(&mut state, data, 1);
    let first: Vec<(u32, u32)> = state
        .encounters
        .sites
        .iter()
        .map(|s| (s.id, s.expires_turn))
        .collect();
    assert!(!first.is_empty());
    let last = first.iter().map(|(_, e)| *e).max().unwrap();
    state.turn = last;
    encounter::start_season(&mut state, data, &mut Vec::new());
    for (id, _) in &first {
        assert!(
            !state.encounters.sites.iter().any(|s| s.id == *id),
            "site {id} expired"
        );
    }
}

#[test]
fn spawning_is_deterministic_and_leaves_the_main_stream_alone() {
    let data = game_data();
    let mut a = start_quiet(data, "fac_france", 42);
    let mut b = start_quiet(data, "fac_france", 42);
    let rng_before = a.rng.clone();
    spawn_seasons(&mut a, data, 8);
    spawn_seasons(&mut b, data, 8);
    assert_eq!(a.encounters, b.encounters);
    assert_eq!(a.rng, rng_before, "the campaign stream is untouched");
    let mut c = start_quiet(data, "fac_france", 43);
    spawn_seasons(&mut c, data, 8);
    assert_ne!(a.encounters.sites, c.encounters.sites);
}

#[test]
fn a_player_march_ending_near_a_site_waits_for_a_choice() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 7);
    let (army, site) = meet(&mut state, data, "enc_marchands_lombards");
    assert_eq!(state.encounters.pending.len(), 1);
    let pending = &state.encounters.pending[0];
    assert_eq!((pending.site, &pending.army), (site, &army));
    let views = state.pending_encounter_views(data, &fac("fac_france"));
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].options.len(), 3);
    assert!(views[0].options[2].default);
    assert!(views[0].options[0].effects_text.contains("Ravitaillement"));
    // A second march does not meet the claimed site again.
    assert_eq!(
        state.encounters.sites[0].claimed_by.as_ref(),
        Some(&army),
        "the site is claimed"
    );

    // Buy provisions: treasury down, supply up, site gone.
    state.armies.get_mut(&army).unwrap().supply = 50;
    state.factions.get_mut(&fac("fac_france")).unwrap().treasury = 5_000;
    state
        .submit_order(
            data,
            Order::ChooseEncounterOption {
                army: army.clone(),
                site,
                option: 0,
            },
        )
        .expect("option taken");
    assert_eq!(state.armies[&army].supply, 75);
    assert!(state.factions[&fac("fac_france")].treasury < 5_000);
    assert!(state.encounters.pending.is_empty());
    assert!(state.encounters.sites.iter().all(|s| s.id != site));
    // Answering twice is refused.
    let again = state.submit_order(
        data,
        Order::ChooseEncounterOption {
            army,
            site,
            option: 0,
        },
    );
    assert!(matches!(
        again,
        Err(OrderError::Encounter(EncounterError::NoPendingEncounter(_)))
    ));
}

#[test]
fn an_option_whose_conditions_fail_is_refused() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 8);
    let (army, site) = meet(&mut state, data, "enc_marchands_lombards");
    state.factions.get_mut(&fac("fac_france")).unwrap().treasury = 0;
    let views = state.pending_encounter_views(data, &fac("fac_france"));
    assert!(!views[0].options[0].available);
    assert!(views[0].options[0]
        .reason
        .as_deref()
        .unwrap()
        .contains("trésor"));
    let refused = state.submit_order(
        data,
        Order::ChooseEncounterOption {
            army: army.clone(),
            site,
            option: 0,
        },
    );
    assert!(matches!(
        refused,
        Err(OrderError::Encounter(EncounterError::OptionUnavailable(_)))
    ));
    let invalid = state.submit_order(
        data,
        Order::ChooseEncounterOption {
            army,
            site,
            option: 9,
        },
    );
    assert!(matches!(
        invalid,
        Err(OrderError::Encounter(EncounterError::InvalidOption(9)))
    ));
    assert_eq!(state.encounters.pending.len(), 1);
}

#[test]
fn an_unanswered_encounter_takes_its_default_option_at_the_end_of_the_turn() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 9);
    let (_army, site) = meet(&mut state, data, "enc_peage_pont");
    assert_eq!(state.encounters.pending.len(), 1);
    state.end_turn(data);
    assert!(state.encounters.pending.is_empty());
    assert!(state.encounters.sites.iter().all(|s| s.id != site));
    assert!(texts(&state).contains("Acquitter le péage"));
}

#[test]
fn an_ai_march_ending_near_a_site_chooses_at_once() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 10);
    let army = army_of(&state, "fac_england");
    let target = nearby_target(&state, data, &army);
    let site = put_site(&mut state, data, "enc_moines_hospitaliers", target);
    march_to(&mut state, data, "fac_england", &army, target);
    assert!(state.encounters.pending.is_empty(), "no player decision");
    assert!(state.encounters.sites.iter().all(|s| s.id != site));
    assert!(texts(&state).contains("Les moines hospitaliers"));
}

fn choose(state: &mut CampaignState, data: &GameData, army: &ArmyId, site: u32, option: usize) {
    state
        .submit_order(
            data,
            Order::ChooseEncounterOption {
                army: army.clone(),
                site,
                option,
            },
        )
        .expect("option taken");
}

#[test]
fn a_battle_outcome_raises_a_troop_and_applies_on_win() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 12);
    state.year = 1360;
    let (army, site) = meet(&mut state, data, "enc_grandes_compagnies");
    state.armies.get_mut(&army).unwrap().units = fresh(data, "unit_knights", 14);
    let armies_before = state.armies.len();
    choose(&mut state, data, &army, site, 0);
    // The troop waits in a pending 3D battle, linked to the site.
    assert_eq!(state.armies.len(), armies_before + 1);
    assert_eq!(state.encounters.battles.len(), 1);
    let rebels = state.encounters.battles[0].rebels.clone();
    assert_eq!(state.armies[&rebels].faction, fac("fac_rebels"));
    assert_eq!(state.armies[&rebels].units.len(), 4);
    let index = state
        .pending_battles
        .iter()
        .position(|b| b.defender == rebels && b.attacker == army)
        .expect("pending battle");
    assert!(state.is_at_war(&fac("fac_france"), &fac("fac_rebels")));
    let province = state.encounters.battles[0].province.clone();
    let unrest_before = state.provinces[&province].population.peasants.unrest;

    state.auto_resolve_pending(data, index).expect("resolved");
    assert!(state.encounters.battles.is_empty());
    assert!(!state.armies.contains_key(&rebels), "the troop is gone");
    assert!(!state.is_at_war(&fac("fac_france"), &fac("fac_rebels")));
    assert!(texts(&state).contains("taillée en pièces"));
    let unrest_after = state.provinces[&province].population.peasants.unrest;
    assert!(unrest_after <= unrest_before);
}

#[test]
fn a_lost_encounter_battle_applies_on_loss() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 13);
    state.interactive_battles = false;
    let (army, site) = meet(&mut state, data, "enc_grandes_compagnies");
    let mut weak = fresh(data, "unit_urban_militia", 1);
    weak[0].strength = 8;
    state.armies.get_mut(&army).unwrap().units = weak;
    let province = state.encounters.sites[0].province.clone();
    let devastation_before = state.provinces[&province].devastation;
    choose(&mut state, data, &army, site, 0);
    assert!(state.encounters.battles.is_empty(), "auto-resolved at once");
    assert!(state.pending_battles.is_empty());
    assert!(texts(&state).contains("repoussent l'ost"));
    assert!(state.provinces[&province].devastation >= devastation_before + 10);
    assert!(state
        .armies
        .values()
        .all(|a| a.faction.as_str() != "fac_rebels"));
}

#[test]
fn a_join_outcome_respects_the_army_size_limit() {
    let data = game_data();
    let max = data.army_rules.cap();
    let mut state = start_quiet(data, "fac_france", 14);
    let (army, site) = meet(&mut state, data, "enc_deserteurs_gallois");
    state.factions.get_mut(&fac("fac_france")).unwrap().treasury = 5_000;
    state.armies.get_mut(&army).unwrap().units = fresh(data, "unit_crossbowmen", max - 1);
    choose(&mut state, data, &army, site, 0);
    assert_eq!(state.armies[&army].units.len(), max);
    assert_eq!(
        state.armies[&army].units.last().unwrap().unit_type.as_str(),
        "unit_welsh_spearmen"
    );

    // A full army cannot take a join option.
    let mut full = start_quiet(data, "fac_france", 15);
    let (army, site) = meet(&mut full, data, "enc_deserteurs_gallois");
    full.factions.get_mut(&fac("fac_france")).unwrap().treasury = 5_000;
    full.armies.get_mut(&army).unwrap().units = fresh(data, "unit_crossbowmen", max);
    let views = full.pending_encounter_views(data, &fac("fac_france"));
    assert!(!views[0].options[0].available);
    assert!(views[0].options[0]
        .reason
        .as_deref()
        .unwrap()
        .contains("complet"));
    let _ = site;
}

#[test]
fn sites_are_seen_only_inside_the_vision() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 16);
    let army = army_of(&state, "fac_france");
    let near = nearby_target(&state, data, &army);
    let near_id = put_site(&mut state, data, "enc_peage_pont", near);
    // Far away: in Granada (no ally of France lends sight there).
    let far_army = army_of(&state, "fac_granada");
    let far = state.army_point(data, &state.armies[&far_army]);
    let far_id = put_site(&mut state, data, "enc_loups_hiver", [far[0] + 40.0, far[1]]);
    let seen: Vec<u32> = state
        .encounter_site_views(data, &fac("fac_france"))
        .iter()
        .map(|v| v.id)
        .collect();
    assert!(seen.contains(&near_id));
    assert!(!seen.contains(&far_id));
}

#[test]
fn a_campaign_runs_with_encounters_and_leaves_no_troop_behind() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 21);
    state.interactive_battles = false;
    let mut met = 0;
    for _ in 0..12 {
        let events = state.end_turn(data);
        met += events
            .iter()
            .filter(|e| {
                data.encounters
                    .values()
                    .any(|x| e.text_fr.starts_with(&format!("{} —", x.title)))
            })
            .count();
        assert!(state.encounters.sites.len() <= data.encounter_rules.max_active as usize);
        assert!(state.encounters.battles.is_empty());
        assert!(state
            .armies
            .values()
            .all(|a| a.faction.as_str() != "fac_rebels"));
        assert!(!state.is_at_war(&fac("fac_france"), &fac("fac_rebels")));
    }
    println!("encounters met by the AI in 12 turns: {met}");
}

#[test]
fn old_saves_without_encounters_still_load() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 17);
    let _ = meet(&mut state, data, "enc_peage_pont");
    spawn_seasons(&mut state, data, 1);
    let saved = state.save_json();
    assert!(saved.contains("\"encounters\""));
    let restored = CampaignState::load_json(&saved).expect("round trip");
    assert_eq!(restored.encounters, state.encounters);

    let mut json: serde_json::Value = serde_json::from_str(&saved).unwrap();
    json.as_object_mut().unwrap().remove("encounters");
    let old = CampaignState::load_json(&json.to_string()).expect("pre-CV3 save loads");
    assert!(old.encounters.sites.is_empty());
    assert!(old.encounters.pending.is_empty());
    assert_eq!(old.state_version, state.state_version);
}

#[test]
fn encounter_orders_parse_from_the_bridge_dict() {
    let order: Order = serde_json::from_str(
        r#"{"type": "choose_encounter_option", "army": "army_0001", "site": 3, "option": 1}"#,
    )
    .expect("order parses");
    assert!(matches!(
        order,
        Order::ChooseEncounterOption {
            site: 3,
            option: 1,
            ..
        }
    ));
}

#[test]
fn every_encounter_references_existing_data() {
    let data = game_data();
    assert!(data.encounters.len() >= 12);
    let regions: BTreeSet<&str> = data.provinces.values().map(|p| p.region.as_str()).collect();
    let check_effect = |id: &str, effect: &EventEffect| {
        if let EventEffect::SpawnArmy { units, .. } = effect {
            for unit in units {
                assert!(data.unit_types.contains_key(unit), "{id}: {unit}");
            }
        }
    };
    let check_condition = |id: &str, condition: &Condition| match condition {
        Condition::Fired { event } | Condition::NotFired { event } => {
            assert!(data.events.contains_key(event), "{id}: {event}");
        }
        Condition::Controls { province, .. } => {
            assert!(data.provinces.contains_key(province), "{id}: {province}");
        }
        _ => {}
    };
    for (id, encounter) in &data.encounters {
        let id = id.as_str();
        assert!(!encounter.sources.is_empty(), "{id}: sources");
        assert!((2..=3).contains(&encounter.options.len()), "{id}: options");
        let spawn = &encounter.spawn;
        for region in &spawn.regions {
            assert!(regions.contains(region.as_str()), "{id}: region {region}");
        }
        for province in &spawn.provinces {
            assert!(data.provinces.contains_key(province), "{id}: {province}");
        }
        spawn.conditions.iter().for_each(|c| check_condition(id, c));
        assert!(spawn.lifetime[0] >= 1 && spawn.lifetime[0] <= spawn.lifetime[1]);
        if let (Some(from), Some(to)) = (spawn.from_year, spawn.to_year) {
            assert!(from <= to, "{id}: years");
        }
        assert!(encounter.options.iter().filter(|o| o.default).count() <= 1);
        let default = &encounter.options[encounter.default_option()];
        assert!(default.conditions.is_empty(), "{id}: default option");
        for option in &encounter.options {
            option
                .conditions
                .iter()
                .for_each(|c| check_condition(id, c));
            option.effects.iter().for_each(|e| check_effect(id, e));
            for effect in &option.army_effects {
                assert!(
                    ARMY_EFFECT_KINDS.contains(&effect.effect),
                    "{id}: army effect"
                );
            }
            let outcome_units = match &option.outcome {
                Some(EncounterOutcome::Battle {
                    units,
                    on_win,
                    on_loss,
                }) => {
                    for result in [on_win, on_loss] {
                        result.effects.iter().for_each(|e| check_effect(id, e));
                        for effect in &result.army_effects {
                            assert!(ARMY_EFFECT_KINDS.contains(&effect.effect));
                        }
                    }
                    units.clone()
                }
                Some(EncounterOutcome::Join { units }) => units.clone(),
                None => Vec::new(),
            };
            for units in outcome_units {
                let unit = data
                    .unit_types
                    .get(&units.unit_type)
                    .unwrap_or_else(|| panic!("{id}: {}", units.unit_type));
                // The troop exists in the period the encounter may appear.
                if let (Some(from), Some(until)) = (spawn.from_year, unit.available_until) {
                    assert!(from <= until, "{id}: {} too late", units.unit_type);
                }
                if let (Some(to), Some(since)) = (spawn.to_year, unit.available_from) {
                    assert!(to >= since, "{id}: {} too early", units.unit_type);
                }
            }
        }
    }
    let _ = ArmyPosition::field([0.0, 0.0]);
}

#[test]
fn a_staged_site_is_seen_and_met_like_a_spawned_one() {
    // CV3-4: `debug_put_encounter_site` (UI tests and screenshots).
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 7);
    let army = army_of(&state, "fac_france");
    let target = nearby_target(&state, data, &army);
    assert!(state
        .debug_put_encounter_site(data, &enc("enc_no_such"), target)
        .is_none());
    let site = state
        .debug_put_encounter_site(data, &enc("enc_marchands_lombards"), target)
        .expect("site placed");
    let views = state.encounter_site_views(data, &fac("fac_france"));
    assert!(views.iter().any(|v| v.id == site), "the player sees it");
    march_to(&mut state, data, "fac_france", &army, target);
    assert_eq!(state.encounters.pending.len(), 1);
    assert_eq!(state.encounters.pending[0].site, site);
}
