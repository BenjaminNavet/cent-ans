//! Lot FK1: map scenes, event presentation, expiry by the AI's option
//! (`docs/design/2026-09-29-carte-vivante-folk.md` § 7, ADR 0122).

use std::collections::BTreeMap;
use std::path::PathBuf;

use data_model::{
    EventId, EventPresentation, FactionId, GameData, ProvinceId, SceneKind, SettlementId,
};
use sim_campaign::chronicle::{Decision, RecentScene};
use sim_campaign::map_scenes::{map_scenes, MapScene};
use sim_campaign::state::{Construction, QueuedRecruit, SiegeState};
use sim_campaign::{CampaignState, EventKind};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn evt(id: &str) -> EventId {
    EventId::new(id).unwrap()
}

fn prov(id: &str) -> ProvinceId {
    ProvinceId::new(id).unwrap()
}

fn start(data: &GameData, player: &str, seed: u64) -> CampaignState {
    CampaignState::new_1337(data, fac(player), seed).expect("1337 start")
}

fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<sim_campaign::Order> {
    Vec::new()
}

/// A calm world: no unrest, devastation, sickness, siege, work or recruit
/// anywhere, and no recent event.
fn calm(state: &mut CampaignState) {
    for province in state.provinces.values_mut() {
        province.devastation = 0;
        for class in [
            &mut province.population.peasants,
            &mut province.population.burghers,
            &mut province.population.clergy,
            &mut province.population.nobility,
        ] {
            class.unrest = 0;
            class.health = 80;
        }
    }
    for settlement in state.settlements.values_mut() {
        settlement.siege = None;
        settlement.construction = None;
        settlement.recruit_queue.clear();
    }
    state.chronicle.recent_scenes.clear();
}

fn find<'a>(
    scenes: &'a [MapScene],
    province: &ProvinceId,
    kind: SceneKind,
) -> Option<&'a MapScene> {
    scenes
        .iter()
        .find(|s| &s.province == province && s.kind == kind)
}

#[test]
fn map_scenes_are_deterministic() {
    let data = data();
    let mut state = start(&data, "fac_france", 1337);
    calm(&mut state);
    let turn = state.turn;
    let paris = prov("prov_ile_de_france");
    let normandie = prov("prov_normandie");
    let picardie = prov("prov_picardie");

    // Revolt: unrest above the threshold.
    {
        let population = &mut state.provinces.get_mut(&paris).unwrap().population;
        for class in [
            &mut population.peasants,
            &mut population.burghers,
            &mut population.clergy,
            &mut population.nobility,
        ] {
            class.unrest = 95;
        }
    }
    // Devastation.
    state.provinces.get_mut(&normandie).unwrap().devastation = 60;
    // Siege, construction and muster in Picardy's city.
    let city: SettlementId = state.provinces[&picardie].city.clone();
    let building = data.buildings.keys().next().unwrap().clone();
    let unit = data.unit_types.keys().next().unwrap().clone();
    {
        let settlement = state.settlements.get_mut(&city).unwrap();
        settlement.siege = Some(SiegeState {
            attacker: fac("fac_england"),
            turns_left: 4,
            turns_elapsed: 1,
            supplies: 60,
            breach: 10,
            started_turn: turn.saturating_sub(1),
        });
        settlement.construction = Some(Construction {
            building,
            turns_left: 1,
            paid: 0,
            drawn: BTreeMap::new(),
        });
        settlement.recruit_queue.push(QueuedRecruit {
            unit_type: unit,
            turns_left: 1,
            ordered_turn: turn,
            drawn: BTreeMap::new(),
        });
    }
    // A recent fair (event fired at the end of the previous turn).
    let bretagne = prov("prov_bretagne");
    state.turn = turn + 1;
    state.chronicle.recent_scenes.push(RecentScene {
        kind: SceneKind::Fair,
        province: bretagne.clone(),
        turn,
        event: Some(evt("evt_foire_prospere")),
    });

    let rng_before = state.rng.clone();
    let first = map_scenes(&state, &data);
    let second = map_scenes(&state, &data);
    assert_eq!(first, second, "pure function of the state");
    assert_eq!(state.rng, rng_before, "no RNG read");

    let revolt = find(&first, &paris, SceneKind::Revolt).expect("revolt scene");
    assert_eq!(
        revolt.settlement.as_ref(),
        Some(&state.provinces[&paris].city)
    );
    assert!(revolt.intensity > 0.5);
    let devastation = find(&first, &normandie, SceneKind::Devastation).expect("devastation");
    assert!((devastation.intensity - 0.6).abs() < 1e-6);
    for kind in [SceneKind::Siege, SceneKind::Construction, SceneKind::Muster] {
        let scene = find(&first, &picardie, kind).unwrap_or_else(|| panic!("{kind:?}"));
        assert_eq!(scene.settlement.as_ref(), Some(&city));
    }
    let fair = find(&first, &bretagne, SceneKind::Fair).expect("fair from the chronicle");
    assert_eq!(fair.since_turn, turn + 1);
    assert!((fair.intensity - 1.0).abs() < 1e-6);
    for scene in &first {
        assert!((0.0..=1.0).contains(&scene.intensity), "{scene:?}");
    }
    // Nothing elsewhere in a calm province.
    assert!(first.iter().all(|s| s.province != prov("prov_berry")));
}

