//! Lot B7c: buildings consistent between data, core and interface —
//! upgrades never regress, `enables_units` gates recruitment, resource
//! costs are drawn or imported, the 1337 seed does not stack a chain.

use std::path::PathBuf;

use data_model::{
    BuildingId, FactionId, GameData, ProvinceId, ResourceId, SettlementId, UnitTypeId,
};
use sim_campaign::{CampaignState, Order};

fn data() -> GameData {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (data, _warnings) = GameData::load(&root).expect("game data loads");
    data
}

fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

fn bld(id: &str) -> BuildingId {
    BuildingId::new(id).unwrap()
}

fn city(state: &CampaignState, province: &str) -> SettlementId {
    state
        .province_city_id(&ProvinceId::new(province).unwrap())
        .unwrap()
        .clone()
}

fn stone() -> ResourceId {
    ResourceId::new("res_stone").unwrap()
}

/// Gives `faction` the technology `building` needs, a full treasury, and
/// clears the settlement's construction.
fn allow(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    settlement: &SettlementId,
    building: &str,
) {
    let definition = &data.buildings[&bld(building)];
    let f = state.factions.get_mut(faction).unwrap();
    if let Some(tech) = &definition.required_technology {
        f.technologies.insert(tech.clone());
    }
    f.treasury = 1_000_000;
    state.settlements.get_mut(settlement).unwrap().construction = None;
}

fn option(
    state: &CampaignState,
    data: &GameData,
    settlement: &SettlementId,
    building: &str,
) -> sim_campaign::BuildOption {
    state
        .buildable(data, settlement)
        .into_iter()
        .find(|o| o.building == bld(building))
        .expect("building listed")
}

#[test]
fn no_upgrade_in_the_data_loses_an_effect() {
    let data = data();
    for building in data.buildings.values() {
        if let Some(base) = &building.upgrades_from {
            let lost = data_model::upgrade_regressions(&data.buildings[base], building);
            assert!(lost.is_empty(), "{} loses {lost:?}", building.id);
        }
    }
    // The check itself: a bastion without its walls would be refused.
    let castle = &data.buildings[&bld("bld_castle")];
    let mut bastion = data.buildings[&bld("bld_artillery_bastion")].clone();
    bastion
        .effects
        .retain(|e| e.effect != data_model::EffectKind::FortificationLevel);
    assert_eq!(data_model::upgrade_regressions(castle, &bastion).len(), 1);
}

#[test]
fn an_upgrade_satisfies_the_prerequisites_of_the_building_it_replaced() {
    let data = data();
    let france = fac("fac_france");
    let mut state = CampaignState::new_1337(&data, france.clone(), 7).unwrap();
    let rouen = city(&state, "prov_normandie");
    allow(&mut state, &data, &france, &rouen, "bld_apothecary");
    state.settlements.get_mut(&rouen).unwrap().buildings = vec![bld("bld_guild_hall")];
    let apothecary = option(&state, &data, &rouen, "bld_apothecary");
    assert!(apothecary.available, "{:?}", apothecary.reason);
    // The market it replaced cannot be built again.
    let market = option(&state, &data, &rouen, "bld_market");
    assert!(!market.available);
}

#[test]
fn the_1337_seed_keeps_only_the_highest_level_of_a_chain() {
    let data = data();
    let state = CampaignState::new_1337(&data, fac("fac_france"), 7).unwrap();
    let paris = city(&state, "prov_ile_de_france");
    let buildings = &state.settlement_state(&paris).unwrap().buildings;
    for replaced in [
        "bld_market",
        "bld_guild_hall",
        "bld_stone_walls",
        "bld_parish_church",
    ] {
        assert!(
            !buildings.contains(&bld(replaced)),
            "{replaced} still listed"
        );
    }
    for kept in ["bld_fair", "bld_castle", "bld_abbey", "bld_cathedral"] {
        assert!(buildings.contains(&bld(kept)), "{kept} missing");
    }
}

