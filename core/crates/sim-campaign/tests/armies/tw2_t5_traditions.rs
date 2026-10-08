//! Lot TW2-T5 (ADR 0112): army traditions (spec
//! `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T5) — experience
//! of the army from its battles (auto-resolved and 3D), ranks and choices,
//! effects (movement, replenishment, shooting, siege, morale), kept name and
//! banner, loss on dissolution, dilution of experience by reinforcements,
//! save.
use data_model::test_support::{fac, game_data};
use sim_campaign::test_support::{capital_city, main_army};

use data_model::GameData;
use sim_campaign::replenish::resolve_replenishment;
use sim_campaign::traditions::{
    add_recruits, army_tradition_effects, grant_army_xp, rank_for_xp, siege_speed_percent,
};
use sim_campaign::{
    ArmyId, ArmyPosition, CampaignState, Order, OrderError, Season, Stance, TraditionError,
};

/// France played by the player, its main army (with its general) in Paris,
/// spring, a full treasury.
fn setup() -> (&'static GameData, CampaignState, ArmyId) {
    let data = game_data();
    let mut state = CampaignState::new_1337(data, fac("fac_france"), 3).unwrap();
    state.season = Season::Spring;
    let army = main_army(&state, "fac_france");
    let paris = capital_city(&state, "fac_france");
    let entry = state.armies.get_mut(&army).unwrap();
    entry.position = ArmyPosition::Settlement(paris);
    entry.stance = Stance::Normal;
    entry.supply = 100;
    state.factions.get_mut(&fac("fac_france")).unwrap().treasury = 100_000;
    (data, state, army)
}

fn thresholds(data: &GameData) -> Vec<u32> {
    data.army_tradition_rules.experience.rank_thresholds.clone()
}

fn choose(
    state: &mut CampaignState,
    data: &GameData,
    army: &ArmyId,
    tradition: &str,
) -> Result<(), OrderError> {
    state.submit_order(
        data,
        Order::ChooseArmyTradition {
            army: army.clone(),
            tradition: tradition.to_owned(),
        },
    )
}

/// The army at `rank` with `traditions` chosen.
fn veteran(
    state: &mut CampaignState,
    data: &GameData,
    army: &ArmyId,
    traditions: &[&str],
) -> Vec<sim_campaign::GameEvent> {
    let mut events = Vec::new();
    let needed = thresholds(data)[traditions.len().max(1) - 1];
    grant_army_xp(state, data, army, needed, &mut events);
    for tradition in traditions {
        choose(state, data, army, tradition).unwrap();
    }
    events
}

#[test]
fn ranks_follow_the_thresholds() {
    let data = game_data();
    let t = thresholds(data);
    assert!((3..=4).contains(&t.len()));
    assert_eq!(rank_for_xp(data, 0), 0);
    assert_eq!(rank_for_xp(data, t[0] - 1), 0);
    assert_eq!(rank_for_xp(data, t[0]), 1);
    assert_eq!(rank_for_xp(data, t[1]), 2);
    assert_eq!(rank_for_xp(data, u32::MAX), t.len() as u8);
}

#[test]
fn an_auto_resolved_battle_gives_experience_to_both_armies() {
    let (data, mut state, french) = setup();
    let english = main_army(&state, "fac_england");
    let index = state.debug_stage_battle(&french, &english).unwrap();
    state.auto_resolve_pending(data, index).unwrap();
    let fought = data.army_tradition_rules.experience.battle_fought;
    let mut survivors = 0;
    for id in [&french, &english] {
        if let Some(army) = state.armies.get(id) {
            survivors += 1;
            assert!(
                army.traditions.xp >= fought / 2,
                "{id}: {:?}",
                army.traditions
            );
        }
    }
    assert!(survivors > 0);
}

