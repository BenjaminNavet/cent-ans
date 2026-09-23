//! F1 « règles inertes » integration tests: building, technology and trait
//! effects that used to be displayed without effect, allied armies joining
//! battles, and the new chronicle effects (capture, ransom, delayed events).
//! See `docs/design/v2-finalisation.md` (lot F1).

use std::path::PathBuf;

use data_model::{
    BuildingId, CharacterId, CharacterRef, Condition, EventEffect, EventId, FactionId, GameData,
    ProvinceId, TechnologyId, TraitId, UnitTypeId,
};
use sim_campaign::{ArmyId, CampaignState, EventContext, Order, Season, Unit};

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

fn tech(id: &str) -> TechnologyId {
    TechnologyId::new(id).unwrap()
}

fn grant_tech(state: &mut CampaignState, faction: &str, id: &str) {
    state
        .factions
        .get_mut(&fac(faction))
        .unwrap()
        .technologies
        .insert(tech(id));
}

fn ruler_of(state: &CampaignState, faction: &str) -> CharacterId {
    state.factions[&fac(faction)]
        .ruler
        .clone()
        .expect("a ruler")
}

fn give_trait(state: &mut CampaignState, character: &CharacterId, id: &str) {
    state
        .characters
        .get_mut(character)
        .unwrap()
        .traits
        .insert(TraitId::new(id).unwrap());
}

fn france(data: &GameData, seed: u64) -> CampaignState {
    CampaignState::new_1337(data, fac("fac_france"), seed).expect("1337 start")
}

/// France at spring 1337 without chronicle events (no random noise).
fn quiet_france(data: &GameData, seed: u64) -> CampaignState {
    let mut state = france(data, seed);
    state.chronicle.disabled = true;
    state
}

/// A planner that does nothing: only the orders the test submits apply.
fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

fn first_army_of(state: &CampaignState, faction: &str) -> ArmyId {
    state
        .armies
        .iter()
        .find(|(_, a)| a.faction == fac(faction))
        .map(|(id, _)| id.clone())
        .expect("an army")
}

// =========================================================================
// 1. Buildings: Garrison, RecruitCost, Supply, class targeting
// =========================================================================

#[test]
fn class_targeted_building_effects_reach_only_their_class() {
    let data = data();
    let province = prov("prov_ile_de_france");
    let mut with = quiet_france(&data, 7);
    let mut without = with.clone();
    with.provinces
        .get_mut(&province)
        .unwrap()
        .buildings
        .push(bld("bld_guild_hall"));
    without
        .provinces
        .get_mut(&province)
        .unwrap()
        .buildings
        .retain(|b| b != &bld("bld_guild_hall"));
    for _ in 0..4 {
        with.end_turn_with(&data, idle);
        without.end_turn_with(&data, idle);
    }
    let a = &with.provinces[&province].population;
    let b = &without.provinces[&province].population;
    assert!(
        a.burghers.wealth > b.burghers.wealth,
        "guild hall: burghers richer ({} vs {})",
        a.burghers.wealth,
        b.burghers.wealth
    );
    assert_eq!(a.peasants.wealth, b.peasants.wealth, "peasants untouched");
    assert_eq!(a.nobility.wealth, b.nobility.wealth, "nobility untouched");
}

#[test]
fn garrison_effect_lowers_garrison_upkeep() {
    let data = data();
    let mut state = quiet_france(&data, 3);
    let france_id = fac("fac_france");
    let province = prov("prov_ile_de_france");
    let walls = ["bld_palisade", "bld_stone_walls", "bld_castle"].map(bld);
    let p = state.provinces.get_mut(&province).unwrap();
    assert!(!p.garrison.is_empty());
    p.buildings.retain(|b| !walls.contains(b));
    assert_eq!(state.province_effects(&data, &province).garrison.flat, 0.0);
    let before = state.faction_upkeep(&data, &france_id);
    state
        .provinces
        .get_mut(&province)
        .unwrap()
        .buildings
        .push(bld("bld_palisade"));
    let after = state.faction_upkeep(&data, &france_id);
    assert!(
        after < before,
        "palisade: the town pays 10 % ({before} -> {after})"
    );
}

