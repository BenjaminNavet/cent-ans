//! M6 (technologies) integration tests: research points, the `research`
//! order, completion, banked progress, unlocks, technology effects on
//! battles and income, anachronism surcharge, save format and determinism.
//! See `docs/design/m6-technologies.md` § 2.

use std::path::PathBuf;

use data_model::{EffectKind, FactionId, GameData, ProvinceId, TechnologyId, UnitTypeId};
use sim_campaign::battle_auto::{effective_armor, side_power};
use sim_campaign::research::{self, effective_cost, BASE_RESEARCH_POINTS};
use sim_campaign::{
    CampaignState, EventKind, Order, OrderError, ResearchError, TechStatus, STATE_VERSION,
};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn tech(id: &str) -> TechnologyId {
    TechnologyId::new(id).unwrap()
}

fn france(data: &GameData, seed: u64) -> CampaignState {
    CampaignState::new_1337(data, fac("fac_france"), seed).expect("1337 start")
}

/// A planner that does nothing: only the orders the test submits apply.
fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

fn research(state: &mut CampaignState, data: &GameData, id: &str) -> Result<(), OrderError> {
    state.submit_order(
        data,
        Order::Research {
            technology: tech(id),
        },
    )
}

#[test]
fn research_points_are_base_plus_buildings_plus_half_governance() {
    let data = data();
    let state = france(&data, 1);
    let france_id = fac("fac_france");
    let buildings: f64 = state
        .settlements
        .values()
        .filter(|s| s.owner == france_id)
        .flat_map(|s| s.buildings.iter())
        .filter_map(|b| data.buildings.get(b))
        .flat_map(|b| b.effects.iter())
        .filter(|e| e.effect == EffectKind::ResearchPoints)
        .map(|e| e.value)
        .sum();
    let techs: f64 = state.factions[&france_id]
        .technologies
        .iter()
        .filter_map(|t| data.technologies.get(t))
        .flat_map(|t| t.effects.iter())
        .filter(|e| e.effect == EffectKind::ResearchPoints)
        .map(|e| e.value)
        .sum();
    let ruler = state.factions[&france_id].ruler.clone().expect("a ruler");
    let governance = state.characters[&ruler].skills.governance;
    let expected =
        f64::from(BASE_RESEARCH_POINTS) + buildings + techs + f64::from(governance.div_ceil(2));
    assert_eq!(
        state.research_points_per_turn(&data, &france_id),
        expected.round() as u32
    );
    assert!(state.research_points_per_turn(&data, &france_id) > BASE_RESEARCH_POINTS);

    // A new research building raises the rate.
    let mut state = state;
    let before = state.research_points_per_turn(&data, &france_id);
    let settlement = state
        .settlements
        .iter()
        .find(|(_, s)| {
            s.owner == france_id
                && !s
                    .buildings
                    .contains(&data_model::BuildingId::new("bld_scriptorium").unwrap())
        })
        .map(|(id, _)| id.clone())
        .unwrap();
    state
        .settlements
        .get_mut(&settlement)
        .unwrap()
        .buildings
        .push(data_model::BuildingId::new("bld_scriptorium").unwrap());
    assert_eq!(
        state.research_points_per_turn(&data, &france_id),
        before + 1
    );
}

#[test]
fn research_order_is_refused_for_missing_prerequisite_known_or_unknown_tech() {
    let data = data();
    let mut state = france(&data, 2);
    assert!(matches!(
        research(&mut state, &data, "tech_bombards"),
        Err(OrderError::Research(ResearchError::MissingPrerequisite(_)))
    ));
    assert!(matches!(
        research(&mut state, &data, "tech_masonry"),
        Err(OrderError::Research(ResearchError::AlreadyKnown))
    ));
    assert!(matches!(
        research(&mut state, &data, "tech_does_not_exist"),
        Err(OrderError::Research(ResearchError::UnknownTechnology(_)))
    ));
    assert!(state.factions[&fac("fac_france")].research.is_none());
    research(&mut state, &data, "tech_gunpowder").expect("gunpowder is available");
    assert_eq!(
        state.factions[&fac("fac_france")].research,
        Some(tech("tech_gunpowder"))
    );
    let status = research::tech_status(
        &state,
        &fac("fac_france"),
        &data.technologies["tech_gunpowder"],
    );
    assert_eq!(status, TechStatus::Researching);
    let status = research::tech_status(
        &state,
        &fac("fac_france"),
        &data.technologies["tech_bombards"],
    );
    assert_eq!(status, TechStatus::Locked);
}

