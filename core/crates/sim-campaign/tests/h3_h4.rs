//! H3 « La Table » and H4 « Médecine » integration tests: the `set_diet`
//! order, diet costs and the « Table » budget line, fallbacks to the default
//! diet, Lent, diet effects, plague resistance, wound recovery, AI diets,
//! old saves and determinism. See `docs/design/2026-09-23-histoire-et-savoir.md`.

use std::path::PathBuf;

use data_model::{BuildingId, DietId, FactionId, GameData, ProvinceId, TechnologyId};
use sim_battle::{BattleOutcome, SideId, SideResult};
use sim_campaign::table::{self, DEFAULT_DIET, LENT_FISH_PIETY, LENT_PIETY_PENALTY};
use sim_campaign::{
    medicine, ArmyId, CampaignState, DietError, EventKind, Order, OrderError, Season,
};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn prov(id: &str) -> ProvinceId {
    ProvinceId::new(id).unwrap()
}

/// Mutable state of the city of a province (lot C4).
fn city_mut<'a>(
    state: &'a mut CampaignState,
    province: &ProvinceId,
) -> &'a mut sim_campaign::SettlementState {
    let id = state.province_city_id(province).unwrap().clone();
    state.settlements.get_mut(&id).unwrap()
}

fn diet(id: &str) -> DietId {
    DietId::new(id).unwrap()
}

fn tech(id: &str) -> TechnologyId {
    TechnologyId::new(id).unwrap()
}

/// France 1337 with the chronicle off (no random noise).
fn france(data: &GameData, seed: u64) -> CampaignState {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), seed).expect("1337 start");
    state.chronicle.disabled = true;
    state
}

fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

fn set_diet(
    state: &mut CampaignState,
    data: &GameData,
    province: &str,
    id: &str,
) -> Result<(), OrderError> {
    state.submit_order(
        data,
        Order::SetDiet {
            province: prov(province),
            diet: diet(id),
        },
    )
}

fn ruler_piety(state: &CampaignState, faction: &str) -> u8 {
    let ruler = state.factions[&fac(faction)].ruler.clone().unwrap();
    state.characters[&ruler].piety
}

#[test]
fn set_diet_is_validated() {
    let data = data();
    let mut state = france(&data, 1);
    assert_eq!(
        state.province_diet(&prov("prov_normandie")).as_str(),
        DEFAULT_DIET
    );

    let unknown = set_diet(&mut state, &data, "prov_normandie", "diet_ambrosia");
    assert!(matches!(
        unknown,
        Err(OrderError::Diet(DietError::UnknownDiet(_)))
    ));

    let foreign = state
        .provinces
        .iter()
        .find(|(id, _)| !state.controls_province(&fac("fac_france"), id))
        .map(|(id, _)| id.to_string())
        .unwrap();
    let error = set_diet(&mut state, &data, &foreign, "diet_pulses").unwrap_err();
    assert!(error.to_string().contains("n'est pas contrôlée"), "{error}");

    // Dairy needs grazing land: heath and forest (sheep and swine country) refuse it.
    let grazing_less = state
        .provinces
        .iter()
        .find(|(id, _)| {
            state.controls_province(&fac("fac_france"), id)
                && matches!(
                    data.provinces[*id].terrain,
                    data_model::Terrain::Heath | data_model::Terrain::Forest
                )
        })
        .map(|(id, _)| id.to_string());
    if let Some(province) = grazing_less {
        let error = set_diet(&mut state, &data, &province, "diet_dairy").unwrap_err();
        assert!(error.to_string().contains("terrain requis"), "{error}");
    }

    // The spiced table needs a market or a fair.
    // (every French province has one in 1337: pull them down in Touraine,
    // in every settlement — lot C4).
    let touraine = prov("prov_touraine");
    for settlement in state
        .settlements
        .values_mut()
        .filter(|s| s.province == touraine)
    {
        settlement
            .buildings
            .retain(|b| b.as_str() != "bld_market" && b.as_str() != "bld_fair");
    }
    let error = set_diet(&mut state, &data, "prov_touraine", "diet_spiced_table").unwrap_err();
    assert!(
        error
            .to_string()
            .contains("bâtiment requis : Marché ou Foire"),
        "{error}"
    );

    // Pulses need the three-field rotation, which France knows.
    set_diet(&mut state, &data, "prov_normandie", "diet_pulses").unwrap();
    assert_eq!(
        state.province_diet(&prov("prov_normandie")),
        diet("diet_pulses")
    );
    let again = set_diet(&mut state, &data, "prov_normandie", "diet_lenten_fish");
    assert!(matches!(
        again,
        Err(OrderError::Diet(DietError::AlreadyChanged(_)))
    ));
    state.end_turn_with(&data, idle);
    set_diet(&mut state, &data, "prov_normandie", "diet_lenten_fish").unwrap();

    // Without the technology, the refusal names it.
    state
        .factions
        .get_mut(&fac("fac_france"))
        .unwrap()
        .technologies
        .remove(&tech("tech_three_field_rotation"));
    let error = set_diet(&mut state, &data, "prov_ile_de_france", "diet_pulses").unwrap_err();
    assert!(
        error
            .to_string()
            .contains("technologie requise : Assolement triennal"),
        "{error}"
    );
}

