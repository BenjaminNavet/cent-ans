//! G1 « dernières règles inertes » integration tests: recruitment slots,
//! piety, levy armour/ranged bonuses, `transfer_province`, player ransoms and
//! allies joining siege assaults. See `docs/wip/g1-rules.md`.

use std::path::PathBuf;

use data_model::{
    BuildingId, CharacterId, EventEffect, FactionId, GameData, ProvinceId, SettlementId, UnitTypeId,
};
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

/// The city of a province (lot C4: recruitment and buildings are per settlement).
fn city(state: &CampaignState, province: &str) -> SettlementId {
    state.province_city_id(&prov(province)).unwrap().clone()
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
    let province = city(&state, "prov_champagne");
    let p = state.settlements.get_mut(&province).unwrap();
    p.buildings.retain(|b| {
        data.buildings[b]
            .effects
            .iter()
            .all(|e| e.effect != data_model::EffectKind::RecruitSlots)
    });
    let base = state.recruit_slots(&data, &province);
    assert_eq!(base, sim_campaign::BASE_RECRUIT_SLOTS);
    let recruit = |state: &mut CampaignState| {
        // TW2-T2: the reserve of the unit type must not be what refuses.
        state
            .settlements
            .get_mut(&province)
            .unwrap()
            .recruit_pool
            .clear();
        state.submit_order(
            &data,
            Order::Recruit {
                settlement: province.clone().into(),
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
        .settlements
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
    for p in without.settlements.values_mut() {
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
    let paris = city(&with, "prov_ile_de_france");
    with.settlements
        .get_mut(&paris)
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

// 3. army_armor / army_ranged ------------------------------------------------

#[test]
fn armoury_and_butts_equip_the_units_levied_there() {
    let data = data();
    let mut state = quiet_france(&data, 4);
    let province = city(&state, "prov_ile_de_france");
    let p = state.settlements.get_mut(&province).unwrap();
    p.garrison.clear();
    for b in ["bld_muster_field", "bld_armoury", "bld_archery_butts"] {
        if !p.buildings.contains(&bld(b)) {
            p.buildings.push(bld(b));
        }
    }
    for u in ["unit_urban_militia", "unit_crossbowmen"] {
        let order = Order::Recruit {
            settlement: province.clone().into(),
            unit_type: unit(u),
        };
        state.submit_order(&data, order).unwrap();
    }
    state.end_turn_with(&data, idle);
    let garrison = &state.settlements[&province].garrison;
    let militia = garrison
        .iter()
        .find(|u| u.unit_type == unit("unit_urban_militia"));
    let crossbows = garrison
        .iter()
        .find(|u| u.unit_type == unit("unit_crossbowmen"));
    let (militia, crossbows) = (militia.unwrap(), crossbows.unwrap());
    assert_eq!((militia.levy_armor, militia.levy_ranged), (3, 0));
    assert_eq!((crossbows.levy_armor, crossbows.levy_ranged), (3, 3));
}

fn first_army_of(state: &CampaignState, faction: &str) -> sim_campaign::ArmyId {
    let found = state.armies.iter().find(|(_, a)| a.faction == fac(faction));
    found.map(|(id, _)| id.clone()).expect("an army")
}

#[test]
fn levy_bonuses_reach_the_auto_resolver_and_the_battle_setup() {
    let data = data();
    let mut state = quiet_france(&data, 5);
    let lead = first_army_of(&state, "fac_france");
    let enemy = first_army_of(&state, "fac_england");
    let plain = sim_campaign::movement::side_from_army(&state, &data, &state.armies[&lead]);
    state.armies.get_mut(&lead).unwrap().units[0].levy_armor = 5;
    let side = sim_campaign::movement::side_from_army(&state, &data, &state.armies[&lead]);
    assert_eq!(side.units[0].armor, plain.units[0].armor + 5);
    let index = state.debug_stage_battle(&lead, &enemy).unwrap();
    let setup = state.battle_setup(&data, index).unwrap();
    // G1: the battle setup now carries the same technology bonuses as the
    // auto-resolver (`side_from_army`/`plain`/`side`), on top of the levy.
    assert_eq!(setup.attacker.units[0].stats.armor, side.units[0].armor);
}

// 4. transfer_province -------------------------------------------------------

fn apply(state: &mut CampaignState, data: &GameData, faction: &str, effect: EventEffect) {
    let ctx = sim_campaign::EventContext {
        faction: Some(fac(faction)),
        province: None,
    };
    sim_campaign::chronicle::apply_effect(state, data, &effect, &ctx, &mut Vec::new());
}

#[test]
fn transfer_province_hands_over_ownership_and_control() {
    let data = data();
    let mut state = quiet_france(&data, 6);
    let dauphine = prov("prov_dauphine");
    let transfer = |from: &str| EventEffect::TransferProvince {
        province: dauphine.clone(),
        faction: None,
        from: Some(fac(from)),
    };
    // `from` must hold it: England does not.
    apply(&mut state, &data, "fac_france", transfer("fac_england"));
    assert_eq!(state.province_owner(&dauphine), Some(&fac("fac_empire")));
    apply(&mut state, &data, "fac_france", transfer("fac_empire"));
    let p = state.city_state(&dauphine).unwrap();
    assert_eq!(
        (&p.owner, &p.controller),
        (&fac("fac_france"), &fac("fac_france"))
    );
    assert!(p.garrison.is_empty() && p.siege.is_none());
    // Never a faction's capital.
    let paris = state.factions[&fac("fac_france")].capital.clone();
    let seize = EventEffect::TransferProvince {
        province: paris.clone(),
        faction: Some(fac("fac_england")),
        from: None,
    };
    apply(&mut state, &data, "fac_france", seize);
    assert_eq!(state.province_owner(&paris), Some(&fac("fac_france")));
}

#[test]
fn the_dauphine_purchase_transfers_the_province() {
    let data = data();
    let event = &data.events[&data_model::EventId::new("evt_achat_dauphine").unwrap()];
    let has_transfer = event.options[0].effects.iter().any(|e| {
        matches!(e, EventEffect::TransferProvince { province, .. } if *province == prov("prov_dauphine"))
    });
    assert!(has_transfer);
}

// 5. Player ransoms ----------------------------------------------------------

fn capture(state: &mut CampaignState, data: &GameData, faction: &str, captor: &str) -> CharacterId {
    let heir = state.factions[&fac(faction)].heir.clone().expect("an heir");
    let effect = EventEffect::CaptureCharacter {
        id: data_model::CharacterRef::Id(heir.clone()),
        faction: None,
        captor: fac(captor),
    };
    apply(state, data, faction, effect);
    assert!(state.characters[&heir].captive);
    heir
}

#[test]
fn the_player_frees_a_prisoner_against_a_fair_ransom() {
    let data = data();
    let mut state = quiet_france(&data, 7);
    let prince = capture(&mut state, &data, "fac_england", "fac_france");
    state
        .factions
        .get_mut(&fac("fac_england"))
        .unwrap()
        .treasury = 1_000_000;
    let fair = sim_campaign::ransom::ransom_amount(&state, &data, &prince);
    let release = |ransom| Order::ReleaseCaptive {
        character: prince.clone(),
        ransom,
    };
    let too_high = state.submit_order(&data, release(Some(fair + 1)));
    assert!(matches!(too_high, Err(OrderError::Ransom(_))));
    let before = state.factions[&fac("fac_france")].treasury;
    state.submit_order(&data, release(None)).unwrap();
    assert!(!state.characters[&prince].captive);
    assert_eq!(state.factions[&fac("fac_france")].treasury, before + fair);
}

#[test]
fn the_player_pays_the_ransom_of_his_captive_heir() {
    let data = data();
    let mut state = quiet_france(&data, 8);
    let dauphin = capture(&mut state, &data, "fac_france", "fac_england");
    let fair = sim_campaign::ransom::ransom_amount(&state, &data, &dauphin);
    let before = state.factions[&fac("fac_france")].treasury;
    let pay = Order::PayRansom {
        character: dauphin.clone(),
        installments: 1,
    };
    state.submit_order(&data, pay).unwrap();
    assert!(!state.characters[&dauphin].captive);
    assert_eq!(state.factions[&fac("fac_france")].treasury, before - fair);
}

// 6. Allies in siege assaults --------------------------------------------------

#[test]
fn allied_armies_in_the_province_join_the_assault() {
    let data = data();
    let mut state = quiet_france(&data, 9);
    let lead = first_army_of(&state, "fac_france");
    let guyenne = prov("prov_guyenne");
    let index = state.debug_stage_siege(&data, &lead, &guyenne).unwrap();
    let units = state.armies[&lead].units.len();
    assert!(units >= 2);
    let ally = state.peek_next_army_id();
    let split = Order::SplitArmy {
        army: lead.clone(),
        unit_indices: vec![units - 1],
    };
    state.submit_order(&data, split).unwrap();
    assert_eq!(
        sim_campaign::siege::assault_coalition(&state, &lead),
        vec![lead.clone(), ally.clone()]
    );
    let setup = state.battle_setup(&data, index).unwrap();
    assert_eq!(setup.attacker.units.len(), units);
    assert!(setup.siege.is_some());
    let before = state.armies[&ally].total_strength();
    let events = state.auto_resolve_pending(&data, index).unwrap();
    let after = state.armies.get(&ally).map_or(0, |a| a.total_strength());
    assert!(after < before, "the ally bleeds too: {before} -> {after}");
    assert!(events.iter().any(|e| e.text_fr.contains("alliée")));
}

/// G1: the battle setup passed to the 3D battle carries the faction's
/// technology bonuses (armour, melee, ranged, morale, per unit category), on
/// top of the levying province's buildings, without double-counting either.
#[test]
fn technology_bonuses_reach_the_battle_setup_without_double_counting() {
    let data = data();
    let mut state = quiet_france(&data, 5);
    let france_id = fac("fac_france");
    let lead = first_army_of(&state, "fac_france");
    let enemy = first_army_of(&state, "fac_england");
    let unit_type = data.unit_types[&state.armies[&lead].units[0].unit_type].clone();
    assert_eq!(unit_type.category, data_model::UnitCategory::Cavalry);

    // Baseline: no technology at all, no building bonus (the 1337 start
    // already grants some techs, so start from a clean slate to isolate
    // what this test is checking).
    state
        .factions
        .get_mut(&france_id)
        .unwrap()
        .technologies
        .clear();
    let index = state.debug_stage_battle(&lead, &enemy).unwrap();
    let setup = state.battle_setup(&data, index).unwrap();
    assert_eq!(setup.attacker.units[0].stats.armor, unit_type.stats.armor);

    // tech_coat_of_plates: +5 armour for infantry and cavalry (data-driven).
    state
        .factions
        .get_mut(&france_id)
        .unwrap()
        .technologies
        .insert(data_model::TechnologyId::new("tech_coat_of_plates").unwrap());
    // Also give the unit a building-derived levy bonus, to check the two
    // sources add up instead of one shadowing or duplicating the other.
    state.armies.get_mut(&lead).unwrap().units[0].levy_armor = 3;

    let index = state.debug_stage_battle(&lead, &enemy).unwrap();
    let setup = state.battle_setup(&data, index).unwrap();
    let expected = unit_type.stats.armor.saturating_add(5).saturating_add(3);
    assert_eq!(setup.attacker.units[0].stats.armor, expected);

    // The auto-resolver (`side_from_army`) computes the very same total: the
    // two code paths do not double-apply the technology on top of each other.
    let side = sim_campaign::movement::side_from_army(&state, &data, &state.armies[&lead]);
    assert_eq!(side.units[0].armor, expected);
}