#[test]
fn garrison_effect_reinforces_a_depleted_garrison() {
    let data = data();
    let mut state = quiet_france(&data, 4);
    let province = prov("prov_ile_de_france");
    let p = state.provinces.get_mut(&province).unwrap();
    assert!(!p.garrison.is_empty());
    p.buildings.push(bld("bld_castle"));
    for unit in &mut p.garrison {
        unit.strength = unit.max_strength / 2;
    }
    let before: u32 = p.garrison.iter().map(|u| u.strength).sum();
    state.end_turn_with(&data, idle);
    let after: u32 = state.provinces[&province]
        .garrison
        .iter()
        .map(|u| u.strength)
        .sum();
    assert!(after > before, "castle levies: {before} -> {after}");
}

#[test]
fn recruit_cost_effects_target_their_unit_family() {
    let data = data();
    let mut state = quiet_france(&data, 5);
    let france_id = fac("fac_france");
    let province = prov("prov_ile_de_france");
    let cost = |state: &CampaignState, id: &str| {
        state
            .recruit_option(&data, &france_id, &province, &unit(id))
            .unwrap()
            .cost
    };
    let knights = cost(&state, "unit_knights");
    let militia = cost(&state, "unit_urban_militia");
    state
        .provinces
        .get_mut(&province)
        .unwrap()
        .buildings
        .push(bld("bld_stables"));
    // −10 % of the base price (percent effects add up).
    let base = data.unit_types[&unit("unit_knights")].cost.money;
    assert_eq!(cost(&state, "unit_knights"), knights - base / 10);
    assert_eq!(cost(&state, "unit_urban_militia"), militia);
    // The treasury pays the discounted price.
    let treasury = state.factions[&france_id].treasury;
    state
        .submit_order(
            &data,
            Order::Recruit {
                province: province.clone(),
                unit_type: unit("unit_knights"),
            },
        )
        .unwrap();
    assert_eq!(
        treasury - state.factions[&france_id].treasury,
        i64::from(cost(&state, "unit_knights"))
    );
}

#[test]
fn supply_buildings_speed_up_recovery_in_the_province() {
    let data = data();
    let base = quiet_france(&data, 6);
    let army = first_army_of(&base, "fac_france");
    let location = base.armies[&army].location.clone();
    let run = |port: bool| {
        let mut state = base.clone();
        state.armies.get_mut(&army).unwrap().supply = 10;
        let p = state.provinces.get_mut(&location).unwrap();
        p.buildings.retain(|b| b != &bld("bld_port"));
        if port {
            p.buildings.push(bld("bld_port"));
        }
        state.end_turn_with(&data, idle);
        state.armies[&army].supply
    };
    assert_eq!(run(true), run(false) + 10);
}

// =========================================================================
// 2. Technologies
// =========================================================================

#[test]
fn army_upkeep_technology_lowers_the_bill() {
    let data = data();
    let mut state = quiet_france(&data, 11);
    let france_id = fac("fac_france");
    let before = state.faction_upkeep(&data, &france_id);
    grant_tech(&mut state, "fac_france", "tech_standing_companies");
    let after = state.faction_upkeep(&data, &france_id);
    let expected = before as f64 * 0.9;
    assert!(
        (after as f64 - expected).abs() <= before as f64 * 0.01 + 5.0,
        "standing companies -10 %: {before} -> {after}"
    );
}

#[test]
fn army_experience_technology_trains_recruits() {
    let data = data();
    let mut state = quiet_france(&data, 12);
    grant_tech(&mut state, "fac_france", "tech_standing_companies");
    let province = prov("prov_ile_de_france");
    state.factions.get_mut(&fac("fac_france")).unwrap().treasury = 100_000;
    let garrison_before = state.provinces[&province].garrison.len();
    state
        .submit_order(
            &data,
            Order::Recruit {
                province: province.clone(),
                unit_type: unit("unit_urban_militia"),
            },
        )
        .unwrap();
    state.end_turn_with(&data, idle);
    let recruit = &state.provinces[&province].garrison[garrison_before];
    assert_eq!(recruit.unit_type, unit("unit_urban_militia"));
    assert!(recruit.experience >= 2, "experience {}", recruit.experience);
}