#[test]
fn diet_cost_scales_with_population_and_winter() {
    let data = data();
    let mut state = france(&data, 2);
    let normandie = prov("prov_normandie");
    let population = state.provinces[&normandie].population.total() as f64;
    let dairy = &data.diets[&diet("diet_dairy")];
    let expected = (population / 1000.0 * dairy.cost_per_thousand).round() as i64;
    assert_eq!(
        state.diet_cost(&data, &normandie, &diet("diet_dairy")),
        expected
    );
    assert_eq!(state.diet_cost(&data, &normandie, &diet(DEFAULT_DIET)), 0);
    state.season = Season::Winter;
    let winter = (population / 1000.0 * dairy.cost_per_thousand * 1.5).round() as i64;
    assert_eq!(
        state.diet_cost(&data, &normandie, &diet("diet_dairy")),
        winter
    );
    // Salted meat keeps: no winter surcharge.
    let meat = &data.diets[&diet("diet_meat_salting")];
    assert_eq!(
        state.diet_cost(&data, &normandie, &diet("diet_meat_salting")),
        (population / 1000.0 * meat.cost_per_thousand).round() as i64
    );
}

#[test]
fn the_table_is_paid_in_the_economy() {
    let data = data();
    let mut reference = france(&data, 3);
    let mut state = france(&data, 3);
    set_diet(&mut state, &data, "prov_normandie", "diet_meat_salting").unwrap();
    set_diet(&mut state, &data, "prov_ile_de_france", "diet_wine_bread").unwrap();
    let expected = state.faction_table_upkeep(&data, &fac("fac_france"));
    assert!(expected > 0);
    let economy = state.faction_economy(&data, &fac("fac_france")).unwrap();
    assert_eq!(economy.table_upkeep, expected);
    reference.end_turn_with(&data, idle);
    state.end_turn_with(&data, idle);
    let paid = state.factions[&fac("fac_france")].table_upkeep_last_turn;
    assert_eq!(paid, expected);
    assert_eq!(
        reference.factions[&fac("fac_france")].table_upkeep_last_turn,
        0
    );
    assert_eq!(
        state.factions[&fac("fac_france")].upkeep_last_turn,
        reference.factions[&fac("fac_france")].upkeep_last_turn + paid
    );
}

#[test]
fn an_empty_treasury_brings_back_the_default_diet() {
    let data = data();
    let mut state = france(&data, 4);
    set_diet(&mut state, &data, "prov_normandie", "diet_meat_salting").unwrap();
    state.factions.get_mut(&fac("fac_france")).unwrap().treasury = -1_000_000;
    let events = state.end_turn_with(&data, idle);
    assert_eq!(
        state.province_diet(&prov("prov_normandie")).as_str(),
        DEFAULT_DIET
    );
    assert_eq!(state.factions[&fac("fac_france")].table_upkeep_last_turn, 0);
    assert!(events
        .iter()
        .any(|e| e.kind == EventKind::Table && e.text_fr.contains("le trésor ne peut payer")));
}

