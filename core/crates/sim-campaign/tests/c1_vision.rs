//! Fog of war: lot C1 (`CampaignState::visible_provinces`), per-cell sight
//! radius since lot M5a (`CampaignState::vision`, `visible_armies`).

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::Instant;

use data_model::entities::movement::FreeMovementRules;
use data_model::{FactionId, GameData, ProvinceId, VisionRules};
use sim_campaign::{ArmyId, ArmyPosition, CampaignState};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn rules(share: bool) -> VisionRules {
    VisionRules {
        share_allied_vision: share,
        own_provinces_visible: false,
        ..VisionRules::default()
    }
}

/// Game data with sight from the radii only (held provinces not seen whole),
/// to test the discs in isolation.
fn radius_only_data() -> GameData {
    let mut data = data();
    data.vision_rules = Some(rules(true));
    data
}

fn px_per_km(data: &GameData) -> f32 {
    data.navgrid().px_per_km() as f32
}

/// `point` moved `km` kilometres east.
fn east(data: &GameData, point: [f32; 2], km: f32) -> [f32; 2] {
    [point[0] + km * px_per_km(data), point[1]]
}

fn dist_km(data: &GameData, a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt() / px_per_km(data)
}

/// Provinces controlled by `faction`.
fn controlled(state: &CampaignState, faction: &FactionId) -> BTreeSet<ProvinceId> {
    state
        .provinces
        .keys()
        .filter(|id| state.province_controller(id) == Some(faction))
        .cloned()
        .collect()
}

/// A settlement point at least `km` (plus a 60 km strip to the east) away
/// from every source of `faction`'s sight.
fn remote_point(state: &CampaignState, data: &GameData, faction: &FactionId, km: f32) -> [f32; 2] {
    let vision = state.vision(data, faction);
    state
        .settlements
        .keys()
        .filter_map(|id| data.settlement_point(id))
        .find(|p| {
            [0.0, 30.0, 60.0].iter().all(|shift| {
                let q = east(data, *p, *shift);
                vision
                    .mask
                    .sources
                    .iter()
                    .all(|s| dist_km(data, s.point, q) > km)
            })
        })
        .expect("a point far from the faction's sight")
}

fn first_army(state: &CampaignState, faction: &FactionId) -> ArmyId {
    state
        .armies
        .iter()
        .find(|(_, a)| &a.faction == faction)
        .map(|(id, _)| id.clone())
        .expect("an army")
}

#[test]
fn vision_rules_come_from_the_data() {
    let data = data();
    let rules = data.vision_rules.as_ref().expect("data/rules/vision.json");
    assert!(rules.share_allied_vision);
    assert!(rules.own_provinces_visible);
    assert_eq!(rules.province_seen_percent, 25);
    let free = data.free_movement_rules();
    assert_eq!(free.vision_army_km, 30.0);
    assert_eq!(free.vision_settlement_km, 20.0);
    assert_eq!(rules.edge_feather_km, 3.0);
}

/// Radius-only data with the sight radii (`data/movement/rules.json`) and
/// the soft edge (`data/rules/vision.json`) overridden.
fn tuned_data(army_km: f64, settlement_km: f64, feather_km: f64) -> GameData {
    let mut data = radius_only_data();
    data.free_movement = Some(FreeMovementRules {
        vision_army_km: army_km,
        vision_settlement_km: settlement_km,
        ..data.free_movement_rules().clone()
    });
    if let Some(rules) = data.vision_rules.as_mut() {
        rules.edge_feather_km = feather_km;
    }
    data
}

#[test]
fn sight_radii_follow_the_data() {
    // SV1: no radius is hard-coded, the core reads both from the data.
    let france = fac("fac_france");
    let wide = tuned_data(50.0, 10.0, 3.0);
    let mut state = CampaignState::new_1337(&wide, france.clone(), 1).unwrap();
    let spot = remote_point(&state, &wide, &france, 120.0);
    let army = first_army(&state, &france);
    state.armies.get_mut(&army).unwrap().position = ArmyPosition::Field {
        x: spot[0],
        y: spot[1],
    };
    let vision = state.vision(&wide, &france);
    assert!(
        vision.mask.sees_point(&wide, east(&wide, spot, 45.0)),
        "a 50 km army radius sees 45 km away"
    );
    assert!(!vision.mask.sees_point(&wide, east(&wide, spot, 55.0)));
    assert!(vision.mask.coverage_at(east(&wide, spot, 45.0)) >= 128);
    let source = vision
        .mask
        .sources
        .iter()
        .find(|s| s.point == spot)
        .expect("the army is a sight source");
    assert!((source.radius_px - 50.0 * px_per_km(&wide)).abs() < 1e-3);
    // Settlements take their own radius from the data too.
    let army_px = 50.0 * px_per_km(&wide);
    let settlement_px = 10.0 * px_per_km(&wide);
    let is = |r: f32, px: f32| (r - px).abs() < 1e-3;
    assert!(vision
        .mask
        .sources
        .iter()
        .all(|s| is(s.radius_px, army_px) || is(s.radius_px, settlement_px)));
    assert!(vision
        .mask
        .sources
        .iter()
        .any(|s| is(s.radius_px, settlement_px)));
    // Same state, default data: the 30 km radius does not reach 45 km.
    let data = radius_only_data();
    let vision = state.vision(&data, &france);
    assert!(!vision.mask.sees_point(&data, east(&data, spot, 45.0)));
}

