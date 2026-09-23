//! Marriages, births, deaths, succession and regency (spec § 2), plus the
//! character queries the Godot bridge reads (spec § 3).

use std::collections::BTreeSet;

use data_model::{
    CharacterId, CharacterStatus, FactionId, Family, GameData, ProvinceId, Role, Sex, SkillId,
    Skills, SuccessionLaw, TraitId,
};
use serde::{Deserialize, Serialize};

use crate::events::{EventKind, GameEvent};
use crate::skills;
use crate::state::{CampaignState, CharacterState};

/// Age of majority (spec § 2): below it, a character cannot command, govern
/// or rule alone (a regency is opened for a ruler).
pub const MAJORITY_AGE: i32 = 15;
/// Minimum age to marry (spec § 2).
pub const MARRIAGE_MIN_AGE: i32 = 14;
/// Fertile age range of the mother for the winter birth roll (spec § 2).
pub const FERTILE_AGE_RANGE: std::ops::RangeInclusive<i32> = 16..=45;
/// Base probability (per mille) of a birth per married, fertile couple per
/// winter (spec § 2: "probabilité 25 % × (1 + Fertility)").
pub const BASE_BIRTH_PERMILLE: u32 = 250;
/// Chance (per mille) of being wounded after a defeat (spec § 2).
pub const WOUNDED_AFTER_DEFEAT_PERMILLE: u32 = 150;
/// Battles fought before `trait_veteran` is granted (spec § 2).
pub const VETERAN_BATTLES: u32 = 5;
/// Sieges won before `trait_siege_master` is granted (spec § 2).
pub const SIEGE_MASTER_SIEGES: u32 = 3;
/// Raids led before `trait_cruel` is granted (spec § 2).
pub const CRUEL_RAIDS: u32 = 3;
/// Chance (per mille) that a defeated general dies on the field (spec § 2).
pub const GENERAL_DEATH_PERMILLE: u32 = 50;
/// Unrest penalty applied to every province of a faction under regency
/// (spec § 2).
pub const REGENCY_UNREST_PENALTY: u8 = 5;
/// Prestige gained for a battle victory / a marriage / a new title.
pub const PRESTIGE_VICTORY: i32 = 5;
pub const PRESTIGE_MARRIAGE: i32 = 3;
pub const PRESTIGE_TITLE: i32 = 10;
/// Fallback first names used only when no `data.names` list matches a
/// culture (spec § 1 asks for ~30 per culture; this is a tiny safety net).
const FALLBACK_MALE_NAMES: &[&str] = &["Jean", "Guillaume", "Pierre", "Robert", "Thomas"];
const FALLBACK_FEMALE_NAMES: &[&str] = &["Jeanne", "Marguerite", "Isabelle", "Agnès", "Blanche"];

fn faction_name(data: &GameData, id: &FactionId) -> String {
    data.factions
        .get(id)
        .map_or_else(|| id.to_string(), |f| f.short_or_display_name().to_owned())
}

// =========================================================================
// Marriage (spec § 2 `propose_marriage`)
// =========================================================================

/// Why `propose_marriage` was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MarriageError {
    #[error("personnage inconnu : {0}")]
    UnknownCharacter(CharacterId),
    #[error("l'un des deux personnages est mort")]
    Dead,
    #[error("les deux personnages doivent être du même sexe biologique différent")]
    SameSex,
    #[error("âge minimum non atteint (14 ans)")]
    TooYoung,
    #[error("l'un des deux personnages est déjà marié")]
    AlreadyMarried,
    #[error("lien de parenté trop proche")]
    Related,
}

/// `true` when `a` and `b` are siblings (share a father or a mother). Direct
/// parent/child is checked separately with [`is_parent_of`].
fn are_siblings(a: &CharacterState, b: &CharacterState) -> bool {
    (a.father.is_some() && a.father == b.father) || (a.mother.is_some() && a.mother == b.mother)
}

fn is_parent_of(parent_id: &CharacterId, child: &CharacterState) -> bool {
    child.father.as_ref() == Some(parent_id) || child.mother.as_ref() == Some(parent_id)
}

