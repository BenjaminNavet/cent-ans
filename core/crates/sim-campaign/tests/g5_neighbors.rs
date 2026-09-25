//! G5 « voisinage réel » tests: `CampaignState::are_neighbors` follows the
//! map graph (not the province files' partial `neighbors`), and the border
//! diplomacy tuning comes from `data/ai/diplomacy.json`.

use std::path::PathBuf;

use data_model::{AiDiplomacy, FactionId, GameData, ProvinceId};
use sim_campaign::diplomacy::plan_diplomacy;
use sim_campaign::movement::land_neighbors;
use sim_campaign::{CampaignState, Order};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn state(data: &GameData) -> CampaignState {
    CampaignState::new_1337(data, fac("fac_france"), 1).expect("1337 state")
}

#[test]
fn neighbours_come_from_the_map_graph() {
    let data = data();
    let state = state(&data);
    // No Scottish province file lists neighbours: the map graph does.
    let (scotland, england) = (fac("fac_scotland"), fac("fac_england"));
    assert!(state.are_neighbors(&data, &scotland, &england));
    assert!(state.are_neighbors(&data, &england, &scotland));
    assert!(!state.are_neighbors(&data, &scotland, &fac("fac_naples")));
    assert!(state.are_neighbors(&data, &fac("fac_france"), &fac("fac_burgundy")));
}

#[test]
fn most_provinces_have_map_neighbours() {
    let data = data();
    let with_files = data
        .provinces
        .values()
        .filter(|p| !p.neighbors.is_empty())
        .count();
    let with_map = data
        .provinces
        .keys()
        .filter(|id| !land_neighbors(&data, id).is_empty())
        .count();
    assert!(
        with_map > 5 * with_files.max(1),
        "{with_map} vs {with_files}"
    );
}

#[test]
fn being_neighbours_is_symmetric() {
    let data = data();
    let state = state(&data);
    let alive: Vec<&FactionId> = state
        .factions
        .iter()
        .filter(|(_, f)| f.alive)
        .map(|(id, _)| id)
        .collect();
    for a in &alive {
        for b in &alive {
            assert_eq!(
                state.are_neighbors(&data, a, b),
                state.are_neighbors(&data, b, a),
                "{a} / {b}"
            );
        }
    }
}

#[test]
fn the_tuning_comes_from_data() {
    let data = data();
    assert_ne!(data.ai_diplomacy, AiDiplomacy::default());
    assert!(data.ai_diplomacy.peace.keep_capital);
    assert!(data.ai_diplomacy.join_war.min_ally_power_ratio > 0.0);
}

/// Scotland at war with England, every Scottish province in English hands
/// (Lothian, its capital, is English in 1337), battles lost: the peace
/// Scotland offers over eight turns.
fn scottish_surrender(data: &GameData) -> Vec<Vec<ProvinceId>> {
    let mut state = state(data);
    let (scotland, england) = (fac("fac_scotland"), fac("fac_england"));
    for (a, b) in [(&scotland, &england), (&england, &scotland)] {
        let f = state.factions.get_mut(a).unwrap();
        f.allies.remove(b);
        f.truces.remove(b);
        f.at_war_with.insert(b.clone());
    }
    let ids: Vec<_> = state.provinces.keys().cloned().collect();
    for id in ids {
        if state.province_owner(&id) == Some(&scotland) {
            state.city_state_mut(&id).unwrap().controller = england.clone();
        }
    }
    // Lost battles on top of the occupation: Scotland is beaten.
    state
        .factions
        .get_mut(&scotland)
        .unwrap()
        .war_scores
        .insert(england.clone(), -20);
    let mut offers = Vec::new();
    for turn in 8..16 {
        state.turn = turn;
        for order in plan_diplomacy(&state, data, &scotland) {
            match order {
                Order::ProposePeace {
                    target, provinces, ..
                } if target == england => offers.push(provinces),
                // DP1: the treaty peace lists the ceded provinces as articles.
                Order::ProposeTreaty { target, articles } if target == england => {
                    offers.push(
                        articles
                            .into_iter()
                            .filter_map(|a| match a {
                                sim_campaign::negotiation::Article::CedeProvince {
                                    province,
                                    ..
                                } => Some(province),
                                _ => None,
                            })
                            .collect(),
                    );
                }
                _ => {}
            }
        }
    }
    offers
}

#[test]
fn a_beaten_crown_keeps_its_capital_and_a_province() {
    let data = data();
    let capital = state(&data).factions[&fac("fac_scotland")].capital.clone();
    let start = state(&data);
    let owned = start
        .provinces
        .keys()
        .filter(|id| start.province_owner(id) == Some(&fac("fac_scotland")))
        .count();
    let offers = scottish_surrender(&data);
    assert!(!offers.is_empty(), "Scotland sues for peace");
    for provinces in &offers {
        assert!(!provinces.contains(&capital), "{provinces:?}");
        assert!(provinces.len() < owned, "{provinces:?}");
    }
    // The F4 rules cede everything: Scotland vanishes from the map.
    let mut f4 = data;
    f4.ai_diplomacy = AiDiplomacy::default();
    let offers = scottish_surrender(&f4);
    assert!(offers.iter().any(|p| p.len() == owned), "{offers:?}");
}
