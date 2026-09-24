//! F4 regression tests: historical heirs of 1337 are linked (Portugal's
//! « la lignée s'éteint » in winter 1337) and sovereigns seldom die in battle.

use std::path::PathBuf;

use data_model::{CharacterId, FactionId, GameData};
use sim_campaign::{CampaignState, EventKind};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn chr(id: &str) -> CharacterId {
    CharacterId::new(id).unwrap()
}

#[test]
fn historical_heirs_are_linked_in_1337() {
    let data = data();
    let state = CampaignState::new_1337(&data, fac("fac_france"), 1337).unwrap();
    for (faction, heir) in [
        ("fac_portugal", "chr_pedro_i_de_portugal"),
        ("fac_savoy", "chr_amedee_vi_de_savoie"),
        ("fac_france", "chr_jean_de_normandie"),
    ] {
        assert_eq!(
            state.factions[&fac(faction)].heir,
            Some(chr(heir)),
            "{faction} heir"
        );
    }
}

#[test]
fn afonso_iv_is_succeeded_by_his_son() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1337).unwrap();
    let mut events = Vec::new();
    sim_campaign::characters::kill(&mut state, &data, &chr("chr_afonso_iv"), &mut events);
    let portugal = &state.factions[&fac("fac_portugal")];
    assert_eq!(portugal.ruler, Some(chr("chr_pedro_i_de_portugal")));
    let succession = events
        .iter()
        .find(|e| e.kind == EventKind::Succession)
        .expect("succession event");
    assert!(
        !succession.text_fr.contains("lignée s'éteint"),
        "{}",
        succession.text_fr
    );
}

#[test]
fn portugal_keeps_its_house_through_1337_in_a_france_game() {
    let data = data();
    let mut state = CampaignState::new_1337(&data, fac("fac_france"), 1337).unwrap();
    for _ in 0..3 {
        let events = state.end_turn(&data);
        assert!(
            !events.iter().any(|e| e.kind == EventKind::Succession
                && e.text_fr.contains("lignée s'éteint")
                && e.faction == Some(fac("fac_portugal"))),
            "Portugal loses its dynasty in {}",
            state.date_label()
        );
    }
}

#[test]
fn sovereigns_are_safer_than_generals_in_defeat() {
    const _: () = assert!(
        sim_campaign::dynasty::RULER_DEATH_PERMILLE < sim_campaign::dynasty::GENERAL_DEATH_PERMILLE
    );
}