/// Validates and applies `propose_marriage { character, spouse }` (spec § 2).
/// The engine already accepts spouses of two different factions (M5
/// diplomacy decides whether the AI proposes them).
pub fn propose_marriage(
    state: &mut CampaignState,
    _data: &GameData,
    character: &CharacterId,
    spouse: &CharacterId,
) -> Result<(), MarriageError> {
    let a = state
        .characters
        .get(character)
        .cloned()
        .ok_or_else(|| MarriageError::UnknownCharacter(character.clone()))?;
    let b = state
        .characters
        .get(spouse)
        .cloned()
        .ok_or_else(|| MarriageError::UnknownCharacter(spouse.clone()))?;
    if !a.alive || !b.alive {
        return Err(MarriageError::Dead);
    }
    if a.sex == b.sex {
        return Err(MarriageError::SameSex);
    }
    if a.age(state.year) < MARRIAGE_MIN_AGE || b.age(state.year) < MARRIAGE_MIN_AGE {
        return Err(MarriageError::TooYoung);
    }
    if a.spouse.is_some() || b.spouse.is_some() {
        return Err(MarriageError::AlreadyMarried);
    }
    if is_parent_of(character, &b) || is_parent_of(spouse, &a) || are_siblings(&a, &b) {
        return Err(MarriageError::Related);
    }
    state
        .characters
        .get_mut(character)
        .expect("checked above")
        .spouse = Some(spouse.clone());
    state
        .characters
        .get_mut(spouse)
        .expect("checked above")
        .spouse = Some(character.clone());
    for id in [character, spouse] {
        let c = state.characters.get_mut(id).expect("checked above");
        c.prestige += PRESTIGE_MARRIAGE;
    }
    // Orders apply immediately and are not part of the turn journal (like
    // `Build`/`AssignGeneral`): `end_turn` overwrites `CampaignState::events`
    // with the turn's own log, so an event pushed here would be silently
    // dropped as soon as the turn resolves.
    Ok(())
}

/// Marriage candidates of `id` (spec § 3 `get_marriage_candidates`): living,
/// unmarried, opposite sex, of age, not a close relative. Cross-faction
/// candidates are included (M5 decides whether the AI actually proposes).
pub fn marriage_candidates(
    state: &CampaignState,
    _data: &GameData,
    id: &CharacterId,
) -> Vec<CharacterId> {
    let Some(character) = state.characters.get(id) else {
        return Vec::new();
    };
    if !character.alive
        || character.spouse.is_some()
        || character.age(state.year) < MARRIAGE_MIN_AGE
    {
        return Vec::new();
    }
    state
        .characters
        .iter()
        .filter(|(candidate_id, candidate)| {
            *candidate_id != id
                && candidate.alive
                && candidate.spouse.is_none()
                && candidate.sex != character.sex
                && candidate.age(state.year) >= MARRIAGE_MIN_AGE
                && !is_parent_of(id, candidate)
                && !is_parent_of(candidate_id, character)
                && !are_siblings(character, candidate)
        })
        .map(|(candidate_id, _)| candidate_id.clone())
        .collect()
}

// =========================================================================
// Governors (spec § 2 `assign_governor`)
// =========================================================================

/// Why `assign_governor` was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GovernorError {
    #[error("personnage inconnu : {0}")]
    UnknownCharacter(CharacterId),
    #[error("province inconnue : {0}")]
    UnknownProvince(ProvinceId),
    #[error("ce personnage ne peut pas gouverner (mort, captif, mineur ou d'une autre faction)")]
    CharacterUnavailable,
    #[error("ce personnage commande déjà une armée")]
    AlreadyCommanding,
    #[error("cette province n'est pas contrôlée par la faction du personnage")]
    NotYourProvince,
}

/// Validates and applies `assign_governor { province, character }` (spec §
/// 2): the character must belong to the controlling faction, be alive,
/// non-captive, an adult, and not already commanding an army (a character
/// governs a province *or* commands an army, not both). One governor per
/// province: a previous governor of `province` is unassigned.
pub fn assign_governor(
    state: &mut CampaignState,
    province: &ProvinceId,
    character: &CharacterId,
) -> Result<(), GovernorError> {
    let province_state = state
        .provinces
        .get(province)
        .ok_or_else(|| GovernorError::UnknownProvince(province.clone()))?;
    let c = state
        .characters
        .get(character)
        .ok_or_else(|| GovernorError::UnknownCharacter(character.clone()))?;
    if c.faction != province_state.controller {
        return Err(GovernorError::NotYourProvince);
    }
    if !c.alive || c.captive || !c.is_major(state.year) {
        return Err(GovernorError::CharacterUnavailable);
    }
    if c.army.is_some() {
        return Err(GovernorError::AlreadyCommanding);
    }
    // Unassign whoever governed this province before.
    let previous: Option<CharacterId> = state
        .characters
        .iter()
        .find(|(id, c)| c.governor_of.as_ref() == Some(province) && *id != character)
        .map(|(id, _)| id.clone());
    if let Some(previous) = previous {
        state
            .characters
            .get_mut(&previous)
            .expect("exists")
            .governor_of = None;
    }
    // A character governs one province at a time: reassigning simply moves
    // `governor_of` to the new province (no-op if it was already `province`).
    state
        .characters
        .get_mut(character)
        .expect("checked above")
        .governor_of = Some(province.clone());
    Ok(())
}

