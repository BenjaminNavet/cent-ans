//! Feudal titles above factions (lot FE, ADR 0098, spec
//! `docs/superpowers/specs/2026-09-28-feodalite-design.md`).
//!
//! The *de jure* hierarchy lives in the data (`data/titles/`); the campaign
//! only stores who holds which title ([`FeudalState`]). Suzerains, vassals
//! and province allegiances are deduced from it, never stored (§ 3.3).
//!
//! F0 provides the state and the deductions; escalation (F2), felony,
//! forfeiture, transfers and objectives (F3) are filled by later lots.

use std::collections::{BTreeMap, BTreeSet};

use data_model::{FactionId, GameData, ProvinceId, TitleId};
use serde::{Deserialize, Serialize};

use crate::state::CampaignState;

/// Guard against malformed hierarchies (the data allows three levels).
const MAX_DEPTH: usize = 4;

/// Who holds which title, and each faction's primary title.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeudalState {
    /// Current holder of every title.
    #[serde(default)]
    pub holders: BTreeMap<TitleId, FactionId>,
    /// Primary title of every faction that still holds one.
    #[serde(default)]
    pub primary: BTreeMap<FactionId, TitleId>,
    /// Open felony cases (§ 4.4), filled from F2/F3.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub felonies: Vec<FelonyCase>,
}

impl FeudalState {
    /// The spring 1337 holdings of `data`.
    pub fn from_data(data: &GameData) -> Self {
        FeudalState {
            holders: data
                .titles
                .iter()
                .map(|(id, title)| (id.clone(), title.holder_1337.faction.clone()))
                .collect(),
            primary: data
                .factions
                .iter()
                .filter_map(|(id, f)| f.primary_title.clone().map(|t| (id.clone(), t)))
                .collect(),
            felonies: Vec::new(),
        }
    }
}

/// Why a felony case was opened (§ 4.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FelonyReason {
    RefusedHost,
    AlliedWithEnemy,
    Revolt,
}

/// An open felony case: `liege` may declare forfeiture against `vassal`
/// until `expires_turn`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FelonyCase {
    pub vassal: FactionId,
    pub liege: FactionId,
    pub reason: FelonyReason,
    pub expires_turn: u32,
}

/// Errors of the feudal actions.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FeudalError {
    #[error("aucun cas de commise ouvert contre {0}")]
    NoFelonyCase(FactionId),
    #[error("titre inconnu {0}")]
    UnknownTitle(TitleId),
}

/// Estimated answer of a suzerain called to war (§ 4.3, F2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Likelihood {
    Likely,
    Uncertain,
    Unlikely,
}

/// One link of the escalation chain shown before a declaration of war.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EscalationStep {
    pub faction: FactionId,
    pub likelihood: Likelihood,
    /// Main reason, in French, for the UI.
    pub reason: String,
}

/// A faction, its titles and its direct vassals, recursively.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeudalNode {
    pub faction: FactionId,
    pub titles: Vec<TitleId>,
    pub vassals: Vec<FeudalNode>,
}

/// Progress of one historical objective of a faction (F3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectiveProgress {
    pub faction: FactionId,
    pub objective: String,
    pub met: bool,
}

/// Current holder of `title`.
pub fn holder_of<'s>(state: &'s CampaignState, title: &TitleId) -> Option<&'s FactionId> {
    state.feudal.holders.get(title)
}

/// Titles held by `faction`, primary first.
pub fn titles_of(state: &CampaignState, faction: &FactionId) -> Vec<TitleId> {
    let primary = state.feudal.primary.get(faction);
    let mut titles: Vec<TitleId> = primary.into_iter().cloned().collect();
    titles.extend(
        state
            .feudal
            .holders
            .iter()
            .filter(|(t, h)| *h == faction && Some(*t) != primary)
            .map(|(t, _)| t.clone()),
    );
    titles
}

/// Direct suzerain of `faction` (§ 3.3): holder of the first title above
/// its primary title that it does not hold itself; `None` when sovereign.
pub fn liege_of(state: &CampaignState, data: &GameData, faction: &FactionId) -> Option<FactionId> {
    let mut title = state.feudal.primary.get(faction)?;
    for _ in 0..MAX_DEPTH {
        title = data.titles.get(title)?.de_jure_liege.as_ref()?;
        match holder_of(state, title) {
            Some(holder) if holder != faction => return Some(holder.clone()),
            _ => {}
        }
    }
    None
}

