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
    state
        .factions
        .get_mut(&france)
        .unwrap()
        .at_war_with
        .insert(england.clone());
    state
        .factions
        .get_mut(&england)
        .unwrap()
        .at_war_with
        .insert(france.clone());
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
    assert!(
        defected_at.is_some(),
        "a loyalty of 5 must end in defection"
    );
    assert_ne!(state.armies[&army].faction, france);
    assert_eq!(
        state.armies[&army].faction,
        state.characters[&general].faction
    );
    assert_eq!(
        i32::from(state.characters[&general].loyalty),
        i32::from(data_model::LoyaltyRules::bundled().recover_to)
    );
}

// ----- top 5 and top 10: agents against characters and armies --------------------

mod agent_actions {
    use data_model::test_support::{fac, game_data};
    use data_model::{
        ActionEffect, AgentActionKind, AgentKind, BuildingCategory, CharacterId, FactionId,
        GameData, SettlementId, SettlementKind,
    };
    use sim_campaign::agents::{self, AgentId};
    use sim_campaign::test_support::start_quiet;
    use sim_campaign::CampaignState;

    /// Rules where every action succeeds, nobody dies, and `kill` / `wound`
    /// percentages of strikes are replaced.
    fn tuned(kill: u32, wound: u32) -> GameData {
        let mut data = game_data().clone();
        let mut rules = agents::rules(&data).clone();
        for action in rules.actions.values_mut() {
            action.base_chance = 100;
            action.death_risk = 0;
            for effect in &mut action.success {
                if let ActionEffect::Strike {
                    kill_percent,
                    kill_per_level,
                    wound_percent,
                    ..
                } = effect
                {
                    *kill_percent = kill;
                    *kill_per_level = 0;
                    *wound_percent = wound;
                }
            }
        }
        rules.max_chance = 100;
        rules.min_chance = 100;
        data.agent_rules = Some(rules);
        data
    }

    fn war(state: &mut CampaignState, a: &FactionId, b: &FactionId) {
        for (x, y) in [(a, b), (b, a)] {
            let f = state.factions.get_mut(x).unwrap();
            f.at_war_with.insert(y.clone());
            f.allies.remove(y);
        }
    }

    fn place(
        state: &mut CampaignState,
        data: &GameData,
        faction: &FactionId,
        kind: AgentKind,
        at: &SettlementId,
    ) -> AgentId {
        state.factions.get_mut(faction).unwrap().treasury += 10_000;
        let religious_city = |s: &sim_campaign::SettlementState| {
            s.kind == SettlementKind::City
                && s.buildings.iter().any(|b| {
                    data.buildings
                        .get(b)
                        .is_some_and(|d| d.category == BuildingCategory::Religious)
                })
        };
        let own = state
            .settlements
            .iter()
            .filter(|(_, s)| &s.controller == faction)
            .find(|(_, s)| religious_city(s))
            .or_else(|| {
                state
                    .settlements
                    .iter()
                    .find(|(_, s)| &s.controller == faction && s.kind == SettlementKind::Abbey)
            })
            .map(|(id, _)| id.clone())
            .expect("a place to recruit");
        let id = state.recruit_agent(data, faction, &own, kind).unwrap();
        let agent = state.agents.agents.get_mut(&id).unwrap();
        agent.location = at.clone();
        agent.movement_points = 500;
        id
    }

    /// A non-ruler English adult made governor of the province of an English
    /// city.
    fn english_governor(state: &mut CampaignState) -> (CharacterId, SettlementId) {
        let england = fac("fac_england");
        let ruler = state.factions[&england].ruler.clone();
        let year = state.year;
        let city = state
            .settlements
            .iter()
            .find(|(_, s)| s.controller == england && s.kind == SettlementKind::City)
            .map(|(id, _)| id.clone())
            .unwrap();
        let province = state.settlement_province(&city).unwrap().clone();
        let who = state
            .characters
            .iter()
            .find(|(id, c)| {
                c.alive
                    && c.faction == england
                    && !c.captive
                    && c.army.is_none()
                    && c.is_major(year)
                    && Some(*id) != ruler.as_ref()
            })
            .map(|(id, _)| id.clone())
            .unwrap();
        state.characters.get_mut(&who).unwrap().governor_of = Some(province);
        (who, city)
    }

