//! Lot WH `charsb` (ADR 0284): data-driven trait triggers, role-bound skills,
//! living character loyalty, assassination/poison by agents, agent aid to
//! armies.

use data_model::test_support::game_data;
use data_model::{CharacterId, SkillId, SkillRole};
use sim_campaign::skills::{learn_skill, learnable_skills, roles_of, LearnSkillError};
use sim_campaign::test_support::start_quiet;

fn chr(id: &str) -> CharacterId {
    CharacterId::new(id).unwrap()
}

// ----- top 8: skills by role ---------------------------------------------------

#[test]
fn a_governor_does_not_see_command_specialties_and_a_general_does() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 1);
    let philippe = chr("chr_philippe_vi");
    // The ruler governs nothing and leads no army here: strip any job.
    {
        let c = state.characters.get_mut(&philippe).unwrap();
        c.army = None;
        c.governor_of = None;
        c.skill_points = 10;
    }
    let banneret = SkillId::new("skill_banneret").unwrap();
    let learnable = learnable_skills(&state, data, &philippe);
    let hardiesse = SkillId::new("skill_hardiesse").unwrap();
    state
        .characters
        .get_mut(&philippe)
        .unwrap()
        .skills_learned
        .insert(hardiesse);
    assert!(
        !learnable_skills(&state, data, &philippe).contains(&banneret),
        "a pure ruler cannot take a general's specialty"
    );
    assert!(learnable
        .iter()
        .all(|s| data.skills[s].requires_role.is_empty()
            || data.skills[s].requires_role.contains(&SkillRole::Ruler)));
    assert_eq!(
        learn_skill(&mut state, data, &philippe, &banneret),
        Err(LearnSkillError::WrongRole)
    );
    // Given an army, the same ruler may learn it.
    let army = state
        .armies
        .iter()
        .find(|(_, a)| a.faction == state.player_faction)
        .map(|(id, _)| id.clone())
        .unwrap();
    state.characters.get_mut(&philippe).unwrap().army = Some(army);
    assert!(roles_of(&state, &philippe).contains(&SkillRole::General));
    learn_skill(&mut state, data, &philippe, &banneret).expect("a general may");
}

#[test]
fn every_specialty_skill_has_reachable_prerequisites_in_its_branch() {
    let data = game_data();
    for skill in data.skills.values().filter(|s| !s.requires_role.is_empty()) {
        for prerequisite in &skill.prerequisites {
            let p = &data.skills[prerequisite];
            assert_eq!(p.branch, skill.branch, "{}", skill.id);
            assert!(
                p.requires_role.is_empty(),
                "{} needs a common root",
                skill.id
            );
        }
    }
}

// ----- top 7: trait triggers ---------------------------------------------------

fn has(state: &sim_campaign::CampaignState, id: &CharacterId, t: &str) -> bool {
    state.characters[id]
        .traits
        .iter()
        .any(|held| held.as_str() == t)
}

#[test]
fn trait_triggers_come_from_data_and_fire_after_a_battle() {
    let data = game_data();
    let rules = data_model::TraitTriggerRules::bundled();
    for trigger in &rules.triggers {
        assert!(
            data.traits.contains_key(&trigger.trait_id),
            "unknown trait {}",
            trigger.trait_id
        );
    }
    let mut state = start_quiet(data, "fac_france", 1);
    let general = chr("chr_philippe_vi");
    {
        let c = state.characters.get_mut(&general).unwrap();
        c.traits.clear();
        c.battles_fought = 3;
        c.battles_won = 3;
    }
    let mut events = Vec::new();
    // Fourth fought battle, won: a hero, not yet a veteran (5 needed).
    sim_campaign::dynasty::on_battle_resolved(&mut state, data, &general, true, 1.0, &mut events);
    assert!(has(&state, &general, "trait_battle_hero"));
    assert!(!has(&state, &general, "trait_veteran"));
    sim_campaign::dynasty::on_battle_resolved(&mut state, data, &general, true, 1.0, &mut events);
    assert!(has(&state, &general, "trait_veteran"));
    assert!(events
        .iter()
        .any(|e| e.kind == sim_campaign::EventKind::TraitAcquired));
}

