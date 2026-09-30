//! Chantier RC, lot RC1 (ADR 0141): a campaign battle fought across a river
//! near a bridge, ford or ferry is a crossing battle (coefficients of
//! `data/rules/river_crossings.json`, named forecast line, `setup.crossing`).

use std::path::PathBuf;

use data_model::{FactionId, GameData, MapCrossing};
use sim_battle::CrossingStructure;
use sim_campaign::river_crossing::crossing_site;
use sim_campaign::state::ArmyPosition;
use sim_campaign::{ArmyId, CampaignState};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn bridge(at: [f32; 2]) -> MapCrossing {
    MapCrossing {
        id: "bridge_test".into(),
        name: "Pont des Tourelles (Orléans)".into(),
        kind: "bridge".into(),
        structure: "stone".into(),
        river: "Loire".into(),
        px: at,
        dir: [1.0, 0.0],
        width: 1.0,
    }
}

/// An English army attacks a French one; both stand in the field at
/// `centre + attacker` and `centre + defender` (map pixels).
fn staged(data: &GameData, attacker: [f32; 2], defender: [f32; 2]) -> (CampaignState, [f32; 2]) {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 11).expect("1337 start");
    state.chronicle.disabled = true;
    for (a, b) in [("fac_france", "fac_england"), ("fac_england", "fac_france")] {
        state
            .factions
            .get_mut(&fac(a))
            .unwrap()
            .at_war_with
            .insert(fac(b));
    }
    let first = |state: &CampaignState, faction: &str| -> ArmyId {
        state
            .armies
            .iter()
            .find(|(_, a)| a.faction == fac(faction))
            .map(|(id, _)| id.clone())
            .unwrap()
    };
    let (a, d) = (first(&state, "fac_england"), first(&state, "fac_france"));
    state.debug_stage_battle(&a, &d).unwrap();
    let centre = state.army_point(data, &state.armies[&a]);
    for (id, offset) in [(&a, attacker), (&d, defender)] {
        state.armies.get_mut(id).unwrap().position =
            ArmyPosition::field([centre[0] + offset[0], centre[1] + offset[1]]);
    }
    (state, centre)
}

fn lead_armies(state: &CampaignState) -> (ArmyId, ArmyId) {
    let r = &state.pending_battles[0];
    (r.attacker.clone(), r.defender.clone())
}

#[test]
fn opposite_banks_near_a_bridge_is_a_crossing_battle() {
    let mut data = data();
    let (state, centre) = staged(&data, [0.0, -3.0], [0.0, 3.0]);
    let (a, d) = lead_armies(&state);

    data.crossings.clear();
    assert_eq!(crossing_site(&state, &data, &a, &d), None);
    let before = state.battle_forecast(&data, 0).unwrap();
    let plain_setup = state.battle_setup(&data, 0).unwrap();
    assert_eq!(plain_setup.crossing, None);

    data.crossings = vec![bridge(centre)];
    let site = crossing_site(&state, &data, &a, &d).expect("a crossing battle");
    assert_eq!(site.structure, CrossingStructure::StoneBridge);
    assert_eq!(site.river_display, "Loire");
    let after = state.battle_forecast(&data, 0).unwrap();
    assert!(
        after.attacker_power < before.attacker_power,
        "{} vs {}",
        after.attacker_power,
        before.attacker_power
    );
    assert!(after.defender_power >= before.defender_power);
    assert!(after.attacker_win_chance <= before.attacker_win_chance);
    let line = after
        .modifiers
        .iter()
        .find(|m| m.contains("pont des Tourelles"))
        .expect("named modifier line");
    assert_eq!(
        line,
        "Passage en force du pont des Tourelles (Orléans) sur la Loire (−40 %, tireurs du défenseur +25 %)"
    );
    assert!(!after
        .modifiers
        .iter()
        .any(|m| m.contains("franchit une rivière")));

    let setup = state.battle_setup(&data, 0).unwrap();
    let crossing = setup.crossing.expect("setup.crossing");
    assert_eq!(crossing.structure, CrossingStructure::StoneBridge);
    assert_eq!(crossing.name, "Pont des Tourelles (Orléans)");
    assert_eq!(crossing.river, "Loire");
    assert!(setup.river);
}

