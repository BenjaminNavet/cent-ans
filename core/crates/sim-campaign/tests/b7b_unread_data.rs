//! Lot B7b: data shown to the player that the simulation used to ignore —
//! the `construction_speed` of traits and skills (Bâtisseur, Urbaniste) and
//! the units' `recruit_time_turns`. The edicts' piety is tested in
//! `edicts.rs`. See `docs/wip/b7b-unread-data.md`.

use std::path::PathBuf;

use data_model::{FactionId, GameData, SettlementId, SkillId, TraitId, UnitTypeId};
use sim_campaign::{CampaignState, Order, Place, QueuedRecruit};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn start(data: &GameData) -> CampaignState {
    CampaignState::new_1337(data, FactionId::new("fac_france").unwrap(), 7).expect("1337 start")
}

fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

fn france() -> FactionId {
    FactionId::new("fac_france").unwrap()
}

fn capital_city(state: &CampaignState) -> SettlementId {
    let capital = state.factions[&france()].capital.clone();
    state.provinces[&capital].city.clone()
}

/// Strips every `construction_speed` trait and skill from the French court
/// so that the test starts from the data's own build times.
fn clear_builders(state: &mut CampaignState, data: &GameData) {
    for character in state.characters.values_mut() {
        character.traits.retain(|t| {
            data.traits[t]
                .effects
                .iter()
                .all(|e| e.effect != data_model::EffectKind::ConstructionSpeed)
        });
        character.skills_learned.retain(|s| {
            data.skills[s]
                .effects
                .iter()
                .all(|e| e.effect != data_model::EffectKind::ConstructionSpeed)
        });
    }
}

fn longest_build(state: &CampaignState, data: &GameData, city: &SettlementId) -> (String, u32) {
    state
        .buildable(data, city)
        .into_iter()
        .map(|o| {
            (
                o.building.to_string(),
                data.buildings[&o.building].build_time_turns,
            )
        })
        .max_by_key(|(id, turns)| (*turns, std::cmp::Reverse(id.clone())))
        .expect("the capital has build options")
}

fn option_turns(state: &CampaignState, data: &GameData, city: &SettlementId, id: &str) -> u32 {
    state
        .buildable(data, city)
        .into_iter()
        .find(|o| o.building.as_str() == id)
        .expect("option listed")
        .turns
}

#[test]
fn builder_ruler_shortens_constructions() {
    let data = data();
    let mut state = start(&data);
    clear_builders(&mut state, &data);
    let city = capital_city(&state);
    let (building, base) = longest_build(&state, &data, &city);
    assert!(base >= 4, "a long build exists ({building}: {base})");
    assert_eq!(option_turns(&state, &data, &city, &building), base);

    let ruler = state.factions[&france()].ruler.clone().expect("ruler");
    let builder = TraitId::new("trait_builder").unwrap();
    let percent: f64 = data.traits[&builder]
        .effects
        .iter()
        .filter(|e| e.effect == data_model::EffectKind::ConstructionSpeed)
        .map(|e| e.value)
        .sum();
    assert!(percent > 0.0, "Bâtisseur speeds up construction");
    state
        .characters
        .get_mut(&ruler)
        .unwrap()
        .traits
        .insert(builder);
    let expected = ((f64::from(base) * 100.0 / (100.0 + percent)).round() as u32).max(1);
    assert!(expected < base);
    assert_eq!(state.construction_speed_percent(&data, &city), percent);
    assert_eq!(option_turns(&state, &data, &city, &building), expected);

    // The construction started takes the shortened time.
    let building_id = data_model::BuildingId::new(&building).unwrap();
    state.factions.get_mut(&france()).unwrap().treasury = 1_000_000;
    state
        .submit_order(
            &data,
            Order::Build {
                settlement: Place::Settlement(city.clone()),
                building: building_id,
            },
        )
        .expect("build order");
    assert_eq!(
        state.settlements[&city]
            .construction
            .as_ref()
            .unwrap()
            .turns_left,
        expected
    );
}