#[test]
fn recruit_cost_technology_targets_its_family() {
    let data = data();
    let mut state = quiet_france(&data, 13);
    let france_id = fac("fac_france");
    let province = prov("prov_ile_de_france");
    let cost = |state: &CampaignState, id: &str| {
        state
            .recruit_option(&data, &france_id, &province, &unit(id))
            .unwrap()
            .cost
    };
    let crossbow = cost(&state, "unit_crossbowmen");
    let knights = cost(&state, "unit_knights");
    grant_tech(&mut state, "fac_france", "tech_francs_archers");
    let base = data.unit_types[&unit("unit_crossbowmen")].cost.money;
    assert_eq!(cost(&state, "unit_crossbowmen"), crossbow - base / 10);
    assert_eq!(cost(&state, "unit_knights"), knights);
}

#[test]
fn siege_trains_slow_armies_until_field_artillery() {
    let data = data();
    let mut state = quiet_france(&data, 14);
    let army_id = first_army_of(&state, "fac_france");
    let trebuchet = Unit::fresh(&data.unit_types[&unit("unit_trebuchet")]);
    let mut army = state.armies[&army_id].clone();
    army.general = None;
    let plain = state.army_movement_allowance(&data, &army);
    assert_eq!(plain, state.season().movement_points());
    army.units.push(trebuchet);
    assert_eq!(state.army_movement_allowance(&data, &army), plain - 1);
    grant_tech(&mut state, "fac_france", "tech_field_artillery");
    assert_eq!(state.army_movement_allowance(&data, &army), plain);
}

#[test]
fn production_buildings_raise_income() {
    let data = data();
    let mut state = quiet_france(&data, 15);
    let france_id = fac("fac_france");
    let province = prov("prov_ile_de_france");
    state
        .provinces
        .get_mut(&province)
        .unwrap()
        .buildings
        .retain(|b| b != &bld("bld_weaving_workshop"));
    let before = state.faction_income_effective(&data, &france_id);
    state
        .provinces
        .get_mut(&province)
        .unwrap()
        .buildings
        .push(bld("bld_weaving_workshop"));
    let after = state.faction_income_effective(&data, &france_id);
    assert!(
        after > before,
        "weaving workshop +15 % production: {before} -> {after}"
    );
}

#[test]
fn siege_resistance_and_masonry_harden_walled_towns() {
    let data = data();
    let mut state = quiet_france(&data, 16);
    let province = prov("prov_ile_de_france");
    let england = fac("fac_england");
    state
        .factions
        .get_mut(&fac("fac_france"))
        .unwrap()
        .technologies
        .remove(&tech("tech_masonry"));
    let p = state.provinces.get_mut(&province).unwrap();
    p.buildings.retain(|b| {
        !["bld_palisade", "bld_stone_walls", "bld_castle"]
            .map(bld)
            .contains(b)
    });
    let open_level = state.fortification_level(&data, &province);
    let open_resistance = state.siege_resistance(&data, &province, &england);
    state
        .provinces
        .get_mut(&province)
        .unwrap()
        .buildings
        .push(bld("bld_stone_walls"));
    let walled = state.fortification_level(&data, &province);
    let walled_resistance = state.siege_resistance(&data, &province, &england);
    assert!(walled_resistance >= open_resistance + 10.0);
    // English siege engineering eats into it.
    grant_tech(&mut state, "fac_england", "tech_siege_engineering");
    assert!(state.siege_resistance(&data, &province, &england) < walled_resistance);
    // French masonry adds a level to walled towns only.
    grant_tech(&mut state, "fac_france", "tech_masonry");
    assert_eq!(state.fortification_level(&data, &province), walled + 1);
    if open_level == 0 {
        let p = state.provinces.get_mut(&province).unwrap();
        p.buildings.retain(|b| b != &bld("bld_stone_walls"));
        assert_eq!(state.fortification_level(&data, &province), 0);
    }
}

