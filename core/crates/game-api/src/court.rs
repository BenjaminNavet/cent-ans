//! Court views (UX lot "court"): character sheet with skill tree and family tree, court list,
//! council of the realm, royal acts, captains, chivalric orders.
//!
//! Replaces `get_character`, `get_faction_characters`, `get_skill_tree`, `get_learnable`,
//! `get_marriage_candidates`, `get_family_tree`, `get_retinue_*`, `get_council*`,
//! `get_royal_acts`, `get_captain_info` and `get_chivalric_orders` of `godot-bridge`. Every
//! action is an order submitted by the client (`submit_order`, `Order::LearnSkill`,
//! `AssignGovernor`, `ProposeMarriage`, `TransferCompanion`, `AppointCouncillor`,
//! `DismissCouncillor`, `RoyalAct`, `HireCaptain`, `FoundChivalricOrder`, `PayRansom`...).

use std::collections::{BTreeMap, BTreeSet};

use data_model::{CharacterId, Effect, FactionId, GameData, Sex, Skill};
use serde::Serialize;
use sim_campaign::dynasty::CharacterView;
use sim_campaign::{chivalry, CampaignState, Order};

pub use crate::diplomacy::{submit_order, OrderResult};
pub use crate::tech::EffectView;

/// Hard cap on the number of nodes of a family tree.
const MAX_NODES: usize = 120;

fn key_of<T: Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn effect_views(effects: &[Effect]) -> Vec<EffectView> {
    effects
        .iter()
        .map(|effect| EffectView {
            kind: key_of(&effect.effect),
            value: effect.value,
            mode: key_of(&effect.mode),
            unit_category: effect.unit_category.map(|c| key_of(&c)).unwrap_or_default(),
            class: effect.class.map_or("", |c| c.key()).to_owned(),
        })
        .collect()
}

fn opt_id(id: &Option<CharacterId>) -> String {
    id.as_ref().map_or(String::new(), ToString::to_string)
}

fn sex_key(sex: Sex) -> &'static str {
    match sex {
        Sex::Male => "male",
        Sex::Female => "female",
    }
}

/// French label of a role (sovereign, heir, ...), as in `campaign_sim.rs::role_label_fr`.
fn role_label(role: data_model::Role) -> &'static str {
    use data_model::Role;
    match role {
        Role::Ruler => "Souverain(e)",
        Role::Heir => "Héritier(e)",
        Role::Consort => "Époux(se)",
        Role::Noble => "Noble",
        Role::Prelate => "Prélat",
        Role::Burgher => "Bourgeois(e)",
        Role::Exile => "Exilé(e)",
        Role::Claimant => "Prétendant(e)",
        Role::Regent => "Régent(e)",
        _ => "Membre de la maison",
    }
}

// ---------------------------------------------------------------- character sheet

/// A trait pill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TraitRow {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    /// Turns before a temporary trait (a wound) wears off; 0 = permanent.
    pub turns_left: u32,
}

/// `{id, name}` of a person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PersonRef {
    pub id: String,
    pub name: String,
    pub age: i32,
}

/// A companion of a general's retinue.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CompanionView {
    pub id: String,
    pub name: String,
    pub category: String,
    pub glyph: String,
    pub description: String,
    pub inheritable: bool,
    pub effects: Vec<EffectView>,
}

/// Counters of the « Faits d'armes » block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Feats {
    pub battles_fought: u32,
    pub battles_won: u32,
    pub sieges_won: u32,
    pub raids_led: u32,
    /// Turns since the last battle, `None` if never.
    pub last_battle_turns_ago: Option<u32>,
    pub last_battle_won: bool,
}

/// Command / governance / court skill values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SkillValues {
    pub command: u8,
    pub governance: u8,
    pub court: u8,
}

/// Everything the character sheet shows, plus what the player may do.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CharacterSheet {
    pub id: String,
    pub name: String,
    pub epithet: String,
    /// `male` or `female`.
    pub sex: String,
    pub age: i32,
    pub alive: bool,
    pub faction: String,
    pub faction_name: String,
    pub house: String,
    /// Title or role label (« Souverain(e) », « Membre de la maison »...).
    pub title: String,
    /// « à la cour », « gouverneur de X », « général de l'armée en X », « Captif(ve) ».
    pub activity: String,
    pub skills: SkillValues,
    pub experience: u32,
    pub skill_points: u32,
    pub xp_to_next: u32,
    pub level: u32,
    pub learned: Vec<String>,
    pub traits: Vec<TraitRow>,
    pub spouse: Option<PersonRef>,
    pub children: Vec<PersonRef>,
    pub father: Option<PersonRef>,
    pub mother: Option<PersonRef>,
    pub location: String,
    pub army: String,
    pub governor_of: String,
    pub captive: bool,
    pub captor_name: String,
    pub ransom: i64,
    /// `release` (we hold him), `pay` (one of ours is held) or empty.
    pub ransom_action: String,
    pub piety: u8,
    pub prestige: i32,
    /// Loyalty of a general or governor, `None` without post.
    pub loyalty: Option<u8>,
    pub birth_year: i32,
    pub death_year: Option<i32>,
    pub retinue: Vec<CompanionView>,
    pub retinue_max: usize,
    pub captain: bool,
    pub feats: Feats,
    /// The character belongs to the player's faction.
    pub own: bool,
    /// The player may give orders to the character (own, alive, free).
    pub can_act: bool,
    /// Skills the character can learn now (prerequisites met, enough points).
    pub learnable: Vec<String>,
    /// Valid spouses (own sheet only).
    pub marriage_candidates: Vec<PersonRef>,
    /// Generals the retinue may be handed to.
    pub transfer_targets: Vec<PersonRef>,
    /// Provinces of the faction a governor may be named to (`Order::AssignGovernor`).
    pub governor_options: Vec<TargetRef>,
    /// Armies of the faction a general may command (`Order::AssignGeneral`).
    pub general_options: Vec<TargetRef>,
    /// The character is neither the sovereign nor the heir (`Order::DesignateHeir` makes sense).
    pub heir_candidate: bool,
}

