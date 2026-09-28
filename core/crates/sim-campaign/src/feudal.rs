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
    /// Effective `de_jure_liege` of the titles whose allegiance changed in
    /// play (homage, release, revolt): `None` makes the title sovereign.
    /// The data stay untouched (F1).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub liege_overrides: BTreeMap<TitleId, Option<TitleId>>,
    /// Remembered events weighing on vassal loyalty (§ 4.2), fed by F2/F3.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub loyalty_events: Vec<LoyaltyEvent>,
    /// `FactionState::suzerain` is derived from the titles. `false` in the
    /// version 7 saves written before F1: [`sync_suzerains`] then adopts
    /// their stored suzerains as liege overrides once.
    #[serde(default)]
    pub derived: bool,
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
            liege_overrides: BTreeMap::new(),
            loyalty_events: Vec::new(),
            derived: true,
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
        title = effective_liege(state, data, title)?;
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
        title = effective_liege(state, data, id);
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
            effective_liege(state, data, title).and_then(|liege| holder_of(state, liege))
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

// ----- F1: effective allegiance, homage, suzerain view, loyalty memory ----

/// Title that `title` currently depends on: the in-play override when
/// there is one, else its `de_jure_liege` in the data.
pub fn effective_liege<'a>(
    state: &'a CampaignState,
    data: &'a GameData,
    title: &TitleId,
) -> Option<&'a TitleId> {
    match state.feudal.liege_overrides.get(title) {
        Some(liege) => liege.as_ref(),
        None => data.titles.get(title)?.de_jure_liege.as_ref(),
    }
}

/// Sets the effective liege of `title`, dropping the override when it
/// matches the data again.
fn set_effective_liege(
    state: &mut CampaignState,
    data: &GameData,
    title: &TitleId,
    liege: Option<TitleId>,
) {
    let de_jure = data.titles.get(title).and_then(|t| t.de_jure_liege.clone());
    if de_jure == liege {
        state.feudal.liege_overrides.remove(title);
    } else {
        state.feudal.liege_overrides.insert(title.clone(), liege);
    }
}

/// Suzerains of `faction`, direct first, up to the sovereign.
pub fn liege_chain(state: &CampaignState, data: &GameData, faction: &FactionId) -> Vec<FactionId> {
    let mut chain: Vec<FactionId> = Vec::new();
    let mut current = faction.clone();
    for _ in 0..MAX_DEPTH {
        match liege_of(state, data, &current) {
            Some(liege) if &liege != faction && !chain.contains(&liege) => {
                chain.push(liege.clone());
                current = liege;
            }
            _ => break,
        }
    }
    chain
}

/// Refreshes `FactionState::suzerain`, the cached view of [`liege_of`]
/// (single source: the title holdings). A dead faction has no suzerain.
/// Saves from before F1 first have their stored suzerains adopted as
/// liege overrides, so that they load unchanged.
pub fn sync_suzerains(state: &mut CampaignState, data: &GameData) {
    if !state.feudal.derived {
        adopt_stored_suzerains(state, data);
        state.feudal.derived = true;
    }
    let lieges: Vec<(FactionId, Option<FactionId>)> = state
        .factions
        .iter()
        .map(|(id, f)| {
            let liege = if f.alive {
                liege_of(state, data, id)
            } else {
                None
            };
            (id.clone(), liege)
        })
        .collect();
    for (id, liege) in lieges {
        if let Some(f) = state.factions.get_mut(&id) {
            f.suzerain = liege;
        }
    }
}

/// Turns the `suzerain` fields of a pre-F1 save into liege overrides of the
/// primary titles.
fn adopt_stored_suzerains(state: &mut CampaignState, data: &GameData) {
    let stored: Vec<(FactionId, Option<FactionId>)> = state
        .factions
        .iter()
        .filter(|(_, f)| f.alive)
        .map(|(id, f)| (id.clone(), f.suzerain.clone()))
        .collect();
    for (faction, suzerain) in stored {
        if liege_of(state, data, &faction) == suzerain {
            continue;
        }
        let Some(own) = state.feudal.primary.get(&faction).cloned() else {
            continue;
        };
        let liege = suzerain.and_then(|s| state.feudal.primary.get(&s).cloned());
        set_effective_liege(state, data, &own, liege);
    }
}

/// `vassal` pays homage to `liege` (`Proposal::Vassalage`, spec § 4.2):
/// the effective liege of its primary title becomes `liege`'s primary
/// title. A `liege` that stood below `vassal` is first freed, so that the
/// hierarchy keeps no cycle. `false` when either has no primary title.
pub fn pay_homage(
    state: &mut CampaignState,
    data: &GameData,
    vassal: &FactionId,
    liege: &FactionId,
) -> bool {
    let (Some(own), Some(above)) = (
        state.feudal.primary.get(vassal).cloned(),
        state.feudal.primary.get(liege).cloned(),
    ) else {
        return false;
    };
    if liege_chain(state, data, liege).contains(vassal) {
        set_effective_liege(state, data, &above, None);
    }
    set_effective_liege(state, data, &own, Some(above));
    sync_suzerains(state, data);
    true
}