#[test]
fn a_3d_battle_gives_experience_and_victory_more() {
    let (data, mut state, french) = setup();
    let english = main_army(&state, "fac_england");
    let index = state.debug_stage_battle(&french, &english).unwrap();
    let setup = state.battle_setup(data, index).unwrap();
    let side = |units: usize, losses: u32, routed: bool, delta: i32| {
        serde_json::json!({
            "losses": vec![losses; units],
            "total_losses": losses * units as u32,
            "morale_delta": delta,
            "routed": routed,
        })
    };
    let outcome: sim_battle::BattleOutcome = serde_json::from_value(serde_json::json!({
        "winner": "attacker",
        "attacker": side(setup.attacker.units.len(), 5, false, 5),
        "defender": side(setup.defender.units.len(), 30, true, -20),
    }))
    .unwrap();
    state.resolve_pending_battle(data, index, &outcome).unwrap();
    let rules = &data.army_tradition_rules.experience;
    let winner = state.armies[&french].traditions.xp;
    assert!(
        winner >= rules.battle_fought + rules.battle_won - 5,
        "{winner}"
    );
    if let Some(loser) = state.armies.get(&english) {
        assert!(loser.traditions.xp < winner, "the victor learns more");
    }
    // The name and banner of the French army are now fixed.
    let traditions = &state.armies[&french].traditions;
    assert!(traditions.name.as_deref().unwrap().starts_with("l'ost"));
    assert!(traditions.banner_house.is_some());
}

#[test]
fn crushing_a_mere_band_teaches_nothing() {
    let (data, mut state, french) = setup();
    let english = main_army(&state, "fac_england");
    let band = state.armies.get_mut(&english).unwrap();
    band.units.truncate(1);
    band.units[0].strength = 5;
    let index = state.debug_stage_battle(&french, &english).unwrap();
    state.auto_resolve_pending(data, index).unwrap();
    assert_eq!(state.armies[&french].traditions.xp, 0);
}

#[test]
fn one_choice_per_rank_and_tiers_in_order() {
    let (data, mut state, army) = setup();
    // No rank yet.
    assert!(matches!(
        choose(&mut state, data, &army, "trad_march_1"),
        Err(OrderError::Tradition(TraditionError::NoRankAvailable))
    ));
    let first = thresholds(data)[0];
    let mut events = Vec::new();
    grant_army_xp(&mut state, data, &army, first, &mut events);
    assert!(
        events.iter().any(|e| e.text_fr.contains("rang 1")),
        "the player hears of the new rank: {events:?}"
    );
    let view = state.army_tradition_view(data, &army).unwrap();
    assert_eq!((view.rank, view.pending), (1, 1));
    assert_eq!(view.next_threshold, Some(thresholds(data)[1]));
    let allowed: Vec<_> = view.options.iter().filter(|o| o.allowed).collect();
    assert_eq!(allowed.len(), 5, "one first tier per branch");
    assert!(allowed.iter().all(|o| o.tier == 1));
    assert!(matches!(
        choose(&mut state, data, &army, "trad_march_2"),
        Err(OrderError::Tradition(TraditionError::MissingPrevious))
    ));
    assert!(matches!(
        choose(&mut state, data, &army, "trad_unknown"),
        Err(OrderError::Tradition(TraditionError::UnknownTradition(_)))
    ));
    choose(&mut state, data, &army, "trad_march_1").unwrap();
    assert!(matches!(
        choose(&mut state, data, &army, "trad_shooting_1"),
        Err(OrderError::Tradition(TraditionError::NoRankAvailable))
    ));
    // Rank 2: the second march tier is open, the first one taken.
    grant_army_xp(
        &mut state,
        data,
        &army,
        thresholds(data)[1] - first,
        &mut Vec::new(),
    );
    let view = state.army_tradition_view(data, &army).unwrap();
    let march_2 = view
        .options
        .iter()
        .find(|o| o.id == "trad_march_2")
        .unwrap();
    assert!(march_2.allowed);
    assert!(view
        .options
        .iter()
        .any(|o| o.id == "trad_march_1" && o.chosen));
    assert!(matches!(
        choose(&mut state, data, &army, "trad_march_1"),
        Err(OrderError::Tradition(TraditionError::AlreadyChosen))
    ));
    choose(&mut state, data, &army, "trad_march_2").unwrap();
    // Another faction cannot choose for this army.
    let english = main_army(&state, "fac_england");
    let refused = state.apply_order(
        data,
        &fac("fac_england"),
        Order::ChooseArmyTradition {
            army: army.clone(),
            tradition: "trad_march_3".to_owned(),
        },
    );
    assert!(
        matches!(refused, Err(OrderError::NotYourArmy(_))),
        "{english}"
    );
}

#[test]
fn march_tradition_lengthens_the_season_march() {
    let (data, mut state, army) = setup();
    let before = state.army_movement_allowance(data, &state.armies[&army]);
    veteran(&mut state, data, &army, &["trad_march_1"]);
    let after = state.army_movement_allowance(data, &state.armies[&army]);
    assert_eq!(
        after,
        (f64::from(before) * 1.08).round() as u32,
        "{before} -> {after}"
    );
}