// =========================================================================
// XP, trait acquisition, ageing helpers used by battle_auto/siege/movement.
// =========================================================================

/// Called once per battle a general fought, after the outcome is known
/// (spec § 2: XP, `trait_veteran`, wounded, death chance).
pub fn on_battle_resolved(
    state: &mut CampaignState,
    data: &GameData,
    general: &CharacterId,
    won: bool,
    events: &mut Vec<GameEvent>,
) {
    let xp = if won {
        skills::BATTLE_VICTORY_XP
    } else {
        skills::BATTLE_XP
    };
    skills::grant_experience(state, general, xp);
    if won {
        let c = state.characters.get_mut(general).expect("exists");
        c.prestige += PRESTIGE_VICTORY;
    }
    let Some(c) = state.characters.get_mut(general) else {
        return;
    };
    c.battles_fought += 1;
    let battles = c.battles_fought;
    if battles >= VETERAN_BATTLES {
        let veteran = TraitId::new("trait_veteran").expect("well-formed id");
        skills::grant_trait(state, data, general, &veteran);
    }
    if !won {
        if state.rng.chance_permille(WOUNDED_AFTER_DEFEAT_PERMILLE) {
            let wounded = TraitId::new("trait_wounded").expect("well-formed id");
            if skills::grant_trait(state, data, general, &wounded) {
                events.push(
                    GameEvent::new(
                        EventKind::TraitAcquired,
                        format!(
                            "{} est blessé au combat.",
                            state.character_name(data, general)
                        ),
                    )
                    .faction(&state.characters[general].faction),
                );
            }
        }
        if state.rng.chance_permille(GENERAL_DEATH_PERMILLE) {
            crate::characters::kill(state, data, general, events);
        }
    }
}

/// Called once per siege won by the besieging general (spec § 2:
/// `trait_siege_master` after 3).
pub fn on_siege_won(state: &mut CampaignState, data: &GameData, general: &CharacterId) {
    let Some(c) = state.characters.get_mut(general) else {
        return;
    };
    c.sieges_won += 1;
    if c.sieges_won >= SIEGE_MASTER_SIEGES {
        let siege_master = TraitId::new("trait_siege_master").expect("well-formed id");
        skills::grant_trait(state, data, general, &siege_master);
    }
}

/// Called once per raid led by a general (spec § 2: `trait_cruel` after 3).
pub fn on_raid_led(state: &mut CampaignState, data: &GameData, general: &CharacterId) {
    let Some(c) = state.characters.get_mut(general) else {
        return;
    };
    c.raids_led += 1;
    if c.raids_led >= CRUEL_RAIDS {
        let cruel = TraitId::new("trait_cruel").expect("well-formed id");
        skills::grant_trait(state, data, general, &cruel);
    }
}

/// Grants `trait_captive_ransomed` when a captive is freed (spec § 2).
pub fn on_ransomed(state: &mut CampaignState, data: &GameData, character: &CharacterId) {
    let ransomed = TraitId::new("trait_captive_ransomed").expect("well-formed id");
    skills::grant_trait(state, data, character, &ransomed);
}

// =========================================================================
// Births (spec § 2, resolved once per winter turn)
// =========================================================================

fn pick_name(
    data: &GameData,
    faction: &FactionId,
    sex: Sex,
    rng: &mut crate::rng::CampaignRng,
) -> String {
    let culture = data.factions.get(faction).map(|f| f.culture.clone());
    let list = culture.as_ref().and_then(|culture| {
        data.names
            .values()
            .find(|list| list.cultures.contains(culture))
    });
    let names: Vec<&str> = match list {
        Some(list) => match sex {
            Sex::Male => list.male_first_names.iter().map(String::as_str).collect(),
            Sex::Female => list.female_first_names.iter().map(String::as_str).collect(),
        },
        None => match sex {
            Sex::Male => FALLBACK_MALE_NAMES.to_vec(),
            Sex::Female => FALLBACK_FEMALE_NAMES.to_vec(),
        },
    };
    if names.is_empty() {
        return match sex {
            Sex::Male => "Sans-Nom".to_owned(),
            Sex::Female => "Sans-Nom".to_owned(),
        };
    }
    let index = rng.below(names.len() as u32) as usize;
    names[index].to_owned()
}