#[test]
fn class_targeted_wealth_technology_enriches_peasants() {
    let data = data();
    let mut with = quiet_france(&data, 17);
    with.factions
        .get_mut(&fac("fac_france"))
        .unwrap()
        .technologies
        .remove(&tech("tech_three_field_rotation"));
    let mut without = with.clone();
    grant_tech(&mut with, "fac_france", "tech_three_field_rotation");
    for _ in 0..4 {
        with.end_turn_with(&data, idle);
        without.end_turn_with(&data, idle);
    }
    let province = prov("prov_ile_de_france");
    let a = &with.provinces[&province].population;
    let b = &without.provinces[&province].population;
    assert!(a.peasants.wealth > b.peasants.wealth);
    assert_eq!(a.burghers.wealth, b.burghers.wealth);
}

#[test]
fn prestige_technology_feeds_the_ruler_every_year() {
    let data = data();
    let mut state = quiet_france(&data, 18);
    let france_id = fac("fac_france");
    let before = sim_campaign::dynasty::yearly_court_prestige(&state, &data, &france_id);
    grant_tech(&mut state, "fac_france", "tech_printing_press");
    let after = sim_campaign::dynasty::yearly_court_prestige(&state, &data, &france_id);
    assert_eq!(after, before + 1, "printing press: +5 / 5 a year");
    // A full year: the ruler actually gains it (other sources aside).
    let ruler = state.factions[&france_id].ruler.clone().unwrap();
    let start = state.characters[&ruler].prestige;
    for _ in 0..4 {
        state.end_turn_with(&data, idle);
    }
    assert!(state.characters[&ruler].prestige >= start + after.min(0) + after);
}

#[test]
fn research_surplus_carries_over_to_the_next_technology() {
    let data = data();
    let mut state = quiet_france(&data, 19);
    let france_id = fac("fac_france");
    let available: Vec<TechnologyId> = data
        .technologies
        .values()
        .filter(|t| {
            sim_campaign::research::tech_status(&state, &france_id, t)
                == sim_campaign::TechStatus::Available
        })
        .map(|t| t.id.clone())
        .collect();
    assert!(available.len() >= 2);
    let (first, second) = (available[0].clone(), available[1].clone());
    state
        .submit_order(
            &data,
            Order::Research {
                technology: first.clone(),
            },
        )
        .unwrap();
    let cost = sim_campaign::research::effective_cost(&data.technologies[&first], state.year());
    state
        .factions
        .get_mut(&france_id)
        .unwrap()
        .research_progress = cost - 1;
    let points = state.research_points_per_turn(&data, &france_id);
    state.end_turn_with(&data, idle);
    let f = &state.factions[&france_id];
    assert!(f.technologies.contains(&first));
    assert!(f.research.is_none());
    assert_eq!(f.research_progress, points - 1);
    state
        .submit_order(
            &data,
            Order::Research {
                technology: second.clone(),
            },
        )
        .unwrap();
    assert_eq!(
        sim_campaign::research::tech_progress(&state, &france_id, &second),
        points - 1
    );
}

// =========================================================================
// 3. Traits and skills: research, Diplomacy, Intrigue, Loyalty
// =========================================================================

#[test]
fn scholar_ruler_speeds_up_civil_research_only() {
    let data = data();
    let mut state = quiet_france(&data, 21);
    let france_id = fac("fac_france");
    let ruler = ruler_of(&state, "fac_france");
    for id in ["trait_scholar", "trait_cultured", "trait_prodigy"] {
        state
            .characters
            .get_mut(&ruler)
            .unwrap()
            .traits
            .remove(&TraitId::new(id).unwrap());
    }
    let available = |branch: data_model::TechBranch| {
        data.technologies
            .values()
            .find(|t| {
                t.branch == branch
                    && sim_campaign::research::tech_status(&state, &france_id, t)
                        == sim_campaign::TechStatus::Available
            })
            .map(|t| t.id.clone())
            .expect("an available technology")
    };
    let civil = available(data_model::TechBranch::Civil);
    let military = available(data_model::TechBranch::Military);
    let points = |state: &mut CampaignState, technology: &TechnologyId| {
        state
            .submit_order(
                &data,
                Order::Research {
                    technology: technology.clone(),
                },
            )
            .unwrap();
        state.research_points_per_turn(&data, &france_id)
    };
    let civil_plain = points(&mut state, &civil);
    let military_plain = points(&mut state, &military);
    assert_eq!(civil_plain, military_plain);
    give_trait(&mut state, &ruler, "trait_scholar");
    assert_eq!(points(&mut state, &military), military_plain);
    let civil_scholar = points(&mut state, &civil);
    assert_eq!(
        civil_scholar,
        (f64::from(civil_plain) * 1.1).round() as u32,
        "scholar: +10 % civil research"
    );
}

