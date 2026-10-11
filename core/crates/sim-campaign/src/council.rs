//! Council of the realm (RG13a, ADR 0341): five offices held by characters of
//! the faction, whose skill sets the strength of each office's effect.
//!
//! - data: `data/council/offices.json` ([`data_model::CouncilOffices`]); the
//!   figures (effect per skill point, skill used, wording) live there. Without
//!   the file there is no council and nothing below does anything;
//! - state: [`Council`] per faction in `FactionState::council`, the holders by
//!   office id. A save without it is completed by [`ensure_councils`] (the
//!   bridge calls it after a load); a new campaign by [`init_councils`];
//! - orders: `AppointCouncillor` / `DismissCouncillor` ([`appoint`],
//!   [`dismiss`]). The ruler presides and never sits; one office per
//!   character; a vacant office has no effect;
//! - effects: derived on read, no per-turn state except the prestige carry.
//!   [`realm_effects`] is merged into `research::faction_tech_effects` (tax,
//!   unrest, recruit experience: the economy, recruitment and population
//!   already read it), [`add_army_effects`] into `skills::character_effects`
//!   (generals' morale), [`diplomatic_points`] is a reason line of the
//!   negotiation context, [`resolve_council`] pays the chamberlain's prestige;
//! - seasonal phase [`resolve_council`]: holders who died, left the faction or
//!   became ruler leave a vacancy (journal line for the player, kind
//!   `council_vacancy`); AI factions refill their vacancies at once.

use std::collections::BTreeMap;

use data_model::{
    CharacterId, CouncilEffectKind, CouncilOfficeDef, CouncilOffices, EffectKind, FactionId,
    GameData, SkillBranch,
};
use serde::{Deserialize, Serialize};

use crate::buildings::EffectTotals;
use crate::events::{EventKind, GameEvent};
use crate::state::CampaignState;

/// The council of one faction.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Council {
    /// The automatic assignment ran (a save without a council, or a new
    /// faction, has it run once).
    #[serde(default)]
    pub initialized: bool,
    /// Holder of each office, by office id; a missing office is vacant.
    #[serde(default)]
    pub holders: BTreeMap<String, CharacterId>,
    /// Hundredths of prestige earned by the chamberlain and not yet paid.
    #[serde(default)]
    pub prestige_hundredths: u32,
}

impl Council {
    /// `true` for a council nobody ever touched (not serialised).
    pub fn is_default(&self) -> bool {
        *self == Council::default()
    }
}

/// Why an appointment or a dismissal was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CouncilError {
    #[error("le royaume n'a pas de conseil")]
    NoCouncil,
    #[error("charge inconnue : {0}")]
    UnknownOffice(String),
    #[error("personnage inconnu ou mort")]
    UnknownCharacter,
    #[error("ce personnage n'est pas de votre royaume")]
    NotOfFaction,
    #[error("le souverain préside le conseil, il n'y siège pas")]
    IsRuler,
    #[error("ce personnage est trop jeune pour siéger")]
    TooYoung,
    #[error("ce personnage tient déjà une autre charge : {0}")]
    AlreadyHoldsOffice(String),
    #[error("ce personnage tient déjà cette charge")]
    AlreadyHolder,
    #[error("la charge est déjà vacante")]
    AlreadyVacant,
}

fn offices(data: &GameData) -> Option<&CouncilOffices> {
    data.council.as_ref()
}

/// Skill of `character` that sets the strength of an office using `branch`.
pub fn skill_of(state: &CampaignState, character: &CharacterId, branch: SkillBranch) -> u8 {
    state.characters.get(character).map_or(0, |c| match branch {
        SkillBranch::Command => c.skills.command,
        SkillBranch::Governance => c.skills.governance,
        SkillBranch::Court => c.skills.court,
    })
}

/// May `character` sit on the council of `faction`? Living, adult, of the
/// faction, and not its ruler.
pub fn check_eligible(
    state: &CampaignState,
    faction: &FactionId,
    character: &CharacterId,
) -> Result<(), CouncilError> {
    let c = state
        .characters
        .get(character)
        .filter(|c| c.alive)
        .ok_or(CouncilError::UnknownCharacter)?;
    if &c.faction != faction {
        return Err(CouncilError::NotOfFaction);
    }
    if state
        .factions
        .get(faction)
        .is_some_and(|f| f.ruler.as_ref() == Some(character))
    {
        return Err(CouncilError::IsRuler);
    }
    if !c.is_major(state.year) {
        return Err(CouncilError::TooYoung);
    }
    Ok(())
}

/// The sitting holder of `office` (`None`: vacant, or the recorded holder no
/// longer qualifies and awaits [`resolve_council`]).
pub fn holder<'a>(
    state: &'a CampaignState,
    faction: &FactionId,
    office: &str,
) -> Option<&'a CharacterId> {
    let id = state.factions.get(faction)?.council.holders.get(office)?;
    check_eligible(state, faction, id).ok()?;
    Some(id)
}

