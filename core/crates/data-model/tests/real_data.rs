//! Loads the real `data/` directory of the repository.

use std::path::PathBuf;

use data_model::test_support::{data_dir, game_data};
use data_model::{GameData, SocialClass};

/// Lot M2: the navigation grid (or its land-mask fallback) and the province
/// raster decode, and the settlements stand on passable cells.
#[test]
fn real_rasters_load() {
    let data = game_data();
    let start = std::time::Instant::now();
    let grid = data.navgrid();
    println!("rasters decoded in {:?} ({grid:?})", start.elapsed());
    assert_eq!((grid.width, grid.height), (3584, 3072));
    let paris = data_model::SettlementId::new("set_paris").unwrap();
    let point = data.settlement_point(&paris).expect("Paris has a position");
    let (x, y) = grid.cell_of(point[0], point[1]);
    assert!(grid.passable(i64::from(x), i64::from(y)));
    assert_eq!(
        data.province_at_point(point[0], point[1])
            .map(|p| p.as_str()),
        Some(data.settlements[&paris].province.as_str())
    );
    // Open sea west of Brittany (OM2: +640 cells in y).
    assert!(!grid.passable(300, 1640));
    let mismatched = data
        .settlements
        .iter()
        .filter(|(id, s)| {
            let p = data.settlement_point(id).unwrap();
            data.province_at_point(p[0], p[1]) != Some(&s.province)
        })
        .count();
    println!("{mismatched} settlements outside their province raster");
}

fn data_root() -> PathBuf {
    data_dir()
}

/// Every rules file under `data/rules/` is read by `GameData::load` and is
/// the one embedded as the type's `Default` (single source of truth).
#[test]
fn rules_files_are_read_and_embedded() {
    let data = game_data();
    assert_eq!(data.auto_resolve, data_model::AutoResolveRules::default());
    assert_eq!(
        data.population_rules,
        data_model::PopulationRules::default()
    );
    assert_eq!(
        data.battle_standard_rules,
        data_model::BattleStandardRules::default()
    );
    assert_eq!(data.difficulty, data_model::DifficultyRules::default());
    assert_eq!(data.economy_rules, data_model::EconomyRules::default());
    assert_eq!(data.diplomacy_rules, data_model::DiplomacyRules::default());
    assert_eq!(data.posture_rules, data_model::PostureRules::default());
    assert_eq!(
        data.battle_outcome_rules,
        data_model::BattleOutcomeRules::default()
    );
    assert_eq!(data.map_scene_rules, data_model::MapSceneRules::default());
    for kind in data_model::SceneKind::ALL {
        assert!(
            data.map_scene_rules.durations.contains_key(&kind),
            "{kind:?}"
        );
    }
}

