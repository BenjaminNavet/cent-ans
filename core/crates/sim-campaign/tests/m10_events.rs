//! M10 (chronicle events) integration tests: data validation, historical
//! triggers and divergence, single firing, deterministic random events, AI
//! choice, player decisions and the `choose_event_option` order, expiry, the
//! Black Death wave and the save format. See `docs/design/m10-events.md` § 2.

use std::path::PathBuf;

use data_model::{EventCategory, EventId, FactionId, GameData, ProvinceId};
use sim_campaign::chronicle::{self, plague_slice, PLAGUE_POPULATION_LOSS};
use sim_campaign::{
    CampaignState, ChronicleError, EventKind, Order, OrderError, Season, STATE_VERSION,
};

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

/// A planner that does nothing: only the orders the test submits apply.
fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<sim_campaign::Order> {
    Vec::new()
}

fn set_date(state: &mut CampaignState, year: i32, season: Season) {
    state.year = year;
    state.season = season;
}

fn set_war(state: &mut CampaignState, a: &str, b: &str, war: bool) {
    for (x, y) in [(a, b), (b, a)] {
        let faction = state.factions.get_mut(&fac(x)).unwrap();
        if war {
            faction.at_war_with.insert(fac(y));
        } else {
            faction.at_war_with.remove(&fac(y));
        }
    }
}

fn decision_for(state: &CampaignState, event: &str) -> Option<u32> {
    state
        .chronicle
        .pending_decisions
        .iter()
        .find(|d| d.event == evt(event))
        .map(|d| d.id)
}

#[test]
fn every_event_loads_and_references_known_ids() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, warnings) = GameData::load(&root).expect("game data loads");
    assert!(data.events.len() >= 40, "{} events", data.events.len());
    let historical = data
        .events
        .values()
        .filter(|e| e.kind == EventCategory::Historical)
        .count();
    assert!(historical >= 20);
    assert!(data.events.len() - historical >= 20);
    let event_warnings: Vec<String> = warnings
        .iter()
        .filter(|w| w.entity.starts_with("evt_"))
        .map(ToString::to_string)
        .collect();
    assert!(event_warnings.is_empty(), "{event_warnings:?}");
    for event in data.events.values() {
        assert!(!event.sources.is_empty(), "{} has no source", event.id);
        assert!((1..=3).contains(&event.options.len()));
    }
}

#[test]
fn historical_event_fires_at_its_date_for_the_player() {
    let data = data();
    let mut state = start(&data, "fac_france", 1);
    set_war(&mut state, "fac_england", "fac_france", true);
    set_date(&mut state, 1340, Season::Spring);
    state.end_turn_with(&data, idle);
    assert!(!state.chronicle.fired_events.contains(&evt("evt_sluys")));
    // Summer 1340: L'Écluse.
    let events = state.end_turn_with(&data, idle);
    assert!(state.chronicle.fired_events.contains(&evt("evt_sluys")));
    assert!(decision_for(&state, "evt_sluys").is_some());
    assert!(events
        .iter()
        .any(|e| e.kind == EventKind::Chronicle && e.text_fr.contains("L'Écluse")));
    let views = state.decision_views(&data, &fac("fac_france"));
    let view = views.iter().find(|v| v.event == evt("evt_sluys")).unwrap();
    assert_eq!(view.options.len(), 2);
    assert!(view.options[0].effects_text.contains("Trésor -6000"));
    assert!(view.historical);
}

#[test]
fn unmet_conditions_let_history_diverge() {
    let data = data();
    let mut state = start(&data, "fac_france", 2);
    set_war(&mut state, "fac_england", "fac_france", false);
    set_date(&mut state, 1340, Season::Summer);
    for _ in 0..12 {
        set_war(&mut state, "fac_england", "fac_france", false);
        state.end_turn_with(&data, idle);
    }
    // Window closed (until 1341) without the war: L'Écluse never happens.
    assert!(state.year > 1341);
    set_war(&mut state, "fac_england", "fac_france", true);
    state.end_turn_with(&data, idle);
    assert!(!state.chronicle.fired_events.contains(&evt("evt_sluys")));
    assert!(decision_for(&state, "evt_sluys").is_none());
}

#[test]
fn historical_event_fires_only_once() {
    let data = data();
    let mut state = start(&data, "fac_france", 3);
    set_war(&mut state, "fac_england", "fac_france", true);
    set_date(&mut state, 1340, Season::Summer);
    let mut sluys = 0;
    for _ in 0..6 {
        let events = state.end_turn_with(&data, idle);
        sluys += events
            .iter()
            .filter(|e| e.text_fr.starts_with("Chronique : La bataille de L'Écluse"))
            .count();
    }
    assert_eq!(sluys, 1);
}