/// Title whose own domain contains `province`.
pub fn title_of_province<'d>(data: &'d GameData, province: &ProvinceId) -> Option<&'d TitleId> {
    data.titles
        .values()
        .find(|t| t.de_jure_provinces.contains(province))
        .map(|t| &t.id)
}

/// Allegiance chain of `province` (§ 3.3): holder of its title first, then
/// the holders of the titles above, without consecutive repeats. Guyenne in
/// 1337 gives `[England, France]`.
pub fn province_lieges(
    state: &CampaignState,
    data: &GameData,
    province: &ProvinceId,
) -> Vec<FactionId> {
    let mut chain: Vec<FactionId> = Vec::new();
    let mut title = title_of_province(data, province);
    for _ in 0..MAX_DEPTH {
        let Some(id) = title else { break };
        if let Some(holder) = holder_of(state, id) {
            if chain.last() != Some(holder) {
                chain.push(holder.clone());
            }
        }
        title = data.titles.get(id).and_then(|t| t.de_jure_liege.as_ref());
    }
    chain
}

/// Factions whose direct suzerain is `faction` (the maxim: never their
/// own vassals).
pub fn direct_vassals(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Vec<FactionId> {
    state
        .feudal
        .primary
        .keys()
        .filter(|f| *f != faction && state.factions.get(*f).is_some_and(|s| s.alive))
        .filter(|f| liege_of(state, data, f).as_ref() == Some(faction))
        .cloned()
        .collect()
}

/// Factions holding a title directly below one held by `faction`, primary
/// or not: England for France through Guyenne (double allegiance).
pub fn title_vassals(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> BTreeSet<FactionId> {
    state
        .feudal
        .holders
        .iter()
        .filter(|(_, holder)| *holder != faction)
        .filter(|(title, _)| {
            data.titles
                .get(*title)
                .and_then(|t| t.de_jure_liege.as_ref())
                .and_then(|liege| holder_of(state, liege))
                == Some(faction)
        })
        .map(|(_, holder)| holder.clone())
        .collect()
}

/// Feudal tree below `faction`.
pub fn feudal_tree(state: &CampaignState, data: &GameData, faction: &FactionId) -> FeudalNode {
    fn build(
        state: &CampaignState,
        data: &GameData,
        faction: &FactionId,
        depth: usize,
    ) -> FeudalNode {
        let vassals = if depth < MAX_DEPTH {
            direct_vassals(state, data, faction)
                .iter()
                .map(|v| build(state, data, v, depth + 1))
                .collect()
        } else {
            Vec::new()
        };
        FeudalNode {
            faction: faction.clone(),
            titles: titles_of(state, faction),
            vassals,
        }
    }
    build(state, data, faction, 0)
}

/// Suzerains called, link by link, if `attacker` attacks `target` (§ 4.3).
/// Filled by F2.
pub fn war_escalation_preview(
    _state: &CampaignState,
    _data: &GameData,
    _attacker: &FactionId,
    _target: &FactionId,
) -> Vec<EscalationStep> {
    Vec::new()
}

/// Opens a felony case of `vassal` against its direct suzerain (§ 4.4).
/// Filled by F3.
pub fn open_felony(
    _state: &mut CampaignState,
    _data: &GameData,
    _vassal: &FactionId,
    _reason: FelonyReason,
) -> Option<FelonyCase> {
    None
}

/// `liege` declares forfeiture against `vassal` (§ 4.4). Filled by F3.
pub fn declare_commise(
    _state: &mut CampaignState,
    _data: &GameData,
    _liege: &FactionId,
    vassal: &FactionId,
) -> Result<(), FeudalError> {
    Err(FeudalError::NoFelonyCase(vassal.clone()))
}

/// Gives `title` to `to` (§ 4.5-4.7). F0 only moves the holding; unions,
/// new and vanished factions, vacant liege titles come with F3.
pub fn transfer_title(
    state: &mut CampaignState,
    data: &GameData,
    title: &TitleId,
    to: &FactionId,
) -> Result<(), FeudalError> {
    if !data.titles.contains_key(title) {
        return Err(FeudalError::UnknownTitle(title.clone()));
    }
    state.feudal.holders.insert(title.clone(), to.clone());
    Ok(())
}

/// Historical objectives of every faction (§ 4.8). Filled by F3.
pub fn evaluate_objectives(_state: &CampaignState, _data: &GameData) -> Vec<ObjectiveProgress> {
    Vec::new()
}