#[test]
fn stewardship_tradition_raises_the_replenishment_rate() {
    let (data, mut state, army) = setup();
    // In the field on its own lands: under the rate's ceiling.
    let paris = capital_city(&state, "fac_france");
    let point = data.settlement_point(&paris).unwrap();
    let entry = state.armies.get_mut(&army).unwrap();
    entry.position = ArmyPosition::field(point);
    for unit in &mut entry.units {
        unit.strength = unit.max_strength / 2;
    }
    let before = state.army_replenishment(data, &army).unwrap();
    veteran(&mut state, data, &army, &["trad_stewardship_1"]);
    let after = state.army_replenishment(data, &army).unwrap();
    assert!(after.rate_bp > before.rate_bp, "{before:?} -> {after:?}");
    assert!(after
        .factors
        .iter()
        .any(|f| f.label == "Traditions de l'armée" && f.percent == 20));
}

#[test]
fn shooting_and_discipline_reach_both_battle_paths() {
    let (data, mut state, army) = setup();
    let english = main_army(&state, "fac_england");
    let side_before = sim_campaign::research::army_battle_side(&state, data, &army).unwrap();
    let index = state.debug_stage_battle(&army, &english).unwrap();
    let setup_before = state.battle_setup(data, index).unwrap();
    veteran(
        &mut state,
        data,
        &army,
        &["trad_shooting_1", "trad_discipline_1"],
    );
    let effects = army_tradition_effects(data, &state.armies[&army]);
    assert_eq!((effects.ranged, effects.morale), (2, 3));
    let side_after = sim_campaign::research::army_battle_side(&state, data, &army).unwrap();
    let setup_after = state.battle_setup(data, index).unwrap();
    let mut ranged_seen = false;
    for (before, after) in side_before.units.iter().zip(&side_after.units) {
        assert!(after.morale >= before.morale && after.morale <= before.morale + 3);
        if before.morale < 97 {
            assert_eq!(after.morale, before.morale + 3);
        }
        if before.is_ranged && before.ranged > 0 {
            ranged_seen = true;
            assert_eq!(after.ranged, before.ranged + 2);
        } else if !before.is_ranged && before.ranged == 0 {
            assert_eq!(after.ranged, 0, "no bow for the men-at-arms");
        }
    }
    assert!(ranged_seen, "the French main army has shooters");
    for (before, after) in setup_before
        .attacker
        .units
        .iter()
        .zip(&setup_after.attacker.units)
    {
        if before.morale < 97 {
            assert_eq!(after.morale, before.morale + 3, "{}", before.unit_type);
        }
        if before.stats.ranged > 0 {
            assert_eq!(after.stats.ranged, before.stats.ranged + 2);
        }
    }
}

#[test]
fn assault_tradition_shortens_sieges() {
    let (data, mut state, army) = setup();
    assert_eq!(siege_speed_percent(data, &state.armies[&army]), 0.0);
    veteran(&mut state, data, &army, &["trad_assault_1"]);
    let percent = siege_speed_percent(data, &state.armies[&army]);
    assert_eq!(percent, 10.0);
    assert!(
        sim_campaign::siege::supplies_drain(data, 3, percent)
            > sim_campaign::siege::supplies_drain(data, 3, 0.0)
    );
}

#[test]
fn name_and_banner_stay_when_the_general_changes() {
    let (data, mut state, army) = setup();
    let original = state.army_name(data, &army);
    assert!(state.armies[&army].general.is_some());
    veteran(&mut state, data, &army, &["trad_discipline_1"]);
    let banner = state.armies[&army].traditions.banner_house.clone();
    assert!(banner.is_some());
    // The general leaves (dies, is captured, is reassigned).
    let general = state.armies.get_mut(&army).unwrap().general.take().unwrap();
    if let Some(c) = state.characters.get_mut(&general) {
        c.army = None;
    }
    assert_eq!(state.army_name(data, &army), original);
    let view = state.army_tradition_view(data, &army).unwrap();
    assert_eq!(view.name, original);
    assert_eq!(view.banner_house, banner);
    assert_eq!(state.armies[&army].traditions.chosen, ["trad_discipline_1"]);
    // A fresh army without a general is not named after anyone.
    let english = main_army(&state, "fac_england");
    let mut events = Vec::new();
    state.armies.get_mut(&english).unwrap().general = None;
    grant_army_xp(&mut state, data, &english, 5, &mut events);
    assert!(state.armies[&english].traditions.name.is_none());
    assert!(events.is_empty(), "no news of foreign armies");
}