#[test]
fn random_events_are_deterministic() {
    let data = data();
    let run = |seed| {
        let mut state = start(&data, "fac_france", seed);
        let mut journal = Vec::new();
        for _ in 0..16 {
            journal.extend(
                state
                    .end_turn_with(&data, idle)
                    .into_iter()
                    .filter(|e| e.kind == EventKind::Chronicle),
            );
        }
        (state.chronicle.clone(), journal)
    };
    let (a, journal_a) = run(11);
    let (b, journal_b) = run(11);
    assert_eq!(a, b);
    assert_eq!(journal_a, journal_b);
    // Sixteen turns are enough for at least one random event for France.
    let random_ids: Vec<&EventId> = a
        .pending_decisions
        .iter()
        .map(|d| &d.event)
        .filter(|id| data.events[*id].kind == EventCategory::Random)
        .collect();
    assert!(
        !random_ids.is_empty() || journal_a.iter().any(|e| e.text_fr.contains("délai écoulé")),
        "no random event in 16 turns"
    );
}

#[test]
fn ai_applies_the_heaviest_option_at_once() {
    let data = data();
    let sluys = &data.events[&evt("evt_sluys")];
    let mut state = start(&data, "fac_england", 4);
    assert_eq!(chronicle::ai_choice(&mut state, sluys), 1);
    // Ties are broken by the RNG, deterministically.
    let fire = &data.events[&evt("evt_incendie")];
    let mut a = start(&data, "fac_england", 5);
    let mut b = start(&data, "fac_england", 5);
    assert_eq!(
        chronicle::ai_choice(&mut a, fire),
        chronicle::ai_choice(&mut b, fire)
    );

    // England plays: France (AI) decides L'Écluse without a decision.
    set_war(&mut state, "fac_england", "fac_france", true);
    set_date(&mut state, 1340, Season::Summer);
    let events = state.end_turn_with(&data, idle);
    assert!(state.chronicle.fired_events.contains(&evt("evt_sluys")));
    assert!(decision_for(&state, "evt_sluys").is_none());
    assert!(events.iter().any(|e| e.kind == EventKind::Chronicle
        && e.text_fr.contains("L'Écluse (France)")
        && e.text_fr.contains("Renoncer à la mer")));
}

