//! Lot WH `chars` (ADR 0276, 0277): royal acts with cooldown, temporary
//! wounds, hiring captains, widened XP with level-up announcements.

use data_model::test_support::game_data;
use data_model::{CharacterId, EffectKind, RoyalActId, TechnologyId, TraitId};
use sim_campaign::royal_acts::{ai_choose_royal_act, royal_act_options};
use sim_campaign::test_support::{idle, start_quiet};
use sim_campaign::{CampaignState, EventKind, Order, OrderError};

fn act(id: &str) -> RoyalActId {
    RoyalActId::new(id).unwrap()
}

fn rich_ruler(state: &mut CampaignState, faction: &str) -> CharacterId {
    let faction_id = data_model::FactionId::new(faction).unwrap();
    let f = state.factions.get_mut(&faction_id).unwrap();
    f.treasury = 50_000;
    let ruler = f.ruler.clone().unwrap();
    state.characters.get_mut(&ruler).unwrap().prestige = 100;
    ruler
}

// ----- royal acts ------------------------------------------------------------

#[test]
fn royal_act_costs_prestige_and_livres_then_cools_down() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 1);
    let ruler = rich_ruler(&mut state, "fac_france");
    let france = state.player_faction.clone();
    let definition = &data.royal_acts[&act("act_sacre_reims")];

    state
        .submit_order(
            data,
            Order::RoyalAct {
                act: act("act_sacre_reims"),
            },
        )
        .expect("France may be crowned at Reims");
    assert_eq!(
        state.characters[&ruler].prestige,
        100 - definition.cost_prestige + definition.gain_prestige
    );
    assert_eq!(
        state.factions[&france].treasury,
        50_000 - definition.cost_livres
    );

    // Cooldown: the same act is refused until it elapses.
    let again = state.submit_order(
        data,
        Order::RoyalAct {
            act: act("act_sacre_reims"),
        },
    );
    assert!(matches!(
        again,
        Err(OrderError::RoyalAct(
            sim_campaign::royal_acts::RoyalActError::Cooldown(_)
        ))
    ));
    state.turn += definition.cooldown_turns;
    state
        .submit_order(
            data,
            Order::RoyalAct {
                act: act("act_sacre_reims"),
            },
        )
        .expect("ready again after the cooldown");
}

#[test]
fn royal_act_is_reserved_to_its_factions_and_needs_prestige() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 1);
    let ruler = rich_ruler(&mut state, "fac_france");
    // The English chevauchée is not for France.
    let wrong = state.submit_order(
        data,
        Order::RoyalAct {
            act: act("act_chevauchee_royale"),
        },
    );
    assert!(matches!(
        wrong,
        Err(OrderError::RoyalAct(
            sim_campaign::royal_acts::RoyalActError::WrongFaction
        ))
    ));
    state.characters.get_mut(&ruler).unwrap().prestige = 0;
    let poor = state.submit_order(
        data,
        Order::RoyalAct {
            act: act("act_sacre_reims"),
        },
    );
    assert!(matches!(
        poor,
        Err(OrderError::RoyalAct(
            sim_campaign::royal_acts::RoyalActError::NotEnoughPrestige { .. }
        ))
    ));
    // The list shows only the acts open to France, the poor one unavailable.
    let options = royal_act_options(&state, data, &state.player_faction);
    assert!(options
        .iter()
        .any(|o| o.id == act("act_sacre_reims") && !o.available));
    assert!(options.iter().all(|o| o.id != act("act_chevauchee_royale")));
}

#[test]
fn royal_act_effects_last_their_duration_only() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 1);
    rich_ruler(&mut state, "fac_france");
    let france = state.player_faction.clone();
    let definition = &data.royal_acts[&act("act_sacre_reims")];
    let before = sim_campaign::research::faction_tech_effects(&state, data, &france);
    state
        .submit_order(
            data,
            Order::RoyalAct {
                act: act("act_sacre_reims"),
            },
        )
        .unwrap();
    let during = sim_campaign::research::faction_tech_effects(&state, data, &france);
    assert_eq!(
        during[EffectKind::Unrest].flat - before[EffectKind::Unrest].flat,
        -6.0
    );
    state.turn += definition.duration_turns;
    let after = sim_campaign::research::faction_tech_effects(&state, data, &france);
    assert_eq!(
        after[EffectKind::Unrest].flat,
        before[EffectKind::Unrest].flat
    );
}