#[test]
fn research_completes_with_an_event_and_clears_the_slot() {
    let data = data();
    let mut state = france(&data, 3);
    let france_id = fac("fac_france");
    research(&mut state, &data, "tech_longbow_drill").unwrap();
    let info = state.research_info(&data, &france_id).expect("researching");
    assert_eq!(info.cost, 120);
    let expected_turns = info.turns_left;
    let mut completed_at = None;
    for turn in 1..=40 {
        let events = state.end_turn_with(&data, idle);
        if events.iter().any(|e| {
            e.kind == EventKind::TechnologyResearched && e.faction.as_ref() == Some(&france_id)
        }) {
            completed_at = Some(turn);
            break;
        }
    }
    let completed_at = completed_at.expect("longbow drill researched within 40 turns");
    // The rate may drift (governance grows), never beyond the first estimate.
    assert!(
        completed_at <= expected_turns,
        "{completed_at} > {expected_turns}"
    );
    let faction = &state.factions[&france_id];
    assert!(faction.technologies.contains(&tech("tech_longbow_drill")));
    assert!(faction.research.is_none());
    // F1: the surplus of the completed technology is carried over (lot C4:
    // the rate now counts every settlement's buildings, so it is not 0).
    assert!(faction.research_progress < faction.research_points_last_turn);
    assert!(faction.research_points_last_turn > 0);
    let text = &state
        .events
        .iter()
        .find(|e| e.kind == EventKind::TechnologyResearched)
        .unwrap()
        .text_fr;
    assert!(text.contains("Entraînement à l'arc long"), "{text}");
}

#[test]
fn switching_research_banks_and_restores_progress() {
    let data = data();
    let mut state = france(&data, 4);
    let france_id = fac("fac_france");
    research(&mut state, &data, "tech_gunpowder").unwrap();
    state.end_turn_with(&data, idle);
    state.end_turn_with(&data, idle);
    let progress = state.factions[&france_id].research_progress;
    assert!(progress > 0);

    research(&mut state, &data, "tech_pavise").unwrap();
    let faction = &state.factions[&france_id];
    assert_eq!(faction.research, Some(tech("tech_pavise")));
    assert_eq!(faction.research_progress, 0);
    assert_eq!(faction.research_banked[&tech("tech_gunpowder")], progress);
    assert_eq!(
        research::tech_progress(&state, &france_id, &tech("tech_gunpowder")),
        progress
    );

    research(&mut state, &data, "tech_gunpowder").unwrap();
    let faction = &state.factions[&france_id];
    assert_eq!(faction.research_progress, progress);
    assert!(!faction
        .research_banked
        .contains_key(&tech("tech_gunpowder")));
    // Pavise had no progress yet: nothing banked for it.
    assert!(faction.research_banked.is_empty());
}

#[test]
fn technology_unlocks_units_and_the_data_is_consistent() {
    let data = data();
    // Every unit/building a technology unlocks requires that technology.
    for t in data.technologies.values() {
        for unit in &t.unlocks.units {
            assert_eq!(
                data.unit_types[unit].required_technology.as_ref(),
                Some(&t.id),
                "{unit} unlocked by {} but requires another technology",
                t.id
            );
        }
        for building in &t.unlocks.buildings {
            assert_eq!(
                data.buildings[building].required_technology.as_ref(),
                Some(&t.id),
                "{building} unlocked by {}",
                t.id
            );
        }
    }

    let mut state = france(&data, 5);
    let france_id = fac("fac_france");
    let longbow = UnitTypeId::new("unit_longbowmen").unwrap();
    let capital: ProvinceId = state.factions[&france_id].capital.clone();
    let option = state
        .recruitable_in_province(&data, &capital)
        .into_iter()
        .find(|o| o.unit_type == longbow)
        .expect("longbowmen listed");
    assert!(!option.available);
    assert!(option.reason.unwrap().contains("technologie requise"));

    state
        .factions
        .get_mut(&france_id)
        .unwrap()
        .technologies
        .insert(tech("tech_longbow_drill"));
    let option = state
        .recruitable_in_province(&data, &capital)
        .into_iter()
        .find(|o| o.unit_type == longbow)
        .unwrap();
    assert!(
        option
            .reason
            .as_deref()
            .is_none_or(|r| !r.contains("technologie")),
        "{:?}",
        option.reason
    );
}

