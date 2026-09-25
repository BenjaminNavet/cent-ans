//! Lot E1: the AI recruits after its faction's doctrine.

use std::collections::BTreeMap;
use std::path::Path;

use ai::doctrine::pick_recruit;
use data_model::{FactionId, GameData, UnitTypeId};
use sim_campaign::RecruitOption;

fn data() -> GameData {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("data").0
}

fn option(data: &GameData, id: &str) -> RecruitOption {
    let t = &data.unit_types[&UnitTypeId::new(id).unwrap()];
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
    }
}

fn units(pairs: &[(&str, u32)]) -> BTreeMap<UnitTypeId, u32> {
    pairs
        .iter()
        .map(|(id, n)| (UnitTypeId::new(*id).unwrap(), *n))
        .collect()
}

#[test]
fn doctrines_load_and_name_real_units() {
    let data = data();
    let doctrines = data.ai_doctrines.as_ref().expect("data/ai/doctrines.json");
    for doctrine in std::iter::once(&doctrines.default).chain(doctrines.factions.values()) {
        for unit in doctrine.mix.keys() {
            assert!(data.unit_types.contains_key(unit), "unknown unit {unit}");
        }
    }
    for faction in doctrines.factions.keys() {
        assert!(
            data.factions.contains_key(faction),
            "unknown faction {faction}"
        );
    }
}

#[test]
fn england_fills_its_archer_ranks_first() {
    let data = data();
    let england = FactionId::new("fac_england").unwrap();
    let options = [
        option(&data, "unit_urban_militia"),
        option(&data, "unit_longbowmen"),
        option(&data, "unit_men_at_arms_foot"),
    ];
    let refs: Vec<&RecruitOption> = options.iter().collect();
    let value = |o: &RecruitOption| 1000.0 / f64::from(o.cost);
    let first = pick_recruit(&data, &england, &refs, &BTreeMap::new(), value).unwrap();
    assert_eq!(first.unit_type.as_str(), "unit_longbowmen");
    // Plenty of archers already: the men-at-arms are now the furthest behind.
    let archers = units(&[("unit_longbowmen", 10)]);
    let next = pick_recruit(&data, &england, &refs, &archers, value).unwrap();
    assert_eq!(next.unit_type.as_str(), "unit_men_at_arms_foot");
}

#[test]
fn militia_is_no_longer_the_only_choice() {
    let data = data();
    let france = FactionId::new("fac_france").unwrap();
    let options = [
        option(&data, "unit_urban_militia"),
        option(&data, "unit_knights"),
        option(&data, "unit_crossbowmen"),
    ];
    let refs: Vec<&RecruitOption> = options.iter().collect();
    // Militia has the best power per livre, but France already has some.
    let value = |o: &RecruitOption| 1000.0 / f64::from(o.cost);
    let mut composition = units(&[("unit_urban_militia", 4)]);
    let mut picked = Vec::new();
    for _ in 0..10 {
        let pick = pick_recruit(&data, &france, &refs, &composition, value).unwrap();
        *composition.entry(pick.unit_type.clone()).or_default() += 1;
        picked.push(pick.unit_type.as_str().to_owned());
    }
    let militia = picked.iter().filter(|p| *p == "unit_urban_militia").count();
    assert!(militia <= 4, "militia picked {militia} times out of 10");
    assert!(picked.iter().any(|p| p == "unit_knights"));
    assert!(picked.iter().any(|p| p == "unit_crossbowmen"));
}