/// `{id, name}` of a province or an army.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TargetRef {
    pub id: String,
    pub name: String,
}

fn governor_options(data: &GameData, state: &CampaignState, faction: &FactionId) -> Vec<TargetRef> {
    let mut out: Vec<TargetRef> = state
        .provinces
        .keys()
        .filter(|p| state.province_controller(p) == Some(faction))
        .map(|p| TargetRef {
            id: p.as_str().to_owned(),
            name: data.province_name(p),
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

fn general_options(data: &GameData, state: &CampaignState, faction: &FactionId) -> Vec<TargetRef> {
    state
        .armies()
        .iter()
        .filter(|(_, army)| &army.faction == faction)
        .map(|(id, army)| TargetRef {
            id: id.to_string(),
            name: format!(
                "Armée en {}",
                state
                    .army_province(data, army)
                    .map_or_else(String::new, |p| data.province_name(&p))
            ),
        })
        .collect()
}

fn person(state: &CampaignState, data: &GameData, id: &CharacterId) -> PersonRef {
    PersonRef {
        id: id.as_str().to_owned(),
        name: state.character_name(data, id),
        age: state.characters.get(id).map_or(0, |c| c.age(state.year())),
    }
}

fn activity(state: &CampaignState, data: &GameData, view: &CharacterView) -> String {
    if view.captive {
        return "Captif(ve)".to_owned();
    }
    if let Some(province) = &view.governor_of {
        return format!("gouverneur de {}", data.province_name(province));
    }
    if let Some(army) = view.army.as_ref().and_then(|a| state.army(a)) {
        let place = state
            .army_province(data, army)
            .map_or_else(String::new, |p| data.province_name(&p));
        return format!("général de l'armée en {place}");
    }
    "à la cour".to_owned()
}

fn companions(state: &CampaignState, data: &GameData, id: &CharacterId) -> Vec<CompanionView> {
    let (Some(retinue), Some(c)) = (&data.retinue, state.characters.get(id)) else {
        return Vec::new();
    };
    c.retinue
        .iter()
        .filter_map(|companion| retinue.companion(companion))
        .map(|c| CompanionView {
            id: c.id.as_str().to_owned(),
            name: c.name.display.clone(),
            category: c.category.key().to_owned(),
            glyph: c.glyph.clone(),
            description: c.description.clone(),
            inheritable: c.inheritable,
            effects: effect_views(&c.effects),
        })
        .collect()
}

/// Sheet of character `id`, `None` for an unknown id.
pub fn character_sheet(
    data: &GameData,
    state: &CampaignState,
    id: &CharacterId,
) -> Option<CharacterSheet> {
    let view = state.character_view(data, id)?;
    let c = state.characters.get(id)?;
    let player = state.player_faction();
    let own = &view.faction == player;
    let captor = c.captive.then(|| c.captor.clone()).flatten();
    let ransom = if captor.is_some() {
        sim_campaign::ransom::ransom_amount(state, data, id)
    } else {
        0
    };
    let ransom_action = match &captor {
        Some(captor) if captor == player => "release",
        Some(_) if own => "pay",
        _ => "",
    };
    let learned = view.skills_learned.len();
    let loyalty = sim_campaign::loyalty::loyalty_target(state, data, id).map(|_| c.loyalty);
    let learnable: Vec<String> = if own && view.alive {
        state
            .learnable_skills(data, id)
            .iter()
            .filter(|skill| {
                data.skills
                    .get(*skill)
                    .is_some_and(|s| s.cost <= c.skill_points)
            })
            .map(|s| s.as_str().to_owned())
            .collect()
    } else {
        Vec::new()
    };
    let marriage_candidates = if own && view.alive {
        state
            .marriage_candidates(data, id)
            .iter()
            .map(|candidate| person(state, data, candidate))
            .collect()
    } else {
        Vec::new()
    };
    let transfer_targets = if own && view.alive {
        sim_campaign::retinue::transfer_targets(state, data, id)
            .iter()
            .map(|target| person(state, data, target))
            .collect()
    } else {
        Vec::new()
    };
    Some(CharacterSheet {
        id: id.as_str().to_owned(),
        name: view.name.clone(),
        epithet: view.epithet.clone().unwrap_or_default(),
        sex: sex_key(view.sex).to_owned(),
        age: view.age,
        alive: view.alive,
        faction: view.faction.as_str().to_owned(),
        faction_name: data.faction_label(&view.faction),
        house: view.house.clone(),
        title: view
            .title
            .clone()
            .or_else(|| view.role.map(|r| role_label(r).to_owned()))
            .unwrap_or_else(|| "Membre de la maison".to_owned()),
        activity: activity(state, data, &view),
        skills: SkillValues {
            command: view.skills.command,
            governance: view.skills.governance,
            court: view.skills.court,
        },
        experience: view.experience,
        skill_points: view.skill_points,
        xp_to_next: sim_campaign::skills::xp_for_next_point(learned),
        level: sim_campaign::skills::level_of(c),
        learned: view
            .skills_learned
            .iter()
            .map(|s| s.as_str().to_owned())
            .collect(),
        traits: view
            .traits
            .iter()
            .map(|t| TraitRow {
                id: t.id.as_str().to_owned(),
                name: t.name.clone(),
                category: t.category.clone(),
                description: data
                    .traits
                    .get(&t.id)
                    .map_or(String::new(), |d| d.description.clone()),
                turns_left: c.trait_expiry.get(&t.id).copied().unwrap_or(0),
            })
            .collect(),
        spouse: view.spouse.as_ref().map(|s| person(state, data, s)),
        children: view
            .children
            .iter()
            .map(|child| PersonRef {
                id: child.id.as_str().to_owned(),
                name: child.name.clone(),
                age: child.age,
            })
            .collect(),
        father: view
            .father
            .as_ref()
            .filter(|f| state.characters.contains_key(*f))
            .map(|f| person(state, data, f)),
        mother: view
            .mother
            .as_ref()
            .filter(|m| state.characters.contains_key(*m))
            .map(|m| person(state, data, m)),
        location: view
            .location
            .as_ref()
            .map_or(String::new(), |p| p.as_str().to_owned()),
        army: opt_army(&view),
        governor_of: view
            .governor_of
            .as_ref()
            .map_or(String::new(), |p| p.as_str().to_owned()),
        captive: view.captive,
        captor_name: captor
            .as_ref()
            .map_or(String::new(), |f| data.faction_label(f)),
        ransom,
        ransom_action: ransom_action.to_owned(),
        piety: view.piety,
        prestige: view.prestige,
        loyalty,
        birth_year: c.birth_year,
        death_year: c.death_year,
        retinue: companions(state, data, id),
        retinue_max: sim_campaign::retinue::max_per_character(data),
        captain: c.captain,
        feats: Feats {
            battles_fought: c.battles_fought,
            battles_won: c.battles_won,
            sieges_won: c.sieges_won,
            raids_led: c.raids_led,
            last_battle_turns_ago: c.last_battle_turn.map(|t| state.turn.saturating_sub(t)),
            last_battle_won: c.last_battle_won,
        },
        own,
        can_act: own && view.alive && !view.captive,
        learnable,
        marriage_candidates,
        transfer_targets,
        governor_options: if own && view.alive {
            governor_options(data, state, &view.faction)
        } else {
            Vec::new()
        },
        general_options: if own && view.alive {
            general_options(data, state, &view.faction)
        } else {
            Vec::new()
        },
        heir_candidate: own
            && view.alive
            && state
                .factions
                .get(&view.faction)
                .is_some_and(|f| f.ruler.as_ref() != Some(id) && f.heir.as_ref() != Some(id)),
    })
}

fn opt_army(view: &CharacterView) -> String {
    view.army
        .as_ref()
        .map_or(String::new(), ToString::to_string)
}

// ---------------------------------------------------------------- skill tree

/// State of a skill for one character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillState {
    Learned,
    /// Prerequisites met and enough points: can be learned now.
    Available,
    /// Prerequisites met but not enough points.
    NeedsPoints,
    Locked,
}

/// One node of the skill tree.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SkillNode {
    pub id: String,
    pub name: String,
    /// `command`, `governance` or `court`.
    pub branch: String,
    pub tier: u32,
    pub cost: u32,
    pub description: String,
    pub prerequisites: Vec<String>,
    pub effects: Vec<EffectView>,
    pub state: SkillState,
}

/// French label of a branch.
pub fn branch_label(branch: &str) -> &'static str {
    match branch {
        "command" => "Commandement",
        "governance" => "Gouvernance",
        "court" => "Cour",
        _ => "Autre",
    }
}

