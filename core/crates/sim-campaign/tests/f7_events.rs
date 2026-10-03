//! F7b (« Chronique enrichie ») integration tests: the new historical events
//! and their chains fire in a simulated 1337-1453 campaign (fixed seed,
//! built-in AI for every faction, the player's decisions taken by the AI
//! weights). See `docs/design/m10-events.md` § F7b.

use std::collections::BTreeMap;
use std::path::PathBuf;

use data_model::{EventCategory, EventId, FactionId, GameData};
use sim_campaign::{ai_minimal, chronicle, CampaignState, Order, Season};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

/// Runs a campaign from 1337 to the end of 1453 and returns the year each
/// event first fired.
fn simulate(data: &GameData, player: &str, seed: u64) -> BTreeMap<EventId, i32> {
    let player = fac(player);
    let mut state = CampaignState::new_1337(data, player.clone(), seed).expect("1337 start");
    let mut fired: BTreeMap<EventId, i32> = BTreeMap::new();
    while state.year <= 1453 {
        // The player plays like an AI faction: same planner, AI choices.
        let pending: Vec<(u32, EventId)> = state
            .chronicle
            .pending_decisions
            .iter()
            .filter(|d| d.faction == player)
            .map(|d| (d.id, d.event.clone()))
            .collect();
        for (decision, event) in pending {
            let option = chronicle::ai_choice(&mut state, &data.events[&event]);
            let _ = state.submit_order(data, Order::ChooseEventOption { decision, option });
        }
        if state.factions[&player].alive {
            for order in ai_minimal::plan_turn(&state, data, &player) {
                let _ = state.submit_order(data, order);
            }
        }
        state.end_turn(data);
        if std::env::var("F7_TRACE").is_ok() && state.season == Season::Spring {
            let wars: Vec<String> = state
                .factions
                .iter()
                .filter(|(_, f)| f.alive)
                .flat_map(|(id, f)| {
                    f.at_war_with
                        .iter()
                        .filter(move |e| id.as_str() < e.as_str())
                        .map(move |e| format!("{}-{}", &id.as_str()[4..], &e.as_str()[4..]))
                })
                .collect();
            let dead: Vec<&str> = state
                .factions
                .iter()
                .filter(|(_, f)| !f.alive)
                .map(|(id, _)| &id.as_str()[4..])
                .collect();
            let rulers: Vec<String> = ["fac_france", "fac_england", "fac_burgundy", "fac_scotland"]
                .iter()
                .map(|f| format!("{:?}", state.factions[&fac(f)].ruler))
                .collect();
            println!(
                "{} wars {wars:?} dead {dead:?} rulers {rulers:?}",
                state.year
            );
        }
        for event in &state.chronicle.fired_events {
            fired.entry(event.clone()).or_insert(state.year);
        }
    }
    fired
}

/// Historical events added by F7b (listed in `docs/design/m10-events.md` § F7b).
const F7_HISTORICAL: &[&str] = &[
    "evt_paix_de_venise",
    "evt_laupen",
    "evt_valdemar_iv",
    "evt_siege_tournai",
    "evt_salado",
    "evt_succession_bretagne",
    "evt_banqueroute_bardi",
    "evt_charles_iv_roi_des_romains",
    "evt_neville_cross",
    "evt_cola_di_rienzo",
    "evt_achat_dauphine",
    "evt_combat_des_trente",
    "evt_bulle_d_or",
    "evt_cocherel",
    "evt_auray",
    "evt_najera",
    "evt_appel_gascon",
    "evt_auld_alliance",
    "evt_la_rochelle",
    "evt_lords_appelants",
    "evt_harfleur",
    "evt_montereau",
    "evt_verneuil",
    "evt_patay",
    "evt_proces_de_rouen",
    "evt_fougeres",
    "evt_formigny",
];

/// Chained events added by F7b (second or third step of a chain).
const F7_CHAINED: &[&str] = &[
    "evt_treve_esplechin",
    "evt_hennebont",
    "evt_rancon_david_ii",
    "evt_chute_de_rienzo",
    "evt_rancon_de_nevers",
    "evt_alliance_anglo_bourguignonne",
    "evt_sacre_de_reims",
];

/// Random events added by F7b for the factions created in F7.
const F7_RANDOM: &[&str] = &[
    "evt_galeres_de_flandre",
    "evt_banque_florentine",
    "evt_piquiers_suisses",
    "evt_argent_de_kutna_hora",
    "evt_harengs_de_scanie",
    "evt_razzia_frontiere",
];

fn evt(id: &str) -> EventId {
    EventId::new(id).unwrap()
}

fn set_war(state: &mut CampaignState, a: &str, b: &str) {
    for (x, y) in [(a, b), (b, a)] {
        state
            .factions
            .get_mut(&fac(x))
            .unwrap()
            .at_war_with
            .insert(fac(y));
    }
}

