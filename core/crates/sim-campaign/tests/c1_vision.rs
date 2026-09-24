//! Lot C1: light fog of war — `CampaignState::visible_provinces`.

use std::collections::BTreeSet;
use std::path::PathBuf;

use data_model::{FactionId, GameData, ProvinceId, VisionRules};
use sim_campaign::movement::land_neighbors;
use sim_campaign::CampaignState;

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn rules(controlled: u32, army: u32, general: u32, share: bool) -> VisionRules {
    VisionRules {
        controlled_range: controlled,
        army_range: army,
        general_bonus: general,
        share_allied_vision: share,
        description: None,
    }
}

/// Provinces controlled by `faction`.
fn controlled(state: &CampaignState, faction: &FactionId) -> BTreeSet<ProvinceId> {
    state
        .provinces
        .iter()
        .filter(|(_, p)| &p.controller == faction)
        .map(|(id, _)| id.clone())
        .collect()
}

#[test]
fn vision_rules_file_is_loaded() {
    let data = data();
    let rules = data.vision_rules.as_ref().expect("data/rules/vision.json");
    assert_eq!(rules.controlled_range, 1);
    assert_eq!(rules.army_range, 1);
    assert!(rules.share_allied_vision);
}

#[test]
fn a_faction_sees_its_provinces_and_their_neighbours_but_not_everything() {
    let data = data();
    let france = fac("fac_france");
    let state = CampaignState::new_1337(&data, france.clone(), 1).unwrap();
    let visible = state.visible_provinces(&data, &france);
    for province in controlled(&state, &france) {
        assert!(visible.contains(&province), "own {province} visible");
        for neighbour in land_neighbors(&data, &province) {
            assert!(
                visible.contains(neighbour),
                "{neighbour} next to {province}"
            );
        }
    }
    assert!(
        visible.len() < state.provinces.len(),
        "fog hides part of the map ({} / {})",
        visible.len(),
        state.provinces.len()
    );
}

#[test]
fn an_army_reveals_its_surroundings_and_a_general_sees_further() {
    let mut data = data();
    data.vision_rules = Some(rules(0, 1, 1, false));
    let france = fac("fac_france");
    let mut state = CampaignState::new_1337(&data, france.clone(), 1).unwrap();
    // One French army is moved far away, beyond sight.
    // A province whose two rings are all out of sight before the move.
    let hidden: ProvinceId = {
        let visible = state.visible_provinces(&data, &france);
        let unseen = |id: &ProvinceId| !visible.contains(id);
        state
            .provinces
            .keys()
            .find(|id| {
                unseen(id)
                    && land_neighbors(&data, id).len() >= 2
                    && land_neighbors(&data, id)
                        .iter()
                        .all(|n| unseen(n) && land_neighbors(&data, n).iter().all(unseen))
            })
            .expect("a province far from French sight")
            .clone()
    };
    let army_id = state
        .armies
        .iter()
        .find(|(_, a)| a.faction == france)
        .map(|(id, _)| id.clone())
        .unwrap();
    let army = state.armies.get_mut(&army_id).unwrap();
    army.location = hidden.clone();
    army.general = None;
    let visible = state.visible_provinces(&data, &france);
    assert!(visible.contains(&hidden));
    let first_ring: Vec<_> = land_neighbors(&data, &hidden).to_vec();
    for neighbour in &first_ring {
        assert!(visible.contains(neighbour), "{neighbour} seen by the army");
    }
    let second_ring: BTreeSet<ProvinceId> = first_ring
        .iter()
        .flat_map(|n| land_neighbors(&data, n).iter().cloned())
        .filter(|p| p != &hidden && !first_ring.contains(p))
        .collect();
    // Without a general the second ring is not seen by this army (other French
    // armies or provinces may still see some of it).
    let led_visible = {
        let general = state
            .characters
            .keys()
            .next()
            .cloned()
            .expect("some character");
        state.armies.get_mut(&army_id).unwrap().general = Some(general);
        state.visible_provinces(&data, &france)
    };
    assert!(
        second_ring.iter().all(|p| led_visible.contains(p)),
        "a general's outriders see two steps"
    );
    assert!(
        second_ring.iter().any(|p| !visible.contains(p)),
        "without a general, some of the second ring stays hidden"
    );
}

#[test]
fn allies_share_their_sight_only_when_the_rule_says_so() {
    let mut data = data();
    let france = fac("fac_france");
    let state = CampaignState::new_1337(&data, france.clone(), 1).unwrap();
    let allied: BTreeSet<ProvinceId> = state
        .provinces
        .iter()
        .filter(|(_, p)| p.controller != france && state.is_allied(&france, &p.controller))
        .map(|(id, _)| id.clone())
        .collect();
    assert!(!allied.is_empty(), "France has allies in 1337 (Scotland)");
    data.vision_rules = Some(rules(0, 0, 0, true));
    let shared = state.visible_provinces(&data, &france);
    assert!(allied.iter().all(|p| shared.contains(p)));
    data.vision_rules = Some(rules(0, 0, 0, false));
    let alone = state.visible_provinces(&data, &france);
    assert!(allied.iter().any(|p| !alone.contains(p)));
    let own = controlled(&state, &france);
    assert!(own.iter().all(|p| alone.contains(p)));
}