#[test]
fn the_soft_edge_width_follows_the_data() {
    let france = fac("fac_france");
    let sharp = tuned_data(30.0, 20.0, 0.0);
    let soft = tuned_data(30.0, 20.0, 20.0);
    let mut state = CampaignState::new_1337(&sharp, france.clone(), 1).unwrap();
    let spot = remote_point(&state, &sharp, &france, 80.0);
    let army = first_army(&state, &france);
    state.armies.get_mut(&army).unwrap().position = ArmyPosition::Field {
        x: spot[0],
        y: spot[1],
    };
    // 5 km inside the radius (a texel is ~6 km): fully seen with a sharp
    // edge, partly with a 20 km fade.
    let inside = east(&sharp, spot, 25.0);
    let sharp_cov = state.vision(&sharp, &france).mask.coverage_at(inside);
    let soft_cov = state.vision(&soft, &france).mask.coverage_at(inside);
    assert_eq!(sharp_cov, 255);
    assert!(
        (128..255).contains(&soft_cov),
        "soft edge coverage {soft_cov}"
    );
}

#[test]
fn a_faction_sees_its_provinces_but_not_everything() {
    let data = data();
    let france = fac("fac_france");
    let state = CampaignState::new_1337(&data, france.clone(), 1).unwrap();
    let vision = state.vision(&data, &france);
    for province in controlled(&state, &france) {
        assert!(vision.provinces.contains(&province), "own {province} seen");
    }
    assert!(
        vision.provinces.len() < state.provinces.len(),
        "fog hides part of the map ({} / {})",
        vision.provinces.len(),
        state.provinces.len()
    );
    let share = vision.mask.seen_share();
    assert!(share > 0.0 && share < 0.5, "seen share {share}");
    assert_eq!(vision.mask.width, 896); // 7168 / 8 (ADR 0115)
    assert_eq!(vision.mask.height, 768);
}

#[test]
fn an_army_sees_thirty_kilometres_around_it() {
    let data = radius_only_data();
    let france = fac("fac_france");
    let mut state = CampaignState::new_1337(&data, france.clone(), 1).unwrap();
    let spot = remote_point(&state, &data, &france, 80.0);
    let near = east(&data, spot, 25.0);
    let far = east(&data, spot, 36.0);
    let before = state.vision(&data, &france);
    assert!(!before.mask.sees_point(&data, spot));
    assert!(!before.mask.sees_point(&data, near));
    let army = first_army(&state, &france);
    state.armies.get_mut(&army).unwrap().position = ArmyPosition::Field {
        x: spot[0],
        y: spot[1],
    };
    let after = state.vision(&data, &france);
    assert!(after.mask.sees_point(&data, spot));
    assert!(after.mask.sees_point(&data, near), "25 km is within sight");
    assert!(!after.mask.sees_point(&data, far), "36 km is beyond sight");
    // The raster agrees away from the soft edge.
    assert!(after.mask.coverage_at(near) >= 128);
    assert!(after.mask.coverage_at(far) < 128);
    assert!(after.mask.coverage_at(east(&data, spot, 45.0)) == 0);
    // The province under the army is visible.
    let province = state
        .army_province(&data, state.army(&army).unwrap())
        .unwrap();
    assert!(after.provinces.contains(&province));
}

#[test]
fn a_held_settlement_sees_twenty_kilometres_around_it() {
    let data = radius_only_data();
    let france = fac("fac_france");
    let mut state = CampaignState::new_1337(&data, france.clone(), 1).unwrap();
    let spot = remote_point(&state, &data, &france, 80.0);
    let (settlement, _) = state
        .settlements
        .iter()
        .find(|(id, _)| data.settlement_point(id) == Some(spot))
        .map(|(id, s)| (id.clone(), s.province.clone()))
        .unwrap();
    let near = east(&data, spot, 15.0);
    let far = east(&data, spot, 26.0);
    let before = state.vision(&data, &france);
    assert!(!before.mask.sees_point(&data, near));
    let province = state.settlement_province(&settlement).unwrap().clone();
    assert!(!before.provinces.contains(&province));
    // Remove any army standing there, then hand the settlement to France.
    let here: Vec<ArmyId> = state
        .armies
        .iter()
        .filter(|(_, a)| a.position == ArmyPosition::Settlement(settlement.clone()))
        .map(|(id, _)| id.clone())
        .collect();
    for id in here {
        state.armies.remove(&id);
    }
    state.settlements.get_mut(&settlement).unwrap().controller = france.clone();
    let after = state.vision(&data, &france);
    assert!(after.mask.sees_point(&data, spot));
    assert!(after.mask.sees_point(&data, near), "15 km is within sight");
    assert!(!after.mask.sees_point(&data, far), "26 km is beyond sight");
    assert!(
        after.provinces.contains(&province),
        "a seen settlement makes its province visible"
    );
}

