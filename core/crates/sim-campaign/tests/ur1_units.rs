//! UR1 « variété des unités » integration tests: regional, faction and
//! period units (`available_from` / `available_until`). See
//! `docs/wip/ur1-unites.md`.

use std::path::PathBuf;

use data_model::{FactionId, GameData, ProvinceId, SettlementId, TechnologyId, UnitTypeId};
use sim_campaign::CampaignState;

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn unit(id: &str) -> UnitTypeId {
    UnitTypeId::new(id).unwrap()
}

fn city(state: &CampaignState, province: &str) -> SettlementId {
    state
        .province_city_id(&ProvinceId::new(province).unwrap())
        .unwrap()
        .clone()
}

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
    unit_type: &str,
) -> Option<String> {
    state
        .recruit_option(data, &fac(faction), settlement, &unit(unit_type))
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
    let data = data();
    for id in UR1_UNITS {
        let t = &data.unit_types[&unit(id)];
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
    let data = data();
    let mut state = start(&data, "fac_france");
    let paris = city(&state, "prov_ile_de_france");
    let why = reason(
        &state,
        &data,
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
        &data,
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
            &data,
            "fac_france",
            &paris,
            "unit_ordonnance_gendarmes"
        ),
        None
    );
}

#[test]
fn mercenary_bands_come_and_go_with_their_period() {
    let data = data();
    let mut state = start(&data, "fac_france");
    let paris = city(&state, "prov_ile_de_france");
    assert_eq!(
        reason(&state, &data, "fac_france", &paris, "unit_routiers").as_deref(),
        Some("disponible à partir de 1356")
    );
    state.year = 1360;
    assert_eq!(
        reason(&state, &data, "fac_france", &paris, "unit_routiers"),
        None
    );
    state.year = 1400;
    assert_eq!(
        reason(&state, &data, "fac_france", &paris, "unit_routiers").as_deref(),
        Some("plus levée après 1395")
    );
}

#[test]
fn regional_units_need_their_culture() {
    let data = data();
    let state = start(&data, "fac_england");
    let wales = city(&state, "prov_gwynedd");
    let london = city(&state, "prov_middlesex");
    assert_eq!(
        reason(&state, &data, "fac_england", &wales, "unit_welsh_spearmen"),
        None
    );
    assert_eq!(
        reason(&state, &data, "fac_england", &london, "unit_welsh_spearmen").as_deref(),
        Some("culture locale inadaptée")
    );
    // The English retinues are England's own.
    let mut france = start(&data, "fac_france");
    france
        .factions
        .get_mut(&fac("fac_france"))
        .unwrap()
        .technologies
        .insert(TechnologyId::new("tech_dismounted_tactics").unwrap());
    let paris = city(&france, "prov_ile_de_france");
    assert_eq!(
        reason(&france, &data, "fac_france", &paris, "unit_english_retinue").as_deref(),
        Some("réservé à d'autres factions")
    );
    assert_ne!(
        reason(
            &state,
            &data,
            "fac_england",
            &london,
            "unit_english_retinue"
        )
        .as_deref(),
        Some("réservé à d'autres factions")
    );
}
