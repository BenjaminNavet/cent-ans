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