#[test]
fn technologies_strengthen_armies_in_battle() {
    let data = data();
    let mut state = france(&data, 6);
    let france_id = fac("fac_france");
    let army = state
        .armies
        .iter()
        .find(|(_, a)| a.faction == france_id)
        .map(|(id, _)| id.clone())
        .expect("a French army");
    state
        .factions
        .get_mut(&france_id)
        .unwrap()
        .technologies
        .clear();
    let without = research::army_battle_side(&state, &data, &army).unwrap();
    for id in [
        "tech_coat_of_plates",
        "tech_crossbow_windlass",
        "tech_compagnies_d_ordonnance",
        "tech_longbow_drill",
    ] {
        state
            .factions
            .get_mut(&france_id)
            .unwrap()
            .technologies
            .insert(tech(id));
    }
    let with = research::army_battle_side(&state, &data, &army).unwrap();
    assert!(effective_armor(&with) > effective_armor(&without));
    assert!(side_power(&with, 30.0, 1.0) > side_power(&without, 30.0, 1.0));
    let bonus =
        research::tech_unit_bonus(&state, &data, &france_id, data_model::UnitCategory::Cavalry);
    assert_eq!(bonus.armor, 5.0);
    assert_eq!(bonus.melee, 5.0);
    assert_eq!(bonus.morale, 5.0);
}

#[test]
fn technologies_raise_income() {
    let data = data();
    let mut state = france(&data, 7);
    let france_id = fac("fac_france");
    let before = state.faction_income_effective(&data, &france_id);
    state
        .factions
        .get_mut(&france_id)
        .unwrap()
        .technologies
        .insert(tech("tech_royal_taxation"));
    let after = state.faction_income_effective(&data, &france_id);
    assert!(after > before, "{after} <= {before}");
}

#[test]
fn anachronistic_technologies_cost_more() {
    let data = data();
    let field_artillery = &data.technologies["tech_field_artillery"];
    assert_eq!(effective_cost(field_artillery, 1337), 750);
    assert_eq!(effective_cost(field_artillery, 1430), 600);
    let longbow = &data.technologies["tech_longbow_drill"];
    assert_eq!(effective_cost(longbow, 1337), 120);
    // 1356 is within 20 years of 1337: no surcharge.
    let hansa = &data.technologies["tech_hanseatic_trade"];
    assert_eq!(effective_cost(hansa, 1337), hansa.cost);
}

#[test]
fn research_state_survives_a_save_round_trip() {
    let data = data();
    let mut state = france(&data, 8);
    research(&mut state, &data, "tech_gunpowder").unwrap();
    state.end_turn_with(&data, idle);
    research(&mut state, &data, "tech_pavise").unwrap();
    state.end_turn_with(&data, idle);
    let json = state.save_json();
    assert!(json.contains(&format!("\"state_version\":{STATE_VERSION}")));
    assert_eq!(STATE_VERSION, 5);
    let loaded = CampaignState::load_json(&json).expect("loads");
    assert_eq!(loaded, state);
    let faction = &loaded.factions[&fac("fac_france")];
    assert_eq!(faction.research, Some(tech("tech_pavise")));
    assert!(faction.research_progress > 0);
    assert!(faction
        .research_banked
        .contains_key(&tech("tech_gunpowder")));
}

#[test]
fn ai_research_is_deterministic_and_alternates_branches() {
    let data = data();
    let run = || {
        let mut state = france(&data, 42);
        for _ in 0..30 {
            state.end_turn(&data);
        }
        state
    };
    let a = run();
    let b = run();
    assert_eq!(a.save_json(), b.save_json());

    // Living AI factions research something (one that just completed a
    // technology picks the next one at the start of the following turn).
    let ai: Vec<_> = a
        .factions
        .iter()
        .filter(|(id, f)| f.alive && **id != a.player_faction)
        .collect();
    let busy = ai.iter().filter(|(_, f)| f.research.is_some()).count();
    assert!(
        busy * 2 >= ai.len(),
        "{busy} of {} AI factions research",
        ai.len()
    );
    // Somebody completed a technology in 30 turns, and the AI picks the
    // cheapest available tech of the branch it has fewer of.
    let started = france(&data, 42);
    let learned = a
        .factions
        .iter()
        .any(|(id, f)| f.technologies.len() > started.factions[id].technologies.len());
    assert!(learned);
    let england = fac("fac_england");
    let choice = research::ai_choose_research(&started, &data, &england).unwrap();
    let chosen = &data.technologies[&choice];
    let known = &started.factions[&england].technologies;
    // H4: three branches; the one with the fewest acquired techs wins
    // (ties in military, civil, medicine order).
    let count = |branch: data_model::TechBranch| {
        known
            .iter()
            .filter(|t| data.technologies[*t].branch == branch)
            .count()
    };
    let expected_branch = data_model::TechBranch::ALL
        .into_iter()
        .min_by_key(|b| count(*b))
        .unwrap();
    assert_eq!(chosen.branch, expected_branch);
}