/// Office `character` sits on in `faction`, if any.
pub fn office_of(
    state: &CampaignState,
    faction: &FactionId,
    character: &CharacterId,
) -> Option<String> {
    state
        .factions
        .get(faction)?
        .council
        .holders
        .iter()
        .find(|(_, id)| *id == character)
        .map(|(office, _)| office.clone())
}

/// A character who could be appointed, with the skill the office reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub character: CharacterId,
    pub skill: u8,
    /// Office the character already holds (an appointment is then refused).
    pub current_office: Option<String>,
}

/// Eligible characters of `faction` for `office`, best skill first (ties by id).
pub fn candidates(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    office: &str,
) -> Vec<Candidate> {
    let Some(def) = offices(data).and_then(|o| o.office(office)) else {
        return Vec::new();
    };
    let mut found: Vec<Candidate> = state
        .characters
        .keys()
        .filter(|id| check_eligible(state, faction, id).is_ok())
        .map(|id| Candidate {
            character: id.clone(),
            skill: skill_of(state, id, def.skill),
            current_office: office_of(state, faction, id),
        })
        .collect();
    found.sort_by(|a, b| {
        b.skill
            .cmp(&a.skill)
            .then_with(|| a.character.cmp(&b.character))
    });
    found
}

/// Validates and applies `AppointCouncillor`. A sitting holder is replaced
/// and loses the loyalty of `dismissal_loyalty_penalty`.
pub fn appoint(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    office: &str,
    character: &CharacterId,
) -> Result<(), CouncilError> {
    let rules = offices(data).ok_or(CouncilError::NoCouncil)?;
    if rules.office(office).is_none() {
        return Err(CouncilError::UnknownOffice(office.to_owned()));
    }
    check_eligible(state, faction, character)?;
    if let Some(other) = office_of(state, faction, character) {
        if other == office {
            return Err(CouncilError::AlreadyHolder);
        }
        let name = rules
            .office(&other)
            .map_or(other, |o| o.name.display.clone());
        return Err(CouncilError::AlreadyHoldsOffice(name));
    }
    let penalty = rules.dismissal_loyalty_penalty;
    let council = &mut state.factions.get_mut(faction).expect("eligible").council;
    council.initialized = true;
    let replaced = council.holders.insert(office.to_owned(), character.clone());
    if let Some(old) = replaced {
        lose_loyalty(state, &old, penalty);
    }
    Ok(())
}

/// Validates and applies `DismissCouncillor`.
pub fn dismiss(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    office: &str,
) -> Result<(), CouncilError> {
    let rules = offices(data).ok_or(CouncilError::NoCouncil)?;
    if rules.office(office).is_none() {
        return Err(CouncilError::UnknownOffice(office.to_owned()));
    }
    let penalty = rules.dismissal_loyalty_penalty;
    let council = &mut state
        .factions
        .get_mut(faction)
        .ok_or(CouncilError::NotOfFaction)?
        .council;
    council.initialized = true;
    let old = council
        .holders
        .remove(office)
        .ok_or(CouncilError::AlreadyVacant)?;
    lose_loyalty(state, &old, penalty);
    Ok(())
}

fn lose_loyalty(state: &mut CampaignState, character: &CharacterId, points: u8) {
    if let Some(c) = state.characters.get_mut(character) {
        c.loyalty = c.loyalty.saturating_sub(points);
    }
}

/// Fills every vacant office of `faction` with the best eligible character
/// (by the office's skill, then by id), offices in file order. Deterministic.
pub fn auto_assign(state: &mut CampaignState, data: &GameData, faction: &FactionId) {
    let Some(rules) = offices(data) else {
        return;
    };
    for def in &rules.offices {
        if holder(state, faction, &def.id).is_some() {
            continue;
        }
        let best = candidates(state, data, faction, &def.id)
            .into_iter()
            .find(|c| c.current_office.is_none());
        let Some(f) = state.factions.get_mut(faction) else {
            return;
        };
        match best {
            Some(candidate) => {
                f.council
                    .holders
                    .insert(def.id.clone(), candidate.character);
            }
            None => {
                f.council.holders.remove(&def.id);
            }
        }
    }
}

/// New campaign: every living faction gets its automatic council.
pub fn init_councils(state: &mut CampaignState, data: &GameData) {
    ensure_councils(state, data);
}

/// Completes the factions whose council was never assigned (a save from
/// before the council, a faction created during play).
pub fn ensure_councils(state: &mut CampaignState, data: &GameData) {
    if data.council.is_none() {
        return;
    }
    let pending: Vec<FactionId> = state
        .factions
        .iter()
        .filter(|(_, f)| f.alive && !f.council.initialized)
        .map(|(id, _)| id.clone())
        .collect();
    for faction in pending {
        auto_assign(state, data, &faction);
        if let Some(f) = state.factions.get_mut(&faction) {
            f.council.initialized = true;
        }
    }
}

/// Total of `kind` over the sitting holders of `faction`.
pub fn effect_value(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    kind: CouncilEffectKind,
) -> f64 {
    let Some(rules) = offices(data) else {
        return 0.0;
    };
    rules
        .offices
        .iter()
        .filter_map(|def| {
            let who = holder(state, faction, &def.id)?;
            let skill = f64::from(skill_of(state, who, def.skill));
            Some(
                def.effects
                    .iter()
                    .filter(|e| e.kind == kind)
                    .map(|e| e.per_point * skill)
                    .sum::<f64>(),
            )
        })
        .sum()
}

