//! R1-0: the campaign chronicle probe plays and measures (40 turns, seed 1).

use seed_bots::timeline::{self, ProbeConfig, Profile, METRICS};

#[test]
fn the_probe_reports_its_nine_metrics() {
    let data = data_model::test_support::game_data();
    let mut config = ProbeConfig::new("fac_france", 1, Profile::Ai);
    config.turns = 40;
    let run = timeline::run(data, &config);
    assert_eq!(run.turns, 40);
    assert_eq!(run.rows.len(), 10, "one row per year");
    let summary = run.summary();
    for name in METRICS {
        let value = summary
            .get(name)
            .unwrap_or_else(|| panic!("metric {name} missing"));
        assert!(value.is_finite(), "{name} = {value}");
    }
    assert!(run.csv().lines().count() == 11);
    assert!(run.report().contains("METRICS fac_france seed 1 ai"));
}