#[test]
fn governor_and_ruler_do_not_stack_but_skills_do() {
    let data = data();
    let mut state = start(&data);
    clear_builders(&mut state, &data);
    let city = capital_city(&state);
    let province = state.settlements[&city].province.clone();
    let ruler = state.factions[&france()].ruler.clone().expect("ruler");
    let skill = |id: &str| SkillId::new(id).unwrap();
    // Ruler: Bâtisseur (skill) + Urbaniste.
    let r = state.characters.get_mut(&ruler).unwrap();
    r.skills_learned.insert(skill("skill_batisseur"));
    r.skills_learned.insert(skill("skill_urbaniste"));
    let ruler_percent = state.construction_speed_percent(&data, &city);
    let sum: f64 = ["skill_batisseur", "skill_urbaniste"]
        .iter()
        .flat_map(|s| data.skills[&skill(s)].effects.iter())
        .filter(|e| e.effect == data_model::EffectKind::ConstructionSpeed)
        .map(|e| e.value)
        .sum();
    assert_eq!(ruler_percent, sum, "a character's skills add up");

    // A lesser builder as governor does not add to the ruler's bonus.
    let governor = state
        .characters
        .iter()
        .find(|(id, c)| c.alive && c.faction == france() && **id != ruler)
        .map(|(id, _)| id.clone())
        .expect("another French character");
    for c in state.characters.values_mut() {
        if c.governor_of.as_ref() == Some(&province) {
            c.governor_of = None;
        }
    }
    let g = state.characters.get_mut(&governor).unwrap();
    g.governor_of = Some(province.clone());
    g.traits.insert(TraitId::new("trait_builder").unwrap());
    assert_eq!(state.construction_speed_percent(&data, &city), ruler_percent);
}

#[test]
fn slow_recruits_train_for_their_recruit_time() {
    let mut data = data();
    let mut state = start(&data);
    let city = capital_city(&state);
    state.factions.get_mut(&france()).unwrap().treasury = 1_000_000;
    let option = state
        .recruitable(&data, &city)
        .into_iter()
        .find(|o| o.available)
        .expect("the capital can recruit");
    let unit: UnitTypeId = option.unit_type.clone();
    data.unit_types.get_mut(&unit).unwrap().recruit_time_turns = Some(3);
    let count = |state: &CampaignState| {
        state.settlements[&city]
            .garrison
            .iter()
            .filter(|u| u.unit_type == unit)
            .count()
    };
    let before = count(&state);
    let free = state.recruit_slots_free(&data, &city);
    state
        .submit_order(
            &data,
            Order::Recruit {
                settlement: Place::Settlement(city.clone()),
                unit_type: unit.clone(),
            },
        )
        .expect("recruit");
    assert_eq!(state.recruit_slots_free(&data, &city), free - 1);
    let queue = &state.settlements[&city].recruit_queue;
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].turns_left, 3);

    state.end_turn_with(&data, idle);
    assert_eq!(count(&state), before, "still training after one turn");
    assert_eq!(state.settlements[&city].recruit_queue[0].turns_left, 2);
    // The slot is freed for the new turn although the recruit still trains.
    assert_eq!(state.recruit_slots_free(&data, &city), free);

    state.end_turn_with(&data, idle);
    assert_eq!(count(&state), before);
    state.end_turn_with(&data, idle);
    assert_eq!(count(&state), before + 1, "joins after three turns");
    assert!(state.settlements[&city].recruit_queue.is_empty());
}

#[test]
fn one_turn_recruits_join_at_the_end_of_the_turn() {
    let mut data = data();
    let mut state = start(&data);
    let city = capital_city(&state);
    state.factions.get_mut(&france()).unwrap().treasury = 1_000_000;
    let unit = state
        .recruitable(&data, &city)
        .into_iter()
        .find(|o| o.available)
        .expect("the capital can recruit")
        .unit_type;
    data.unit_types.get_mut(&unit).unwrap().recruit_time_turns = Some(1);
    let before = state.settlements[&city].garrison.len();
    state
        .submit_order(
            &data,
            Order::Recruit {
                settlement: Place::Settlement(city.clone()),
                unit_type: unit,
            },
        )
        .expect("recruit");
    state.end_turn_with(&data, idle);
    assert_eq!(state.settlements[&city].garrison.len(), before + 1);
}

#[test]
fn legacy_queue_entries_load_as_one_turn_recruits() {
    let entry: QueuedRecruit = serde_json::from_str("\"unit_knights\"").expect("legacy id");
    assert_eq!(entry.unit_type.as_str(), "unit_knights");
    assert_eq!(entry.turns_left, 1);
    assert!(entry.ordered_during(12));
    let full = serde_json::to_string(&QueuedRecruit {
        unit_type: UnitTypeId::new("unit_knights").unwrap(),
        turns_left: 2,
        ordered_turn: 5,
    })
    .unwrap();
    let back: QueuedRecruit = serde_json::from_str(&full).unwrap();
    assert_eq!(back.turns_left, 2);
    assert!(back.ordered_during(5) && !back.ordered_during(6));
}
