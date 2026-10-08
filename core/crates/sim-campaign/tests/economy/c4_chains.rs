//! Lot C4 integration tests: Medieval-II-style building chains (upgrade
//! replaces the previous level, no stacking) and their prerequisites.
//! `docs/design/2026-09-24-analyse-total-war.md` § 2.1 « Bâtiments en
//! chaînes/arbres ».

use data_model::{GameData, SettlementId};
use sim_campaign::test_support::{bld, city, idle, start};
use sim_campaign::{CampaignState, Order};

use data_model::test_support::{fac, game_data};

/// Runs the current construction of `settlement` to completion.
fn finish_construction(state: &mut CampaignState, data: &GameData, settlement: &SettlementId) {
    for _ in 0..50 {
        if state
            .settlement_state(settlement)
            .unwrap()
            .construction
            .is_none()
        {
            break;
        }
        state.end_turn_with(data, idle);
    }
    assert!(
        state
            .settlement_state(settlement)
            .unwrap()
            .construction
            .is_none(),
        "construction should have completed"
    );
}

#[test]
fn market_chain_upgrades_replace_the_previous_level() {
    let data = game_data();
    let mut state = start(data, "fac_france", 401);
    let agen = city(&state, "prov_agenais");
    // prov_agenais's city starts with `bld_market` but no `bld_guild_hall`
    // or `bld_fair` (data/settlements/prov_agenais.json).
    assert!(state
        .settlement_state(&agen)
        .unwrap()
        .buildings
        .contains(&bld("bld_market")));

    state
        .submit_order(
            data,
            Order::Build {
                settlement: agen.clone().into(),
                building: bld("bld_guild_hall"),
            },
        )
        .unwrap();
    finish_construction(&mut state, data, &agen);
    let buildings = &state.settlement_state(&agen).unwrap().buildings;
    assert!(buildings.contains(&bld("bld_guild_hall")));
    assert!(
        !buildings.contains(&bld("bld_market")),
        "the guild hall replaces the market, it does not stack with it"
    );

    // Building the fair now requires the guild hall (not the market
    // directly): the chain is linear, market -> guild hall -> fair.
    state
        .submit_order(
            data,
            Order::Build {
                settlement: agen.clone().into(),
                building: bld("bld_fair"),
            },
        )
        .unwrap();
    finish_construction(&mut state, data, &agen);
    let buildings = &state.settlement_state(&agen).unwrap().buildings;
    assert!(buildings.contains(&bld("bld_fair")));
    assert!(!buildings.contains(&bld("bld_guild_hall")));
    assert!(!buildings.contains(&bld("bld_market")));
}

#[test]
fn fair_is_blocked_without_the_guild_hall() {
    let data = game_data();
    let state = start(data, "fac_france", 402);
    let agen = city(&state, "prov_agenais");
    let options = state.buildable(data, &agen);
    let fair = options
        .iter()
        .find(|o| o.building == bld("bld_fair"))
        .unwrap();
    assert!(!fair.available, "{:?}", fair.reason);
    assert!(fair.reason.as_deref().unwrap().contains("nécessite"));
}

#[test]
fn religious_chain_branches_after_the_collegiate_church() {
    let data = game_data();
    let mut state = start(data, "fac_france", 403);
    let agen = city(&state, "prov_agenais");
    assert!(state
        .settlement_state(&agen)
        .unwrap()
        .buildings
        .contains(&bld("bld_parish_church")));

    // Abbey/cathedral are blocked before the collegiate church exists.
    let options = state.buildable(data, &agen);
    let abbey = options
        .iter()
        .find(|o| o.building == bld("bld_abbey"))
        .unwrap();
    assert!(!abbey.available);

    state
        .submit_order(
            data,
            Order::Build {
                settlement: agen.clone().into(),
                building: bld("bld_collegiate_church"),
            },
        )
        .unwrap();
    finish_construction(&mut state, data, &agen);
    assert!(state
        .settlement_state(&agen)
        .unwrap()
        .buildings
        .contains(&bld("bld_collegiate_church")));
    assert!(!state
        .settlement_state(&agen)
        .unwrap()
        .buildings
        .contains(&bld("bld_parish_church")));

    // Now the abbey (one of the two tier-3 branches) can be built; it
    // replaces the collegiate church.
    state
        .submit_order(
            data,
            Order::Build {
                settlement: agen.clone().into(),
                building: bld("bld_abbey"),
            },
        )
        .unwrap();
    finish_construction(&mut state, data, &agen);
    let buildings = &state.settlement_state(&agen).unwrap().buildings;
    assert!(buildings.contains(&bld("bld_abbey")));
    assert!(!buildings.contains(&bld("bld_collegiate_church")));

    // The other branch (cathedral) is no longer available: the chain does
    // not allow building both from a single collegiate church.
    let options = state.buildable(data, &agen);
    let cathedral = options
        .iter()
        .find(|o| o.building == bld("bld_cathedral"))
        .unwrap();
    assert!(!cathedral.available, "{:?}", cathedral.reason);
}