/// "Hugues de Valois", "Jeanne d'Évreux": first name + house.
fn generated_full_name(first_name: &str, house: &str) -> String {
    let starts_with_vowel = house
        .chars()
        .next()
        .is_some_and(|c| "AEIOUYÉÈÊÂÎÔaeiouyéèêâîô".contains(c));
    if house.is_empty() {
        first_name.to_owned()
    } else if starts_with_vowel {
        format!("{first_name} d'{house}")
    } else {
        format!("{first_name} de {house}")
    }
}

/// Allocates the next generated character id (`chr_gen_NNNN`).
fn next_generated_id(state: &CampaignState) -> CharacterId {
    let mut n = state.characters.len() as u32 + 1;
    loop {
        let candidate = CharacterId::new(format!("chr_gen_{n:05}")).expect("well-formed id");
        if !state.characters.contains_key(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

fn random_starting_skills(rng: &mut crate::rng::CampaignRng) -> Skills {
    Skills {
        command: rng.below(4) as u8,
        governance: rng.below(4) as u8,
        court: rng.below(4) as u8,
    }
}

/// Brings a new adult ruler to a faction whose dynasty has died out: an
/// elected pontiff or emperor for elective realms, otherwise a lord of a new
/// house named after the capital (M5: realms with land never simply vanish).
pub(crate) fn spawn_ruler(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> CharacterId {
    let first_name = pick_name(data, faction, Sex::Male, &mut state.rng);
    let elective = data
        .factions
        .get(faction)
        .is_some_and(|f| f.succession_law == SuccessionLaw::Elective);
    let capital = state.factions.get(faction).map(|f| f.capital.clone());
    // The capital city ("Lisbonne (Alcáçova)" -> "Lisbonne"), else the province.
    let seat = data
        .factions
        .get(faction)
        .and_then(|f| f.capital_city.as_deref())
        .map(|city| city.split(" (").next().unwrap_or(city).to_owned())
        .or_else(|| {
            capital
                .as_ref()
                .and_then(|c| data.provinces.get(c))
                .map(|p| p.name.display.clone())
        })
        .unwrap_or_else(|| faction.to_string());
    let house = if elective {
        format!("élu de {seat}")
    } else {
        seat.clone()
    };
    let age = 30 + state.rng.below(20) as i32;
    let id = next_generated_id(state);
    let mut skills = random_starting_skills(&mut state.rng);
    skills.command += 2;
    skills.governance += 2;
    skills.court += 2;
    let name = if elective {
        first_name.clone()
    } else {
        generated_full_name(&first_name, &house)
    };
    state.characters.insert(
        id.clone(),
        CharacterState {
            name: Some(name),
            faction: faction.clone(),
            alive: true,
            birth_year: state.year - age,
            sex: Sex::Male,
            house,
            location: capital,
            army: None,
            skills,
            captive: false,
            captor: None,
            experience: 0,
            skill_points: 0,
            skills_learned: BTreeSet::new(),
            traits: BTreeSet::new(),
            spouse: None,
            children: Vec::new(),
            father: None,
            mother: None,
            piety: 50,
            prestige: 0,
            loyalty: 100,
            title: None,
            governor_of: None,
            battles_fought: 0,
            sieges_won: 0,
            raids_led: 0,
        },
    );
    id
}

/// Identity of a newborn, historical or generated (`spawn_child`).
struct NewChild {
    id: CharacterId,
    /// `None` for historical characters, named by `data.characters`.
    name: Option<String>,
    faction: FactionId,
    house: String,
    sex: Sex,
    father: CharacterId,
    mother: CharacterId,
    location: Option<ProvinceId>,
}

fn spawn_child(state: &mut CampaignState, data: &GameData, child: NewChild) -> CharacterId {
    let NewChild {
        id,
        name,
        faction,
        house,
        sex,
        father,
        mother,
        location,
    } = child;
    let birth_year = state.year;
    let mut traits = BTreeSet::new();
    let personality: Vec<TraitId> = data
        .traits
        .values()
        .filter(|t| matches!(t.category, data_model::TraitCategory::Personality))
        .map(|t| t.id.clone())
        .collect();
    if !personality.is_empty() {
        let index = state.rng.below(personality.len() as u32) as usize;
        traits.insert(personality[index].clone());
    }
    let skills = random_starting_skills(&mut state.rng);
    state.characters.insert(
        id.clone(),
        CharacterState {
            name,
            faction,
            alive: true,
            birth_year,
            sex,
            house,
            location,
            army: None,
            skills,
            captive: false,
            captor: None,
            experience: 0,
            skill_points: 0,
            skills_learned: BTreeSet::new(),
            traits,
            spouse: None,
            children: Vec::new(),
            father: Some(father.clone()),
            mother: Some(mother.clone()),
            piety: 50,
            prestige: 0,
            loyalty: 100,
            title: None,
            governor_of: None,
            battles_fought: 0,
            sieges_won: 0,
            raids_led: 0,
        },
    );
    for parent in [&father, &mother] {
        if let Some(p) = state.characters.get_mut(parent) {
            p.children.push(id.clone());
        }
    }
    id
}

/// Phase: winter births (generated children of every married, fertile
/// couple) and historical births (`CharacterStatus::Unborn` characters whose
/// `birth` year matches, spec § 2).
pub(crate) fn resolve_births(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    if state.season != crate::state::Season::Winter {
        return;
    }
    let year = state.year;

    // Historical unborn characters: born at their real date if both parents
    // are alive and married to each other; otherwise the timeline diverges
    // and they never are (spec § 1/§ 2).
    let unborn: Vec<CharacterId> = data
        .characters
        .iter()
        .filter(|(id, c)| {
            c.status == Some(CharacterStatus::Unborn)
                && c.birth.year() == Some(year)
                && !state.characters.contains_key(*id)
        })
        .map(|(id, _)| id.clone())
        .collect();
    for id in unborn {
        let Some(character) = data.characters.get(&id) else {
            continue;
        };
        let Some(Family {
            father: Some(father),
            mother: Some(mother),
            ..
        }) = &character.family
        else {
            continue;
        };
        let parents_ready = state
            .characters
            .get(father)
            .is_some_and(|f| f.alive && f.spouse.as_ref() == Some(mother))
            && state
                .characters
                .get(mother)
                .is_some_and(|m| m.alive && m.spouse.as_ref() == Some(father));
        if !parents_ready {
            continue;
        }
        let location = state
            .characters
            .get(mother)
            .and_then(|m| m.location.clone());
        spawn_child(
            state,
            data,
            NewChild {
                id: id.clone(),
                name: None,
                faction: character.faction.clone(),
                house: character.house.clone(),
                sex: character.sex,
                father: father.clone(),
                mother: mother.clone(),
                location,
            },
        );
        events.push(
            GameEvent::new(
                EventKind::Birth,
                format!("Naissance de {}.", state.character_name(data, &id)),
            )
            .faction(&character.faction),
        );
    }

    // Generated children of every married, fertile couple.
    let mothers: Vec<CharacterId> = state
        .characters
        .iter()
        .filter(|(_, c)| {
            c.alive
                && c.sex == Sex::Female
                && c.spouse.is_some()
                && FERTILE_AGE_RANGE.contains(&c.age(year))
        })
        .map(|(id, _)| id.clone())
        .collect();
    for mother_id in mothers {
        let mother = &state.characters[&mother_id];
        let Some(father_id) = mother.spouse.clone() else {
            continue;
        };
        let Some(father) = state.characters.get(&father_id) else {
            continue;
        };
        if !father.alive {
            continue;
        }
        let fertility_percent = skills::character_effects(state, data, &mother_id)
            .fertility
            .flat
            .max(
                skills::character_effects(state, data, &father_id)
                    .fertility
                    .flat,
            );
        let permille = (f64::from(BASE_BIRTH_PERMILLE) * (1.0 + fertility_percent / 100.0))
            .round()
            .clamp(0.0, 1000.0) as u32;
        if !state.rng.chance_permille(permille) {
            continue;
        }
        let sex = if state.rng.below(2) == 0 {
            Sex::Male
        } else {
            Sex::Female
        };
        let faction = mother.faction.clone();
        let house = state.characters[&father_id].house.clone();
        let location = mother.location.clone();
        let first_name = pick_name(data, &faction, sex, &mut state.rng);
        let full_name = generated_full_name(&first_name, &house);
        let id = next_generated_id(state);
        spawn_child(
            state,
            data,
            NewChild {
                id,
                name: Some(full_name.clone()),
                faction: faction.clone(),
                house,
                sex,
                father: father_id.clone(),
                mother: mother_id.clone(),
                location,
            },
        );
        events.push(
            GameEvent::new(
                EventKind::Birth,
                format!(
                    "Naissance de {full_name} ({}), {} de {}.",
                    faction_name(data, &faction),
                    if sex == Sex::Male { "fils" } else { "fille" },
                    state.character_name(data, &father_id)
                ),
            )
            .faction(&faction),
        );
    }
}

// =========================================================================
// Majority, regency (spec § 2)
// =========================================================================

/// Phase: opens/lifts regencies for every faction whose ruler is a minor
/// and applies the regency unrest penalty each turn it lasts (spec § 2: +5
/// while regent). The journal reports the start and the end only.
pub(crate) fn resolve_regencies(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let year = state.year;
    let ids: Vec<FactionId> = state.factions.keys().cloned().collect();
    for faction_id in ids {
        let faction = &state.factions[&faction_id];
        if !faction.alive {
            continue;
        }
        let was_regency = faction.regency;
        let ruler = faction.ruler.clone();
        let minor_ruler = ruler
            .as_ref()
            .and_then(|r| state.characters.get(r))
            .filter(|r| r.alive && !r.is_major(year))
            .is_some();
        state.factions.get_mut(&faction_id).expect("exists").regency = minor_ruler;
        let ruler_name = ruler
            .as_ref()
            .map(|r| state.character_name(data, r))
            .unwrap_or_default();
        if minor_ruler {
            for province in state
                .provinces
                .values_mut()
                .filter(|p| p.controller == faction_id)
            {
                province.unrest = province
                    .unrest
                    .saturating_add(REGENCY_UNREST_PENALTY)
                    .min(100);
            }
            if !was_regency {
                events.push(
                    GameEvent::new(
                        EventKind::Regency,
                        format!(
                            "{ruler_name} est mineur : une régence gouverne {}.",
                            faction_name(data, &faction_id)
                        ),
                    )
                    .faction(&faction_id),
                );
            }
        } else if was_regency && ruler.is_some() {
            events.push(
                GameEvent::new(
                    EventKind::Regency,
                    format!(
                        "{ruler_name} atteint sa majorité : fin de la régence en {}.",
                        faction_name(data, &faction_id)
                    ),
                )
                .faction(&faction_id),
            );
        }
    }
}

/// Prestige effects (buildings, technologies, the ruler's traits and skills)
/// are yearly figures divided by this (F1): a cathedral adds 2 a year.
pub const PRESTIGE_EFFECT_DIVISOR: f64 = 5.0;

/// Yearly prestige of a faction's ruler from its `Prestige` effects (F1):
/// buildings of the provinces it controls, its technologies (gothic
/// flamboyant, printing press) and the ruler's own traits and skills.
pub fn yearly_court_prestige(state: &CampaignState, data: &GameData, faction: &FactionId) -> i32 {
    let Some(ruler) = state
        .factions
        .get(faction)
        .and_then(|f| f.ruler.clone())
        .filter(|r| state.characters.get(r).is_some_and(|c| c.alive))
    else {
        return 0;
    };
    let buildings: f64 = state
        .provinces
        .values()
        .filter(|p| &p.controller == faction)
        .map(|p| {
            crate::buildings::effects_of(data, &p.buildings)
                .prestige
                .apply(0.0)
        })
        .sum();
    let tech = crate::research::faction_tech_effects(state, data, faction)
        .prestige
        .apply(0.0);
    let own = skills::character_effects(state, data, &ruler)
        .prestige
        .apply(0.0);
    ((buildings + tech + own) / PRESTIGE_EFFECT_DIVISOR).round() as i32
}

/// Phase (winter): every ruler gains its [`yearly_court_prestige`].
pub(crate) fn resolve_court_prestige(state: &mut CampaignState, data: &GameData) {
    if state.season != crate::state::Season::Winter {
        return;
    }
    let factions: Vec<FactionId> = state
        .factions
        .iter()
        .filter(|(_, f)| f.alive)
        .map(|(id, _)| id.clone())
        .collect();
    for faction in factions {
        let gain = yearly_court_prestige(state, data, &faction);
        if gain == 0 {
            continue;
        }
        let ruler = state.factions[&faction].ruler.clone();
        if let Some(c) = ruler.and_then(|r| state.characters.get_mut(&r)) {
            c.prestige += gain;
        }
    }
}

/// Phase: governors whose province was lost, or who died or were captured,
/// lose the post; the others gain governance XP (spec § 2: +2 per turn).
pub(crate) fn resolve_governance(state: &mut CampaignState) {
    let governors: Vec<(CharacterId, ProvinceId)> = state
        .characters
        .iter()
        .filter_map(|(id, c)| c.governor_of.clone().map(|p| (id.clone(), p)))
        .collect();
    for (id, province) in governors {
        let character = &state.characters[&id];
        let keeps_post = character.alive
            && !character.captive
            && state
                .provinces
                .get(&province)
                .is_some_and(|p| p.controller == character.faction);
        if keeps_post {
            skills::grant_experience(state, &id, skills::GOVERNANCE_XP_PER_TURN);
        } else {
            state.characters.get_mut(&id).expect("exists").governor_of = None;
        }
    }
}

// =========================================================================
// Succession (spec § 2)
// =========================================================================

/// Living characters of `faction` eligible to inherit from `ruler`: members
/// of `house`, plus the ruler's own children whatever their house (a queen's
/// son belongs to his father's house, e.g. Charles d'Évreux for Navarre).
fn house_members<'a>(
    state: &'a CampaignState,
    faction: &FactionId,
    house: &str,
    ruler: &CharacterId,
) -> Vec<(&'a CharacterId, &'a CharacterState)> {
    state
        .characters
        .iter()
        .filter(|(id, c)| {
            c.alive
                && &c.faction == faction
                && *id != ruler
                && (c.house == house || is_parent_of(ruler, c))
        })
        .collect()
}

/// Heir of `faction` according to its `succession_law` (spec § 2). Falls
/// back to the eldest living male of the house for any other value, matching
/// the previous (M2) behaviour.
pub(crate) fn pick_heir_by_law(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    ruler: &CharacterId,
) -> Option<CharacterId> {
    let ruler_house = state.characters.get(ruler)?.house.clone();
    let law = data.factions.get(faction).map(|f| f.succession_law);
    let members = house_members(state, faction, &ruler_house, ruler);
    let ruler_state = state.characters.get(ruler)?;

    match law {
        Some(SuccessionLaw::Salic) => {
            // Ruler's sons, eldest first.
            let mut sons: Vec<_> = members
                .iter()
                .filter(|(_, c)| c.sex == Sex::Male && is_parent_of(ruler, c))
                .collect();
            sons.sort_by_key(|(id, c)| (c.birth_year, (*id).clone()));
            if let Some((id, _)) = sons.first() {
                return Some((*id).clone());
            }
            // Then brothers, eldest first.
            let mut siblings: Vec<_> = members
                .iter()
                .filter(|(_, c)| {
                    c.sex == Sex::Male
                        && ((c.father.is_some() && c.father == ruler_state.father)
                            || (c.mother.is_some() && c.mother == ruler_state.mother))
                })
                .collect();
            siblings.sort_by_key(|(id, c)| (c.birth_year, (*id).clone()));
            if let Some((id, _)) = siblings.first() {
                return Some((*id).clone());
            }
            // Then the wider male line.
            let mut males: Vec<_> = members.iter().filter(|(_, c)| c.sex == Sex::Male).collect();
            males.sort_by_key(|(id, c)| (c.birth_year, (*id).clone()));
            males.first().map(|(id, _)| (*id).clone())
        }
        Some(SuccessionLaw::MalePreferencePrimogeniture) => {
            let mut sons: Vec<_> = members
                .iter()
                .filter(|(_, c)| c.sex == Sex::Male && is_parent_of(ruler, c))
                .collect();
            sons.sort_by_key(|(id, c)| (c.birth_year, (*id).clone()));
            if let Some((id, _)) = sons.first() {
                return Some((*id).clone());
            }
            let mut daughters: Vec<_> = members
                .iter()
                .filter(|(_, c)| c.sex == Sex::Female && is_parent_of(ruler, c))
                .collect();
            daughters.sort_by_key(|(id, c)| (c.birth_year, (*id).clone()));
            if let Some((id, _)) = daughters.first() {
                return Some((*id).clone());
            }
            let mut rest = members.clone();
            rest.sort_by_key(|(id, c)| (c.sex == Sex::Female, c.birth_year, (*id).clone()));
            rest.first().map(|(id, _)| (*id).clone())
        }
        Some(SuccessionLaw::CognaticPrimogeniture) => {
            let mut children: Vec<_> = members
                .iter()
                .filter(|(_, c)| is_parent_of(ruler, c))
                .collect();
            children.sort_by_key(|(id, c)| (c.birth_year, (*id).clone()));
            if let Some((id, _)) = children.first() {
                return Some((*id).clone());
            }
            let mut rest = members.clone();
            rest.sort_by_key(|(id, c)| (c.birth_year, (*id).clone()));
            rest.first().map(|(id, _)| (*id).clone())
        }
        Some(SuccessionLaw::Elective) => members
            .iter()
            .max_by_key(|(id, c)| (c.skills.court, std::cmp::Reverse((*id).clone())))
            .map(|(id, _)| (*id).clone()),
        None => {
            let mut males: Vec<_> = members.iter().filter(|(_, c)| c.sex == Sex::Male).collect();
            males.sort_by_key(|(id, c)| (c.birth_year, (*id).clone()));
            males.first().map(|(id, _)| (*id).clone())
        }
    }
}

// =========================================================================
// Bridge queries (spec § 3)
// =========================================================================

/// One trait line of [`CharacterView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TraitView {
    pub id: TraitId,
    pub name: String,
    pub category: String,
}

/// One child line of [`CharacterView`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChildView {
    pub id: CharacterId,
    pub name: String,
    pub age: i32,
}

/// Full character sheet for the bridge (spec § 3 `get_character`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterView {
    pub id: CharacterId,
    pub name: String,
    pub epithet: Option<String>,
    pub sex: Sex,
    pub age: i32,
    pub alive: bool,
    pub faction: FactionId,
    pub house: String,
    pub title: Option<String>,
    pub role: Option<Role>,
    pub skills: Skills,
    pub experience: u32,
    pub skill_points: u32,
    pub skills_learned: Vec<SkillId>,
    pub traits: Vec<TraitView>,
    pub spouse: Option<CharacterId>,
    pub spouse_name: Option<String>,
    pub children: Vec<ChildView>,
    pub father: Option<CharacterId>,
    pub mother: Option<CharacterId>,
    pub location: Option<ProvinceId>,
    pub army: Option<crate::state::ArmyId>,
    pub governor_of: Option<ProvinceId>,
    pub captive: bool,
    pub piety: u8,
    pub prestige: i32,
}