fn skill_description(skill: &Skill) -> String {
    if skill.requires_role.is_empty() {
        return skill.description.clone();
    }
    let roles: Vec<&str> = skill
        .requires_role
        .iter()
        .map(|r| match r {
            data_model::SkillRole::Ruler => "souverain",
            data_model::SkillRole::General => "général",
            data_model::SkillRole::Governor => "gouverneur",
            data_model::SkillRole::Consort => "épouse du souverain",
        })
        .collect();
    format!("{} Réservé : {}.", skill.description, roles.join(", "))
}

/// The whole tree (sorted by branch, tier, id) with the state of each skill for `character`.
pub fn skill_tree(
    data: &GameData,
    state: &CampaignState,
    character: &CharacterId,
) -> Vec<SkillNode> {
    let learned: BTreeSet<String> = state
        .characters
        .get(character)
        .map(|c| {
            c.skills_learned
                .iter()
                .map(|s| s.as_str().to_owned())
                .collect()
        })
        .unwrap_or_default();
    let points = state
        .characters
        .get(character)
        .map_or(0, |c| c.skill_points);
    let learnable: BTreeSet<String> = state
        .learnable_skills(data, character)
        .iter()
        .map(|s| s.as_str().to_owned())
        .collect();
    let mut skills: Vec<&Skill> = data.skills.values().collect();
    skills.sort_by_key(|s| (s.branch.key(), s.tier, s.id.clone()));
    skills
        .into_iter()
        .map(|skill| {
            let id = skill.id.as_str().to_owned();
            let node_state = if learned.contains(&id) {
                SkillState::Learned
            } else if learnable.contains(&id) {
                if skill.cost <= points {
                    SkillState::Available
                } else {
                    SkillState::NeedsPoints
                }
            } else {
                SkillState::Locked
            };
            SkillNode {
                id,
                name: skill.name.display.clone(),
                branch: skill.branch.key().to_owned(),
                tier: u32::from(skill.tier),
                cost: skill.cost,
                description: skill_description(skill),
                prerequisites: skill
                    .prerequisites
                    .iter()
                    .map(|p| p.as_str().to_owned())
                    .collect(),
                effects: effect_views(&skill.effects),
                state: node_state,
            }
        })
        .collect()
}