#[test]
fn lost_requirements_bring_back_the_default_diet() {
    let data = data();
    let mut state = france(&data, 5);
    set_diet(&mut state, &data, "prov_normandie", "diet_pulses").unwrap();
    state
        .factions
        .get_mut(&fac("fac_france"))
        .unwrap()
        .technologies
        .remove(&tech("tech_three_field_rotation"));
    let events = state.end_turn_with(&data, idle);
    assert_eq!(
        state.province_diet(&prov("prov_normandie")).as_str(),
        DEFAULT_DIET
    );
    assert!(events.iter().any(|e| e.kind == EventKind::Table
        && e.text_fr.contains("ne peut plus tenir")
        && e.text_fr.contains("Assolement triennal")));
}

#[test]
fn a_new_controller_does_not_inherit_the_diet() {
    let data = data();
    let mut state = france(&data, 6);
    set_diet(&mut state, &data, "prov_normandie", "diet_pulses").unwrap();
    city_mut(&mut state, &prov("prov_normandie")).controller = fac("fac_england");
    assert_eq!(
        state.province_diet(&prov("prov_normandie")).as_str(),
        DEFAULT_DIET
    );
}

#[test]
fn lent_punishes_meat_and_rewards_fish() {
    let data = data();
    let mut reference = france(&data, 7);
    assert!(reference.is_lent());
    let mut meat = reference.clone();
    let mut fish = reference.clone();
    set_diet(&mut meat, &data, "prov_carcassonne", "diet_meat_salting").unwrap();
    set_diet(&mut fish, &data, "prov_normandie", "diet_lenten_fish").unwrap();
    let before = ruler_piety(&reference, "fac_france");
    reference.end_turn_with(&data, idle);
    let meat_events = meat.end_turn_with(&data, idle);
    fish.end_turn_with(&data, idle);
    let base = i32::from(ruler_piety(&reference, "fac_france"));
    assert_eq!(
        i32::from(ruler_piety(&meat, "fac_france")),
        (base - LENT_PIETY_PENALTY).max(0)
    );
    assert_eq!(
        i32::from(ruler_piety(&fish, "fac_france")),
        (base + LENT_FISH_PIETY).min(100)
    );
    assert!(before > 0);
    let carcassonne = prov("prov_carcassonne");
    assert!(
        meat.provinces[&carcassonne].population.clergy.unrest
            > reference.provinces[&carcassonne].population.clergy.unrest
    );
    assert!(meat_events
        .iter()
        .any(|e| e.kind == EventKind::Table && e.text_fr.starts_with("Carême")));

    // Summer: no Lent, no penalty.
    assert!(!meat.is_lent());
    let piety = ruler_piety(&meat, "fac_france");
    let mut summer_reference = reference.clone();
    summer_reference.end_turn_with(&data, idle);
    meat.end_turn_with(&data, idle);
    let drift = i32::from(ruler_piety(&summer_reference, "fac_france"))
        - i32::from(ruler_piety(&reference, "fac_france"));
    assert_eq!(
        i32::from(ruler_piety(&meat, "fac_france")),
        i32::from(piety) + drift
    );
}