#[test]
fn a_diplomat_ruler_improves_foreign_attitudes() {
    let data = data();
    let mut state = quiet_france(&data, 22);
    let (england, france_id) = (fac("fac_england"), fac("fac_france"));
    let before = state.attitude(&data, &england, &france_id).0;
    let ruler = ruler_of(&state, "fac_france");
    give_trait(&mut state, &ruler, "trait_diplomat");
    let (after, reasons) = state.attitude(&data, &england, &france_id);
    assert!(after > before, "{before} -> {after}");
    assert!(reasons
        .iter()
        .any(|(text, value)| text == "Diplomatie de son souverain" && *value > 0));
}

#[test]
fn intrigue_makes_captures_likelier() {
    use sim_campaign::battle_auto::capture_chance_percent;
    use sim_campaign::{resolve_auto, BattleContext, BattleUnit, CampaignRng, Side};
    assert_eq!(capture_chance_percent(0.0, 0.0), 10);
    assert!(capture_chance_percent(4.0, 0.0) > 10);
    assert!(capture_chance_percent(0.0, 4.0) < 10);
    let unit = |strength| BattleUnit {
        strength,
        max_strength: strength,
        experience: 0,
        morale: 60,
        melee: 50,
        ranged: 0,
        armor: 30,
        is_ranged: false,
    };
    let loser = Side {
        units: vec![unit(100); 2],
        general_command: 3,
        ..Side::default()
    };
    let winner = |intrigue| Side {
        units: vec![unit(100); 8],
        general_command: 3,
        general_intrigue: intrigue,
        ..Side::default()
    };
    let captures = |intrigue| {
        (0..400)
            .filter(|seed| {
                let mut rng = CampaignRng::from_seed(*seed);
                resolve_auto(
                    &winner(intrigue),
                    &loser,
                    &BattleContext::default(),
                    &mut rng,
                )
                .defender
                .general_captured
            })
            .count()
    };
    assert!(captures(10.0) > captures(0.0) + 40);
}

#[test]
fn loyal_rulers_and_castles_keep_vassals_and_nobles_loyal() {
    let data = data();
    let mut state = quiet_france(&data, 23);
    let (brittany, france_id) = (fac("fac_brittany"), fac("fac_france"));
    assert_eq!(
        state.factions[&brittany].suzerain.as_ref(),
        Some(&france_id)
    );
    let duke = ruler_of(&state, "fac_brittany");
    state.characters.get_mut(&duke).unwrap().traits.clear();
    let before = sim_campaign::diplomacy::loyalty_target(&state, &data, &brittany, &france_id);
    give_trait(&mut state, &duke, "trait_loyal");
    let after = sim_campaign::diplomacy::loyalty_target(&state, &data, &brittany, &france_id);
    assert!(after > before, "loyal duke: {before} -> {after}");

    // A castle's `Loyalty` calms the local nobility only.
    let province = prov("prov_ile_de_france");
    let mut with = quiet_france(&data, 24);
    with.provinces
        .get_mut(&province)
        .unwrap()
        .buildings
        .retain(|b| b != &bld("bld_castle"));
    // A ravaged province: the nobility's unrest target is well above zero.
    with.provinces.get_mut(&province).unwrap().devastation = 100;
    let mut without = with.clone();
    for p in with.provinces.values_mut() {
        for class in [&mut p.population.nobility, &mut p.population.peasants] {
            class.unrest = 50;
        }
    }
    for p in without.provinces.values_mut() {
        for class in [&mut p.population.nobility, &mut p.population.peasants] {
            class.unrest = 50;
        }
    }
    with.provinces
        .get_mut(&province)
        .unwrap()
        .buildings
        .push(bld("bld_castle"));
    for _ in 0..3 {
        with.end_turn_with(&data, idle);
        without.end_turn_with(&data, idle);
    }
    let a = &with.provinces[&province].population;
    let b = &without.provinces[&province].population;
    assert!(a.nobility.unrest < b.nobility.unrest);
    assert_eq!(a.peasants.unrest, b.peasants.unrest);
}