// ---------------------------------------------------------------- family tree

/// One person of a family tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FamilyNode {
    pub id: String,
    pub name: String,
    pub epithet: String,
    pub sex: String,
    pub alive: bool,
    pub birth_year: i32,
    pub death_year: Option<i32>,
    pub age: i32,
    pub house: String,
    pub faction: String,
    pub title: String,
    /// Relative to the root: negative above, positive below.
    pub generation: i32,
    /// False for a spouse joined by marriage.
    pub blood: bool,
    pub father: String,
    pub mother: String,
    pub spouse: String,
    pub children: Vec<String>,
}

/// Family tree around a root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FamilyTree {
    pub root: String,
    pub ruler: String,
    pub heir: String,
    pub nodes: Vec<FamilyNode>,
}

/// Ancestors up to `up` generations, the root's siblings, descendants of the root and of its
/// siblings down to `down` generations, and the spouse of every blood relative.
pub fn family_tree(
    data: &GameData,
    state: &CampaignState,
    root: &CharacterId,
    up: i32,
    down: i32,
) -> Option<FamilyTree> {
    let root_state = state.characters.get(root)?;
    let generations = family_generations(state, data, root, up.max(0), down.max(0));
    let faction = state.factions.get(&root_state.faction);
    let nodes = generations
        .iter()
        .filter_map(|(id, (generation, blood))| {
            let c = state.characters.get(id)?;
            Some(FamilyNode {
                id: id.as_str().to_owned(),
                name: state.character_name(data, id),
                epithet: data
                    .characters
                    .get(id)
                    .and_then(|d| d.epithet.clone())
                    .unwrap_or_default(),
                sex: sex_key(c.sex).to_owned(),
                alive: c.alive,
                birth_year: c.birth_year,
                death_year: c.death_year,
                age: c.age(state.year()),
                house: c.house.clone(),
                faction: c.faction.as_str().to_owned(),
                title: c.title.clone().unwrap_or_default(),
                generation: *generation,
                blood: *blood,
                father: opt_id(&c.father),
                mother: opt_id(&c.mother),
                spouse: opt_id(&c.spouse),
                children: c
                    .children
                    .iter()
                    .filter(|child| state.characters.contains_key(*child))
                    .map(|child| child.as_str().to_owned())
                    .collect(),
            })
        })
        .collect();
    Some(FamilyTree {
        root: root.as_str().to_owned(),
        ruler: opt_id(&faction.and_then(|f| f.ruler.clone())),
        heir: opt_id(&faction.and_then(|f| f.heir.clone())),
        nodes,
    })
}

fn family_generations(
    state: &CampaignState,
    data: &GameData,
    root: &CharacterId,
    up: i32,
    down: i32,
) -> BTreeMap<CharacterId, (i32, bool)> {
    let mut out: BTreeMap<CharacterId, (i32, bool)> = BTreeMap::new();
    let known = |id: &CharacterId| state.characters.contains_key(id);
    out.insert(root.clone(), (0, true));
    let mut frontier = vec![root.clone()];
    for level in 1..=up {
        let mut next = Vec::new();
        for id in &frontier {
            let Some(c) = state.characters.get(id) else {
                continue;
            };
            for parent in [&c.father, &c.mother].into_iter().flatten() {
                if known(parent) && !out.contains_key(parent) && out.len() < MAX_NODES {
                    out.insert(parent.clone(), (-level, true));
                    next.push(parent.clone());
                }
            }
        }
        frontier = next;
    }
    let mut siblings: BTreeSet<CharacterId> = BTreeSet::new();
    if let Some(c) = state.characters.get(root) {
        for parent in [&c.father, &c.mother].into_iter().flatten() {
            if let Some(p) = state.characters.get(parent) {
                siblings.extend(p.children.iter().cloned());
            }
        }
    }
    if let Some(family) = data.characters.get(root).and_then(|c| c.family.as_ref()) {
        siblings.extend(family.siblings.iter().cloned());
    }
    siblings.retain(|s| s != root && known(s));
    let mut frontier = vec![root.clone()];
    for sibling in siblings {
        if out.len() < MAX_NODES && !out.contains_key(&sibling) {
            out.insert(sibling.clone(), (0, true));
            frontier.push(sibling);
        }
    }
    for level in 1..=down {
        let mut next = Vec::new();
        for id in &frontier {
            let Some(c) = state.characters.get(id) else {
                continue;
            };
            for child in &c.children {
                if known(child) && !out.contains_key(child) && out.len() < MAX_NODES {
                    out.insert(child.clone(), (level, true));
                    next.push(child.clone());
                }
            }
        }
        frontier = next;
    }
    let blood: Vec<(CharacterId, i32)> = out.iter().map(|(id, (g, _))| (id.clone(), *g)).collect();
    for (id, generation) in blood {
        let Some(spouse) = state.characters.get(&id).and_then(|c| c.spouse.clone()) else {
            continue;
        };
        if known(&spouse) && !out.contains_key(&spouse) && out.len() < MAX_NODES {
            out.insert(spouse, (generation, false));
        }
    }
    out
}