#[test]
fn allies_lend_their_sight_only_when_the_rule_says_so() {
    let mut data = data();
    let france = fac("fac_france");
    let mut state = CampaignState::new_1337(&data, france.clone(), 1).unwrap();
    let ally = state
        .factions
        .keys()
        .find(|f| *f != &france && state.is_allied(&france, f))
        .cloned()
        .expect("France has allies in 1337 (Scotland)");
    data.vision_rules = Some(rules(false));
    let spot = remote_point(&state, &data, &france, 80.0);
    // An allied army far from French sight.
    let army = match state.armies.iter().find(|(_, a)| a.faction == ally) {
        Some((id, _)) => id.clone(),
        None => {
            let id = first_army(&state, &france);
            state.armies.get_mut(&id).unwrap().faction = ally.clone();
            id
        }
    };
    state.armies.get_mut(&army).unwrap().position = ArmyPosition::Field {
        x: spot[0],
        y: spot[1],
    };
    let near = east(&data, spot, 20.0);
    let alone = state.vision(&data, &france);
    let sees_near_alone = alone.mask.sees_point(&data, near);
    data.vision_rules = Some(rules(true));
    let shared = state.vision(&data, &france);
    assert!(
        shared.mask.sees_point(&data, near),
        "the ally's army lends sight"
    );
    assert!(state.visible_armies(&data, &france).contains(&army));
    // Without sharing, the spot is far from every French source.
    assert!(!sees_near_alone);
    assert!(alone.mask.sources.len() < shared.mask.sources.len());
}

#[test]
fn a_foreign_army_out_of_sight_is_hidden() {
    let data = radius_only_data();
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    let mut state = CampaignState::new_1337(&data, france.clone(), 1).unwrap();
    let spot = remote_point(&state, &data, &france, 80.0);
    let english = first_army(&state, &england);
    state.armies.get_mut(&english).unwrap().position = ArmyPosition::Field {
        x: spot[0],
        y: spot[1],
    };
    let seen = state.visible_armies(&data, &france);
    assert!(!seen.contains(&english), "English army out of sight");
    // Own armies are always shown.
    let french = first_army(&state, &france);
    assert!(seen.contains(&french));
    // A French army 20 km away brings it into sight.
    let watch = east(&data, spot, 20.0);
    state.armies.get_mut(&french).unwrap().position = ArmyPosition::Field {
        x: watch[0],
        y: watch[1],
    };
    assert!(state.visible_armies(&data, &france).contains(&english));
}

#[test]
fn held_province_land_is_seen_far_from_any_source() {
    let data = data();
    let (france, england) = (fac("fac_france"), fac("fac_england"));
    let mut state = CampaignState::new_1337(&data, france.clone(), 1).unwrap();
    let sources = state.vision(&radius_only_data(), &france).mask.sources;
    let raster = data.province_raster().expect("province_ids.png");
    let mut found = None;
    'search: for y in (0..raster.height).step_by(16) {
        for x in (0..raster.width).step_by(16) {
            let point = [x as f32 + 0.5, y as f32 + 0.5];
            let Some(province) = data.province_at_point(point[0], point[1]) else {
                continue;
            };
            if state.province_controller(province) == Some(&france)
                && sources
                    .iter()
                    .all(|s| dist_km(&data, s.point, point) > 40.0)
            {
                found = Some((point, province.clone()));
                break 'search;
            }
        }
    }
    let (point, province) = found.expect("French land 40 km from every French source");
    let vision = state.vision(&data, &france);
    assert!(vision.mask.sees_point(&data, point), "own land is seen");
    assert!(
        vision.mask.coverage_at(point) >= 128,
        "own land is seen in the mask"
    );
    assert!(vision.provinces.contains(&province));
    // The same place under an enemy is not seen.
    let held: Vec<_> = state
        .settlements
        .iter()
        .filter(|(_, s)| s.province == province)
        .map(|(id, _)| id.clone())
        .collect();
    for id in held {
        state.settlements.get_mut(&id).unwrap().controller = england.clone();
    }
    assert_eq!(state.province_controller(&province), Some(&england));
    let vision = state.vision(&data, &france);
    assert!(
        !vision.mask.sees_point(&data, point),
        "enemy land is not seen"
    );
    assert!(vision.mask.coverage_at(point) < 128);
}

#[test]
fn vision_is_cheap() {
    let data = data();
    let france = fac("fac_france");
    let state = CampaignState::new_1337(&data, france.clone(), 1).unwrap();
    let _ = state.vision(&data, &france); // rasters decoded once
                                          // Best of 20 runs for France (the machine may be busy with other work).
    let per_call = (0..20)
        .map(|_| {
            let start = Instant::now();
            let _ = state.vision(&data, &france);
            start.elapsed().as_secs_f64() * 1000.0
        })
        .fold(f64::INFINITY, f64::min);
    println!("vision: {per_call:.2} ms per faction");
    // Generous bound for debug builds on CI; release is a few ms.
    assert!(per_call < 500.0, "vision took {per_call:.1} ms");
}