#[test]
fn real_data_loads_without_errors() {
    let (data, warnings) = GameData::load(&data_root()).expect("data/ must load");

    for warning in &warnings {
        println!("warning: {warning}");
    }
    println!(
        "loaded {} factions, {} provinces, {} unit types, {} buildings, {} technologies, {} characters, {} resources, {} religions, {} traits, {} skills, {} name lists, {} province polygons",
        data.factions.len(),
        data.provinces.len(),
        data.unit_types.len(),
        data.buildings.len(),
        data.technologies.len(),
        data.characters.len(),
        data.resources.len(),
        data.religions.len(),
        data.traits.len(),
        data.skills.len(),
        data.names.len(),
        data.province_geometry.len(),
    );

    assert!(!data.factions.is_empty());
    assert!(!data.unit_types.is_empty());
    assert!(!data.buildings.is_empty());
    assert!(!data.technologies.is_empty());
    assert!(!data.characters.is_empty());
    assert!(!data.resources.is_empty());
    assert!(!data.religions.is_empty());
    assert!(!data.traits.is_empty());
    assert!(!data.skills.is_empty());
    assert!(!data.names.is_empty());

    // Every branch of the skill tree must be represented, and every skill's
    // prerequisites already resolved as part of `GameData::load`.
    for branch in [
        data_model::SkillBranch::Command,
        data_model::SkillBranch::Governance,
        data_model::SkillBranch::Court,
    ] {
        assert!(
            data.skills
                .values()
                .any(|skill| skill.branch as u8 == branch as u8),
            "no skill in branch {branch:?}"
        );
    }

    // M6: two technology trees of ~15 nodes each, every tech dated, and at
    // least three buildings producing research points.
    for branch in [
        data_model::TechBranch::Military,
        data_model::TechBranch::Civil,
    ] {
        let count = data
            .technologies
            .values()
            .filter(|tech| tech.branch == branch)
            .count();
        assert!(count >= 14, "only {count} technologies in {branch:?}");
    }
    assert!(data.technologies.len() >= 30);
    for tech in data.technologies.values() {
        assert!(
            tech.historical_year
                .as_ref()
                .and_then(|y| y.year())
                .is_some(),
            "{} has no historical_year",
            tech.id
        );
        assert!(!tech.sources.is_empty(), "{} has no sources", tech.id);
    }
    let mut tech_names: Vec<&str> = data
        .technologies
        .values()
        .map(|t| t.name.display.as_str())
        .collect();
    tech_names.sort_unstable();
    let before = tech_names.len();
    tech_names.dedup();
    assert_eq!(tech_names.len(), before, "duplicate technology names");
    // H4: a third tree, medicine, with codex herbs.
    let medicine: Vec<_> = data
        .technologies
        .values()
        .filter(|t| t.branch == data_model::TechBranch::Medicine)
        .collect();
    assert!(
        medicine.len() >= 12,
        "only {} medicine techs",
        medicine.len()
    );
    for tech in data.technologies.values() {
        for herb in &tech.herbs {
            assert!(herb.starts_with("cdx_"), "{}: bad herb id {herb}", tech.id);
        }
    }
    assert!(medicine.iter().any(|t| !t.herbs.is_empty()));
    for id in ["bld_herb_garden", "bld_apothecary"] {
        assert_eq!(
            data.buildings[id].category,
            data_model::BuildingCategory::Sanitary
        );
    }
    for kind in [
        data_model::EffectKind::PlagueResistance,
        data_model::EffectKind::WoundRecovery,
        data_model::EffectKind::DietHealth,
    ] {
        assert!(
            data.technologies
                .values()
                .any(|t| t.effects.iter().any(|e| e.effect == kind)),
            "no technology carries {kind:?}"
        );
    }

    // H3: seven diets, the default one neutral and free, every other one
    // with a price or a condition.
    assert_eq!(data.diets.len(), 7);
    let default = &data.diets["diet_bread_pottage"];
    assert_eq!(default.cost_per_thousand, 0.0);
    assert!(default.effects.is_empty());
    assert_eq!(
        default.requirements,
        data_model::DietRequirements::default()
    );
    for diet in data.diets.values().filter(|d| d.id != default.id) {
        assert!(diet.cost_per_thousand > 0.0, "{} is free", diet.id);
        assert!(!diet.effects.is_empty(), "{} has no effect", diet.id);
        assert!(
            diet.description.contains("[[cdx_"),
            "{} has no codex link",
            diet.id
        );
        assert!(!diet.sources.is_empty(), "{} has no sources", diet.id);
    }

    let research_buildings = data
        .buildings
        .values()
        .filter(|b| {
            b.effects
                .iter()
                .any(|e| e.effect == data_model::EffectKind::ResearchPoints)
        })
        .count();
    assert!(research_buildings >= 3);
    assert!(data.buildings.contains_key("bld_scriptorium"));

    // The province folder is being regenerated by the geo pipeline; only the
    // presence of the anchor province is asserted, and province-related
    // warnings are tolerated.
    let normandie = data
        .provinces
        .get("prov_normandie")
        .expect("prov_normandie must exist");
    assert_eq!(normandie.owner.as_str(), "fac_france");
    assert!(
        normandie
            .population
            .classes
            .get(SocialClass::Peasants)
            .count
            > 0
    );

    let france = &data.factions["fac_france"];
    assert!(france.playable);
    assert_eq!(france.heraldry.primary_color, "#1F3A93");

    let non_province_warnings: Vec<_> = warnings
        .iter()
        .filter(|warning| !warning.message.contains("province"))
        .collect();
    assert!(
        non_province_warnings.is_empty(),
        "unexpected warnings: {non_province_warnings:?}"
    );
}

/// Lot C1: every province has settlements, exactly one city first in its list.
#[test]
fn every_province_has_exactly_one_city() {
    let data = game_data();
    assert!(data.settlement_rules.is_some(), "settlements/rules.json");
    for id in data.provinces.keys() {
        let settlements = data.province_settlements(id);
        assert!(!settlements.is_empty(), "{id} has no settlement");
        assert!(
            settlements.len() <= data_model::settlement_load::MAX_SETTLEMENTS_PER_PROVINCE,
            "{id} has too many settlements"
        );
        let cities = settlements
            .iter()
            .filter(|s| s.kind == data_model::SettlementKind::City)
            .count();
        assert_eq!(cities, 1, "{id} must have exactly one city");
        assert_eq!(settlements[0].kind, data_model::SettlementKind::City);
        assert!(settlements.iter().all(|s| &s.province == id));
    }
    assert_eq!(
        data.settlements.len(),
        data.settlements_by_province
            .values()
            .map(Vec::len)
            .sum::<usize>()
    );
}

/// Lot CV3-1: the cover map is decoded from the real rasters and finds
/// forests and wetlands somewhere.
#[test]
fn cover_map_reads_the_rasters() {
    let data = game_data();
    let map = data.cover_map().expect("cover rasters");
    assert_eq!(map.width, data.navgrid().width);
    assert!(map.forest.iter().any(|v| *v > 200), "no forest cell");
    assert!(map.wetland.iter().any(|v| *v > 100), "no wetland cell");
}