// ---------------------------------------------------------------- court list

/// One line of the court list (living members of a faction).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CourtRow {
    pub id: String,
    pub name: String,
    pub age: i32,
    pub sex: String,
    pub title: String,
    pub activity: String,
    pub skills: SkillValues,
    pub skill_points: u32,
    pub ruler: bool,
    pub heir: bool,
    /// Council office held, if any.
    pub office: String,
}

/// Living characters of `faction`: ruler, heir, then by age.
pub fn court_rows(data: &GameData, state: &CampaignState, faction: &FactionId) -> Vec<CourtRow> {
    let (ruler, heir) = state
        .factions
        .get(faction)
        .map_or((None, None), |f| (f.ruler.clone(), f.heir.clone()));
    state
        .faction_characters(faction)
        .iter()
        .filter_map(|id| {
            let view = state.character_view(data, id)?;
            let office = sim_campaign::council::office_of(state, faction, id)
                .map(|o| o.to_owned())
                .unwrap_or_default();
            Some(CourtRow {
                id: id.as_str().to_owned(),
                name: view.name.clone(),
                age: view.age,
                sex: sex_key(view.sex).to_owned(),
                title: view
                    .title
                    .clone()
                    .or_else(|| view.role.map(|r| role_label(r).to_owned()))
                    .unwrap_or_else(|| "Membre de la maison".to_owned()),
                activity: activity(state, data, &view),
                skills: SkillValues {
                    command: view.skills.command,
                    governance: view.skills.governance,
                    court: view.skills.court,
                },
                skill_points: view.skill_points,
                ruler: ruler.as_ref() == Some(id),
                heir: heir.as_ref() == Some(id),
                office,
            })
        })
        .collect()
}

// ---------------------------------------------------------------- council

/// One office of the council.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OfficeView {
    pub id: String,
    pub name: String,
    /// Skill read by the office: `command`, `governance`, `court`.
    pub skill: String,
    /// Empty when vacant.
    pub holder_id: String,
    pub holder_name: String,
    pub holder_skill: u8,
    pub effect_text: String,
    pub advice_domains: Vec<String>,
}

/// A candidate for an office.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CandidateView {
    pub id: String,
    pub name: String,
    pub skill_value: u8,
    /// Office the character already holds (cannot be appointed before being dismissed).
    pub current_office: String,
}

/// Offices of the council of `faction`, in file order.
pub fn council(data: &GameData, state: &CampaignState, faction: &FactionId) -> Vec<OfficeView> {
    let Some(offices) = data.council.as_ref() else {
        return Vec::new();
    };
    offices
        .offices
        .iter()
        .map(|def| {
            let holder = sim_campaign::council::holder(state, faction, &def.id);
            let skill = holder.map(|h| sim_campaign::council::skill_of(state, h, def.skill));
            OfficeView {
                id: def.id.clone(),
                name: def.name.display.clone(),
                skill: def.skill.key().to_owned(),
                holder_id: holder.map_or(String::new(), |h| h.as_str().to_owned()),
                holder_name: holder
                    .map(|h| state.character_name(data, h))
                    .unwrap_or_default(),
                holder_skill: skill.unwrap_or(0),
                effect_text: sim_campaign::council::effect_text(def, skill),
                advice_domains: def.advice_kinds.iter().map(ToString::to_string).collect(),
            }
        })
        .collect()
}

/// Characters who may sit in `office`, best skill first.
pub fn council_candidates(
    data: &GameData,
    state: &CampaignState,
    faction: &FactionId,
    office: &str,
) -> Vec<CandidateView> {
    sim_campaign::council::candidates(state, data, faction, office)
        .into_iter()
        .map(|candidate| CandidateView {
            id: candidate.character.as_str().to_owned(),
            name: state.character_name(data, &candidate.character),
            skill_value: candidate.skill,
            current_office: candidate.current_office.unwrap_or_default(),
        })
        .collect()
}

// ---------------------------------------------------------------- royal acts, captains, orders

/// A royal act open to the faction.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RoyalActView {
    pub id: String,
    pub name: String,
    pub description: String,
    pub cost_prestige: i32,
    pub cost_livres: i64,
    pub cooldown_turns: u32,
    pub duration_turns: u32,
    pub gain_prestige: i32,
    pub gain_piety: i32,
    pub effects: Vec<EffectView>,
    pub available: bool,
    /// French refusal when not available.
    pub reason: String,
    pub cooldown_left: u32,
    pub active_left: u32,
    pub sources: Vec<String>,
}