#[test]
fn old_age_and_debt_traits_are_seasonal_and_debt_wears_off() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 1);
    let ruler = state.factions[&state.player_faction].ruler.clone().unwrap();
    {
        let c = state.characters.get_mut(&ruler).unwrap();
        c.traits.clear();
        c.birth_year = state.year - 65;
    }
    let player = state.player_faction.clone();
    state.factions.get_mut(&player).unwrap().treasury = -500;
    let mut events = Vec::new();
    sim_campaign::trait_triggers::evaluate_all(&mut state, data, &mut events);
    assert!(has(&state, &ruler, "trait_old_age"));
    assert!(has(&state, &ruler, "trait_debtor"));
    // The same pass is idempotent.
    let before = state.characters[&ruler].traits.clone();
    sim_campaign::trait_triggers::evaluate_all(&mut state, data, &mut events);
    assert_eq!(state.characters[&ruler].traits, before);
    assert!(
        state.characters[&ruler]
            .trait_expiry
            .keys()
            .any(|t| t.as_str() == "trait_debtor"),
        "the debt trait is temporary"
    );
}

// ----- top 4: loyalty ----------------------------------------------------------

/// A non-ruler adult of `faction` put at the head of one of its armies.
fn general_of(state: &mut sim_campaign::CampaignState, faction: &str) -> CharacterId {
    let f = data_model::FactionId::new(faction).unwrap();
    let ruler = state.factions[&f].ruler.clone();
    let year = state.year;
    let id = state
        .characters
        .iter()
        .find(|(id, c)| {
            c.alive
                && c.faction == f
                && !c.captive
                && c.army.is_none()
                && c.is_major(year)
                && Some(*id) != ruler.as_ref()
        })
        .map(|(id, _)| id.clone())
        .expect("a candidate");
    let army = state
        .armies
        .iter()
        .find(|(_, a)| a.faction == f)
        .map(|(a, _)| a.clone())
        .unwrap();
    state.armies.get_mut(&army).unwrap().general = Some(id.clone());
    let c = state.characters.get_mut(&id).unwrap();
    c.army = Some(army);
    c.governor_of = None;
    id
}

#[test]
fn loyalty_drifts_to_a_target_that_traits_and_unrest_move() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 1);
    let general = general_of(&mut state, "fac_france");
    {
        let c = state.characters.get_mut(&general).unwrap();
        c.traits.clear();
        c.skills_learned.clear();
        c.last_battle_won = true;
    }
    let rules = data_model::LoyaltyRules::bundled();
    let plain = sim_campaign::loyalty::loyalty_target(&state, data, &general).unwrap();
    assert_eq!(plain, rules.base_target);
    state
        .characters
        .get_mut(&general)
        .unwrap()
        .traits
        .insert(data_model::TraitId::new("trait_loyal").unwrap());
    let loyal = sim_campaign::loyalty::loyalty_target(&state, data, &general).unwrap();
    assert!(loyal > plain, "a loyal man stays loyal longer");
    // A rout lowers it; the drift moves by `drift` per season only.
    {
        let c = state.characters.get_mut(&general).unwrap();
        c.last_battle_won = false;
        c.last_battle_turn = Some(state.turn);
        c.loyalty = 100;
    }
    let lowered = sim_campaign::loyalty::loyalty_target(&state, data, &general).unwrap();
    assert!(lowered < loyal);
    let mut events = Vec::new();
    sim_campaign::loyalty::resolve_loyalty(&mut state, data, &mut events);
    assert_eq!(
        i32::from(state.characters[&general].loyalty),
        100 - rules.drift
    );
}

#[test]
fn a_disloyal_general_eventually_takes_his_army_to_the_enemy() {
    let data = game_data();
    let mut state = start_quiet(data, "fac_france", 1);
    let france = data_model::FactionId::new("fac_france").unwrap();
    let england = data_model::FactionId::new("fac_england").unwrap();
    state.factions.get_mut(&france).unwrap().at_war_with.insert(england.clone());
    state.factions.get_mut(&england).unwrap().at_war_with.insert(france.clone());
    let general = general_of(&mut state, "fac_france");
    let army = state.characters[&general].army.clone().unwrap();
    if let sim_campaign::state::ArmyPosition::Settlement(s) = state.armies[&army].position.clone() {
        state.settlements.get_mut(&s).unwrap().siege = None;
    }
    let mut events = Vec::new();
    let mut defected_at = None;
    for turn in 0..80 {
        state.turn += 1;
        {
            let c = state.characters.get_mut(&general).unwrap();
            if c.faction != france {
                defected_at = Some(turn);
                break;
            }
            c.loyalty = 5;
        }
        sim_campaign::loyalty::resolve_loyalty(&mut state, data, &mut events);
    }
    assert!(defected_at.is_some(), "a loyalty of 5 must end in defection");
    assert_ne!(state.armies[&army].faction, france);
    assert_eq!(state.armies[&army].faction, state.characters[&general].faction);
    assert_eq!(
        i32::from(state.characters[&general].loyalty),
        i32::from(data_model::LoyaltyRules::bundled().recover_to)
    );
}