// =========================================================================
// 4. Allied armies of the province join the battle
// =========================================================================

#[test]
fn allied_armies_in_the_province_join_the_battle() {
    let data = data();
    let mut state = quiet_france(&data, 31);
    let (france_id, england, brittany) =
        (fac("fac_france"), fac("fac_england"), fac("fac_brittany"));
    assert!(
        state.is_at_war(&france_id, &england),
        "the war starts in 1337"
    );
    let lead = first_army_of(&state, "fac_france");
    let enemy = first_army_of(&state, "fac_england");
    let units = state.armies[&lead].units.len();
    assert!(units >= 2);
    // A second French army in the same province (split from the first)...
    let split = state.peek_next_army_id();
    state
        .submit_order(
            &data,
            Order::SplitArmy {
                army: lead.clone(),
                unit_indices: vec![units - 1],
            },
        )
        .unwrap();
    // ...and a Breton ally at war with England.
    let breton = first_army_of(&state, "fac_brittany");
    let location = state.armies[&lead].location.clone();
    state.armies.get_mut(&breton).unwrap().location = location.clone();
    state
        .factions
        .get_mut(&brittany)
        .unwrap()
        .at_war_with
        .insert(england.clone());
    state
        .factions
        .get_mut(&england)
        .unwrap()
        .at_war_with
        .insert(brittany.clone());
    let breton_units = state.armies[&breton].units.len();
    let coalition = sim_campaign::movement::battle_coalition(&state, &lead, &england);
    assert_eq!(coalition[0], lead, "the army of the encounter leads");
    let mut allies = coalition[1..].to_vec();
    allies.sort();
    let mut expected = vec![split.clone(), breton.clone()];
    expected.sort();
    assert_eq!(allies, expected);

    let index = state.debug_stage_battle(&lead, &enemy).unwrap();
    let setup = state.battle_setup(&data, index).unwrap();
    assert_eq!(setup.attacker.units.len(), units + breton_units);
    assert_eq!(setup.attacker.army, lead.to_string());
    assert_eq!(setup.attacker.faction, france_id.to_string());

    let strength =
        |state: &CampaignState, id: &ArmyId| state.armies.get(id).map_or(0, |a| a.total_strength());
    let (split_before, breton_before) = (strength(&state, &split), strength(&state, &breton));
    let events = state.auto_resolve_pending(&data, index).unwrap();
    assert!(
        events.iter().any(|e| e.text_fr.contains("alliée")),
        "{events:?}"
    );
    assert!(
        strength(&state, &split) < split_before,
        "the split army took losses"
    );
    assert!(
        strength(&state, &breton) < breton_before,
        "the Breton ally took losses"
    );
}

#[test]
fn a_3d_battle_result_spreads_losses_over_the_coalition() {
    use sim_battle::{BattleOutcome, SideId, SideResult};
    let data = data();
    let mut state = quiet_france(&data, 32);
    let lead = first_army_of(&state, "fac_france");
    let enemy = first_army_of(&state, "fac_england");
    let units = state.armies[&lead].units.len();
    let split = state.peek_next_army_id();
    state
        .submit_order(
            &data,
            Order::SplitArmy {
                army: lead.clone(),
                unit_indices: vec![units - 1],
            },
        )
        .unwrap();
    let index = state.debug_stage_battle(&lead, &enemy).unwrap();
    let setup = state.battle_setup(&data, index).unwrap();
    let side = |count: usize, lost: u32| SideResult {
        losses: vec![lost; count],
        total_losses: lost * count as u32,
        morale_delta: 0,
        routed: false,
        general_killed: false,
        general_captured: false,
    };
    // A result sized for the lead army alone is refused...
    let short = BattleOutcome {
        winner: SideId::Attacker,
        attacker: side(units - 1, 1),
        defender: side(setup.defender.units.len(), 1),
        duration: 60.0,
    };
    assert!(state.resolve_pending_battle(&data, index, &short).is_err());
    let index = state.debug_stage_battle(&lead, &enemy).unwrap();
    let split_before = state.armies[&split].total_strength();
    let outcome = BattleOutcome {
        winner: SideId::Attacker,
        attacker: side(units, 7),
        defender: side(setup.defender.units.len(), 1),
        duration: 60.0,
    };
    state
        .resolve_pending_battle(&data, index, &outcome)
        .unwrap();
    assert_eq!(state.armies[&split].total_strength(), split_before - 7);
}