/// Council effects on the realm as [`EffectTotals`]: tax income (percent),
/// unrest (flat, per province) and recruit experience (flat). Merged into
/// `research::faction_tech_effects`.
pub fn realm_effects(state: &CampaignState, data: &GameData, faction: &FactionId) -> EffectTotals {
    let mut totals = EffectTotals::default();
    if data.council.is_none() {
        return totals;
    }
    totals[EffectKind::TaxIncome].percent +=
        effect_value(state, data, faction, CouncilEffectKind::TaxIncomePercent);
    totals[EffectKind::Unrest].flat +=
        effect_value(state, data, faction, CouncilEffectKind::UnrestPerProvince);
    totals[EffectKind::ArmyExperience].flat +=
        effect_value(state, data, faction, CouncilEffectKind::RecruitExperience);
    totals
}

/// Adds the constable's morale to a character's effects (read for generals).
pub(crate) fn add_army_effects(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
    totals: &mut EffectTotals,
) {
    totals[EffectKind::ArmyMorale].flat +=
        effect_value(state, data, faction, CouncilEffectKind::ArmyMorale);
}

/// Acceptance points the chancellor of `faction` adds to its proposals.
pub fn diplomatic_points(state: &CampaignState, data: &GameData, faction: &FactionId) -> i32 {
    effect_value(
        state,
        data,
        faction,
        CouncilEffectKind::DiplomaticAcceptance,
    )
    .round() as i32
}

/// Signed figure with a decimal comma (`+3,5`, `+3`, `−0,3`).
pub fn format_value(value: f64) -> String {
    let rounded = (value * 10.0).round() / 10.0;
    let magnitude = format!("{:.1}", rounded.abs());
    let magnitude = magnitude
        .strip_suffix(".0")
        .unwrap_or(&magnitude)
        .replace('.', ",");
    if rounded < 0.0 {
        format!("−{magnitude}")
    } else {
        format!("+{magnitude}")
    }
}

/// French effect line of `office`: at `skill` for a holder, per skill point
/// when `skill` is `None` (a vacant seat). Effects are joined with « · ».
pub fn effect_text(def: &CouncilOfficeDef, skill: Option<u8>) -> String {
    def.effects
        .iter()
        .map(|e| match skill {
            Some(s) => e
                .text
                .replace("{value}", &format_value(e.per_point * f64::from(s))),
            None => format!(
                "{} par point de compétence",
                e.text.replace("{value}", &format_value(e.per_point))
            ),
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

/// Seasonal phase: assigns the councils not yet assigned, vacates the seats
/// whose holder no longer qualifies (journal line for the player), refills the
/// AI factions' vacancies, pays the chamberlain's prestige.
pub(crate) fn resolve_council(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let Some(rules) = offices(data) else {
        return;
    };
    ensure_councils(state, data);
    let factions: Vec<FactionId> = state
        .factions
        .iter()
        .filter(|(_, f)| f.alive)
        .map(|(id, _)| id.clone())
        .collect();
    for faction in factions {
        let lapsed: Vec<(String, CharacterId)> = state.factions[&faction]
            .council
            .holders
            .iter()
            .filter(|(office, id)| holder(state, &faction, office).is_none_or(|h| h != *id))
            .map(|(office, id)| (office.clone(), id.clone()))
            .collect();
        for (office, id) in lapsed {
            state
                .factions
                .get_mut(&faction)
                .expect("alive faction")
                .council
                .holders
                .remove(&office);
            if faction == state.player_faction {
                let office_name = rules
                    .office(&office)
                    .map_or(office.as_str(), |o| o.name.display.as_str());
                let who = state.character_name(data, &id);
                let cause = if state.characters.get(&id).is_some_and(|c| c.alive) {
                    "ne siège plus au conseil"
                } else {
                    "n'est plus"
                };
                events.push(
                    GameEvent::new(
                        EventKind::CouncilVacancy,
                        format!("{who} {cause} : la charge de {office_name} est vacante."),
                    )
                    .faction(&faction)
                    .loss(),
                );
            }
        }
        if faction != state.player_faction {
            auto_assign(state, data, &faction);
        }
        pay_prestige(state, data, &faction);
    }
}

/// The chamberlain's prestige of the season; fractions carry over.
fn pay_prestige(state: &mut CampaignState, data: &GameData, faction: &FactionId) {
    let earned = (effect_value(state, data, faction, CouncilEffectKind::PrestigePerSeason) * 100.0)
        .round()
        .max(0.0) as u32;
    if earned == 0 {
        return;
    }
    let council = &mut state.factions.get_mut(faction).expect("alive").council;
    council.prestige_hundredths += earned;
    let whole = council.prestige_hundredths / 100;
    council.prestige_hundredths %= 100;
    if whole > 0 {
        state.change_ruler_prestige(faction, whole as i32);
    }
}
