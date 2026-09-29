//! Lot OM3 (ADR 0116): steppe and desert terrains, arid and steppe climates,
//! the Orthodox church as a kindred (schismatic) faith. Values come from
//! `data/rules/*`; the provinces here are synthetic (a French province
//! switched to steppe or desert), the geography is another lot's.

use std::path::PathBuf;

use data_model::{Climate, FactionId, GameData, ProvinceId, ReligionId, Terrain};
use sim_campaign::economy::seasonal_supply_change;
use sim_campaign::religion::{religions_relation, same_faith, FaithRelation};
use sim_campaign::weather::chances_for;
use sim_campaign::{CampaignState, Season};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn prov(id: &str) -> ProvinceId {
    ProvinceId::new(id).unwrap()
}

fn rel(id: &str) -> ReligionId {
    ReligionId::new(id).unwrap()
}

/// `data` with `province` turned into `terrain`.
fn with_terrain(mut data: GameData, province: &str, terrain: Terrain) -> GameData {
    data.provinces.get_mut(&prov(province)).unwrap().terrain = terrain;
    data
}

#[test]
fn enums_parse_from_the_json_keys() {
    let terrain: Terrain = serde_json::from_str("\"steppe\"").unwrap();
    assert_eq!(terrain, Terrain::Steppe);
    assert_eq!(Terrain::Desert.key(), "desert");
    let climate: Climate = serde_json::from_str("\"arid\"").unwrap();
    assert_eq!(climate, Climate::Arid);
    let climate: Climate = serde_json::from_str("\"steppe\"").unwrap();
    assert_eq!(climate, Climate::Steppe);
}

#[test]
fn rules_files_carry_the_eastern_terrains() {
    let data = data();
    let desert = data.economy_rules.terrain_supply(Terrain::Desert);
    let steppe = data.economy_rules.terrain_supply(Terrain::Steppe);
    let plains = data.economy_rules.terrain_supply(Terrain::Plains);
    assert!(desert.recovery_percent < steppe.recovery_percent);
    assert!(steppe.recovery_percent < plains.recovery_percent);
    assert!(desert.summer_loss > 0);
    let costs = &data.free_movement_rules().terrain_costs;
    assert_eq!(costs.steppe, costs.plains, "steppe moves like plains");
    assert!(costs.desert > costs.plains, "desert is slower");
    assert!(costs.province_factor(Terrain::Desert).unwrap() > 1.0);
    let auto = &data.auto_resolve;
    assert!(auto.terrain_effects(Terrain::Steppe).charge > 1.0);
    assert!(auto.terrain_effects(Terrain::Desert).charge < 1.0);
}

#[test]
fn arid_climate_is_dry_and_steppe_snowy_in_winter() {
    let data = data();
    let rules = &data.campaign_weather;
    let arid = chances_for(rules, Some(Climate::Arid), Season::Summer);
    let oceanic = chances_for(rules, Some(Climate::Oceanic), Season::Summer);
    assert!(arid.rain * 5 < oceanic.rain, "almost no summer rain");
    assert!(arid.clear > oceanic.clear);
    let steppe = chances_for(rules, Some(Climate::Steppe), Season::Winter);
    assert!(steppe.snow > steppe.rain);
}

#[test]
fn forage_is_thin_on_the_steppe_and_the_desert_bites_in_summer() {
    let base = data();
    let france = FactionId::new("fac_france").unwrap();
    let id = prov("prov_champagne");
    let change = |terrain: Terrain, friendly: bool, season: Season| {
        let data = with_terrain(base.clone(), "prov_champagne", terrain);
        let state = CampaignState::new_1337(&data, france.clone(), 7).expect("1337 start");
        seasonal_supply_change(&state, &data, &id, friendly, None, season)
    };
    // Friendly territory: less recovery on the steppe, less still in the desert.
    let plains = change(Terrain::Plains, true, Season::Spring);
    let steppe = change(Terrain::Steppe, true, Season::Spring);
    let desert = change(Terrain::Desert, true, Season::Spring);
    assert!(
        plains > steppe && steppe > desert && desert > 0,
        "{plains} {steppe} {desert}"
    );
    // The desert summer eats supply even at home.
    assert!(change(Terrain::Desert, true, Season::Summer) < desert);
    assert!(change(Terrain::Desert, true, Season::Summer) <= 0);
    // Hostile territory: heavier losses.
    let plains = change(Terrain::Plains, false, Season::Spring);
    let steppe = change(Terrain::Steppe, false, Season::Spring);
    let desert = change(Terrain::Desert, false, Season::Spring);
    assert!(
        plains < 0 && steppe < plains && desert < steppe,
        "{plains} {steppe} {desert}"
    );
    assert!(change(Terrain::Desert, false, Season::Summer) < desert);
}

#[test]
fn orthodox_are_schismatics_not_infidels() {
    let data = data();
    let (orthodox, catholic) = (rel("rel_orthodox"), rel("rel_catholic"));
    assert_eq!(
        religions_relation(&data, &orthodox, &catholic),
        FaithRelation::Kindred
    );
    assert_eq!(
        religions_relation(&data, &catholic, &orthodox),
        FaithRelation::Kindred
    );
    // An obedience counts as its parent church.
    assert_eq!(
        religions_relation(&data, &rel("rel_catholic_rome"), &orthodox),
        FaithRelation::Kindred
    );
    assert!(!same_faith(&data, &orthodox, &catholic));
    for other in ["rel_islam", "rel_pagan", "rel_armenian"] {
        assert_eq!(
            religions_relation(&data, &rel(other), &catholic),
            FaithRelation::Different,
            "{other}"
        );
    }
}