#[test]
fn event_scenes_fade_with_age() {
    let data = data();
    let mut state = start(&data, "fac_france", 3);
    calm(&mut state);
    let turn = state.turn;
    let province = prov("prov_champagne");
    state.chronicle.recent_scenes.push(RecentScene {
        kind: SceneKind::Plague,
        province: province.clone(),
        turn,
        event: None,
    });
    let duration = data.map_scene_rules.duration(SceneKind::Plague);
    assert!(duration >= 2);
    assert!(find(&map_scenes(&state, &data), &province, SceneKind::Plague).is_none());
    let mut last = f32::INFINITY;
    for age in 0..duration {
        state.turn = turn + 1 + age;
        let scenes = map_scenes(&state, &data);
        let plague = find(&scenes, &province, SceneKind::Plague).expect("still shown");
        assert!(plague.intensity < last, "age {age}: {}", plague.intensity);
        assert!(plague.intensity > 0.0);
        last = plague.intensity;
    }
    state.turn = turn + 1 + duration;
    assert!(find(&map_scenes(&state, &data), &province, SceneKind::Plague).is_none());
}

#[test]
fn chronicle_records_and_forgets_event_scenes() {
    let data = data();
    let mut state = start(&data, "fac_france", 5);
    state.chronicle.disabled = true;
    let turn = state.turn;
    state.chronicle.recent_scenes.push(RecentScene {
        kind: SceneKind::Flood,
        province: prov("prov_touraine"),
        turn,
        event: Some(evt("evt_crue")),
    });
    let duration = data.map_scene_rules.duration(SceneKind::Flood);
    for _ in 0..duration {
        state.end_turn_with(&data, idle);
        assert_eq!(state.chronicle.recent_scenes.len(), 1);
    }
    state.end_turn_with(&data, idle);
    assert!(state.chronicle.recent_scenes.is_empty(), "pruned");

    // Older saves without the field still load.
    let mut json: serde_json::Value = serde_json::to_value(&state).unwrap();
    json["chronicle"]
        .as_object_mut()
        .unwrap()
        .remove("recent_scenes");
    let loaded: CampaignState = serde_json::from_value(json).unwrap();
    assert!(loaded.chronicle.recent_scenes.is_empty());
}

fn expire(state: &mut CampaignState, data: &GameData, event: &str, options: Vec<usize>) -> String {
    state.chronicle.disabled = true;
    let id = state.chronicle.next_decision_id.max(1);
    state.chronicle.next_decision_id = id + 1;
    state.chronicle.pending_decisions.push(Decision {
        id,
        event: evt(event),
        faction: fac("fac_france"),
        province: None,
        options,
        expires_turn: state.turn,
    });
    let events = state.end_turn_with(data, idle);
    assert!(state.chronicle.pending_decisions.iter().all(|d| d.id != id));
    events
        .into_iter()
        .find(|e| e.kind == EventKind::Chronicle && e.text_fr.contains("délai écoulé"))
        .expect("expiry journaled")
        .text_fr
}