#[test]
fn dissolution_split_and_merge_lose_the_traditions() {
    let (data, mut state, army) = setup();
    veteran(&mut state, data, &army, &["trad_march_1"]);
    // A detachment starts afresh.
    let count = state.armies.len();
    state
        .submit_order(
            data,
            Order::SplitArmy {
                army: army.clone(),
                unit_indices: vec![0],
            },
        )
        .unwrap();
    assert_eq!(state.armies.len(), count + 1);
    let detachment = state
        .armies
        .keys()
        .max()
        .cloned()
        .filter(|id| id != &army)
        .unwrap();
    assert!(state.armies[&detachment].traditions.is_empty());
    // Merging the veteran army into the detachment loses its traditions.
    state
        .submit_order(
            data,
            Order::MergeArmies {
                source: army.clone(),
                target: detachment.clone(),
            },
        )
        .unwrap();
    assert!(!state.armies.contains_key(&army));
    assert!(state.armies[&detachment].traditions.chosen.is_empty());
    // Disbanding every unit dissolves the army.
    veteran(&mut state, data, &detachment, &["trad_assault_1"]);
    while state.armies.contains_key(&detachment) {
        state
            .submit_order(
                data,
                Order::DisbandUnit {
                    army: Some(detachment.clone()),
                    settlement: None,
                    unit_index: 0,
                },
            )
            .unwrap();
    }
    assert!(!state.armies.contains_key(&detachment));
}

#[test]
fn reinforcements_dilute_experience_pro_rata() {
    let (data, state, army) = setup();
    let mut unit = state.armies[&army].units[0].clone();
    unit.max_strength = 100;
    unit.strength = 50;
    unit.experience = 6;
    unit.experience_residue = 0;
    add_recruits(&mut unit, 50, 0);
    assert_eq!((unit.strength, unit.experience), (100, 3));
    // Small refills add up in thousandths of a level.
    unit.strength = 90;
    unit.experience = 4;
    unit.experience_residue = 0;
    add_recruits(&mut unit, 5, 0);
    assert_eq!(unit.experience, 3, "4 × 90/95 = 3.79");
    assert_eq!(unit.experience_residue, 789);
    add_recruits(&mut unit, 5, 0);
    assert_eq!(unit.experience, 3, "3.789 × 95/100 = 3.60");
    // Seasoned recruits (buildings) lift a green unit.
    unit.strength = 50;
    unit.experience = 0;
    unit.experience_residue = 0;
    add_recruits(&mut unit, 50, 4);
    assert_eq!(unit.experience, 2);

    // The T2 replenishment dilutes the army's veterans.
    let (data2, mut state, army) = (data, state, army);
    for unit in &mut state.armies.get_mut(&army).unwrap().units {
        unit.strength = unit.max_strength / 2;
        unit.experience = 6;
        unit.experience_residue = 0;
    }
    let preview = state.army_replenishment(data2, &army).unwrap();
    assert!(preview.men > 0);
    resolve_replenishment(&mut state, data2, &mut Vec::new());
    for (unit, men) in state.armies[&army].units.iter().zip(&preview.per_unit) {
        if *men > 0 {
            let milli = u32::from(unit.experience) * 1000 + u32::from(unit.experience_residue);
            let expected = 6000 * (unit.strength - men) / unit.strength;
            assert_eq!(milli, expected, "{}", unit.unit_type);
            assert!(unit.experience < 6);
        }
    }
}

#[test]
fn traditions_survive_a_save() {
    let (data, mut state, army) = setup();
    veteran(
        &mut state,
        data,
        &army,
        &["trad_stewardship_1", "trad_stewardship_2"],
    );
    state.armies.get_mut(&army).unwrap().units[0].experience_residue = 321;
    let loaded = CampaignState::load_json(&state.save_json()).unwrap();
    assert_eq!(
        loaded.armies[&army].traditions,
        state.armies[&army].traditions
    );
    assert_eq!(loaded.armies[&army].units[0].experience_residue, 321);
    // Armies without traditions add nothing to the save.
    let english = main_army(&state, "fac_england");
    let json = serde_json::to_value(&state.armies[&english]).unwrap();
    assert!(json.get("traditions").is_none());
}