#[test]
fn royal_acts_survive_save_and_load() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 1);
    rich_ruler(&mut state, "fac_france");
    state
        .submit_order(
            data,
            Order::RoyalAct {
                act: act("act_lit_de_justice"),
            },
        )
        .unwrap();
    let france = state.player_faction.clone();
    let loaded = CampaignState::load_json(&state.save_json()).expect("round trip");
    assert_eq!(
        loaded.factions[&france].royal_acts,
        state.factions[&france].royal_acts
    );
    assert_eq!(loaded.save_json(), state.save_json());
}

#[test]
fn ai_royal_acts_are_valid_deterministic_and_keep_a_reserve() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 1);
    let england = data_model::FactionId::new("fac_england").unwrap();
    rich_ruler(&mut state, "fac_england");
    let orders = ai_choose_royal_act(&state, data, &england);
    assert_eq!(orders.len(), 1, "a rich realm performs one act");
    assert_eq!(orders, ai_choose_royal_act(&state, data, &england));
    let mut probe = state.clone();
    probe
        .apply_order(data, &england, orders[0].clone())
        .expect("valid");
    // Just after, the cooldown stops it; and a poor treasury stops it too.
    state.factions.get_mut(&england).unwrap().treasury = 10;
    assert!(ai_choose_royal_act(&state, data, &england).is_empty());
}

// ----- temporary wounds -------------------------------------------------------

fn wounded() -> TraitId {
    TraitId::new("trait_wounded").unwrap()
}

#[test]
fn a_wound_heals_after_its_duration() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 1);
    let general = state.factions[&state.player_faction].ruler.clone().unwrap();
    assert!(sim_campaign::skills::grant_trait(
        &mut state,
        data,
        &general,
        &wounded()
    ));
    assert_eq!(state.characters[&general].trait_expiry[&wounded()], 4);
    let mut healed_at = None;
    for turn in 1..=6 {
        state.end_turn_with(data, idle);
        if healed_at.is_none() && !state.characters[&general].traits.contains(&wounded()) {
            healed_at = Some(turn);
        }
    }
    assert_eq!(healed_at, Some(4), "four seasons, as the trait says");
    assert!(state.characters[&general].trait_expiry.is_empty());
}

#[test]
fn wound_recovery_shortens_the_wound() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 1);
    let france = state.player_faction.clone();
    let general = state.factions[&france].ruler.clone().unwrap();
    for tech in [
        "tech_barber_surgeons",
        "tech_chauliac_surgery",
        "tech_soporific_sponge",
    ] {
        state
            .factions
            .get_mut(&france)
            .unwrap()
            .technologies
            .insert(TechnologyId::new(tech).unwrap());
    }
    sim_campaign::skills::grant_trait(&mut state, data, &general, &wounded());
    assert!(state.characters[&general].trait_expiry[&wounded()] < 4);
}

#[test]
fn a_wound_in_progress_survives_save_and_load() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 1);
    let general = state.factions[&state.player_faction].ruler.clone().unwrap();
    sim_campaign::skills::grant_trait(&mut state, data, &general, &wounded());
    let loaded = CampaignState::load_json(&state.save_json()).unwrap();
    assert_eq!(
        loaded.characters[&general].trait_expiry,
        state.characters[&general].trait_expiry
    );
}

// ----- captains ---------------------------------------------------------------

#[test]
fn hiring_a_captain_costs_money_and_is_capped() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 1);
    let france = state.player_faction.clone();
    rich_ruler(&mut state, "fac_france");
    state
        .characters
        .get_mut(&state.factions[&france].ruler.clone().unwrap())
        .unwrap()
        .prestige = 0;
    let capital = state.factions[&france].capital.clone();
    let before = state.factions[&france].treasury;
    let cost = sim_campaign::captains::captain_cost(&state, &france);
    let cap = sim_campaign::captains::captain_cap(&state, &france);
    assert_eq!(cap, 2);

    let mut hired = Vec::new();
    for _ in 0..cap {
        state
            .submit_order(
                data,
                Order::HireCaptain {
                    settlement: capital.clone().into(),
                },
            )
            .expect("hire");
        hired.push(state.characters.iter().filter(|(_, c)| c.captain).count());
        state.turn += 10; // beyond the hiring cooldown
    }
    assert_eq!(hired, vec![1, 2]);
    assert_eq!(
        state.factions[&france].treasury,
        before - cost * i64::from(cap)
    );
    let captain = state.characters.values().find(|c| c.captain).unwrap();
    assert!(captain
        .name
        .as_deref()
        .is_some_and(|n| !n.is_empty() && !n.starts_with("chr_")));
    assert!((1..=3).contains(&captain.skills.command));
    assert!(state
        .pending_events
        .iter()
        .any(|e| e.kind == EventKind::Captain));

    let over = state.submit_order(
        data,
        Order::HireCaptain {
            settlement: capital.into(),
        },
    );
    assert!(matches!(
        over,
        Err(OrderError::Captain(
            sim_campaign::captains::CaptainError::CapReached { cap: 2 }
        ))
    ));
}