impl CampaignState {
    /// Full character sheet for the bridge (spec § 3).
    pub fn character_view(&self, data: &GameData, id: &CharacterId) -> Option<CharacterView> {
        let c = self.characters.get(id)?;
        let static_data = data.characters.get(id);
        let traits = c
            .traits
            .iter()
            .map(|trait_id| {
                data.traits.get(trait_id).map_or_else(
                    || TraitView {
                        id: trait_id.clone(),
                        name: trait_id.to_string(),
                        category: "unknown".to_owned(),
                    },
                    |def| TraitView {
                        id: trait_id.clone(),
                        name: def.name.display.clone(),
                        category: format!("{:?}", def.category).to_lowercase(),
                    },
                )
            })
            .collect();
        let children = c
            .children
            .iter()
            .filter_map(|child_id| {
                let child = self.characters.get(child_id)?;
                Some(ChildView {
                    id: child_id.clone(),
                    name: self.character_name(data, child_id),
                    age: child.age(self.year),
                })
            })
            .collect();
        Some(CharacterView {
            id: id.clone(),
            name: self.character_name(data, id),
            epithet: static_data.and_then(|s| s.epithet.clone()),
            sex: c.sex,
            age: c.age(self.year),
            alive: c.alive,
            faction: c.faction.clone(),
            house: c.house.clone(),
            title: c.title.clone(),
            role: static_data.map(|s| s.role),
            skills: c.skills,
            experience: c.experience,
            skill_points: c.skill_points,
            skills_learned: c.skills_learned.iter().cloned().collect(),
            traits,
            spouse: c.spouse.clone(),
            spouse_name: c.spouse.as_ref().map(|s| self.character_name(data, s)),
            children,
            father: c.father.clone(),
            mother: c.mother.clone(),
            location: c.location.clone(),
            army: c.army.clone(),
            governor_of: c.governor_of.clone(),
            captive: c.captive,
            piety: c.piety,
            prestige: c.prestige,
        })
    }

