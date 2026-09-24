//! Experience, the skill tree and trait/skill effects (spec § 2).
//!
//! Trait and skill *definitions* (effects, prerequisites, branch, tier) live
//! in `data/traits/*.json` and `data/skills/*.json`, loaded into
//! `GameData::{traits, skills}`. This module only holds the rules that read
//! them: XP accrual, `learn_skill`, and [`character_effects`], which
//! aggregates a character's traits and learned skills into an
//! [`EffectTotals`] the rest of the crate already knows how to read
//! (`battle_auto` for the general, `province_effects` for the governor,
//! `siege` for `SiegeSpeed`, `movement` for `Movement`).

use std::collections::BTreeSet;

use data_model::{CharacterId, GameData, SkillId, TraitId};

use crate::buildings::EffectTotals;
use crate::state::CampaignState;

/// XP granted to a general per battle (spec § 2): +10, +20 if victorious.
pub const BATTLE_XP: u32 = 10;
pub const BATTLE_VICTORY_XP: u32 = 20;
/// XP granted to a governor per turn of governance (spec § 2).
pub const GOVERNANCE_XP_PER_TURN: u32 = 2;

/// XP needed for the next skill point, `100 × current tier` (spec § 2).
/// The "current tier" is the skill-tree tier the character is working
/// through: 1 until three skills are learned, 2 until six, then 3. (Using the
/// 0-10 branch levels instead would make historical rulers, often rated 7-9,
/// need eight victories per point.)
pub fn xp_for_next_point(skills_learned: usize) -> u32 {
    100 * (skills_learned / 3 + 1).min(3) as u32
}

/// Grants `amount` XP to `character` and converts any XP threshold crossed
/// into skill points (spec § 2: "un point de compétence par 100 XP × tier
/// courant").
pub fn grant_experience(state: &mut CampaignState, character: &CharacterId, amount: u32) {
    let Some(c) = state.characters.get_mut(character) else {
        return;
    };
    c.experience += amount;
    let threshold = xp_for_next_point(c.skills_learned.len());
    while c.experience >= threshold {
        c.experience -= threshold;
        c.skill_points += 1;
    }
}

/// Why `learn_skill` was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LearnSkillError {
    #[error("personnage inconnu ou mort")]
    UnknownCharacter,
    #[error("compétence inconnue : {0}")]
    UnknownSkill(SkillId),
    #[error("compétence déjà apprise")]
    AlreadyLearned,
    #[error("prérequis manquant : {0}")]
    MissingPrerequisite(SkillId),
    #[error("points de compétence insuffisants")]
    InsufficientPoints,
}

/// Learnable skills of `character` (spec § 2 `get_learnable`): known,
/// unlearned skills whose prerequisites are already learned.
pub fn learnable_skills(state: &CampaignState, data: &GameData, id: &CharacterId) -> Vec<SkillId> {
    let Some(character) = state.characters.get(id) else {
        return Vec::new();
    };
    if !character.alive {
        return Vec::new();
    }
    data.skills
        .values()
        .filter(|skill| {
            !character.skills_learned.contains(&skill.id)
                && skill
                    .prerequisites
                    .iter()
                    .all(|p| character.skills_learned.contains(p))
        })
        .map(|skill| skill.id.clone())
        .collect()
}

/// Validates and applies `learn_skill { character, skill }` (spec § 2:
/// prerequisites, points, branch — the branch is implicit in the skill's own
/// definition, prerequisites are always within the same branch).
pub fn learn_skill(
    state: &mut CampaignState,
    data: &GameData,
    id: &CharacterId,
    skill_id: &SkillId,
) -> Result<(), LearnSkillError> {
    let skill = data
        .skills
        .get(skill_id)
        .ok_or_else(|| LearnSkillError::UnknownSkill(skill_id.clone()))?;
    let character = state
        .characters
        .get(id)
        .filter(|c| c.alive)
        .ok_or(LearnSkillError::UnknownCharacter)?;
    if character.skills_learned.contains(&skill.id) {
        return Err(LearnSkillError::AlreadyLearned);
    }
    for prereq in &skill.prerequisites {
        if !character.skills_learned.contains(prereq) {
            return Err(LearnSkillError::MissingPrerequisite(prereq.clone()));
        }
    }
    if character.skill_points < skill.cost {
        return Err(LearnSkillError::InsufficientPoints);
    }
    let character = state.characters.get_mut(id).expect("checked above");
    character.skill_points -= skill.cost;
    character.skills_learned.insert(skill.id.clone());
    // Each learned skill raises its branch level by one (0-10 scale).
    let level = match skill.branch {
        data_model::SkillBranch::Command => &mut character.skills.command,
        data_model::SkillBranch::Governance => &mut character.skills.governance,
        data_model::SkillBranch::Court => &mut character.skills.court,
    };
    *level = (*level + 1).min(10);
    Ok(())
}

/// Aggregated effects of `id`'s traits and learned skills (spec § 2), used by
/// `battle_auto` (general), `province_effects` (governor), `siege`
/// (`SiegeSpeed`) and `movement` (`Movement`).
pub fn character_effects(state: &CampaignState, data: &GameData, id: &CharacterId) -> EffectTotals {
    let mut totals = EffectTotals::default();
    let Some(character) = state.characters.get(id) else {
        return totals;
    };
    for trait_id in &character.traits {
        if let Some(def) = data.traits.get(trait_id) {
            for effect in &def.effects {
                totals.add_effect(effect);
            }
        }
    }
    for skill_id in &character.skills_learned {
        if let Some(def) = data.skills.get(skill_id) {
            for effect in &def.effects {
                totals.add_effect(effect);
            }
        }
    }
    // H6: companions of a chivalric order lead with more fire.
    totals.army_morale.flat += crate::chivalry::member_morale(state, data, id);
    totals
}

/// `true` when `traits` may still gain `candidate` (no opposite already held,
/// spec § 2 "opposés s'excluent").
pub fn can_acquire(data: &GameData, traits: &BTreeSet<TraitId>, candidate: &TraitId) -> bool {
    if traits.contains(candidate) {
        return false;
    }
    let Some(def) = data.traits.get(candidate) else {
        return true;
    };
    if def.opposites.iter().any(|o| traits.contains(o)) {
        return false;
    }
    traits.iter().all(|held| {
        data.traits
            .get(held)
            .is_none_or(|held_def| !held_def.opposites.contains(candidate))
    })
}

/// Grants `trait_id` to `character` unless an opposite trait is already held.
/// Returns `true` when the trait was added.
pub fn grant_trait(
    state: &mut CampaignState,
    data: &GameData,
    character: &CharacterId,
    trait_id: &TraitId,
) -> bool {
    let Some(c) = state.characters.get_mut(character) else {
        return false;
    };
    if !can_acquire(data, &c.traits, trait_id) {
        return false;
    }
    c.traits.insert(trait_id.clone());
    true
}