#[test]
fn enables_units_gates_recruitment() {
    let data = data();
    let france = fac("fac_france");
    let mut state = CampaignState::new_1337(&data, france.clone(), 7).unwrap();
    let paris = city(&state, "prov_ile_de_france");
    let mangonel = UnitTypeId::new("unit_mangonel").unwrap();
    state
        .settlements
        .get_mut(&paris)
        .unwrap()
        .buildings
        .retain(|b| b.as_str() != "bld_siege_workshop");
    let blocked = state
        .recruit_option(&data, &france, &paris, &mangonel)
        .unwrap();
    assert!(
        blocked
            .reason
            .as_deref()
            .is_some_and(|r| r.contains("Atelier d'engins")),
        "{:?}",
        blocked.reason
    );
    state
        .settlements
        .get_mut(&paris)
        .unwrap()
        .buildings
        .push(bld("bld_siege_workshop"));
    let open = state
        .recruit_option(&data, &france, &paris, &mangonel)
        .unwrap();
    assert!(
        !open
            .reason
            .as_deref()
            .is_some_and(|r| r.contains("bâtiment requis")),
        "{:?}",
        open.reason
    );
    // Units no building lists stay free of any building requirement.
    let militia = UnitTypeId::new("unit_urban_militia").unwrap();
    let free = state
        .recruit_option(&data, &france, &paris, &militia)
        .unwrap();
    assert!(!free
        .reason
        .as_deref()
        .is_some_and(|r| r.contains("bâtiment requis")));
}

#[test]
fn stone_is_drawn_from_the_quarries_then_imported() {
    let data = data();
    let france = fac("fac_france");
    let mut state = CampaignState::new_1337(&data, france.clone(), 7).unwrap();
    let supply = state
        .free_supply(&data, &france)
        .get(&stone())
        .copied()
        .unwrap_or(0);
    assert!(supply >= 3, "France reaches {supply} stone provinces");

    // A first castle draws its 3 stone from the French quarries.
    let angers = city(&state, "prov_anjou");
    allow(&mut state, &data, &france, &angers, "bld_castle");
    let castle = option(&state, &data, &angers, "bld_castle");
    assert!(castle.available, "{:?}", castle.reason);
    assert_eq!(castle.import_cost, 0);
    assert!(castle.imported.is_empty());
    let before = state.factions[&france].treasury;
    state
        .submit_order(
            &data,
            Order::Build {
                settlement: angers.clone().into(),
                building: bld("bld_castle"),
            },
        )
        .unwrap();
    let construction = state
        .settlement_state(&angers)
        .unwrap()
        .construction
        .clone();
    let construction = construction.expect("castle under way");
    assert_eq!(construction.drawn.get(&stone()), Some(&3));
    assert_eq!(construction.paid, castle.cost);
    assert_eq!(
        state.factions[&france].treasury,
        before - i64::from(castle.cost)
    );
    let left = supply - 3;
    assert_eq!(
        state.free_supply(&data, &france).get(&stone()).copied(),
        Some(left)
    );

    // While it lasts, a second castle imports what the quarries lack.
    let rouen = city(&state, "prov_normandie");
    allow(&mut state, &data, &france, &rouen, "bld_castle");
    {
        let rouen_state = state.settlements.get_mut(&rouen).unwrap();
        rouen_state.buildings.retain(|b| b.as_str() != "bld_castle");
        rouen_state.buildings.push(bld("bld_stone_walls"));
    }
    let second = option(&state, &data, &rouen, "bld_castle");
    let missing = 3 - left.min(3);
    assert_eq!(second.imported.get(&stone()).copied().unwrap_or(0), missing);
    let price = i64::from(data.resources[&stone()].base_price)
        * i64::from(data.economy_rules.resource_import_multiplier)
        * i64::from(missing);
    assert_eq!(
        i64::from(second.import_cost),
        sim_campaign::coinage::priced(&state, &france, price)
    );

    // Cancelling refunds half of what was paid and frees the stone.
    let treasury = state.factions[&france].treasury;
    state
        .submit_order(
            &data,
            Order::CancelBuild {
                settlement: angers.clone().into(),
            },
        )
        .unwrap();
    assert_eq!(
        state.factions[&france].treasury,
        treasury + i64::from(construction.paid / 2)
    );
    assert_eq!(
        state.free_supply(&data, &france).get(&stone()).copied(),
        Some(supply)
    );
}

#[test]
fn normandy_quarries_caen_stone() {
    let data = data();
    let province = &data.provinces[&ProvinceId::new("prov_normandie_ouest").unwrap()];
    assert!(province.resources.contains(&stone()));
}
