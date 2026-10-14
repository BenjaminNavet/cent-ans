//! Cross-check of the Rust composed projection against the Python reference
//! (`ComposedGrid`, fixture written by `tools`: 250 lon/lat -> px, 250 px -> lon/lat) on the real
//! `data/map/compose` files. Skipped when `data/map/map.json` is not the composed CRS.

use std::path::PathBuf;

use data_model::MapMeta;
use serde_json::Value;

#[test]
fn rust_projection_matches_python_composed_grid() {
    let map_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data/map");
    let meta = match MapMeta::load(&map_dir) {
        Ok(meta) => meta,
        Err(error) if map_dir.join("map.json").is_file() => panic!("map.json: {error}"),
        Err(_) => return,
    };
    if !meta.is_composed() {
        return;
    }
    let text = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/composed_cross.json"),
    )
    .unwrap();
    let fixture: Value = serde_json::from_str(&text).unwrap();
    let rows = |key: &str| -> Vec<[f64; 4]> {
        fixture[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                let v: Vec<f64> = r
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|x| x.as_f64().unwrap())
                    .collect();
                [v[0], v[1], v[2], v[3]]
            })
            .collect()
    };
    println!(
        "composed map: {} forward / {} inverse samples",
        rows("forward").len(),
        rows("inverse").len()
    );
    let (mut worst_px, mut worst_deg) = (0.0f64, 0.0f64);
    for [lon, lat, x, y] in rows("forward") {
        let p = meta.lonlat_to_px(lon, lat).expect("inside the canvas");
        worst_px = worst_px.max((p[0] - x).abs()).max((p[1] - y).abs());
    }
    for [x, y, lon, lat] in rows("inverse") {
        let (l, a) = meta.px_to_lonlat(x, y).expect("inside the canvas");
        worst_deg = worst_deg.max((l - lon).abs()).max((a - lat).abs());
    }
    assert!(worst_px < 0.5, "forward worst error {worst_px} px");
    assert!(worst_deg < 1e-4, "inverse worst error {worst_deg} deg");
}
