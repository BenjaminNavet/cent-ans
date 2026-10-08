//! Lot OMR R5: eastern unit types of 1337 (mamluks, steppe horse archers,
//! akıncı, yaya, Serbian heavy cavalry, pronoiars, druzhina, Lithuanian
//! light cavalry, Teutonic knights, almogavars). Recruitment uses the
//! existing culture / faction / period rules; the AI levies them through
//! its doctrines. See `docs/archive/chantiers.md`.

use sim_campaign::test_support::{city, start_quiet, unit_type};
use std::collections::BTreeMap;

use ai::doctrine::pick_recruit;
use data_model::GameData;
use sim_campaign::{CampaignState, RecruitOption};

const EASTERN_UNITS: [&str; 10] = [
    "unit_mamluk_cavalry",
    "unit_steppe_horse_archers",
    "unit_akinci",
    "unit_yaya",
    "unit_serbian_heavy_cavalry",
    "unit_pronoiars",
    "unit_druzhina",
    "unit_lithuanian_light_cavalry",
    "unit_teutonic_knights",
    "unit_almogavars",
];

/// Refusals that come from the unit's own culture / faction / period gates.
const GATES: [&str; 4] = [
    "culture locale inadaptée",
    "réservé à d'autres factions",
    "disponible à partir de",
    "plus levée après",
];

use data_model::test_support::{fac, game_data};

/// Recruitment refusal of `unit_id` for `faction` in the city of `province`.
fn reason(
    state: &CampaignState,
    data: &GameData,
    faction: &str,
    province: &str,
    unit_id: &str,
) -> Option<String> {
    let settlement = city(state, province);
    state
        .recruit_option(data, &fac(faction), &settlement, &unit_type(unit_id))
        .expect("known unit")
        .reason
}

fn gated(why: &Option<String>) -> bool {
    why.as_deref()
        .is_some_and(|w| GATES.iter().any(|g| w.starts_with(g)))
}

#[test]
fn eastern_units_are_sourced_and_have_a_figure() {
    let data = game_data();
    for id in EASTERN_UNITS {
        let t = &data.unit_types[&unit_type(id)];
        assert!(t.sources.len() >= 3, "{id}: at least three sources");
        assert!(t.figure.is_some(), "{id}: battle figurine");
        assert!(t.equipment.is_some() && t.description.is_some(), "{id}");
        assert!(
            !t.required_culture.is_empty() || !t.required_faction.is_empty(),
            "{id}: a regional unit"
        );
        // No anachronism: every eastern type is in service in 1337.
        assert!(t.available_from.is_none_or(|y| y <= 1337), "{id}");
        assert!(t.available_until.is_none_or(|y| y >= 1337), "{id}");
    }
}

#[test]
fn each_eastern_unit_is_levied_at_home_in_1337() {
    let data = game_data();
    let state = start_quiet(data, "fac_france", 7);
    let home = [
        ("unit_mamluk_cavalry", "fac_mamluks", "prov_cairo"),
        (
            "unit_steppe_horse_archers",
            "fac_golden_horde",
            "prov_saray",
        ),
        ("unit_akinci", "fac_ottoman", "prov_bursa"),
        ("unit_yaya", "fac_ottoman", "prov_bursa"),
        ("unit_serbian_heavy_cavalry", "fac_serbia", "prov_kosovo"),
        ("unit_pronoiars", "fac_byzantium", "prov_constantinople"),
        ("unit_druzhina", "fac_moscow", "prov_moscow"),
        (
            "unit_lithuanian_light_cavalry",
            "fac_lithuania",
            "prov_vilnius",
        ),
        ("unit_teutonic_knights", "fac_teutonic", "prov_pomesania"),
        ("unit_almogavars", "fac_aragon", "prov_barcelona"),
    ];
    for (id, faction, province) in home {
        let why = reason(&state, data, faction, province, id);
        assert!(!gated(&why), "{id} in {province} for {faction}: {why:?}");
    }
}

#[test]
fn eastern_units_stay_in_their_region() {
    let data = game_data();
    let state = start_quiet(data, "fac_france", 7);
    // Culture gate: no steppe archers in Paris, no druzhina in Cairo.
    assert_eq!(
        reason(
            &state,
            data,
            "fac_france",
            "prov_ile_de_france",
            "unit_steppe_horse_archers"
        )
        .as_deref(),
        Some("culture locale inadaptée")
    );
    assert_eq!(
        reason(&state, data, "fac_mamluks", "prov_cairo", "unit_druzhina").as_deref(),
        Some("culture locale inadaptée")
    );
    // Faction gate: mamluks, akıncı and Teutonic brethren are institutions,
    // not local levies.
    assert_eq!(
        reason(
            &state,
            data,
            "fac_golden_horde",
            "prov_saray",
            "unit_mamluk_cavalry"
        )
        .as_deref(),
        Some("réservé à d'autres factions")
    );
    assert_eq!(
        reason(
            &state,
            data,
            "fac_france",
            "prov_ile_de_france",
            "unit_akinci"
        )
        .as_deref(),
        Some("réservé à d'autres factions")
    );
    assert_eq!(
        reason(
            &state,
            data,
            "fac_lithuania",
            "prov_vilnius",
            "unit_teutonic_knights"
        )
        .as_deref(),
        Some("réservé à d'autres factions")
    );
}

#[test]
fn almogavars_and_yaya_have_a_period() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 7);
    state.year = 1401;
    assert_eq!(
        reason(
            &state,
            data,
            "fac_aragon",
            "prov_barcelona",
            "unit_almogavars"
        )
        .as_deref(),
        Some("plus levée après 1400")
    );
    state.year = 1451;
    assert_eq!(
        reason(&state, data, "fac_ottoman", "prov_bursa", "unit_yaya").as_deref(),
        Some("plus levée après 1450")
    );
}

fn option(data: &GameData, id: &str) -> RecruitOption {
    let t = &data.unit_types[&unit_type(id)];
    RecruitOption {
        unit_type: t.id.clone(),
        name: t.name.display.clone(),
        cost: t.cost.money,
        upkeep: t.upkeep,
        available: true,
        reason: None,
        resources: t.cost.resources.clone(),
        import_cost: 0,
        imported: BTreeMap::new(),
        pool: Default::default(),
    }
}

#[test]
fn eastern_doctrines_pick_their_own_troops() {
    let data = game_data();
    let value = |o: &RecruitOption| 1000.0 / f64::from(o.cost);
    let cases = [
        ("fac_mamluks", "unit_mamluk_cavalry"),
        ("fac_golden_horde", "unit_steppe_horse_archers"),
        ("fac_lithuania", "unit_lithuanian_light_cavalry"),
        ("fac_moscow", "unit_druzhina"),
    ];
    for (faction, expected) in cases {
        let options = [
            option(data, "unit_urban_militia"),
            option(data, "unit_crossbowmen"),
            option(data, expected),
        ];
        let refs: Vec<&RecruitOption> = options.iter().collect();
        let pick = pick_recruit(data, &fac(faction), &refs, &BTreeMap::new(), value).unwrap();
        assert_eq!(pick.unit_type.as_str(), expected, "{faction}");
    }
}