#[test]
fn diets_feed_the_population_and_the_regimen_boosts_them() {
    let data = data();
    let mut reference = france(&data, 8);
    reference.season = Season::Summer;
    let mut pulses = reference.clone();
    set_diet(&mut pulses, &data, "prov_normandie", "diet_pulses").unwrap();
    let normandie = prov("prov_normandie");
    let class = data_model::SocialClass::Peasants;
    let plain = table::diet_class_effects(&pulses, &data, &normandie, class);
    assert_eq!(plain.health.flat, 3.0);
    // Pulses only target the peasants' health.
    let nobles = table::diet_class_effects(
        &pulses,
        &data,
        &normandie,
        data_model::SocialClass::Nobility,
    );
    assert_eq!(nobles.health.flat, 0.0);
    assert_eq!(nobles.growth.percent, 5.0);
    pulses
        .factions
        .get_mut(&fac("fac_france"))
        .unwrap()
        .technologies
        .insert(tech("tech_regimen_sanitatis"));
    let boosted = table::diet_class_effects(&pulses, &data, &normandie, class);
    assert!((boosted.health.flat - 3.75).abs() < 1e-9);

    for _ in 0..4 {
        reference.end_turn_with(&data, idle);
        pulses.end_turn_with(&data, idle);
    }
    assert!(
        pulses.provinces[&normandie].population.peasants.health
            > reference.provinces[&normandie].population.peasants.health
    );
}

#[test]
fn salted_meat_raises_the_morale_of_levies() {
    let data = data();
    let mut state = france(&data, 9);
    set_diet(&mut state, &data, "prov_carcassonne", "diet_meat_salting").unwrap();
    let carcassonne = prov("prov_carcassonne");
    let (option, base_morale) = state
        .recruitable_in_province(&data, &carcassonne)
        .into_iter()
        .filter(|o| o.available)
        .map(|o| {
            let morale = data.unit_types[&o.unit_type].stats.morale;
            (o, morale)
        })
        .find(|(_, morale)| *morale <= 90)
        .expect("a recruitable unit");
    state
        .submit_order(
            &data,
            Order::Recruit {
                settlement: carcassonne.clone().into(),
                unit_type: option.unit_type.clone(),
            },
        )
        .unwrap();
    state.end_turn_with(&data, idle);
    let recruit = state
        .city_state(&carcassonne)
        .unwrap()
        .garrison
        .iter()
        .rev()
        .find(|u| u.unit_type == option.unit_type)
        .unwrap();
    assert!(
        recruit.morale >= base_morale + 5 - 1,
        "{} vs {base_morale}",
        recruit.morale
    );
}

fn make_plague(state: &mut CampaignState, province: &ProvinceId) {
    let p = state.provinces.get_mut(province).unwrap();
    for class in [
        &mut p.population.peasants,
        &mut p.population.burghers,
        &mut p.population.clergy,
        &mut p.population.nobility,
    ] {
        class.health = 5;
    }
}

fn give_medicine(state: &mut CampaignState, province: &ProvinceId) {
    let france = state.factions.get_mut(&fac("fac_france")).unwrap();
    for id in ["tech_plague_consilia", "tech_leprosaria", "tech_quarantine"] {
        france.technologies.insert(tech(id));
    }
    city_mut(state, province)
        .buildings
        .push(BuildingId::new("bld_apothecary").unwrap());
}

#[test]
fn plague_resistance_softens_the_local_plague() {
    let data = data();
    let normandie = prov("prov_normandie");
    let mut reference = france(&data, 10);
    assert_eq!(
        medicine::plague_resistance(&reference, &data, &normandie),
        0.0
    );
    let mut resistant = reference.clone();
    give_medicine(&mut resistant, &normandie);
    // 15 + 10 + 10 (techs) + 10 (apothecary) = 45 %.
    assert!((medicine::plague_resistance(&resistant, &data, &normandie) - 0.45).abs() < 1e-9);
    // Capped at 50 %.
    let mut capped = resistant.clone();
    city_mut(&mut capped, &normandie)
        .buildings
        .extend(["bld_apothecary", "bld_apothecary"].map(|b| BuildingId::new(b).unwrap()));
    assert_eq!(medicine::plague_resistance(&capped, &data, &normandie), 0.5);

    make_plague(&mut reference, &normandie);
    make_plague(&mut resistant, &normandie);
    let before = reference.provinces[&normandie].population.total() as f64;
    let reference_events = reference.end_turn_with(&data, idle);
    let events = resistant.end_turn_with(&data, idle);
    assert!(reference_events
        .iter()
        .any(|e| e.kind == EventKind::Plague && e.province.as_ref() == Some(&normandie)));
    let lost_reference = before - reference.provinces[&normandie].population.total() as f64;
    let lost_resistant = before - resistant.provinces[&normandie].population.total() as f64;
    assert!(lost_reference > before * 0.08);
    assert!(
        lost_resistant < lost_reference * 0.8,
        "{lost_resistant} vs {lost_reference}"
    );
    let spared = events
        .iter()
        .any(|e| e.kind == EventKind::Medicine && e.province.as_ref() == Some(&normandie));
    let struck = events
        .iter()
        .any(|e| e.kind == EventKind::Plague && e.province.as_ref() == Some(&normandie));
    assert!(spared != struck, "either spared or struck");
}

