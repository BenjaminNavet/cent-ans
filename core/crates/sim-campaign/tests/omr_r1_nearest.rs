//! OMR R1: the nearest settlement read on the settlement grid is the one the
//! walk over every settlement finds (distance, then id), on the real map.

use std::path::PathBuf;

use data_model::settlement_grid::nearest_by_walk;
use data_model::GameData;
use sim_campaign::march::{nearest_settlement, nearest_settlement_where};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    GameData::load(&root).expect("game data loads").0
}

#[test]
fn grid_matches_the_walk() {
    let data = data();
    let points: Vec<[f32; 2]> = data.settlement_px.values().copied().collect();
    assert!(points.len() > 1000);
    let (mut min, mut max) = ([f32::MAX; 2], [f32::MIN; 2]);
    for p in &points {
        for k in 0..2 {
            min[k] = min[k].min(p[k]);
            max[k] = max[k].max(p[k]);
        }
    }
    let mut queries: Vec<[f32; 2]> = Vec::new();
    // A lattice over and beyond the map.
    for i in -3..=40 {
        for j in -3..=40 {
            queries.push([
                min[0] + (max[0] - min[0]) * i as f32 / 37.0,
                min[1] + (max[1] - min[1]) * j as f32 / 37.0,
            ]);
        }
    }
    // On settlements, and halfway between neighbours (near ties).
    for w in points.windows(2).step_by(3) {
        queries.push(w[0]);
        queries.push([(w[0][0] + w[1][0]) / 2.0, (w[0][1] + w[1][1]) / 2.0]);
    }
    queries.push([-1.0e6, 3.0e5]);
    queries.push([f32::NAN, 10.0]);
    for q in &queries {
        let walk = nearest_settlement_where(&data, *q, |_| true);
        assert_eq!(nearest_settlement(&data, *q), walk, "{q:?}");
        assert_eq!(nearest_by_walk(&data, *q), walk, "{q:?}");
    }
    assert!(queries.len() > 2000);
}