    #[test]
    fn an_assassin_kills_a_governor_and_the_victims_court_resents_it() {
        let data = tuned(100, 100);
        let mut state = start_quiet(&data, "fac_france", 3);
        let (france, england) = (fac("fac_france"), fac("fac_england"));
        war(&mut state, &france, &england);
        let (victim, city) = english_governor(&mut state);
        let spy = place(&mut state, &data, &france, AgentKind::Spy, &city);
        let report = state
            .agent_act(
                &data,
                &france,
                &spy,
                AgentActionKind::Assassinate,
                None,
                None,
            )
            .expect("a governor may be struck");
        assert!(report.success);
        assert!(!state.characters[&victim].alive, "{}", report.text_fr);
        let scandal = state.factions[&england]
            .modifiers
            .iter()
            .any(|m| m.with == france && m.value < 0 && m.reason_fr.contains("Attentat"));
        assert!(scandal, "the court of the dead governor resents it");
    }

    #[test]
    fn poison_wounds_in_secret_and_the_sovereign_is_out_of_reach_at_seal_one() {
        let data = tuned(0, 100);
        let mut state = start_quiet(&data, "fac_france", 3);
        let (france, england) = (fac("fac_france"), fac("fac_england"));
        war(&mut state, &france, &england);
        let (victim, city) = english_governor(&mut state);
        let spy = place(&mut state, &data, &france, AgentKind::Spy, &city);
        state
            .agent_act(&data, &france, &spy, AgentActionKind::Poison, None, None)
            .expect("poison");
        assert!(state.characters[&victim].alive);
        assert!(state.characters[&victim]
            .traits
            .iter()
            .any(|t| t.as_str() == "trait_wounded"));
        // Only the sovereign governs there: refused for a seal-1 spy.
        let ruler = state.factions[&england].ruler.clone().unwrap();
        state.characters.get_mut(&victim).unwrap().governor_of = None;
        state.characters.get_mut(&ruler).unwrap().governor_of =
            state.settlement_province(&city).cloned();
        let spy2 = place(&mut state, &data, &france, AgentKind::Spy, &city);
        let refused = state.agent_act(
            &data,
            &france,
            &spy2,
            AgentActionKind::Assassinate,
            None,
            None,
        );
        assert!(refused.is_err(), "the sovereign needs seal 4");
        state.agents.agents.get_mut(&spy2).unwrap().level = 4;
        state
            .agent_act(
                &data,
                &france,
                &spy2,
                AgentActionKind::Assassinate,
                None,
                None,
            )
            .expect("a seal-4 spy may strike the sovereign");
        assert!(
            state.characters[&ruler]
                .traits
                .iter()
                .any(|t| t.as_str() == "trait_wounded"),
            "the sovereign is wounded (kill chance tuned to 0)"
        );
    }

    #[test]
    fn a_herald_guides_a_friendly_army_and_a_spy_ambushes_an_enemy_one() {
        let data = tuned(0, 0);
        let mut state = start_quiet(&data, "fac_france", 3);
        let (france, england) = (fac("fac_france"), fac("fac_england"));
        war(&mut state, &france, &england);
        // A French army standing in a settlement.
        let (army_id, here) = state
            .armies
            .iter()
            .find_map(|(id, a)| {
                (a.faction == france)
                    .then(|| a.settlement().cloned().map(|s| (id.clone(), s)))
                    .flatten()
            })
            .expect("a stationed French army");
        let herald = place(&mut state, &data, &france, AgentKind::Emissary, &here);
        let before = state.armies[&army_id].movement_left;
        state
            .agent_act(
                &data,
                &france,
                &herald,
                AgentActionKind::GuideArmy,
                None,
                None,
            )
            .expect("guide");
        assert!(state.armies[&army_id].movement_left > before);

        // An English army near its own settlement, ambushed by a French spy.
        let (enemy_army, there) = state
            .armies
            .iter()
            .find_map(|(id, a)| {
                (a.faction == england)
                    .then(|| a.settlement().cloned().map(|s| (id.clone(), s)))
                    .flatten()
            })
            .expect("a stationed English army");
        state.armies.get_mut(&enemy_army).unwrap().movement_left = 400;
        let spy = place(&mut state, &data, &france, AgentKind::Spy, &there);
        state
            .agent_act(&data, &france, &spy, AgentActionKind::Ambush, None, None)
            .expect("ambush");
        let army = &state.armies[&enemy_army];
        assert!(army.movement_left < 400, "the march is delayed");
        assert!(army.morale_modifiers.iter().any(|m| m.value < 0));
    }
}