#[test]
fn f7_events_load_with_their_kind_and_sources() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, warnings) = GameData::load(&root).expect("game data loads");
    let kinds = [
        (F7_HISTORICAL, EventCategory::Historical),
        (F7_CHAINED, EventCategory::Chained),
        (F7_RANDOM, EventCategory::Random),
    ];
    for (ids, kind) in kinds {
        for id in ids {
            let event = &data.events[&evt(id)];
            assert_eq!(event.kind, kind, "{id}");
            assert!(!event.sources.is_empty(), "{id} has no source");
            assert!(
                event.options.iter().all(|o| !o.effects.is_empty()),
                "{id}: every choice has effects"
            );
        }
    }
    let event_warnings: Vec<_> = warnings
        .iter()
        .filter(|w| w.entity.starts_with("evt_"))
        .collect();
    assert!(event_warnings.is_empty(), "{event_warnings:?}");
    assert!(data.events.len() >= 80, "{} events", data.events.len());
}

/// Acceptance criterion of F7b: in a simulated 1337-1453 campaign (fixed
/// seed, AI everywhere) at least 20 of the new historical events fire, and
/// the chains reach their later steps.
#[test]
fn a_simulated_campaign_fires_the_new_history() {
    let data = data();
    let fired = simulate(&data, "fac_france", 1337);
    let historical: Vec<&str> = F7_HISTORICAL
        .iter()
        .copied()
        .filter(|id| fired.contains_key(&evt(id)))
        .collect();
    let missing: Vec<&str> = F7_HISTORICAL
        .iter()
        .copied()
        .filter(|id| !fired.contains_key(&evt(id)))
        .collect();
    assert!(
        historical.len() >= 20,
        "{} new historical events fired; missing: {missing:?}",
        historical.len()
    );
    let chained = F7_CHAINED
        .iter()
        .filter(|id| fired.contains_key(&evt(id)))
        .count();
    assert!(chained >= 4, "{chained} chained steps fired");
    // History keeps its order: a chain step never precedes its cause.
    for (cause, effect) in [
        ("evt_siege_tournai", "evt_treve_esplechin"),
        ("evt_neville_cross", "evt_rancon_david_ii"),
        ("evt_montereau", "evt_alliance_anglo_bourguignonne"),
        ("evt_jeanne_d_arc", "evt_sacre_de_reims"),
    ] {
        if let Some(later) = fired.get(&evt(effect)) {
            assert!(fired[&evt(cause)] <= *later, "{cause} before {effect}");
        }
    }
}

#[test]
fn nevilles_cross_captures_david_and_berwick_frees_him() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_england"), 3).unwrap();
    let david = data_model::CharacterId::new("chr_david_ii").unwrap();
    state.year = 1346;
    state.season = Season::Autumn;
    set_war(&mut state, "fac_scotland", "fac_england");
    state.end_turn_with(&data, idle);
    assert!(state
        .chronicle
        .fired_events
        .contains(&evt("evt_neville_cross")));
    assert!(state.characters[&david].captive);
    assert_eq!(state.characters[&david].captor, Some(fac("fac_england")));
    assert!(state
        .chronicle
        .scheduled
        .iter()
        .any(|s| s.event == evt("evt_rancon_david_ii")));
    for _ in 0..20 {
        state.end_turn_with(&data, idle);
    }
    assert!(state
        .chronicle
        .fired_events
        .contains(&evt("evt_rancon_david_ii")));
    assert!(!state.characters[&david].captive);
}

#[test]
fn montereau_leads_to_the_alliance_then_troyes() {
    let data = data();
    // Seed 2 (seed 7 before LR-17, 6 before the HV8 data): the chain depends
    // on the random stream, which any new random event or character shifts
    // (seeds 4 and 9 missed the alliance within three turns).
    let mut state = CampaignState::new_1337(&data, fac("fac_scotland"), 2).unwrap();
    state.year = 1419;
    state.season = Season::Autumn;
    for _ in 0..3 {
        set_war(&mut state, "fac_england", "fac_france");
        state.end_turn_with(&data, idle);
    }
    for id in [
        "evt_montereau",
        "evt_alliance_anglo_bourguignonne",
        "evt_troyes",
    ] {
        assert!(
            state.chronicle.fired_events.contains(&evt(id)),
            "{id}: {:?} {} {:?} {:?}",
            state.chronicle.fired_events,
            state.year,
            state.season,
            state
                .faction_state(&fac("fac_france"))
                .map(|f| (f.alive, &f.at_war_with))
        );
    }
}

#[test]
fn nicopolis_leads_to_the_ransom_of_nevers() {
    let data = data();
    // Seed 3 (5 before LR-17's new character shifted the random stream).
    let mut state = CampaignState::new_1337(&data, fac("fac_scotland"), 3).unwrap();
    state.year = 1396;
    state.season = Season::Autumn;
    state.end_turn_with(&data, idle);
    assert!(state.chronicle.fired_events.contains(&evt("evt_nicopolis")));
    assert!(!state
        .chronicle
        .fired_events
        .contains(&evt("evt_rancon_de_nevers")));
    for _ in 0..3 {
        state.end_turn_with(&data, idle);
    }
    assert!(state
        .chronicle
        .fired_events
        .contains(&evt("evt_rancon_de_nevers")));
}

