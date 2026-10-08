//! UR1 « variété des unités » integration tests: regional, faction and
//! period units (`available_from` / `available_until`). See
//! `docs/archive/chantiers.md`.

use data_model::{GameData, SettlementId, TechnologyId};
use sim_campaign::test_support::{city, unit_type};
use sim_campaign::CampaignState;

use data_model::test_support::{fac, game_data};

fn start(data: &GameData, faction: &str) -> CampaignState {
    let mut state = CampaignState::new_1337(data, fac(faction), 7).expect("1337 start");
    state.chronicle.disabled = true;
    state.factions.get_mut(&fac(faction)).unwrap().treasury = 200_000;
    state
}

fn reason(
    state: &CampaignState,
    data: &GameData,
    faction: &str,
    settlement: &SettlementId,
    unit_id: &str,
) -> Option<String> {
    state
        .recruit_option(data, &fac(faction), settlement, &unit_type(unit_id))
        .expect("known unit")
        .reason
}

const UR1_UNITS: [&str; 14] = [
    "unit_welsh_spearmen",
    "unit_hobelars",
    "unit_english_retinue",
    "unit_scottish_spearmen",
    "unit_goedendag_militia",
    "unit_coutiliers",
    "unit_francs_archers",
    "unit_ordonnance_gendarmes",
    "unit_routiers",
    "unit_ecorcheurs",
    "unit_jinetes",
    "unit_breton_knights",
    "unit_gascon_crossbowmen",
    "unit_culveriners",
];

#[test]
fn ur1_units_are_sourced_and_have_a_figure() {
    let data = game_data();
    for id in UR1_UNITS {
        let t = &data.unit_types[&unit_type(id)];
        assert!(t.sources.len() >= 2, "{id}: at least two sources");
        assert!(t.figure.is_some(), "{id}: battle figurine");
        assert!(t.equipment.is_some() && t.description.is_some(), "{id}");
        if let (Some(from), Some(until)) = (t.available_from, t.available_until) {
            assert!(from < until, "{id}: period");
        }
    }
}

#[test]
fn ordonnance_companies_wait_for_1445_and_the_technology() {
    let data = game_data();
    let mut state = start(data, "fac_france");
    let paris = city(&state, "prov_ile_de_france");
    let why = reason(
        &state,
        data,
        "fac_france",
        &paris,
        "unit_ordonnance_gendarmes",
    )
    .unwrap();
    assert!(why.contains("technologie") || why.contains("1445"), "{why}");
    state
        .factions
        .get_mut(&fac("fac_france"))
        .unwrap()
        .technologies
        .insert(TechnologyId::new("tech_compagnies_d_ordonnance").unwrap());
    let why = reason(
        &state,
        data,
        "fac_france",
        &paris,
        "unit_ordonnance_gendarmes",
    )
    .unwrap();
    assert_eq!(why, "disponible à partir de 1445");
    state.year = 1446;
    assert_eq!(
        reason(
            &state,
            data,
            "fac_france",
            &paris,
            "unit_ordonnance_gendarmes"
        ),
        None
    );
}

#[test]
fn mercenary_bands_come_and_go_with_their_period() {
    // TW2-T3 (ADR 0103): companies are hired by an army from its region's
    // reserve, over the band's period; towns never levy them.
    let data = game_data();
    let mut state = start(data, "fac_france");
    let paris = city(&state, "prov_ile_de_france");
    let army = state
        .armies
        .iter()
        .find(|(_, a)| a.faction == fac("fac_france"))
        .map(|(id, _)| id.clone())
        .unwrap();
    state.armies.get_mut(&army).unwrap().position =
        sim_campaign::ArmyPosition::Settlement(paris.clone());
    let offered = |state: &CampaignState| {
        state
            .mercenary_market(data, &army)
            .unwrap()
            .options
            .iter()
            .any(|o| o.unit_type.as_str() == "unit_routiers")
    };
    assert!(!offered(&state));
    state.year = 1360;
    assert!(offered(&state));
    assert_eq!(
        reason(&state, data, "fac_france", &paris, "unit_routiers").as_deref(),
        Some("compagnie de mercenaires : à engager depuis une armée")
    );
    state.year = 1400;
    assert!(!offered(&state));
}

#[test]
fn regional_units_need_their_culture() {
    let data = game_data();
    let state = start(data, "fac_england");
    let wales = city(&state, "prov_gwynedd");
    let london = city(&state, "prov_middlesex");
    assert_eq!(
        reason(&state, data, "fac_england", &wales, "unit_welsh_spearmen"),
        None
    );
    assert_eq!(
        reason(&state, data, "fac_england", &london, "unit_welsh_spearmen").as_deref(),
        Some("culture locale inadaptée")
    );
    // The English retinues are England's own.
    let mut france = start(data, "fac_france");
    france
        .factions
        .get_mut(&fac("fac_france"))
        .unwrap()
        .technologies
        .insert(TechnologyId::new("tech_dismounted_tactics").unwrap());
    let paris = city(&france, "prov_ile_de_france");
    assert_eq!(
        reason(&france, data, "fac_france", &paris, "unit_english_retinue").as_deref(),
        Some("réservé à d'autres factions")
    );
    assert_ne!(
        reason(&state, data, "fac_england", &london, "unit_english_retinue").as_deref(),
        Some("réservé à d'autres factions")
    );
}