#[test]
fn a_poor_realm_cannot_hire_and_a_foreign_place_is_refused() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 1);
    let france = state.player_faction.clone();
    let capital = state.factions[&france].capital.clone();
    state.factions.get_mut(&france).unwrap().treasury = 1;
    let poor = state.submit_order(
        data,
        Order::HireCaptain {
            settlement: capital.into(),
        },
    );
    assert!(matches!(
        poor,
        Err(OrderError::Captain(
            sim_campaign::captains::CaptainError::NotEnoughFunds { .. }
        ))
    ));
    let england = data_model::FactionId::new("fac_england").unwrap();
    let foreign = state.factions[&england].capital.clone();
    state.factions.get_mut(&france).unwrap().treasury = 10_000;
    let refused = state.submit_order(
        data,
        Order::HireCaptain {
            settlement: foreign.into(),
        },
    );
    assert!(matches!(
        refused,
        Err(OrderError::Captain(
            sim_campaign::captains::CaptainError::NotYours
        ))
    ));
}

#[test]
fn the_ai_hires_a_captain_for_a_leaderless_army_only_when_nobody_is_free() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 1);
    let england = data_model::FactionId::new("fac_england").unwrap();
    // Nobody leaderless: nothing to do.
    for army in state.armies.values_mut().filter(|a| a.faction == england) {
        army.general = Some(CharacterId::new("chr_edward_iii").unwrap());
    }
    assert!(sim_campaign::captains::ai_hire_captain(&state, data, &england).is_empty());
    // One leaderless army and every adult busy or dead: a captain is hired.
    let army = state
        .armies
        .iter()
        .find(|(_, a)| a.faction == england)
        .map(|(id, _)| id.clone());
    if let Some(army) = army {
        state.armies.get_mut(&army).unwrap().general = None;
        for c in state
            .characters
            .values_mut()
            .filter(|c| c.faction == england)
        {
            c.alive = false;
        }
        state.factions.get_mut(&england).unwrap().treasury = 10_000;
        let orders = sim_campaign::captains::ai_hire_captain(&state, data, &england);
        assert_eq!(orders.len(), 1);
        state
            .apply_order(data, &england, orders[0].clone())
            .expect("valid");
    }
}

// ----- XP and levels ----------------------------------------------------------

#[test]
fn sieges_raids_treaties_and_ransoms_give_experience() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 1);
    let general = state.factions[&state.player_faction].ruler.clone().unwrap();
    let xp = |s: &CampaignState| s.characters[&general].experience;
    let rules = sim_campaign::dynasty::rules();
    let start = xp(&state);
    sim_campaign::dynasty::on_siege_won(&mut state, data, &general);
    assert_eq!(xp(&state), start + rules.siege_xp);
    sim_campaign::dynasty::on_raid_led(&mut state, data, &general);
    assert_eq!(xp(&state), start + rules.siege_xp + rules.raid_xp);
    let england = data_model::FactionId::new("fac_england").unwrap();
    let france = state.player_faction.clone();
    sim_campaign::dynasty::on_treaty_signed(&mut state, data, [&france, &england]);
    assert_eq!(
        xp(&state),
        start + rules.siege_xp + rules.raid_xp + rules.treaty_xp
    );
    sim_campaign::dynasty::on_ransomed(&mut state, data, &general);
    assert_eq!(
        xp(&state),
        start + rules.siege_xp + rules.raid_xp + rules.treaty_xp + rules.ransom_xp
    );
}

#[test]
fn crossing_a_threshold_announces_a_level_up() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 1);
    let general = state.factions[&state.player_faction].ruler.clone().unwrap();
    let c = &state.characters[&general];
    let level = sim_campaign::skills::level_of(c);
    state.pending_events.clear();
    sim_campaign::skills::grant_experience(&mut state, data, &general, 1000);
    let event = state
        .pending_events
        .iter()
        .find(|e| e.kind == EventKind::LevelUp)
        .expect("level-up event");
    assert!(event.text_fr.contains("niveau"));
    assert!(sim_campaign::skills::level_of(&state.characters[&general]) > level);
    // A grant that crosses nothing is silent.
    state.pending_events.clear();
    sim_campaign::skills::grant_experience(&mut state, data, &general, 0);
    assert!(state
        .pending_events
        .iter()
        .all(|e| e.kind != EventKind::LevelUp));
}