/// Royal acts open to `faction`, in id order.
pub fn royal_acts(
    data: &GameData,
    state: &CampaignState,
    faction: &FactionId,
) -> Vec<RoyalActView> {
    sim_campaign::royal_acts::royal_act_options(state, data, faction)
        .into_iter()
        .filter_map(|option| {
            let act = data.royal_acts.get(&option.id)?;
            Some(RoyalActView {
                id: act.id.as_str().to_owned(),
                name: act.name.display.clone(),
                description: act.description.clone(),
                cost_prestige: act.cost_prestige,
                cost_livres: act.cost_livres,
                cooldown_turns: act.cooldown_turns,
                duration_turns: act.duration_turns,
                gain_prestige: act.gain_prestige,
                gain_piety: act.gain_piety,
                effects: effect_views(&act.effects),
                available: option.available,
                reason: option.reason.unwrap_or_default(),
                cooldown_left: option.cooldown_left,
                active_left: option.active_left,
                sources: act.sources.clone(),
            })
        })
        .collect()
}

/// Captain hiring in the capital's city.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CaptainInfo {
    pub cost: i64,
    pub count: u32,
    pub cap: u32,
    pub cooldown_left: u32,
    pub can_hire: bool,
    pub reason: String,
    /// Settlement id to pass to `Order::HireCaptain`.
    pub settlement: String,
}

/// Captain hiring info for `faction`, `None` without a capital city.
pub fn captain_info(state: &CampaignState, faction: &FactionId) -> Option<CaptainInfo> {
    let capital = state.factions.get(faction)?.capital.clone();
    let city = state.province_city_id(&capital)?.clone();
    let refusal = sim_campaign::captains::hire_refusal(state, faction, &city);
    Some(CaptainInfo {
        cost: sim_campaign::captains::captain_cost(state, faction),
        count: sim_campaign::captains::captain_count(state, faction),
        cap: sim_campaign::captains::captain_cap(state, faction),
        cooldown_left: sim_campaign::captains::cooldown_left(state, faction),
        can_hire: refusal.is_none(),
        reason: refusal.map(|e| e.to_string()).unwrap_or_default(),
        settlement: city.as_str().to_owned(),
    })
}

/// A chivalric order the faction may found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChivalricOption {
    pub id: String,
    pub name: String,
    pub description: String,
    pub cost: i64,
    pub prestige_required: i32,
    pub members: u32,
    pub historical_members: String,
    pub available: bool,
    pub reason: String,
    /// Faction that already founded it, empty if none.
    pub founder: String,
}

/// The order the faction founded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FoundedOrder {
    pub id: String,
    pub name: String,
    pub founded_turn: u32,
    pub collapsed: bool,
    pub members: Vec<PersonRef>,
}

/// Chivalry of the player: founded order and options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Chivalry {
    pub founded: Option<FoundedOrder>,
    pub options: Vec<ChivalricOption>,
}

/// Chivalric orders of `faction`.
pub fn chivalric_orders(data: &GameData, state: &CampaignState, faction: &FactionId) -> Chivalry {
    let founded = state
        .faction_state(faction)
        .and_then(|f| f.chivalric_order.as_ref())
        .map(|o| FoundedOrder {
            id: o.order.as_str().to_owned(),
            name: data
                .chivalric_orders
                .get(&o.order)
                .map_or_else(|| o.order.as_str().to_owned(), |d| d.name.display.clone()),
            founded_turn: o.founded_turn,
            collapsed: o.collapsed,
            members: o.members.iter().map(|m| person(state, data, m)).collect(),
        });
    let options = chivalry::orders_for(data, faction)
        .into_iter()
        .map(|order| {
            let blocker = chivalry::found_blocker(state, data, faction, &order.id);
            ChivalricOption {
                id: order.id.as_str().to_owned(),
                name: order.name.display.clone(),
                description: order.description.clone(),
                cost: chivalry::foundation_cost(state, faction, order),
                prestige_required: order.prestige_required,
                members: order.members,
                historical_members: order.historical_members.clone(),
                available: blocker.is_none(),
                reason: blocker.map(|e| e.to_string()).unwrap_or_default(),
                founder: chivalry::founder_of(state, &order.id)
                    .map_or(String::new(), |f| f.as_str().to_owned()),
            }
        })
        .collect();
    Chivalry { founded, options }
}

// ---------------------------------------------------------------- orders

fn refused(message: &str) -> OrderResult {
    OrderResult {
        ok: false,
        message: message.to_owned(),
    }
}

/// Runs `build` on the parsed ids and submits the order; an invalid id is a refusal.
fn send(
    data: &GameData,
    state: &mut CampaignState,
    build: impl FnOnce() -> Option<Order>,
) -> OrderResult {
    match build() {
        Some(order) => submit_order(data, state, order),
        None => refused("Identifiant inconnu."),
    }
}

/// Spends a skill point of `character` on `skill`.
pub fn learn_skill(
    data: &GameData,
    state: &mut CampaignState,
    character: &str,
    skill: &str,
) -> OrderResult {
    send(data, state, || {
        Some(Order::LearnSkill {
            character: CharacterId::new(character).ok()?,
            skill: data_model::SkillId::new(skill).ok()?,
        })
    })
}

