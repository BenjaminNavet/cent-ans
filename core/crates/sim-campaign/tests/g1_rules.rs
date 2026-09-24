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

// 2. Piety -----------------------------------------------------------------

fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

#[test]
fn trait_piety_raises_the_effective_piety_and_papal_favour() {
    let data = data();
    let base = quiet_france(&data, 2);
    let ruler = base.factions[&fac("fac_france")].ruler.clone().unwrap();
    let run = |pious: bool| {
        let mut state = base.clone();
        let c = state.characters.get_mut(&ruler).unwrap();
        c.piety = 50;
        c.traits.retain(|t| t.as_str() != "trait_pious");
        if pious {
            c.traits
                .insert(data_model::TraitId::new("trait_pious").unwrap());
        }
        let piety = sim_campaign::religion::effective_piety(&state, &data, &ruler);
        state
            .factions
            .get_mut(&fac("fac_france"))
            .unwrap()
            .papal_favor = 0;
        state.end_turn_with(&data, idle);
        (piety, state.factions[&fac("fac_france")].papal_favor)
    };
    let (plain, plain_favor) = run(false);
    let (pious, pious_favor) = run(true);
    assert_eq!(pious, plain + 10);
    assert!(pious_favor >= plain_favor, "{plain_favor} -> {pious_favor}");
}

#[test]
fn religious_buildings_raise_the_ruler_piety_each_winter() {
    let data = data();
    let france = fac("fac_france");
    let mut without = quiet_france(&data, 3);
    for p in without.provinces.values_mut() {
        p.buildings.retain(|b| {
            data.buildings[b]
                .effects
                .iter()
                .all(|e| e.effect != data_model::EffectKind::Piety)
        });
    }
    let yearly =
        |s: &CampaignState| sim_campaign::dynasty::yearly_building_piety(s, &data, &france);
    assert_eq!(yearly(&without), 0);
    let mut with = without.clone();
    with.provinces
        .get_mut(&prov("prov_ile_de_france"))
        .unwrap()
        .buildings
        .push(bld("bld_cathedral"));
    assert_eq!(yearly(&with), 1);
    let ruler = without.factions[&france].ruler.clone().unwrap();
    let piety_after_year = |mut state: CampaignState| {
        state.characters.get_mut(&ruler).unwrap().piety = 50;
        for _ in 0..4 {
            state.end_turn_with(&data, idle);
        }
        state.characters[&ruler].piety
    };
    assert_eq!(piety_after_year(with), piety_after_year(without) + 1);
}