#[test]
fn player_decision_is_answered_by_order() {
    let data = data();
    let mut state = start(&data, "fac_france", 6);
    set_war(&mut state, "fac_england", "fac_france", true);
    set_date(&mut state, 1340, Season::Summer);
    state.end_turn_with(&data, idle);
    let id = decision_for(&state, "evt_sluys").expect("decision");

    assert_eq!(
        state.submit_order(
            &data,
            Order::ChooseEventOption {
                decision: 9999,
                option: 0
            }
        ),
        Err(OrderError::Chronicle(ChronicleError::UnknownDecision(9999)))
    );
    assert_eq!(
        state.submit_order(
            &data,
            Order::ChooseEventOption {
                decision: id,
                option: 5
            }
        ),
        Err(OrderError::Chronicle(ChronicleError::InvalidOption(5)))
    );

    let treasury = state.factions[&fac("fac_france")].treasury;
    state
        .submit_order(
            &data,
            Order::ChooseEventOption {
                decision: id,
                option: 0,
            },
        )
        .expect("valid choice");
    assert_eq!(state.factions[&fac("fac_france")].treasury, treasury - 6000);
    assert!(decision_for(&state, "evt_sluys").is_none());
    assert!(state
        .pending_events
        .iter()
        .any(|e| e.text_fr.contains("Reconstruire la flotte")));
    // The order form used by the bridge.
    let parsed: Order =
        serde_json::from_str(r#"{"type":"choose_event_option","decision":3,"option":1}"#).unwrap();
    assert_eq!(
        parsed,
        Order::ChooseEventOption {
            decision: 3,
            option: 1
        }
    );
}

#[test]
fn unanswered_decision_expires_with_its_first_option() {
    let data = data();
    let mut state = start(&data, "fac_france", 7);
    set_war(&mut state, "fac_england", "fac_france", true);
    set_date(&mut state, 1340, Season::Summer);
    state.end_turn_with(&data, idle);
    let id = decision_for(&state, "evt_sluys").expect("decision");
    let views = state.decision_views(&data, &fac("fac_france"));
    assert_eq!(views.iter().find(|v| v.id == id).unwrap().expires_in, 2);
    state.end_turn_with(&data, idle);
    assert!(decision_for(&state, "evt_sluys").is_some(), "still open");
    let events = state.end_turn_with(&data, idle);
    assert!(decision_for(&state, "evt_sluys").is_none());
    assert!(events.iter().any(|e| e.kind == EventKind::Chronicle
        && e.text_fr.contains("Reconstruire la flotte")
        && e.text_fr.contains("délai écoulé")));
}

#[test]
fn black_death_strikes_south_first_once_per_province() {
    let data = data();
    let mut state = start(&data, "fac_france", 8);
    set_date(&mut state, 1347, Season::Autumn);
    state.end_turn_with(&data, idle);
    assert!(state
        .chronicle
        .fired_events
        .contains(&evt("evt_black_death")));
    let id = decision_for(&state, "evt_black_death").expect("global event: the player sees it");
    state
        .submit_order(
            &data,
            Order::ChooseEventOption {
                decision: id,
                option: 0,
            },
        )
        .unwrap();
    let wave = state.chronicle.plague_wave.clone().expect("wave started");
    assert_eq!(wave.duration, 12);

    // Every province is struck exactly once over the twelve steps.
    let mut struck: Vec<ProvinceId> = (0..wave.duration)
        .flat_map(|step| plague_slice(&state, &data, step, wave.duration))
        .collect();
    let total = struck.len();
    struck.sort();
    struck.dedup();
    assert_eq!(total, struck.len());
    assert_eq!(total, state.provinces.len());
    let first = plague_slice(&state, &data, 0, wave.duration);
    assert!(first.contains(&prov("prov_malaga")), "{first:?}");
    let last = plague_slice(&state, &data, wave.duration - 1, wave.duration);
    assert!(last.contains(&prov("prov_highlands")), "{last:?}");

    let south = prov("prov_malaga");
    let north = prov("prov_highlands");
    let before_south = state.provinces[&south].population.total();
    let before_north = state.provinces[&north].population.total();
    let health_before = state.provinces[&south].population.peasants.health;
    state.end_turn_with(&data, idle);
    let after_south = state.provinces[&south].population.total();
    let max_kept = 1.0 - f64::from(PLAGUE_POPULATION_LOSS.0) / 100.0 + 0.01;
    assert!((after_south as f64) < before_south as f64 * max_kept);
    assert!(state.provinces[&south].population.peasants.health < health_before);
    assert!(state.provinces[&north].population.total() as f64 > before_north as f64 * 0.97);

    for _ in 0..12 {
        state.end_turn_with(&data, idle);
    }
    assert!(state.chronicle.plague_wave.is_none());
}

#[test]
fn chronicle_state_survives_save_and_old_saves_load() {
    let data = data();
    let mut state = start(&data, "fac_france", 9);
    set_war(&mut state, "fac_england", "fac_france", true);
    set_date(&mut state, 1340, Season::Summer);
    state.end_turn_with(&data, idle);
    assert!(!state.chronicle.pending_decisions.is_empty());
    let json = state.save_json();
    let loaded = CampaignState::load_json(&json).expect("loads");
    assert_eq!(loaded.chronicle, state.chronicle);
    assert_eq!(loaded.state_version, STATE_VERSION);
    assert_eq!(STATE_VERSION, 4);

    // A version-4 save written before M10 has no `chronicle` key.
    let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
    value.as_object_mut().unwrap().remove("chronicle");
    let old = CampaignState::load_json(&value.to_string()).expect("old save loads");
    assert!(old.chronicle.fired_events.is_empty());
    assert!(old.chronicle.pending_decisions.is_empty());
}

#[test]
fn province_scoped_event_targets_its_province() {
    let data = data();
    let mut state = start(&data, "fac_france", 10);
    set_date(&mut state, 1358, Season::Spring);
    state.end_turn_with(&data, idle);
    let decision = state
        .chronicle
        .pending_decisions
        .iter()
        .find(|d| d.event == evt("evt_jacquerie"))
        .cloned()
        .expect("Jacquerie in Picardie");
    assert_eq!(decision.province, Some(prov("prov_picardie")));
    let devastation = state.provinces[&prov("prov_picardie")].devastation;
    state
        .submit_order(
            &data,
            Order::ChooseEventOption {
                decision: decision.id,
                option: 0,
            },
        )
        .unwrap();
    assert_eq!(
        state.provinces[&prov("prov_picardie")].devastation,
        (devastation + 15).min(100)
    );
}

#[test]
fn sixty_turns_bring_historical_and_random_events() {
    let data = data();
    let mut state = start(&data, "fac_france", 12);
    let mut historical = 0;
    let mut random = 0;
    for _ in 0..60 {
        state.end_turn(&data);
        for decision in state.chronicle.pending_decisions.clone() {
            match data.events[&decision.event].kind {
                EventCategory::Historical | EventCategory::Chained => historical += 1,
                EventCategory::Random => random += 1,
            }
            state
                .submit_order(
                    &data,
                    Order::ChooseEventOption {
                        decision: decision.id,
                        option: 0,
                    },
                )
                .unwrap();
        }
    }
    assert!(historical >= 1, "historical {historical}");
    assert!(random >= 1, "random {random}");
    assert!(state.chronicle.pending_decisions.is_empty());
}

#[test]
fn disabled_chronicle_fires_nothing() {
    let data = data();
    let mut state = start(&data, "fac_france", 13);
    state.chronicle.disabled = true;
    set_war(&mut state, "fac_england", "fac_france", true);
    set_date(&mut state, 1340, Season::Summer);
    for _ in 0..8 {
        state.end_turn_with(&data, idle);
    }
    assert!(state.chronicle.fired_events.is_empty());
    assert!(state.chronicle.pending_decisions.is_empty());
}