#[test]
fn same_bank_or_far_away_is_an_ordinary_battle() {
    let mut data = data();
    let (state, centre) = staged(&data, [2.0, -3.0], [-2.0, -6.0]);
    let (a, d) = lead_armies(&state);
    data.crossings = vec![bridge(centre)];
    assert_eq!(crossing_site(&state, &data, &a, &d), None);
    assert_eq!(state.battle_setup(&data, 0).unwrap().crossing, None);

    let (state, centre) = staged(&data, [0.0, -3.0], [0.0, 3.0]);
    let (a, d) = lead_armies(&state);
    data.crossings = vec![bridge([centre[0] + 200.0, centre[1]])];
    assert_eq!(crossing_site(&state, &data, &a, &d), None);
}

#[test]
fn anonymous_crossings_are_described_by_their_structure() {
    let mut data = data();
    let (state, centre) = staged(&data, [0.0, -3.0], [0.0, 3.0]);
    let (a, d) = lead_armies(&state);
    let mut ford = bridge(centre);
    ford.name.clear();
    ford.kind = "ford".into();
    ford.structure = "ford".into();
    ford.river = "Seine".into();
    data.crossings = vec![ford];
    let site = crossing_site(&state, &data, &a, &d).unwrap();
    assert_eq!(site.structure, CrossingStructure::Ford);
    let forecast = state.battle_forecast(&data, 0).unwrap();
    assert!(
        forecast
            .modifiers
            .iter()
            .any(|m| m.starts_with("Passage à gué de la Seine (−25 %")),
        "{:?}",
        forecast.modifiers
    );
}

#[test]
fn real_crossings_and_river_names_load() {
    let data = data();
    assert!(data.crossings.len() > 1000, "{}", data.crossings.len());
    assert!(data
        .crossings
        .iter()
        .any(|c| c.id == "bridge_pont_des_tourelles_orleans"));
    assert_eq!(data.river_display_name("Aare"), "Aar");
    assert_eq!(data.river_display_name("Unknown river"), "Unknown river");
}

#[test]
fn road_bridges_scale_with_the_river_width() {
    let mut data = data();
    let (state, centre) = staged(&data, [0.0, -3.0], [0.0, 3.0]);
    let (a, d) = lead_armies(&state);
    let rules = data.river_crossing_rules.clone();
    let mut road = bridge(centre);
    road.name.clear();
    road.kind = "road".into();
    road.structure = "wood".into();
    road.river = "Seine".into();

    // A stream too narrow (or of unknown width): no crossing battle, the
    // province river flag applies as before.
    for width in [0.0, rules.min_width_px as f32 - 0.05] {
        road.width = width;
        data.crossings = vec![road.clone()];
        assert_eq!(crossing_site(&state, &data, &a, &d), None, "width {width}");
        assert_eq!(state.battle_setup(&data, 0).unwrap().crossing, None);
    }

    // Halfway between the thresholds: half the penalty.
    road.width = ((rules.min_width_px + rules.full_width_px) / 2.0) as f32;
    data.crossings = vec![road.clone()];
    let site = crossing_site(&state, &data, &a, &d).expect("half a crossing");
    assert!((site.strength - 0.5).abs() < 1e-3, "{}", site.strength);
    let half = state.battle_forecast(&data, 0).unwrap();
    let expected = format!(
        "Passage en force d'un pont de bois sur la Seine (−{} %",
        ((1.0 - rules.attacker_factor.wood_bridge) * 50.0).round()
    );
    assert!(
        half.modifiers.iter().any(|m| m.starts_with(&expected)),
        "{:?}",
        half.modifiers
    );

    // Full width: the full penalty, lower than half.
    road.width = rules.full_width_px as f32 + 0.5;
    data.crossings = vec![road];
    let full = state.battle_forecast(&data, 0).unwrap();
    assert!(full.attacker_power < half.attacker_power);
    assert!(full
        .modifiers
        .iter()
        .any(|m| m.starts_with("Passage en force d'un pont de bois sur la Seine (−35 %")));
}