// =========================================================================
// 5. Chronicle: capture, ransom, delayed events, Charles VI
// =========================================================================

fn chr(id: &str) -> CharacterId {
    CharacterId::new(id).unwrap()
}

fn apply(state: &mut CampaignState, data: &GameData, faction: &str, effect: EventEffect) {
    let ctx = EventContext {
        faction: Some(fac(faction)),
        province: None,
    };
    let mut events = Vec::new();
    sim_campaign::chronicle::apply_effect(state, data, &effect, &ctx, &mut events);
}

#[test]
fn capture_and_ransom_move_the_king_and_the_money() {
    let data = data();
    let mut state = quiet_france(&data, 41);
    let jean = chr("chr_jean_de_normandie");
    let (france_id, england) = (fac("fac_france"), fac("fac_england"));
    apply(
        &mut state,
        &data,
        "fac_france",
        EventEffect::CaptureCharacter {
            id: CharacterRef::Id(jean.clone()),
            faction: None,
            captor: england.clone(),
        },
    );
    let c = &state.characters[&jean];
    assert!(c.captive);
    assert_eq!(c.captor.as_ref(), Some(&england));
    assert!(c.army.is_none());
    assert!(state.condition_holds(
        &data,
        &Condition::CharacterCaptive { id: jean.clone() },
        &EventContext::default()
    ));

    let (france_before, england_before) = (
        state.factions[&france_id].treasury,
        state.factions[&england].treasury,
    );
    apply(
        &mut state,
        &data,
        "fac_france",
        EventEffect::ReleaseCharacter {
            id: CharacterRef::Id(jean.clone()),
            faction: None,
            ransom: 5000,
        },
    );
    let c = &state.characters[&jean];
    assert!(!c.captive && c.captor.is_none());
    assert!(c
        .traits
        .contains(&TraitId::new("trait_captive_ransomed").unwrap()));
    assert_eq!(state.factions[&france_id].treasury, france_before - 5000);
    assert_eq!(state.factions[&england].treasury, england_before + 5000);
    // A second release costs nothing: he is free.
    apply(
        &mut state,
        &data,
        "fac_france",
        EventEffect::ReleaseCharacter {
            id: CharacterRef::Id(jean),
            faction: None,
            ransom: 5000,
        },
    );
    assert_eq!(state.factions[&france_id].treasury, france_before - 5000);
}

#[test]
fn poitiers_captures_the_king_and_bretigny_requires_it() {
    let data = data();
    let mut state = quiet_france(&data, 42);
    let france_id = fac("fac_france");
    let jean = chr("chr_jean_de_normandie");
    state.factions.get_mut(&france_id).unwrap().ruler = Some(jean.clone());
    let poitiers = &data.events[&EventId::new("evt_poitiers").unwrap()];
    for effect in &poitiers.options[1].effects {
        apply(&mut state, &data, "fac_france", effect.clone());
    }
    assert!(state.characters[&jean].captive);
    assert_eq!(state.chronicle.scheduled.len(), 1, "the ransom follows");
    let bretigny = &data.events[&EventId::new("evt_bretigny").unwrap()];
    let ctx = EventContext {
        faction: Some(france_id.clone()),
        province: None,
    };
    assert!(state.event_conditions_hold(&data, bretigny, &ctx));
    // Signing the peace frees the king against the ransom.
    let england = fac("fac_england");
    let england_before = state.factions[&england].treasury;
    for effect in &bretigny.options[0].effects {
        apply(&mut state, &data, "fac_france", effect.clone());
    }
    assert!(!state.characters[&jean].captive);
    assert!(!state.is_at_war(&france_id, &england));
    assert_eq!(state.factions[&england].treasury, england_before + 40_000);
}