#[test]
fn water_mill_upgrades_from_the_windmill() {
    let data = game_data();
    let mut state = start(data, "fac_france", 404);
    let agen = city(&state, "prov_agenais");
    assert!(state
        .settlement_state(&agen)
        .unwrap()
        .buildings
        .contains(&bld("bld_windmill")));

    state
        .submit_order(
            data,
            Order::Build {
                settlement: agen.clone().into(),
                building: bld("bld_water_mill"),
            },
        )
        .unwrap();
    finish_construction(&mut state, data, &agen);
    let buildings = &state.settlement_state(&agen).unwrap().buildings;
    assert!(buildings.contains(&bld("bld_water_mill")));
    assert!(
        !buildings.contains(&bld("bld_windmill")),
        "the water mill replaces the windmill, it does not stack with it"
    );
}

#[test]
fn fortification_chain_still_upgrades_through_four_tiers() {
    // Pre-existing chain (palisade -> stone walls -> castle -> artillery
    // bastion); C4 only adds new chains, this one must keep working.
    let data = game_data();
    let mut state = start(data, "fac_france", 405);
    let paris = city(&state, "prov_ile_de_france");
    // Paris starts with `bld_stone_walls` and `bld_castle` (already
    // upgraded past the palisade).
    let buildings = &state.settlement_state(&paris).unwrap().buildings;
    assert!(buildings.contains(&bld("bld_castle")));
    // The bastion also requires a technology (unrelated to the chain
    // mechanic under test here).
    state
        .factions
        .get_mut(&fac("fac_france"))
        .unwrap()
        .technologies
        .insert(data_model::TechnologyId::new("tech_artillery_fortification").unwrap());
    let artillery_bastion = state.buildable(data, &paris);
    let option = artillery_bastion
        .iter()
        .find(|o| o.building == bld("bld_artillery_bastion"))
        .unwrap();
    assert!(option.available, "{:?}", option.reason);
}

/// Compatibility: settlement/save data written before lot C4 (a building
/// list that predates the chain restructuring, e.g. several tiers of the
/// same chain listed together) must still load and compute effects without
/// error — `Building` ids never changed, only their `upgrades_from`/`tier`
/// metadata, so `effects_of` (which just sums whatever is in the list)
/// keeps working; only future construction is gated by the new chain.
#[test]
fn pre_c4_stacked_chain_data_still_loads_and_computes_effects() {
    let data = game_data();
    let mut state = start(data, "fac_france", 406);
    // A save from before the chain restructuring (and before EQ2 cleaned
    // the seed data) lists both `bld_abbey` and `bld_cathedral` (and
    // `bld_market`/`bld_guild_hall`/`bld_fair`) together in Paris.
    let paris = city(&state, "prov_ile_de_france");
    state
        .settlements
        .get_mut(&paris)
        .unwrap()
        .buildings
        .extend([bld("bld_market"), bld("bld_guild_hall"), bld("bld_abbey")]);
    let buildings = &state.settlement_state(&paris).unwrap().buildings;
    assert!(buildings.contains(&bld("bld_abbey")));
    assert!(buildings.contains(&bld("bld_cathedral")));
    // No panic computing effects on the "stacked" list.
    let _ = state.settlement_effects(data, &paris);
    let _ = state.province_effects(
        data,
        &data_model::ProvinceId::new("prov_ile_de_france").unwrap(),
    );
}