/// HV10 historical events of the East and the South (1337-1354):
/// (event, year, season, player faction used for the start).
const HV10_HISTORICAL: &[(&str, i32, Season)] = &[
    ("evt_chute_de_tlemcen", 1337, Season::Spring),
    ("evt_mort_d_ivan_kalita", 1340, Season::Spring),
    ("evt_mort_d_an_nasir", 1341, Season::Summer),
    ("evt_mort_d_andronic_iii", 1341, Season::Summer),
    ("evt_mort_d_ozbeg", 1341, Season::Spring),
    ("evt_mort_de_gediminas", 1341, Season::Winter),
    ("evt_nuit_de_la_saint_georges", 1343, Season::Spring),
    ("evt_paix_de_kalisz", 1343, Season::Summer),
    ("evt_prise_d_algesiras", 1344, Season::Spring),
    ("evt_sainte_ligue_a_smyrne", 1344, Season::Autumn),
    ("evt_dusan_prend_serres", 1345, Season::Autumn),
    ("evt_couronnement_de_dusan", 1346, Season::Spring),
    ("evt_siege_de_caffa", 1346, Season::Summer),
    ("evt_bataille_de_la_streva", 1347, Season::Winter),
    ("evt_desastre_de_kairouan", 1348, Season::Spring),
    ("evt_zakonik", 1349, Season::Spring),
    ("evt_tzympe", 1352, Season::Autumn),
    ("evt_seisme_de_gallipoli", 1354, Season::Spring),
];

/// Each HV10 event fires on its date in a plausible state (the 1337 start,
/// idle AI, the player being a faction with no stake in the East).
#[test]
fn hv10_events_fire_on_their_date_in_a_plausible_state() {
    let data = data();
    for (id, year, season) in HV10_HISTORICAL {
        let event = &data.events[&evt(id)];
        assert_eq!(event.kind, EventCategory::Historical, "{id}");
        let mut state = CampaignState::new_1337(&data, fac("fac_scotland"), 11).unwrap();
        state.year = *year;
        state.season = *season;
        state.end_turn_with(&data, idle);
        assert!(
            state.chronicle.fired_events.contains(&evt(id)),
            "{id} did not fire in {year} {season:?}"
        );
    }
}

/// The conditions matter: without them an HV10 event stays silent.
#[test]
fn hv10_events_stay_silent_when_their_conditions_fail() {
    let data = data();
    // Algeciras no longer Marinid.
    let mut state = CampaignState::new_1337(&data, fac("fac_scotland"), 11).unwrap();
    state.year = 1344;
    state.season = Season::Spring;
    let province = data_model::ProvinceId::new("prov_algeciras").unwrap();
    let city = state.provinces[&province].city.clone();
    let settlement = state.settlements.get_mut(&city).unwrap();
    settlement.owner = fac("fac_castile");
    settlement.controller = fac("fac_castile");
    state.end_turn_with(&data, idle);
    assert!(!state
        .chronicle
        .fired_events
        .contains(&evt("evt_prise_d_algesiras")));
    // Out of its window (after `until_year`).
    let mut state = CampaignState::new_1337(&data, fac("fac_scotland"), 11).unwrap();
    state.year = 1360;
    state.season = Season::Spring;
    state.end_turn_with(&data, idle);
    assert!(!state
        .chronicle
        .fired_events
        .contains(&evt("evt_chute_de_tlemcen")));
}

/// A planner that does nothing: only the chronicle moves the world.
fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

#[test]
#[ignore = "exploration: cargo test --release -- --ignored --nocapture"]
fn print_campaign_chronicle() {
    let data = data();
    for (player, seed) in [
        ("fac_france", 1337),
        ("fac_england", 7),
        ("fac_burgundy", 42),
    ] {
        let fired = simulate(&data, player, seed);
        let mut by_year: Vec<(i32, &EventId)> = fired.iter().map(|(e, y)| (*y, e)).collect();
        by_year.sort();
        for (year, event) in by_year {
            println!("{year} {event}");
        }
        let missing: Vec<&EventId> = data
            .events
            .iter()
            .filter(|(id, e)| e.kind != EventCategory::Random && !fired.contains_key(*id))
            .map(|(id, _)| id)
            .collect();
        let new_fired = F7_HISTORICAL
            .iter()
            .filter(|id| fired.contains_key(&evt(id)))
            .count();
        println!("{player}/{seed}: {new_fired} new historical fired; not fired: {missing:?}");
    }
}

#[test]
fn an_event_reserved_to_another_faction_costs_no_roll() {
    use data_model::EventScope;
    let venice = fac("fac_venice");
    let france = fac("fac_france");
    let reserved = EventScope::Faction {
        faction: Some(venice.clone()),
    };
    assert!(chronicle::scope_allows(&reserved, &venice));
    assert!(!chronicle::scope_allows(&reserved, &france));
    assert!(chronicle::scope_allows(
        &EventScope::Faction { faction: None },
        &france
    ));
}