#[test]
fn a_scheduled_event_fires_k_turns_later() {
    let data = data();
    let mut state = france(&data, 43);
    let jean = chr("chr_jean_de_normandie");
    apply(
        &mut state,
        &data,
        "fac_france",
        EventEffect::CaptureCharacter {
            id: CharacterRef::Id(jean),
            faction: None,
            captor: fac("fac_england"),
        },
    );
    let ransom = EventId::new("evt_rancon_du_roi").unwrap();
    apply(
        &mut state,
        &data,
        "fac_france",
        EventEffect::ScheduleEvent {
            event: ransom.clone(),
            delay: 3,
        },
    );
    let pending = |state: &CampaignState| {
        state
            .chronicle
            .pending_decisions
            .iter()
            .any(|d| d.event == ransom)
    };
    // Survives a save/load round trip.
    let json = state.save_json();
    let mut state = CampaignState::load_json(&json).unwrap();
    // Scheduled at the turn N = 0 with k = 3: it fires with the end of turn
    // N + 3, as the original event fired with the end of turn N.
    for _ in 0..=3 {
        assert!(!pending(&state));
        state.end_turn_with(&data, idle);
    }
    assert!(pending(&state), "fired at N+3 as a decision for the player");
    assert!(state.chronicle.scheduled.is_empty());
    assert!(state.chronicle.fired_events.contains(&ransom));
}

#[test]
fn charles_vi_is_born_to_charles_and_jeanne_and_goes_mad() {
    let data = data();
    let mut state = quiet_france(&data, 44);
    // Winter 1338: Charles (Jean and Bonne's son) and Jeanne de Bourbon
    // (no modelled parents) are born.
    state.year = 1338;
    state.season = Season::Winter;
    state.end_turn_with(&data, idle);
    let (charles, jeanne) = (chr("chr_charles_v"), chr("chr_jeanne_de_bourbon"));
    assert!(state.characters.contains_key(&charles), "Charles V born");
    assert!(
        state.characters.contains_key(&jeanne),
        "Jeanne de Bourbon born"
    );
    // 1350: the dauphin's wedding.
    let wedding = &data.events[&EventId::new("evt_noces_du_dauphin").unwrap()];
    for effect in &wedding.options[0].effects {
        apply(&mut state, &data, "fac_france", effect.clone());
    }
    assert_eq!(state.characters[&charles].spouse.as_ref(), Some(&jeanne));
    // Winter 1368: Charles VI.
    state.year = 1368;
    state.season = Season::Winter;
    state.end_turn_with(&data, idle);
    let charles_vi = chr("chr_charles_vi");
    let child = state.characters.get(&charles_vi).expect("Charles VI born");
    assert_eq!(child.father.as_ref(), Some(&charles));
    assert_eq!(child.mother.as_ref(), Some(&jeanne));
    // 1392: the madness strikes him once he reigns.
    let madness = &data.events[&EventId::new("evt_folie_charles_vi").unwrap()];
    let ctx = EventContext {
        faction: Some(fac("fac_france")),
        province: None,
    };
    state.year = 1392;
    assert!(!state.event_conditions_hold(&data, madness, &ctx));
    state.factions.get_mut(&fac("fac_france")).unwrap().ruler = Some(charles_vi.clone());
    assert!(state.event_conditions_hold(&data, madness, &ctx));
    for effect in &madness.options[0].effects {
        apply(&mut state, &data, "fac_france", effect.clone());
    }
    assert!(state.characters[&charles_vi]
        .traits
        .contains(&TraitId::new("trait_mad").unwrap()));
}

#[test]
fn new_chronicle_data_loads_without_warnings() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data");
    let (_, warnings) = GameData::load(&root).expect("game data loads");
    for id in [
        "evt_poitiers",
        "evt_bretigny",
        "evt_rancon_du_roi",
        "evt_noces_du_dauphin",
        "evt_folie_charles_vi",
        "chr_charles_vi",
        "chr_jeanne_de_bourbon",
    ] {
        assert!(
            !warnings.iter().any(|w| w.entity == id),
            "{id}: {warnings:?}"
        );
    }
}