#[test]
fn plague_resistance_softens_the_black_death() {
    let data = data();
    let mut reference = france(&data, 11);
    reference.chronicle.plague_wave = Some(sim_campaign::PlagueWave {
        start_turn: reference.turn,
        duration: 1,
    });
    let mut resistant = reference.clone();
    let france_provinces: Vec<ProvinceId> = resistant
        .provinces
        .iter()
        .filter(|(id, _)| resistant.controls_province(&fac("fac_france"), id))
        .map(|(id, _)| id.clone())
        .collect();
    for id in &france_provinces {
        give_medicine(&mut resistant, id);
    }
    let total = |state: &CampaignState| -> u64 {
        france_provinces
            .iter()
            .map(|id| state.provinces[id].population.total())
            .sum()
    };
    let before = total(&reference);
    reference.end_turn_with(&data, idle);
    resistant.end_turn_with(&data, idle);
    let lost_reference = before - total(&reference);
    let lost_resistant = before - total(&resistant);
    assert!(lost_reference > before / 10);
    assert!(
        lost_resistant * 10 < lost_reference * 7,
        "{lost_resistant} vs {lost_reference}"
    );
}

fn first_army(state: &CampaignState, faction: &str) -> ArmyId {
    state
        .armies
        .iter()
        .find(|(_, a)| a.faction == fac(faction))
        .map(|(id, _)| id.clone())
        .unwrap()
}

/// Stages France (attacker) against England and applies an outcome where
/// each side loses a third of every unit.
fn fight(state: &mut CampaignState, data: &GameData) -> (ArmyId, ArmyId) {
    for (a, b) in [("fac_france", "fac_england"), ("fac_england", "fac_france")] {
        state
            .factions
            .get_mut(&fac(a))
            .unwrap()
            .at_war_with
            .insert(fac(b));
    }
    let attacker = first_army(state, "fac_france");
    let defender = first_army(state, "fac_england");
    let index = state.debug_stage_battle(&attacker, &defender).unwrap();
    let losses = |army: &ArmyId| -> Vec<u32> {
        state.armies[army]
            .units
            .iter()
            .map(|u| u.strength / 3)
            .collect()
    };
    let side = |losses: Vec<u32>, delta: i32| SideResult {
        total_losses: losses.iter().sum(),
        losses,
        morale_delta: delta,
        routed: delta < 0,
        general_killed: false,
        general_captured: false,
        no_quarter: false,
        withdrew: false,
        standards_taken: Vec::new(),
        standards_lost: 0,
    };
    let outcome = BattleOutcome {
        winner: SideId::Attacker,
        attacker: side(losses(&attacker), 5),
        defender: side(losses(&defender), -10),
        duration: 0.0,
    };
    state.resolve_pending_battle(data, index, &outcome).unwrap();
    (attacker, defender)
}

