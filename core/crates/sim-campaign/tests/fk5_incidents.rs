//! Lot FK5a: staging of map incidents for the UI tests
//! (`docs/design/2026-09-29-carte-vivante-folk.md` § 3.4, ADR 0122).

use std::path::PathBuf;

use data_model::{EventId, EventPresentation, FactionId, GameData, ProvinceId};
use sim_campaign::{CampaignState, EventKind};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<sim_campaign::Order> {
    Vec::new()
}

#[test]
fn offered_decision_is_a_map_incident_that_expires_with_a_journal_entry() {
    let data = data();
    let player = FactionId::new("fac_france").unwrap();
    let mut state = CampaignState::new_1337(&data, player.clone(), 5).expect("1337 start");
    state.chronicle.disabled = true;
    let province = ProvinceId::new("prov_touraine").unwrap();
    let crue = EventId::new("evt_crue").unwrap();

    // Unknown event or province: refused.
    let unknown = EventId::new("evt_does_not_exist").unwrap();
    assert!(state
        .debug_offer_decision(&data, &unknown, Some(&province))
        .is_none());
    let nowhere = ProvinceId::new("prov_nowhere").unwrap();
    assert!(state
        .debug_offer_decision(&data, &crue, Some(&nowhere))
        .is_none());

    let id = state
        .debug_offer_decision(&data, &crue, Some(&province))
        .expect("offered");
    let views = state.decision_views(&data, &player);
    let view = views.iter().find(|v| v.id == id).expect("pending");
    assert_eq!(view.presentation, EventPresentation::Map);
    assert_eq!(view.province.as_ref(), Some(&province));
    assert!(view.expires_in >= 1);

    // Left alone, it expires: the AI's option applies, journaled with
    // « (délai écoulé) » on the province.
    let mut expiry = None;
    for _ in 0..6 {
        let events = state.end_turn_with(&data, idle);
        if let Some(entry) = events
            .into_iter()
            .find(|e| e.kind == EventKind::Chronicle && e.text_fr.contains("délai écoulé"))
        {
            expiry = Some(entry);
            break;
        }
    }
    let entry = expiry.expect("expiry journaled");
    assert!(state.chronicle.pending_decisions.iter().all(|d| d.id != id));
    assert_eq!(entry.province.as_ref(), Some(&province));
    assert_eq!(entry.faction.as_ref(), Some(&player));
    assert!(
        entry.text_fr.starts_with(&data.events[&crue].title),
        "{}",
        entry.text_fr
    );
}
