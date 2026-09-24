//! G1 « dernières règles inertes » integration tests: recruitment slots,
//! piety, levy armour/ranged bonuses, `transfer_province`, player ransoms and
//! allies joining siege assaults. See `docs/wip/g1-rules.md`.

use std::path::PathBuf;

use data_model::{BuildingId, FactionId, GameData, ProvinceId, UnitTypeId};
use sim_campaign::{CampaignState, Order, OrderError};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn prov(id: &str) -> ProvinceId {
    ProvinceId::new(id).unwrap()
}

fn bld(id: &str) -> BuildingId {
    BuildingId::new(id).unwrap()
}

fn unit(id: &str) -> UnitTypeId {
    UnitTypeId::new(id).unwrap()
}

/// France at spring 1337 without chronicle events (no random noise).
fn quiet_france(data: &GameData, seed: u64) -> CampaignState {
    let mut state = CampaignState::new_1337(data, fac("fac_france"), seed).expect("1337 start");
    state.chronicle.disabled = true;
    state.factions.get_mut(&fac("fac_france")).unwrap().treasury = 100_000;
    state
}

// 1. recruit_slots ---------------------------------------------------------

#[test]
fn recruit_slots_cap_the_province_queue() {
    let data = data();
    let mut state = quiet_france(&data, 1);
    let province = prov("prov_champagne");
    let p = state.provinces.get_mut(&province).unwrap();
    p.buildings.retain(|b| {
        data.buildings[b]
            .effects
            .iter()
            .all(|e| e.effect != data_model::EffectKind::RecruitSlots)
    });
    let base = state.recruit_slots(&data, &province);
    assert_eq!(base, sim_campaign::BASE_RECRUIT_SLOTS);
    let recruit = |state: &mut CampaignState| {
        state.submit_order(
            &data,
            Order::Recruit {
                province: province.clone(),
                unit_type: unit("unit_urban_militia"),
            },
        )
    };
    for _ in 0..base {
        recruit(&mut state).unwrap();
    }
    assert_eq!(
        recruit(&mut state),
        Err(OrderError::RecruitQueueFull { slots: base })
    );
    // A muster field adds one slot.
    state
        .provinces
        .get_mut(&province)
        .unwrap()
        .buildings
        .push(bld("bld_muster_field"));
    assert_eq!(state.recruit_slots(&data, &province), base + 1);
    recruit(&mut state).unwrap();
    assert!(recruit(&mut state).is_err());
}