#[test]
fn wound_recovery_returns_part_of_the_losses() {
    let data = data();
    let base = france(&data, 12);
    let mut plain = base.clone();
    let mut tended = base.clone();
    {
        let france = tended.factions.get_mut(&fac("fac_france")).unwrap();
        for id in [
            "tech_barber_surgeons",
            "tech_soporific_sponge",
            "tech_chauliac_surgery",
        ] {
            france.technologies.insert(tech(id));
        }
    }
    assert!((medicine::wound_recovery(&tended, &data, &fac("fac_france")) - 0.35).abs() < 1e-9);
    assert_eq!(
        medicine::wound_recovery(&tended, &data, &fac("fac_england")),
        0.0
    );
    let attacker = first_army(&base, "fac_france");
    let start: Vec<u32> = base.armies[&attacker]
        .units
        .iter()
        .map(|u| u.strength)
        .collect();
    let (a, d) = fight(&mut plain, &data);
    fight(&mut tended, &data);
    let plain_units: Vec<u32> = plain.armies[&a].units.iter().map(|u| u.strength).collect();
    let tended_units: Vec<u32> = tended.armies[&a].units.iter().map(|u| u.strength).collect();
    assert_eq!(plain_units.len(), tended_units.len());
    for ((strength, plain), tended) in start.iter().zip(&plain_units).zip(&tended_units) {
        let lost = strength / 3;
        assert_eq!(*plain, strength - lost);
        assert_eq!(
            *tended,
            strength - lost + medicine::recovered_wounded(lost, 0.35)
        );
    }
    // England has no surgeons: same losses in both runs.
    assert_eq!(plain.armies[&d].units, tended.armies[&d].units);
    assert!(tended
        .events
        .iter()
        .any(|e| e.kind == EventKind::Medicine && e.text_fr.contains("blessés soignés")));
}

#[test]
fn ai_diets_are_valid_and_deterministic() {
    let data = data();
    let mut a = CampaignState::new_1337(&data, fac("fac_france"), 13).unwrap();
    let mut b = a.clone();
    for _ in 0..8 {
        a.end_turn(&data);
        b.end_turn(&data);
    }
    assert_eq!(a.save_json(), b.save_json());
    // Every order the AI proposes is accepted.
    for faction in a.factions.keys().cloned().collect::<Vec<_>>() {
        let mut probe = a.clone();
        for order in table::ai_choose_diets(&a, &data, &faction) {
            probe.apply_order(&data, &faction, order).unwrap();
        }
    }
    // Some AI realm has adopted a diet other than the default.
    let chosen = a.provinces.keys().any(|id| {
        !a.controls_province(&fac("fac_france"), id) && a.province_diet(id).as_str() != DEFAULT_DIET
    });
    assert!(chosen, "no AI diet after 8 turns");
}

#[test]
fn diets_survive_saves_and_old_saves_load() {
    let data = data();
    let mut state = france(&data, 14);
    set_diet(&mut state, &data, "prov_normandie", "diet_pulses").unwrap();
    state.end_turn_with(&data, idle);
    let json = state.save_json();
    let loaded = CampaignState::load_json(&json).unwrap();
    assert_eq!(
        loaded.province_diet(&prov("prov_normandie")),
        diet("diet_pulses")
    );
    assert_eq!(loaded, state);

    // A save written before H3 has neither `diet` nor `table_upkeep_last_turn`.
    let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
    for province in value["provinces"].as_object_mut().unwrap().values_mut() {
        province.as_object_mut().unwrap().remove("diet");
    }
    for faction in value["factions"].as_object_mut().unwrap().values_mut() {
        faction
            .as_object_mut()
            .unwrap()
            .remove("table_upkeep_last_turn");
    }
    let old = CampaignState::load_json(&value.to_string()).expect("old save loads");
    assert_eq!(
        old.province_diet(&prov("prov_normandie")).as_str(),
        DEFAULT_DIET
    );
}

#[test]
fn medicine_branch_alternates_in_ai_research() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 15).unwrap();
    for _ in 0..40 {
        state.end_turn(&data);
    }
    let medicine_known = state.factions.values().any(|f| {
        f.technologies.iter().any(|t| {
            data.technologies[t].branch == data_model::TechBranch::Medicine
                && t.as_str() != "tech_hospital_reform"
        })
    });
    assert!(
        medicine_known,
        "no AI faction researched medicine in 40 turns"
    );
}