    /// Living characters of `faction`, ruler and heir first, then by age
    /// (spec § 3 `get_faction_characters`).
    /// Living governor of `province`, if any.
    pub fn province_governor(&self, province: &ProvinceId) -> Option<&CharacterId> {
        self.characters
            .iter()
            .find(|(_, c)| c.alive && c.governor_of.as_ref() == Some(province))
            .map(|(id, _)| id)
    }

    pub fn faction_characters(&self, faction: &FactionId) -> Vec<CharacterId> {
        let faction_state = self.factions.get(faction);
        let ruler = faction_state.and_then(|f| f.ruler.clone());
        let heir = faction_state.and_then(|f| f.heir.clone());
        let mut ids: Vec<CharacterId> = self
            .characters
            .iter()
            .filter(|(_, c)| c.alive && &c.faction == faction)
            .map(|(id, _)| id.clone())
            .collect();
        ids.sort_by_key(|id| {
            let rank = if Some(id) == ruler.as_ref() {
                0
            } else if Some(id) == heir.as_ref() {
                1
            } else {
                2
            };
            let age = self.characters[id].birth_year;
            (rank, age, id.clone())
        });
        ids
    }

    /// Learnable skills of `id` (spec § 3 `get_learnable`).
    pub fn learnable_skills(&self, data: &GameData, id: &CharacterId) -> Vec<SkillId> {
        skills::learnable_skills(self, data, id)
    }

    /// Marriage candidates of `id` (spec § 3 `get_marriage_candidates`).
    pub fn marriage_candidates(&self, data: &GameData, id: &CharacterId) -> Vec<CharacterId> {
        marriage_candidates(self, data, id)
    }
}