/// `vassal` leaves its suzerain (release, revolt, refused call, vanished
/// suzerain): its primary title becomes sovereign. Needs no game data, so
/// that the callers without it stay in sync.
pub fn release_from_liege(state: &mut CampaignState, vassal: &FactionId) {
    if let Some(own) = state.feudal.primary.get(vassal).cloned() {
        state.feudal.liege_overrides.insert(own, None);
    }
    if let Some(f) = state.factions.get_mut(vassal) {
        f.suzerain = None;
    }
}

/// Kind of a remembered event weighing on a vassal's loyalty (§ 4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoyaltyEventKind {
    ProtectionGranted,
    ProtectionRefused,
    TitleGranted,
    PeerForfeiture,
    LiegeDefeat,
    RivalClaimant,
}

/// A remembered event: counts towards the loyalty of `vassal` for `liege`
/// until `expires_turn`, as long as `liege` stays its direct suzerain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoyaltyEvent {
    pub vassal: FactionId,
    pub liege: FactionId,
    pub kind: LoyaltyEventKind,
    pub expires_turn: u32,
}

fn remember(
    state: &mut CampaignState,
    data: &GameData,
    vassal: &FactionId,
    liege: &FactionId,
    kind: LoyaltyEventKind,
) {
    let expires_turn = state.turn + data.feudal_rules.loyalty.memory_turns;
    state.feudal.loyalty_events.push(LoyaltyEvent {
        vassal: vassal.clone(),
        liege: liege.clone(),
        kind,
        expires_turn,
    });
}

/// `liege` answered (`granted`) or not the call of its attacked direct
/// vassal `vassal` (§ 4.3, fed by F2).
pub fn record_protection(
    state: &mut CampaignState,
    data: &GameData,
    liege: &FactionId,
    vassal: &FactionId,
    granted: bool,
) {
    let kind = if granted {
        LoyaltyEventKind::ProtectionGranted
    } else {
        LoyaltyEventKind::ProtectionRefused
    };
    remember(state, data, vassal, liege, kind);
}

/// `liege` granted a title to `vassal` (§ 4.4, 4.6, fed by F3).
pub fn record_title_grant(
    state: &mut CampaignState,
    data: &GameData,
    liege: &FactionId,
    vassal: &FactionId,
) {
    remember(state, data, vassal, liege, LoyaltyEventKind::TitleGranted);
}

/// Remembers `kind` for every direct vassal of `liege` but `except`.
fn remember_for_vassals(
    state: &mut CampaignState,
    data: &GameData,
    liege: &FactionId,
    except: Option<&FactionId>,
    kind: LoyaltyEventKind,
) {
    for vassal in direct_vassals(state, data, liege) {
        if Some(&vassal) != except {
            remember(state, data, &vassal, liege, kind);
        }
    }
}

/// `liege` struck its vassal `felon` with forfeiture: its other direct
/// vassals fear for their fiefs (§ 4.4, fed by F3).
pub fn record_peer_forfeiture(
    state: &mut CampaignState,
    data: &GameData,
    liege: &FactionId,
    felon: &FactionId,
) {
    remember_for_vassals(
        state,
        data,
        liege,
        Some(felon),
        LoyaltyEventKind::PeerForfeiture,
    );
}

/// `liege` lost a battle or a war: its direct vassals doubt it (fed by F2).
pub fn record_liege_defeat(state: &mut CampaignState, data: &GameData, liege: &FactionId) {
    remember_for_vassals(state, data, liege, None, LoyaltyEventKind::LiegeDefeat);
}

/// A rival claimant to `liege`'s primary title stands up (fed by F3).
pub fn record_rival_claimant(state: &mut CampaignState, data: &GameData, liege: &FactionId) {
    remember_for_vassals(state, data, liege, None, LoyaltyEventKind::RivalClaimant);
}

/// Forgets the expired loyalty events (each turn).
pub fn forget_expired(state: &mut CampaignState) {
    let turn = state.turn;
    state
        .feudal
        .loyalty_events
        .retain(|e| e.expires_turn > turn);
}

/// Loyalty points of `vassal` towards `liege` owed to the remembered
/// events (§ 4.2), weighted by `feudal.json:loyalty`.
pub fn remembered_loyalty(
    state: &CampaignState,
    data: &GameData,
    vassal: &FactionId,
    liege: &FactionId,
) -> i32 {
    let w = &data.feudal_rules.loyalty;
    state
        .feudal
        .loyalty_events
        .iter()
        .filter(|e| &e.vassal == vassal && &e.liege == liege && e.expires_turn > state.turn)
        .map(|e| match e.kind {
            LoyaltyEventKind::ProtectionGranted => w.protection_granted,
            LoyaltyEventKind::ProtectionRefused => w.protection_refused,
            LoyaltyEventKind::TitleGranted => w.title_granted,
            LoyaltyEventKind::PeerForfeiture => w.peer_forfeiture,
            LoyaltyEventKind::LiegeDefeat => w.liege_defeat,
            LoyaltyEventKind::RivalClaimant => w.rival_claimant,
        })
        .sum()
}