/// Makes `character` governor of `province`.
pub fn assign_governor(
    data: &GameData,
    state: &mut CampaignState,
    character: &str,
    province: &str,
) -> OrderResult {
    send(data, state, || {
        Some(Order::AssignGovernor {
            province: data_model::ProvinceId::new(province).ok()?,
            character: CharacterId::new(character).ok()?,
        })
    })
}

/// Makes `character` general of `army`.
pub fn assign_general(
    data: &GameData,
    state: &mut CampaignState,
    character: &str,
    army: &str,
) -> OrderResult {
    send(data, state, || {
        Some(Order::AssignGeneral {
            army: sim_campaign::ArmyId::parse(army)?,
            character: CharacterId::new(character).ok()?,
        })
    })
}

/// Marries `character` and `spouse`.
pub fn marry(
    data: &GameData,
    state: &mut CampaignState,
    character: &str,
    spouse: &str,
) -> OrderResult {
    send(data, state, || {
        Some(Order::ProposeMarriage {
            character: CharacterId::new(character).ok()?,
            spouse: CharacterId::new(spouse).ok()?,
        })
    })
}

/// Names `heir` successor of the sovereign.
pub fn designate_heir(data: &GameData, state: &mut CampaignState, heir: &str) -> OrderResult {
    send(data, state, || {
        Some(Order::DesignateHeir {
            heir: CharacterId::new(heir).ok()?,
        })
    })
}

/// Hands a companion of `from` to the general `to`.
pub fn transfer_companion(
    data: &GameData,
    state: &mut CampaignState,
    from: &str,
    to: &str,
    companion: &str,
) -> OrderResult {
    send(data, state, || {
        Some(Order::TransferCompanion {
            from: CharacterId::new(from).ok()?,
            to: CharacterId::new(to).ok()?,
            companion: data_model::CompanionId::new(companion).ok()?,
        })
    })
}

/// Appoints `character` to the council `office`.
pub fn appoint_councillor(
    data: &GameData,
    state: &mut CampaignState,
    office: &str,
    character: &str,
) -> OrderResult {
    send(data, state, || {
        Some(Order::AppointCouncillor {
            office: office.to_owned(),
            character: CharacterId::new(character).ok()?,
        })
    })
}

/// Dismisses the holder of the council `office`.
pub fn dismiss_councillor(data: &GameData, state: &mut CampaignState, office: &str) -> OrderResult {
    submit_order(
        data,
        state,
        Order::DismissCouncillor {
            office: office.to_owned(),
        },
    )
}

/// Performs a royal act.
pub fn perform_royal_act(data: &GameData, state: &mut CampaignState, act: &str) -> OrderResult {
    send(data, state, || {
        Some(Order::RoyalAct {
            act: data_model::RoyalActId::new(act).ok()?,
        })
    })
}

/// Hires a captain in the capital's city.
pub fn hire_captain(data: &GameData, state: &mut CampaignState, settlement: &str) -> OrderResult {
    send(data, state, || {
        Some(Order::HireCaptain {
            settlement: data_model::SettlementId::new(settlement).ok()?.into(),
        })
    })
}

/// Founds a chivalric order.
pub fn found_order(data: &GameData, state: &mut CampaignState, order: &str) -> OrderResult {
    send(data, state, || {
        Some(Order::FoundChivalricOrder {
            order: data_model::ChivalricOrderId::new(order).ok()?,
        })
    })
}

/// Pays in full the ransom of one of our captives.
pub fn pay_ransom(data: &GameData, state: &mut CampaignState, character: &str) -> OrderResult {
    send(data, state, || {
        Some(Order::PayRansom {
            character: CharacterId::new(character).ok()?,
            installments: 0,
        })
    })
}

