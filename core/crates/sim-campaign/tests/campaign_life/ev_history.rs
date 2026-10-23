//! EVENTS (ADR 0779): history that reacts to the game. Poitiers, Crécy,
//! Brétigny and Joan of Arc depend on the battles really fought and on the
//! state of the realms; three chains fork on the player's choices and leave
//! lasting flags. See `docs/wip/events-histoire-conditionnee.md`.

use std::collections::BTreeMap;

use data_model::test_support::{fac, game_data};
use data_model::{
    CharacterId, CharacterRef, Condition, EventEffect, EventId, GameData, ProvinceId,
};
use sim_campaign::test_support::{idle, start};
use sim_campaign::{ai_minimal, chronicle, CampaignState, EventContext, Order, Season};

fn evt(id: &str) -> EventId {
    EventId::new(id).unwrap()
}

fn prov(id: &str) -> ProvinceId {
    ProvinceId::new(id).unwrap()
}

fn chr(id: &str) -> CharacterId {
    CharacterId::new(id).unwrap()
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

fn fired(state: &CampaignState, id: &str) -> bool {
    state.chronicle.fired_events.contains(&evt(id))
}

fn apply(state: &mut CampaignState, data: &GameData, faction: &str, effect: EventEffect) {
    let ctx = EventContext {
        faction: Some(fac(faction)),
        province: None,
    };
    sim_campaign::effects::apply_effect(state, data, &effect, &ctx, &mut Vec::new());
}

/// A staged England-France war at `year`/`season`, with a third faction as
/// the (idle) player so that both realms are decided by the AI weights.
fn staged(year: i32, season: Season) -> CampaignState {
    let data = game_data();
    let mut state = start(data, "fac_scotland", 5);
    state.year = year;
    state.season = season;
    set_war(&mut state, "fac_england", "fac_france");
    state
}

/// The king of France is a prisoner, whoever he is.
fn ruler_captive(state: &CampaignState) -> bool {
    let ruler = state.factions[&fac("fac_france")]
        .ruler
        .clone()
        .expect("ruler");
    state.characters[&ruler].captive
}

fn england_wins(state: &mut CampaignState, count: u32) {
    for _ in 0..count {
        state
            .chronicle
            .record_victory(&fac("fac_england"), &fac("fac_france"));
    }
}

fn france_wins(state: &mut CampaignState, count: u32) {
    for _ in 0..count {
        state
            .chronicle
            .record_victory(&fac("fac_france"), &fac("fac_england"));
    }
}

#[test]
fn history_conditions_read_the_game_state() {
    let data = game_data();
    let mut state = staged(1340, Season::Spring);
    let ctx = EventContext {
        faction: Some(fac("fac_france")),
        province: None,
    };
    let balance = |faction: &str, rival: &str, at_least: i32| Condition::BattleBalance {
        faction: Some(fac(faction)),
        rival: fac(rival),
        at_least,
    };
    assert!(state.condition_holds(data, &balance("fac_england", "fac_france", 0), &ctx));
    assert!(!state.condition_holds(data, &balance("fac_england", "fac_france", 1), &ctx));
    england_wins(&mut state, 2);
    france_wins(&mut state, 1);
    assert!(state.condition_holds(data, &balance("fac_england", "fac_france", 1), &ctx));
    assert!(!state.condition_holds(data, &balance("fac_england", "fac_france", 2), &ctx));
    assert!(state.condition_holds(data, &balance("fac_france", "fac_england", -1), &ctx));
    // `not` negates, `flag` reads what `set_flag` raised.
    let flag = Condition::Flag {
        flag: "essai".to_owned(),
    };
    let not_flag = Condition::Not {
        condition: Box::new(flag.clone()),
    };
    assert!(state.condition_holds(data, &not_flag, &ctx));
    apply(
        &mut state,
        data,
        "fac_france",
        EventEffect::SetFlag {
            flag: "essai".to_owned(),
        },
    );
    assert!(state.condition_holds(data, &flag, &ctx));
    assert!(!state.condition_holds(data, &not_flag, &ctx));
    // The domain is the share of the 1337 provinces still held.
    let below = |percent| Condition::DomainBelow {
        faction: Some(fac("fac_france")),
        percent,
    };
    assert!(!state.condition_holds(data, &below(100), &ctx));
    for p in ["prov_normandie", "prov_touraine", "prov_poitou"] {
        apply(
            &mut state,
            data,
            "fac_england",
            EventEffect::TransferProvince {
                province: prov(p),
                faction: None,
                from: Some(fac("fac_france")),
                price: 0,
                payer: None,
            },
        );
    }
    assert!(state.condition_holds(data, &below(80), &ctx));
    assert!(!state.condition_holds(data, &below(70), &ctx));
}

#[test]
fn field_victories_are_tallied_for_good() {
    let mut state = staged(1340, Season::Spring);
    assert_eq!(
        state
            .chronicle
            .victories(&fac("fac_england"), &fac("fac_france")),
        0
    );
    england_wins(&mut state, 3);
    assert_eq!(
        state
            .chronicle
            .victories(&fac("fac_england"), &fac("fac_france")),
        3
    );
    assert_eq!(
        state
            .chronicle
            .victories(&fac("fac_france"), &fac("fac_england")),
        0
    );
}

#[test]
fn poitiers_takes_the_king_only_after_an_english_victory() {
    let data = game_data();
    let mut state = staged(1356, Season::Autumn);
    england_wins(&mut state, 1);
    state.end_turn_with(data, idle);
    assert!(fired(&state, "evt_poitiers"));
    assert!(!fired(&state, "evt_poitiers_victoire_francaise"));
    assert!(ruler_captive(&state));
    assert!(state.chronicle.flags.contains("roi_jean_captif"));
}

#[test]
fn poitiers_takes_the_prince_after_a_french_victory() {
    let data = game_data();
    let mut state = staged(1356, Season::Autumn);
    france_wins(&mut state, 1);
    state.end_turn_with(data, idle);
    assert!(!fired(&state, "evt_poitiers"));
    assert!(fired(&state, "evt_poitiers_victoire_francaise"));
    assert!(!ruler_captive(&state));
    assert!(state.characters[&chr("chr_edward_of_woodstock")].captive);
}

#[test]
fn poitiers_is_inconclusive_when_nobody_won() {
    let data = game_data();
    let mut state = staged(1356, Season::Autumn);
    state.end_turn_with(data, idle);
    assert!(fired(&state, "evt_maupertuis_sans_bataille"));
    assert!(!fired(&state, "evt_poitiers"));
    assert!(!ruler_captive(&state));
}

#[test]
fn crecy_has_a_french_a_neutral_and_an_english_reading() {
    let data = game_data();
    let mut lost = staged(1346, Season::Summer);
    england_wins(&mut lost, 1);
    lost.end_turn_with(data, idle);
    assert!(fired(&lost, "evt_crecy"));
    assert!(fired(&lost, "evt_crecy_triomphe_anglais"));
    assert!(!fired(&lost, "evt_crecy_victoire_francaise"));

    let mut won = staged(1346, Season::Summer);
    france_wins(&mut won, 1);
    won.end_turn_with(data, idle);
    assert!(!fired(&won, "evt_crecy"));
    assert!(fired(&won, "evt_crecy_victoire_francaise"));
    assert!(fired(&won, "evt_crecy_revers_anglais"));

    let mut drawn = staged(1346, Season::Summer);
    drawn.end_turn_with(data, idle);
    assert!(fired(&drawn, "evt_crecy_chevauchee_sans_bataille"));
    assert!(!fired(&drawn, "evt_crecy"));
}

#[test]
fn bretigny_cedes_the_provinces_when_the_king_is_captive() {
    let data = game_data();
    let mut state = staged(1360, Season::Spring);
    england_wins(&mut state, 1);
    apply(
        &mut state,
        data,
        "fac_france",
        EventEffect::CaptureCharacter {
            id: CharacterRef::Ruler,
            faction: None,
            captor: fac("fac_england"),
        },
    );
    state.end_turn_with(data, idle);
    assert!(fired(&state, "evt_bretigny"));
    assert!(!fired(&state, "evt_bretigny_paix_blanche"));
    assert!(!fired(&state, "evt_bretigny_angleterre_a_bout"));
    for p in [
        "prov_poitou",
        "prov_saintonge",
        "prov_agenais",
        "prov_boulonnais",
    ] {
        assert_eq!(
            state.province_owner(&prov(p)),
            Some(&fac("fac_england")),
            "{p} must pass to England"
        );
    }
    assert!(!state.is_at_war(&fac("fac_england"), &fac("fac_france")));
    assert!(state.chronicle.flags.contains("bretigny_signe"));
}

#[test]
fn bretigny_cedes_after_a_string_of_defeats_even_with_a_free_king() {
    let data = game_data();
    let mut state = staged(1360, Season::Spring);
    england_wins(&mut state, 3);
    assert!(!ruler_captive(&state));
    state.end_turn_with(data, idle);
    assert!(fired(&state, "evt_bretigny"));
    assert_eq!(
        state.province_owner(&prov("prov_poitou")),
        Some(&fac("fac_england"))
    );
}

#[test]
fn bretigny_is_a_white_peace_when_nobody_leads() {
    let data = game_data();
    let mut state = staged(1360, Season::Spring);
    state.end_turn_with(data, idle);
    assert!(fired(&state, "evt_bretigny_paix_blanche"));
    assert!(!fired(&state, "evt_bretigny"));
    for p in [
        "prov_poitou",
        "prov_saintonge",
        "prov_agenais",
        "prov_boulonnais",
    ] {
        assert_eq!(state.province_owner(&prov(p)), Some(&fac("fac_france")));
    }
}

#[test]
fn bretigny_turns_when_england_is_broke() {
    let data = game_data();
    let mut state = staged(1360, Season::Spring);
    state
        .factions
        .get_mut(&fac("fac_england"))
        .unwrap()
        .treasury = 0;
    // England is deep in debt and has no army left under arms.
    state
        .factions
        .get_mut(&fac("fac_england"))
        .unwrap()
        .treasury = -200_000;
    state
        .armies
        .retain(|_, army| army.faction != fac("fac_england"));
    state.end_turn_with(data, idle);
    assert!(fired(&state, "evt_bretigny_angleterre_a_bout"), "ratio");
    assert!(!fired(&state, "evt_bretigny"));
}

#[test]
fn joan_of_arc_needs_a_realm_on_the_brink() {
    let data = game_data();
    let mut state = staged(1429, Season::Spring);
    state.end_turn_with(data, idle);
    assert!(!fired(&state, "evt_jeanne_d_arc"), "France is whole");
    let mut state = staged(1429, Season::Spring);
    for p in [
        "prov_normandie",
        "prov_boulonnais",
        "prov_touraine",
        "prov_poitou",
        "prov_saintonge",
        "prov_flandre_wallonne",
    ] {
        apply(
            &mut state,
            data,
            "fac_england",
            EventEffect::TransferProvince {
                province: prov(p),
                faction: None,
                from: Some(fac("fac_france")),
                price: 0,
                payer: None,
            },
        );
    }
    state.end_turn_with(data, idle);
    assert!(fired(&state, "evt_jeanne_d_arc"));
}

#[test]
fn the_national_surge_needs_a_collapsed_domain() {
    let data = game_data();
    let event = &data.events[&evt("evt_sursaut_national")];
    let mut state = staged(1400, Season::Spring);
    let ctx = EventContext {
        faction: Some(fac("fac_france")),
        province: None,
    };
    assert!(!state.event_conditions_hold(data, event, &ctx));
    for p in [
        "prov_normandie",
        "prov_boulonnais",
        "prov_touraine",
        "prov_poitou",
        "prov_saintonge",
        "prov_agenais",
    ] {
        apply(
            &mut state,
            data,
            "fac_england",
            EventEffect::TransferProvince {
                province: prov(p),
                faction: None,
                from: Some(fac("fac_france")),
                price: 0,
                payer: None,
            },
        );
    }
    assert!(state.event_conditions_hold(data, event, &ctx));
}

#[test]
fn the_castilian_fork_leaves_a_claim_and_a_war() {
    let data = game_data();
    // John of Gaunt is born in 1340: let the births of that year happen first.
    let mut state = staged(1340, Season::Spring);
    for _ in 0..4 {
        state.end_turn_with(data, idle);
    }
    assert!(state.characters.contains_key(&chr("chr_jean_de_gand")));
    state.year = 1371;
    state.season = Season::Autumn;
    state.end_turn_with(data, idle);
    assert!(fired(&state, "evt_mariage_de_jean_de_gand"));
    // The AI weights take the claim: the flag stays for fifteen years.
    assert!(state.chronicle.flags.contains("gand_roi_de_castille"));
    state.year = 1386;
    state.season = Season::Summer;
    state.end_turn_with(data, idle);
    assert!(fired(&state, "evt_expedition_de_galice"));
    assert!(state.is_at_war(&fac("fac_england"), &fac("fac_castile")));
    assert!(state.chronicle.flags.contains("expedition_de_galice"));
}

#[test]
fn without_the_claim_the_castilian_chain_stays_silent() {
    let data = game_data();
    let mut state = staged(1386, Season::Summer);
    state.end_turn_with(data, idle);
    assert!(!fired(&state, "evt_expedition_de_galice"));
    assert!(!fired(&state, "evt_traite_de_bayonne"));
}

#[test]
fn the_brittany_chain_depends_on_the_oath() {
    let data = game_data();
    let mut state = staged(1378, Season::Summer);
    state.end_turn_with(data, idle);
    assert!(
        !fired(&state, "evt_confiscation_de_la_bretagne"),
        "no English alliance, no confiscation"
    );
    let mut state = staged(1378, Season::Summer);
    state
        .chronicle
        .flags
        .insert("bretagne_alliance_anglaise".to_owned());
    state.end_turn_with(data, idle);
    assert!(fired(&state, "evt_confiscation_de_la_bretagne"));
}

/// A staged war after the births of 1337-1340 (Charles V, John of Gaunt).
fn staged_after_births(year: i32, season: Season) -> CampaignState {
    let data = game_data();
    let mut state = staged(1337, Season::Spring);
    for _ in 0..16 {
        state.end_turn_with(data, idle);
    }
    state.year = year;
    state.season = season;
    set_war(&mut state, "fac_england", "fac_france");
    state
}

/// Every other conditioned event has a staged state in which it fires, so
/// that none is dead data even if the simulated games of the AI draw only
/// some branches of the forks.
#[test]
fn the_remaining_events_fire_in_their_staged_states() {
    let data = game_data();
    let raise = |state: &mut CampaignState, flag: &str| {
        state.chronicle.flags.insert(flag.to_owned());
    };
    // Castile's side of the Galician landing, and the treaty of Bayonne.
    let mut state = staged(1386, Season::Autumn);
    raise(&mut state, "expedition_de_galice");
    state.end_turn_with(data, idle);
    assert!(fired(&state, "evt_castille_menacee"));
    let mut state = staged(1388, Season::Autumn);
    raise(&mut state, "gand_roi_de_castille");
    state.end_turn_with(data, idle);
    assert!(fired(&state, "evt_traite_de_bayonne"));
    assert!(state.chronicle.flags.contains("castille_reconciliee"));
    // Charles V's reforms follow the restored order in Paris.
    let mut state = staged_after_births(1364, Season::Summer);
    raise(&mut state, "dauphin_maitre_de_paris");
    state.end_turn_with(data, idle);
    assert!(fired(&state, "evt_ordonnances_de_charles_v"));
    // The oath of Guérande follows Auray; the end of Marcel follows 1357.
    let mut state = staged(1365, Season::Spring);
    state.chronicle.fired_events.insert(evt("evt_auray"));
    state.end_turn_with(data, idle);
    assert!(fired(&state, "evt_hommage_de_guerande"));
    let mut state = staged(1358, Season::Summer);
    state
        .chronicle
        .fired_events
        .insert(evt("evt_etienne_marcel"));
    state.end_turn_with(data, idle);
    assert!(fired(&state, "evt_fin_d_etienne_marcel"));
    // The English readings, when the player is England (he holds the pen).
    let mut state = start(data, "fac_england", 5);
    state.year = 1360;
    state.season = Season::Spring;
    set_war(&mut state, "fac_england", "fac_france");
    state.end_turn_with(data, idle);
    assert!(fired(&state, "evt_bretigny_paix_blanche_anglaise"));
    assert!(!fired(&state, "evt_bretigny_paix_blanche"));
    let mut state = start(data, "fac_england", 5);
    state.year = 1360;
    state.season = Season::Spring;
    set_war(&mut state, "fac_england", "fac_france");
    england_wins(&mut state, 3);
    state.end_turn_with(data, idle);
    assert!(fired(&state, "evt_bretigny_angleterre"));
    assert!(!fired(&state, "evt_bretigny"));
    let mut state = start(data, "fac_england", 5);
    state.year = 1360;
    state.season = Season::Spring;
    set_war(&mut state, "fac_england", "fac_france");
    state
        .factions
        .get_mut(&fac("fac_england"))
        .unwrap()
        .treasury = -200_000;
    state
        .armies
        .retain(|_, army| army.faction != fac("fac_england"));
    state.end_turn_with(data, idle);
    assert!(fired(&state, "evt_bretigny_roi_sans_argent"));
}

#[test]
fn paris_can_turn_to_the_king_of_navarre_only_after_the_ordinance() {
    let data = game_data();
    let event = &data.events[&evt("evt_fin_d_etienne_marcel")];
    let navarre_option = event
        .options
        .iter()
        .position(|o| o.text.contains("roi de Navarre"))
        .expect("Navarre option");
    let mut state = staged(1358, Season::Summer);
    let ctx = EventContext {
        faction: Some(fac("fac_france")),
        province: Some(prov("prov_ile_de_france")),
    };
    assert!(state
        .option_unavailable(data, event, navarre_option, &ctx)
        .is_some());
    state
        .chronicle
        .flags
        .insert("grande_ordonnance_acceptee".to_owned());
    assert!(state
        .option_unavailable(data, event, navarre_option, &ctx)
        .is_none());
}

#[test]
fn every_flag_read_by_an_event_is_raised_by_another() {
    let data = game_data();
    let mut raised = std::collections::BTreeSet::new();
    let mut read = std::collections::BTreeSet::new();
    fn walk(c: &Condition, read: &mut std::collections::BTreeSet<String>) {
        match c {
            Condition::Flag { flag } => {
                read.insert(flag.clone());
            }
            Condition::Not { condition } => walk(condition, read),
            Condition::AnyOf { conditions } => conditions.iter().for_each(|c| walk(c, read)),
            _ => {}
        }
    }
    for event in data.events.values() {
        event
            .trigger
            .conditions
            .iter()
            .for_each(|c| walk(c, &mut read));
        for option in &event.options {
            option.requires.iter().for_each(|c| walk(c, &mut read));
            for effect in &option.effects {
                if let EventEffect::SetFlag { flag } = effect {
                    raised.insert(flag.clone());
                }
            }
        }
    }
    let orphans: Vec<_> = read.difference(&raised).collect();
    assert!(
        orphans.is_empty(),
        "flags read but never raised: {orphans:?}"
    );
}

// ---------------------------------------------------------------------------
// Simulation: how much of the new history a whole AI game really draws.
// ---------------------------------------------------------------------------

/// The events this lot made conditional or added.
const EVENTS_HISTORY: &[&str] = &[
    "evt_bretigny",
    "evt_bretigny_angleterre",
    "evt_bretigny_paix_blanche",
    "evt_bretigny_paix_blanche_anglaise",
    "evt_bretigny_angleterre_a_bout",
    "evt_bretigny_roi_sans_argent",
    "evt_poitiers",
    "evt_poitiers_victoire_francaise",
    "evt_maupertuis_sans_bataille",
    "evt_crecy",
    "evt_crecy_triomphe_anglais",
    "evt_crecy_revers_anglais",
    "evt_crecy_victoire_francaise",
    "evt_crecy_chevauchee_sans_bataille",
    "evt_jeanne_d_arc",
    "evt_sursaut_national",
    "evt_mariage_de_jean_de_gand",
    "evt_expedition_de_galice",
    "evt_traite_de_bayonne",
    "evt_castille_menacee",
    "evt_hommage_de_guerande",
    "evt_confiscation_de_la_bretagne",
    "evt_fin_d_etienne_marcel",
    "evt_ordonnances_de_charles_v",
];

/// Plays 1337-1453 with the AI everywhere (the player included) and returns
/// the events fired.
fn simulate(data: &GameData, player: &str, seed: u64) -> BTreeMap<EventId, i32> {
    let player = fac(player);
    let mut state = CampaignState::new_1337(data, player.clone(), seed).expect("1337 start");
    let mut fired: BTreeMap<EventId, i32> = BTreeMap::new();
    while state.year <= 1453 {
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
        for event in &state.chronicle.fired_events {
            fired.entry(event.clone()).or_insert(state.year);
        }
        if std::env::var("EV_TRACE").is_ok() && state.season == Season::Spring {
            let at_war = state.is_at_war(&fac("fac_england"), &fac("fac_france"));
            let france = state.controlled_provinces(&fac("fac_france")).count();
            let domain = state.domain_share_percent(data, &fac("fac_france"));
            println!(
                "{} war={at_war} france_provinces={france} domain={domain:?} tally={:?} ruler_captive={}",
                state.year,
                state
                    .chronicle
                    .battle_tally
                    .iter()
                    .map(|t| format!("{}>{}:{}", &t.winner.as_str()[4..], &t.loser.as_str()[4..], t.victories))
                    .collect::<Vec<_>>(),
                ruler_captive(&state)
            );
        }
    }
    fired
}

/// The minimal AI of `sim-campaign` never meets the enemy in the field (the
/// strategic AI of the `ai` crate does: see `examples/events_probe.rs`), so a
/// game it plays draws the neutral readings of the conditioned events and the
/// forks that need no battle. They stay reachable (fixed seed 1341 of the
/// HIST lot).
#[test]
fn a_simulated_game_draws_the_neutral_readings() {
    let data = game_data();
    let fired = simulate(data, "fac_france", 1341);
    for id in [
        "evt_maupertuis_sans_bataille",
        "evt_crecy_chevauchee_sans_bataille",
    ] {
        assert!(fired.contains_key(&evt(id)), "{id} did not fire");
    }
    let hits = EVENTS_HISTORY
        .iter()
        .filter(|id| fired.contains_key(&evt(id)))
        .count();
    assert!(hits >= 4, "only {hits}/{} fired", EVENTS_HISTORY.len());
}