#[test]
fn expired_decision_applies_ai_option() {
    let data = data();
    // evt_sluys: option 0 « Reconstruire la flotte » (ai_weight 1, 6 000 ₶),
    // option 1 « Renoncer à la mer » (ai_weight 2).
    let mut state = start(&data, "fac_france", 11);
    let text = expire(&mut state, &data, "evt_sluys", vec![0, 1]);
    assert!(text.contains("Renoncer à la mer"), "{text}");

    // Restricted to the options the decision offered.
    let mut state = start(&data, "fac_france", 11);
    let text = expire(&mut state, &data, "evt_sluys", vec![0]);
    assert!(text.contains("Reconstruire la flotte"), "{text}");
}

#[test]
fn random_province_event_defaults_to_map() {
    let data = data();
    let event = |id: &str| &data.events[&evt(id)];
    // Random, province scope: map.
    assert_eq!(event("evt_crue").presentation(), EventPresentation::Map);
    assert_eq!(event("evt_brigands").presentation(), EventPresentation::Map);
    // Historical and faction events: dialog.
    assert_eq!(event("evt_sluys").presentation(), EventPresentation::Dialog);
    assert_eq!(
        event("evt_jacquerie").presentation(),
        EventPresentation::Dialog
    );
    assert_eq!(
        event("evt_tournoi").presentation(),
        EventPresentation::Dialog
    );
    assert_eq!(event("evt_crue").map_scene, Some(SceneKind::Flood));

    // The pending decision exposes its province and presentation.
    let mut state = start(&data, "fac_france", 2);
    let province = prov("prov_touraine");
    state.chronicle.pending_decisions.push(Decision {
        id: 99,
        event: evt("evt_crue"),
        faction: fac("fac_france"),
        province: Some(province.clone()),
        options: vec![0],
        expires_turn: state.turn + 2,
    });
    assert_eq!(
        state.chronicle.pending_decisions[0].presentation(&data),
        EventPresentation::Map
    );
    let views = state.decision_views(&data, &fac("fac_france"));
    let view = views.iter().find(|v| v.id == 99).unwrap();
    assert_eq!(view.presentation, EventPresentation::Map);
    assert_eq!(view.province.as_ref(), Some(&province));
}

/// Spec § 7: from 1337 to 1400 (seed 1337), the player should meet on
/// average between one map incident every 2 turns and one every 4 turns.
///
/// Measured by FK1 (France, built-in minimal AI for the others, player
/// idle): see the report in `docs/wip/fk.md`. Below the target until FK5
/// adds its ~15 random province events: the bounds are then enforced.
#[test]
#[ignore = "FK5: below 1 incident / 4 turns until the 15 new events land"]
fn player_incident_rate_1337_1400() {
    let (turns, map, dialog) = measure_incidents();
    let rate = map as f64 / turns as f64;
    eprintln!("FK1 incident rate: {map} map incidents, {dialog} dialogs over {turns} turns ({rate:.3}/turn)");
    assert!(
        (0.25..=0.5).contains(&rate),
        "{map} map incidents over {turns} turns"
    );
}

/// Runs the 1337-1400 campaign; returns (turns, map incidents, dialogs)
/// offered to the player.
fn measure_incidents() -> (u32, u32, u32) {
    let data = data();
    let mut state = start(&data, "fac_france", 1337);
    let player = fac("fac_france");
    let mut seen = 0u32;
    let (mut turns, mut map, mut dialog) = (0u32, 0u32, 0u32);
    while state.year < 1400 {
        state.end_turn(&data);
        turns += 1;
        for decision in &state.chronicle.pending_decisions {
            if decision.id <= seen || decision.faction != player {
                continue;
            }
            match decision.presentation(&data) {
                EventPresentation::Map => map += 1,
                EventPresentation::Dialog => dialog += 1,
            }
        }
        seen = state
            .chronicle
            .pending_decisions
            .iter()
            .map(|d| d.id)
            .max()
            .unwrap_or(seen)
            .max(seen);
        if !state.factions[&player].alive {
            break;
        }
    }
    (turns, map, dialog)
}