/// Frees a prisoner of ours against his computed ransom.
pub fn release_captive(data: &GameData, state: &mut CampaignState, character: &str) -> OrderResult {
    send(data, state, || {
        Some(Order::ReleaseCaptive {
            character: CharacterId::new(character).ok()?,
            ransom: None,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn start() -> (&'static GameData, CampaignState, FactionId) {
        let data = data_model::test_support::game_data();
        let state = sim_campaign::test_support::start_quiet(data, "fac_france", 1337);
        (data, state, FactionId::new("fac_france").unwrap())
    }

    fn ruler(state: &CampaignState, faction: &FactionId) -> CharacterId {
        state.factions[faction].ruler.clone().expect("ruler")
    }

    #[test]
    fn ruler_sheet_is_complete() {
        let (data, state, faction) = start();
        let id = ruler(&state, &faction);
        let sheet = character_sheet(data, &state, &id).expect("sheet");
        assert_eq!(sheet.id, id.as_str());
        assert!(sheet.own && sheet.alive && sheet.can_act);
        assert_eq!(sheet.faction, "fac_france");
        assert!(!sheet.name.is_empty() && !sheet.title.is_empty());
        assert!(!sheet.governor_options.is_empty(), "France owns provinces");
        assert!(
            !sheet.general_options.is_empty(),
            "France starts with armies"
        );
        assert!(!sheet.heir_candidate, "the ruler is not a heir candidate");
        assert!(serde_json::to_string(&sheet)
            .unwrap()
            .contains("marriage_candidates"));
    }

    #[test]
    fn unknown_character_is_none() {
        let (data, state, _) = start();
        let id = CharacterId::new("chr_nobody").unwrap();
        assert!(character_sheet(data, &state, &id).is_none());
        assert!(family_tree(data, &state, &id, 2, 2).is_none());
    }

    #[test]
    fn foreign_sheet_gives_no_orders() {
        let (data, state, _) = start();
        let england = FactionId::new("fac_england").unwrap();
        let id = ruler(&state, &england);
        let sheet = character_sheet(data, &state, &id).expect("sheet");
        assert!(!sheet.own && !sheet.can_act);
        assert!(sheet.learnable.is_empty() && sheet.marriage_candidates.is_empty());
        assert!(sheet.governor_options.is_empty() && !sheet.heir_candidate);
    }

    #[test]
    fn skill_tree_states_follow_the_sim() {
        let (data, state, faction) = start();
        let id = ruler(&state, &faction);
        let tree = skill_tree(data, &state, &id);
        assert_eq!(tree.len(), data.skills.len());
        let sheet = character_sheet(data, &state, &id).unwrap();
        let available: Vec<&str> = tree
            .iter()
            .filter(|n| n.state == SkillState::Available)
            .map(|n| n.id.as_str())
            .collect();
        let mut learnable: Vec<&str> = sheet.learnable.iter().map(String::as_str).collect();
        learnable.sort_unstable();
        let mut available_sorted = available.clone();
        available_sorted.sort_unstable();
        assert_eq!(available_sorted, learnable);
        assert_eq!(
            tree.iter()
                .filter(|n| n.state == SkillState::Learned)
                .count(),
            sheet.learned.len()
        );
    }

    #[test]
    fn learning_a_skill_goes_through_an_order() {
        let (data, mut state, faction) = start();
        let id = ruler(&state, &faction);
        let grant = Order::DebugGrantXp {
            character: id.clone(),
            amount: 5000,
        };
        assert!(submit_order(data, &mut state, grant).ok);
        let sheet = character_sheet(data, &state, &id).unwrap();
        let Some(skill) = sheet.learnable.first().cloned() else {
            // No learnable skill with the granted points: nothing to prove.
            return;
        };
        assert!(learn_skill(data, &mut state, id.as_str(), &skill).ok);
        let after = character_sheet(data, &state, &id).unwrap();
        assert!(after.learned.contains(&skill));
    }

    #[test]
    fn orders_refuse_unknown_ids_in_french() {
        let (data, mut state, _) = start();
        let result = learn_skill(data, &mut state, "chr_nobody", "skill_nothing");
        assert!(!result.ok && !result.message.is_empty());
        assert!(!assign_general(data, &mut state, "chr_nobody", "not an army").ok);
    }

    #[test]
    fn governor_and_general_orders_go_through_the_sim() {
        let (data, mut state, faction) = start();
        let id = ruler(&state, &faction);
        let sheet = character_sheet(data, &state, &id).unwrap();
        let province = &sheet.governor_options[0].id;
        // The sim decides; the answer must be a clean ok or a French refusal, never a panic.
        let result = assign_governor(data, &mut state, id.as_str(), province);
        assert!(result.ok || !result.message.is_empty());
    }

    #[test]
    fn family_tree_contains_the_root_and_links() {
        let (data, state, faction) = start();
        let id = ruler(&state, &faction);
        let tree = family_tree(data, &state, &id, 2, 2).expect("tree");
        assert_eq!(tree.root, id.as_str());
        let root = tree.nodes.iter().find(|n| n.id == tree.root).expect("root");
        assert_eq!(root.generation, 0);
        assert!(root.blood);
        for node in &tree.nodes {
            assert!(node.generation.abs() <= 2 || !node.blood);
        }
    }

    #[test]
    fn court_lists_ruler_first() {
        let (data, state, faction) = start();
        let rows = court_rows(data, &state, &faction);
        assert!(!rows.is_empty());
        assert!(rows[0].ruler);
        assert_eq!(rows.iter().filter(|r| r.ruler).count(), 1);
    }

    #[test]
    fn council_offices_and_appointment() {
        let (data, mut state, faction) = start();
        let offices = council(data, &state, &faction);
        assert!(!offices.is_empty(), "France has a council");
        let office = offices[0].id.clone();
        let candidates = council_candidates(data, &state, &faction, &office);
        let Some(free) = candidates.iter().find(|c| c.current_office.is_empty()) else {
            return;
        };
        assert!(appoint_councillor(data, &mut state, &office, &free.id).ok);
        let after = council(data, &state, &faction);
        let seat = after.iter().find(|o| o.id == office).unwrap();
        assert_eq!(seat.holder_id, free.id);
        assert!(dismiss_councillor(data, &mut state, &office).ok);
        let after = council(data, &state, &faction);
        assert!(after
            .iter()
            .find(|o| o.id == office)
            .unwrap()
            .holder_id
            .is_empty());
    }

    #[test]
    fn royal_acts_chivalry_and_captains_are_listed() {
        let (data, state, faction) = start();
        let acts = royal_acts(data, &state, &faction);
        assert!(!acts.is_empty());
        assert!(acts.iter().all(|a| a.available || !a.reason.is_empty()));
        let chivalry = chivalric_orders(data, &state, &faction);
        assert!(chivalry.founded.is_none());
        let captains = captain_info(&state, &faction).expect("capital city");
        assert!(captains.cap >= captains.count);
        assert!(!captains.settlement.is_empty());
    }
}
