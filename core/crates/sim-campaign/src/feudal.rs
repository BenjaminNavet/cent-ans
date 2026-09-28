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

use data_model::{CharacterId, FactionId, GameData, ProvinceId, TitleId};
use serde::{Deserialize, Serialize};

use crate::state::CampaignState;

mod felony;
mod inherit;
mod objectives;
mod transfer;

pub use felony::{
    has_forfeiture, on_host_refused, on_revolt, open_felony_towards, settle_forfeitures,
};
pub(crate) use inherit::{contested_succession, heir_comes_home, inherit_titles_on_extinction};
pub(crate) use objectives::resolve_feudal;
pub use objectives::{generic_victory, objective_status, GenericVictory};
pub use transfer::{
    conquer_title, grant_title, on_faction_destroyed, vacate_title, Grantee, TitleDemandOutcome,
};

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
    /// Forfeitures declared and not yet settled by a peace (§ 4.4, F3).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub forfeitures: Vec<Forfeiture>,
    /// Contested successions arbitrated so far (§ 4.5, F3), newest last.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub disputes: Vec<SuccessionDispute>,
    /// Crown above each faction's primary title in 1337: the factions that
    /// started as vassals, and the crown of their generic victory (§ 4.8).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub start_crowns: BTreeMap<FactionId, TitleId>,
    /// Consecutive turns towards the generic victories (§ 4.8, F3).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub streaks: BTreeMap<FactionId, VictoryStreaks>,
    /// Historical objectives already announced as met (id per faction).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub objectives_met: BTreeMap<FactionId, BTreeSet<String>>,
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
        let primary: BTreeMap<FactionId, TitleId> = data
            .factions
            .iter()
            .filter_map(|(id, f)| f.primary_title.clone().map(|t| (id.clone(), t)))
            .collect();
        let start_crowns = primary
            .iter()
            .filter_map(|(faction, title)| {
                let crown = objectives::crown_above(data, title)?;
                (crown != *title).then(|| (faction.clone(), crown))
            })
            .collect();
        FeudalState {
            holders: data
                .titles
                .iter()
                .map(|(id, title)| (id.clone(), title.holder_1337.faction.clone()))
                .collect(),
            primary,
            felonies: Vec::new(),
            forfeitures: Vec::new(),
            disputes: Vec::new(),
            start_crowns,
            streaks: BTreeMap::new(),
            objectives_met: BTreeMap::new(),
            liege_overrides: BTreeMap::new(),
            loyalty_events: Vec::new(),
            derived: true,
        }
    }
}

/// A forfeiture declared by `liege` against `vassal` (§ 4.4): the war runs
/// against the felon alone; at the peace, `titles` go to the liege if it won.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Forfeiture {
    pub liege: FactionId,
    pub vassal: FactionId,
    pub titles: Vec<TitleId>,
    pub declared_turn: u32,
}

/// A contested succession arbitrated by the suzerain (§ 4.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuccessionDispute {
    /// Faction whose ruler died.
    pub faction: FactionId,
    /// Its primary title, the prize.
    pub title: TitleId,
    pub claimants: Vec<CharacterId>,
    /// Suzerain who judged (holder of the title above).
    pub arbiter: FactionId,
    pub winner: CharacterId,
    /// Faction that took up the loser's cause and went to war, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sponsor: Option<FactionId>,
    pub turn: u32,
}

/// Consecutive turns spent on each generic victory path (§ 4.8).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VictoryStreaks {
    /// Turns without a suzerain.
    #[serde(default)]
    pub independent: u32,
    /// Turns as the most powerful direct vassal of the crown's holder.
    #[serde(default)]
    pub first_vassal: u32,
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
    #[error("faction inconnue ou disparue {0}")]
    DeadFaction(FactionId),
    #[error("{0} ne détient pas le titre {1}")]
    NotHolder(FactionId, TitleId),
    #[error("{0} ne tient aucun titre de {1}")]
    NotVassal(FactionId, FactionId),
    #[error("personnage inconnu, mort ou déjà souverain : {0}")]
    InvalidGrantee(CharacterId),
    #[error("guerre impossible : {0}")]
    War(String),
    #[error("on ne concède pas son titre principal {0}")]
    PrimaryTitle(TitleId),
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

/// Rank of the primary title of `faction`, if it holds any.
pub fn primary_rank(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Option<data_model::TitleRank> {
    state
        .feudal
        .primary
        .get(faction)
        .and_then(|t| data.titles.get(t))
        .map(|t| t.rank)
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
/// its primary title that it does not hold itself; `None` when sovereign,
/// or when that title is vacant (§ 4.7: the vassals of a vacant title are
/// independent, the maxim forbids reaching past it).
pub fn liege_of(state: &CampaignState, data: &GameData, faction: &FactionId) -> Option<FactionId> {
    let mut title = state.feudal.primary.get(faction)?;
    for _ in 0..MAX_DEPTH {
        title = effective_liege(state, data, title)?;
        match holder_of(state, title) {
            Some(holder) if holder != faction => return Some(holder.clone()),
            Some(_) => {}
            None => return None,
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

/// Opens a felony case of `vassal` against its direct suzerain (§ 4.4):
/// refused host, alliance with the suzerain's enemy or revolt. The case
/// stays open `feudal_rules.felony_window_turns` turns; `None` when the
/// vassal has no suzerain. See [`open_felony_towards`] for a title vassal
/// (England towards France through Guyenne).
pub fn open_felony(
    state: &mut CampaignState,
    data: &GameData,
    vassal: &FactionId,
    reason: FelonyReason,
) -> Option<FelonyCase> {
    let liege = liege_of(state, data, vassal)?;
    open_felony_towards(state, data, vassal, &liege, reason)
}

/// `liege` declares forfeiture against `vassal` (§ 4.4): needs an open
/// felony case; a casus belli against the felon alone. The titles `vassal`
/// holds of `liege` go to `liege` at the peace if it wins
/// ([`settle_forfeitures`]).
pub fn declare_commise(
    state: &mut CampaignState,
    data: &GameData,
    liege: &FactionId,
    vassal: &FactionId,
) -> Result<(), FeudalError> {
    felony::declare_commise(state, data, liege, vassal)
}

/// Gives `title` to `to` (§ 4.5-4.7): the title's own provinces owned by
/// the former holder follow it; `to` takes it as primary title if it is its
/// highest; a former holder left without any title vanishes into `to`
/// (personal union, forfeiture of a last fief). Events go to the journal of
/// the next turn.
pub fn transfer_title(
    state: &mut CampaignState,
    data: &GameData,
    title: &TitleId,
    to: &FactionId,
) -> Result<(), FeudalError> {
    let mut events = Vec::new();
    transfer::transfer(state, data, title, to, &mut events)?;
    for event in events {
        state.push_order_event(event);
    }
    Ok(())
}

/// Historical objectives of every living faction whose primary title has
/// some (§ 4.8), evaluated against the current state.
pub fn evaluate_objectives(state: &CampaignState, data: &GameData) -> Vec<ObjectiveProgress> {
    objectives::evaluate_objectives(state, data)
}

// =========================================================================
// War escalation and private war (§ 4.3, lot F2)
// =========================================================================

/// Opinion modifier of an attacked vassal whose suzerain came to its help
/// (worth `feudal.loyalty.protection_granted`, feeds the loyalty target).
pub const PROTECTION_GRANTED_REASON: &str = "Protection accordée par le suzerain";
/// Opinion modifier of an attacked vassal whose suzerain shirked
/// (worth `feudal.loyalty.protection_refused`).
pub const PROTECTION_REFUSED_REASON: &str = "Protection refusée par le suzerain";

/// Verdict of a lord on a private war between two of its direct vassals
/// (§ 4.3.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Arbitration {
    /// White peace and truce between the two vassals.
    ImposePeace,
    /// The lord goes to war against the other party.
    TakeSide { side: FactionId },
    /// The lord lets them settle their quarrel.
    LetBe,
}

/// Changes the loyalty of `vassal` towards its suzerain by `delta`, clamped
/// to 0-100 (a one-off shock; the loyalty then drifts back to its target).
pub fn adjust_loyalty(state: &mut CampaignState, vassal: &FactionId, delta: i32) {
    if let Some(f) = state.factions.get_mut(vassal) {
        f.loyalty = (i32::from(f.loyalty) + delta).clamp(0, 100) as u8;
    }
}

/// Lord of both `a` and `b` when they are direct vassals of the same
/// suzerain: their war is a private war (§ 4.3.5).
pub fn common_liege(
    state: &CampaignState,
    data: &GameData,
    a: &FactionId,
    b: &FactionId,
) -> Option<FactionId> {
    let lord = liege_of(state, data, a)?;
    (liege_of(state, data, b).as_ref() == Some(&lord)).then_some(lord)
}

/// Provisional AI score of `liege` called to protect `vassal` against
/// `aggressor` (§ 4.3.4: power, relation, treasury, ongoing wars), with
/// its main reason in French. Kept apart so that F5 replaces it.
pub fn protection_score(
    state: &CampaignState,
    data: &GameData,
    liege: &FactionId,
    vassal: &FactionId,
    aggressor: &FactionId,
) -> (i32, String) {
    let rules = &data.feudal_rules.escalation.score;
    let mut terms: Vec<(i32, String)> = vec![(
        rules.base,
        format!(
            "devoir de protection envers {}",
            faction_label(data, vassal)
        ),
    )];
    let ratio = state.faction_power(liege) / state.faction_power(aggressor).max(1.0);
    if ratio >= rules.power_ratio {
        terms.push((
            rules.power_favourable,
            "plus fort que l'agresseur".to_owned(),
        ));
    } else {
        terms.push((
            rules.power_unfavourable,
            "plus faible que l'agresseur".to_owned(),
        ));
    }
    let attitude = state.attitude(data, liege, vassal).0 / rules.attitude_divisor.max(1);
    let label = if attitude >= 0 {
        "bonne entente avec le vassal"
    } else {
        "mauvaise entente avec le vassal"
    };
    terms.push((attitude, label.to_owned()));
    if state.factions.get(liege).is_some_and(|f| f.treasury < 0) {
        terms.push((rules.empty_treasury, "trésor vide".to_owned()));
    }
    let wars = state.factions.get(liege).map_or(0, |f| {
        f.at_war_with
            .iter()
            .filter(|e| !crate::diplomacy::is_rebels(e))
            .count()
    });
    if wars > 0 {
        terms.push((
            rules.per_ongoing_war * wars as i32,
            format!("déjà engagé dans {wars} guerre(s)"),
        ));
    }
    if state.is_allied(liege, aggressor) {
        terms.push((
            rules.allied_with_aggressor,
            "allié de l'agresseur".to_owned(),
        ));
    }
    let score: i32 = terms.iter().map(|(v, _)| v).sum();
    let main = if score >= rules.intervene_at {
        terms.iter().max_by_key(|(v, _)| *v)
    } else {
        terms.iter().min_by_key(|(v, _)| *v)
    };
    let reason = main.map_or_else(String::new, |(_, r)| r.clone());
    (score, reason)
}

/// Likelihood shown for a protection score.
pub fn likelihood_of(data: &GameData, score: i32) -> Likelihood {
    let rules = &data.feudal_rules.escalation.score;
    if score >= rules.intervene_at + rules.certainty_margin {
        Likelihood::Likely
    } else if score < rules.intervene_at - rules.certainty_margin {
        Likelihood::Unlikely
    } else {
        Likelihood::Uncertain
    }
}

/// Provisional AI verdict of `lord` on the private war of `attacker`
/// against `target` (replaced in F5), with its reason in French.
pub fn ai_arbitration(
    state: &CampaignState,
    data: &GameData,
    lord: &FactionId,
    attacker: &FactionId,
    target: &FactionId,
) -> (Arbitration, String) {
    let rules = &data.feudal_rules.escalation.arbitration;
    let towards_attacker = state.attitude(data, lord, attacker).0;
    let towards_target = state.attitude(data, lord, target).0;
    for (favoured, other) in [
        (target, towards_target - towards_attacker),
        (attacker, towards_attacker - towards_target),
    ] {
        if other >= rules.take_side_attitude_gap {
            return (
                Arbitration::TakeSide {
                    side: favoured.clone(),
                },
                format!("préfère {}", faction_label(data, favoured)),
            );
        }
    }
    let ratio = state.faction_power(lord) / state.faction_power(attacker).max(1.0);
    if ratio >= rules.impose_peace_power_ratio {
        (
            Arbitration::ImposePeace,
            "assez fort pour imposer la paix".to_owned(),
        )
    } else {
        (
            Arbitration::LetBe,
            "trop faible pour imposer la paix".to_owned(),
        )
    }
}

fn faction_label(data: &GameData, id: &FactionId) -> String {
    crate::diplomacy::faction_name(data, id)
}

fn capitalized(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

/// Suzerains called, link by link, if `attacker` attacks `target` (§ 4.3):
/// each step assumes the previous suzerains intervened (the cascade stops
/// in fact at the first one who shirks). A private war gives a single step,
/// the arbitrating lord: `likely` when it would impose peace or side with
/// the target.
pub fn war_escalation_preview(
    state: &CampaignState,
    data: &GameData,
    attacker: &FactionId,
    target: &FactionId,
) -> Vec<EscalationStep> {
    if let Some(lord) = common_liege(state, data, attacker, target) {
        let (verdict, why) = ai_arbitration(state, data, &lord, attacker, target);
        let (likelihood, what) = match &verdict {
            Arbitration::ImposePeace => (Likelihood::Likely, "imposera la paix".to_owned()),
            Arbitration::TakeSide { side } if side == target => (
                Likelihood::Likely,
                format!("prendra le parti de {}", faction_label(data, target)),
            ),
            Arbitration::TakeSide { .. } => (
                Likelihood::Unlikely,
                format!("prendra le parti de {}", faction_label(data, attacker)),
            ),
            Arbitration::LetBe => (Likelihood::Unlikely, "laissera faire".to_owned()),
        };
        return vec![EscalationStep {
            faction: lord,
            likelihood,
            reason: format!("Guerre privée, l'arbitre {what} ({why})"),
        }];
    }
    let mut steps = Vec::new();
    let mut vassal = target.clone();
    for _ in 0..MAX_DEPTH {
        let Some(liege) = liege_of(state, data, &vassal) else {
            break;
        };
        if &liege == attacker {
            break;
        }
        if state.is_at_war(&liege, attacker) {
            steps.push(EscalationStep {
                faction: liege,
                likelihood: Likelihood::Likely,
                reason: "Déjà en guerre contre l'agresseur".to_owned(),
            });
            break;
        }
        let (score, why) = protection_score(state, data, &liege, &vassal, attacker);
        steps.push(EscalationStep {
            faction: liege.clone(),
            likelihood: likelihood_of(data, score),
            reason: capitalized(&why),
        });
        vassal = liege;
    }
    steps
}

/// Entry point from `declare_war`: a private war is arbitrated by the
/// common lord, any other attack calls the target's suzerain (§ 4.3).
pub(crate) fn escalate_war(
    state: &mut CampaignState,
    data: &GameData,
    attacker: &FactionId,
    target: &FactionId,
) {
    use crate::diplomacy::is_rebels;
    if is_rebels(attacker) || is_rebels(target) {
        return;
    }
    let Some(lord) = common_liege(state, data, attacker, target) else {
        call_liege(state, data, target, attacker);
        return;
    };
    if lord == state.player_faction {
        let text = format!(
            "Guerre privée : {} attaque {}, tous deux vos vassaux. Accepter : imposer la paix ; \
             refuser : laisser faire ; ou prendre parti.",
            faction_label(data, attacker),
            faction_label(data, target)
        );
        let proposal = crate::diplomacy::Proposal::Arbitration {
            attacker: attacker.clone(),
            target: target.clone(),
        };
        push_feudal_offer(state, data, target, proposal, text);
    } else {
        let (verdict, _) = ai_arbitration(state, data, &lord, attacker, target);
        apply_arbitration(state, data, &lord, attacker, target, &verdict);
    }
}

/// The direct suzerain of `vassal` is called to protect it against
/// `aggressor`: the player gets an offer, the AI answers at once.
pub(crate) fn call_liege(
    state: &mut CampaignState,
    data: &GameData,
    vassal: &FactionId,
    aggressor: &FactionId,
) {
    let Some(liege) = liege_of(state, data, vassal) else {
        return;
    };
    if &liege == aggressor
        || !state.factions.get(&liege).is_some_and(|f| f.alive)
        || state.is_at_war(&liege, aggressor)
    {
        return;
    }
    if liege == state.player_faction {
        let text = format!(
            "{} est attaqué par {} et réclame votre protection. Accepter : entrer en guerre et \
             convoquer l'ost de vos vassaux ; refuser : vous dérober (prestige, loyauté).",
            faction_label(data, vassal),
            faction_label(data, aggressor)
        );
        let proposal = crate::diplomacy::Proposal::Protection {
            aggressor: aggressor.clone(),
        };
        push_feudal_offer(state, data, vassal, proposal, text);
        return;
    }
    let (score, _) = protection_score(state, data, &liege, vassal, aggressor);
    if score >= data.feudal_rules.escalation.score.intervene_at {
        intervene(state, data, &liege, vassal, aggressor);
    } else {
        shirk(state, data, &liege, vassal, aggressor);
    }
}

/// Offer to the player, outside the diplomatic cooldown (a feudal call
/// cannot be missed).
fn push_feudal_offer(
    state: &mut CampaignState,
    data: &GameData,
    from: &FactionId,
    proposal: crate::diplomacy::Proposal,
    text: String,
) {
    use crate::events::{EventKind, GameEvent};
    let player = state.player_faction.clone();
    let id = state.next_offer_id;
    state.next_offer_id += 1;
    let expires_turn = state.turn + data.feudal_rules.escalation.answer_turns + 1;
    if let Some(f) = state.factions.get_mut(&player) {
        f.offers.push(crate::diplomacy::Offer {
            id,
            from: from.clone(),
            proposal,
            expires_turn,
            text_fr: text.clone(),
        });
    }
    state.push_order_event(GameEvent::new(EventKind::DiplomaticOffer, text).faction(from));
}

/// `liege` protects `vassal`: it enters the war against `aggressor`,
/// summons the host of its own direct vassals, then its own suzerain is
/// called in turn (cascade).
pub(crate) fn intervene(
    state: &mut CampaignState,
    data: &GameData,
    liege: &FactionId,
    vassal: &FactionId,
    aggressor: &FactionId,
) {
    use crate::events::{EventKind, GameEvent};
    let alive = |s: &CampaignState, f: &FactionId| s.factions.get(f).is_some_and(|f| f.alive);
    if !alive(state, aggressor) || !alive(state, liege) || state.is_at_war(liege, aggressor) {
        return;
    }
    let rules = &data.feudal_rules.escalation;
    state.start_war(liege, aggressor);
    state.change_ruler_prestige(liege, rules.intervene_prestige);
    state.add_modifier(
        vassal,
        liege,
        data.feudal_rules.loyalty.protection_granted,
        PROTECTION_GRANTED_REASON,
        rules.protection_memory_turns,
    );
    let text = format!(
        "{} accourt au secours de son vassal {} contre {}.",
        faction_label(data, liege),
        faction_label(data, vassal),
        faction_label(data, aggressor)
    );
    state.push_order_event(GameEvent::new(EventKind::WarDeclared, text).faction(liege));
    summon_host(state, data, liege, aggressor);
    call_liege(state, data, liege, aggressor);
}

/// `liege` shirks the protection of `vassal`: its ruler loses prestige,
/// every direct vassal loses loyalty, the attacked vassal remembers it.
pub(crate) fn shirk(
    state: &mut CampaignState,
    data: &GameData,
    liege: &FactionId,
    vassal: &FactionId,
    aggressor: &FactionId,
) {
    use crate::events::{EventKind, GameEvent};
    let rules = &data.feudal_rules.escalation;
    state.change_ruler_prestige(liege, rules.shirk_prestige);
    for v in direct_vassals(state, data, liege) {
        adjust_loyalty(state, &v, -i32::from(rules.shirk_loyalty_drop));
    }
    state.add_modifier(
        vassal,
        liege,
        data.feudal_rules.loyalty.protection_refused,
        PROTECTION_REFUSED_REASON,
        rules.protection_memory_turns,
    );
    let text = format!(
        "{} se dérobe et laisse son vassal {} seul face à {}.",
        faction_label(data, liege),
        faction_label(data, vassal),
        faction_label(data, aggressor)
    );
    state.push_order_event(GameEvent::new(EventKind::AllianceBroken, text).faction(liege));
}

/// `liege` summons the host of its direct vassals against `enemy` (never
/// its rear vassals). A vassal below `call_to_arms_loyalty` refuses, which
/// opens a felony case (§ 4.4).
pub fn summon_host(
    state: &mut CampaignState,
    data: &GameData,
    liege: &FactionId,
    enemy: &FactionId,
) {
    use crate::events::{EventKind, GameEvent};
    for vassal in direct_vassals(state, data, liege) {
        if &vassal == enemy || state.is_at_war(&vassal, enemy) || state.is_allied(&vassal, enemy) {
            continue;
        }
        if state.factions[&vassal].loyalty >= data.feudal_rules.call_to_arms_loyalty {
            state.start_war(&vassal, enemy);
            let text = format!(
                "{} répond à l'ost de son suzerain {} contre {}.",
                faction_label(data, &vassal),
                faction_label(data, liege),
                faction_label(data, enemy)
            );
            state.push_order_event(GameEvent::new(EventKind::WarDeclared, text).faction(&vassal));
        } else {
            let text = format!(
                "{} refuse l'ost de son suzerain {}.",
                faction_label(data, &vassal),
                faction_label(data, liege)
            );
            state.push_order_event(GameEvent::new(EventKind::Diplomacy, text).faction(&vassal));
            felony::on_host_refused(state, data, &vassal, liege);
        }
    }
}

/// Applies the verdict of `lord` on the private war of `attacker` against
/// `target`.
pub(crate) fn apply_arbitration(
    state: &mut CampaignState,
    data: &GameData,
    lord: &FactionId,
    attacker: &FactionId,
    target: &FactionId,
    verdict: &Arbitration,
) {
    use crate::events::{EventKind, GameEvent};
    let rules = &data.feudal_rules.escalation.arbitration;
    if !state.is_at_war(attacker, target) {
        return; // settled meanwhile
    }
    let (kind, text) = match verdict {
        Arbitration::ImposePeace => {
            state.make_peace_between(data, attacker, target, &[], 0, rules.truce_turns);
            adjust_loyalty(
                state,
                attacker,
                -i32::from(rules.imposed_peace_loyalty_drop),
            );
            (
                EventKind::PeaceSigned,
                format!(
                    "{} impose la paix à ses vassaux {} et {}.",
                    faction_label(data, lord),
                    faction_label(data, attacker),
                    faction_label(data, target)
                ),
            )
        }
        Arbitration::TakeSide { side } => {
            let other = if side == attacker { target } else { attacker };
            if !state.is_at_war(lord, other) {
                state.start_war(lord, other);
            }
            adjust_loyalty(state, other, -i32::from(rules.opposed_loyalty_drop));
            (
                EventKind::WarDeclared,
                format!(
                    "{} prend le parti de {} contre {}.",
                    faction_label(data, lord),
                    faction_label(data, side),
                    faction_label(data, other)
                ),
            )
        }
        Arbitration::LetBe => (
            EventKind::Diplomacy,
            format!(
                "{} laisse {} et {} vider leur querelle.",
                faction_label(data, lord),
                faction_label(data, attacker),
                faction_label(data, target)
            ),
        ),
    };
    state.push_order_event(GameEvent::new(kind, text).faction(lord));
}

/// The player refused (or let expire) a feudal call: a call for protection
/// is shirked, an arbitration is left alone.
pub(crate) fn refuse_feudal_call(
    state: &mut CampaignState,
    data: &GameData,
    player: &FactionId,
    offer: &crate::diplomacy::Offer,
) {
    use crate::diplomacy::Proposal;
    match &offer.proposal {
        Proposal::Protection { aggressor } => {
            if state.is_at_war(&offer.from, aggressor) {
                shirk(state, data, player, &offer.from, aggressor);
            }
        }
        Proposal::Arbitration { attacker, target } => {
            apply_arbitration(state, data, player, attacker, target, &Arbitration::LetBe);
        }
        _ => {}
    }
}

impl CampaignState {
    /// The player, lord of both parties of a private war, answers the
    /// arbitration offer `offer_id` with `verdict` (§ 4.3.5).
    pub fn arbitrate(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        offer_id: u32,
        verdict: Arbitration,
    ) -> Result<(), crate::diplomacy::DiplomacyError> {
        use crate::diplomacy::{DiplomacyError, Proposal};
        let offers = &self
            .factions
            .get(faction)
            .ok_or(DiplomacyError::UnknownOffer)?
            .offers;
        let index = offers
            .iter()
            .position(|o| o.id == offer_id)
            .ok_or(DiplomacyError::UnknownOffer)?;
        let Proposal::Arbitration { attacker, target } = offers[index].proposal.clone() else {
            return Err(DiplomacyError::Refused(
                "cette offre n'est pas un arbitrage".to_owned(),
            ));
        };
        if let Arbitration::TakeSide { side } = &verdict {
            if side != &attacker && side != &target {
                return Err(DiplomacyError::Refused(
                    "on ne prend parti que pour l'une des deux parties".to_owned(),
                ));
            }
        }
        self.factions
            .get_mut(faction)
            .expect("checked")
            .offers
            .remove(index);
        apply_arbitration(self, data, faction, &attacker, &target, &verdict);
        Ok(())
    }
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
            LoyaltyEventKind::TitleGranted => w.title_granted,
            LoyaltyEventKind::PeerForfeiture => w.peer_forfeiture,
            LoyaltyEventKind::LiegeDefeat => w.liege_defeat,
            LoyaltyEventKind::RivalClaimant => w.rival_claimant,
        })
        .sum()
}

/// Tribute `vassal` owes this turn, and to whom: its direct suzerain only,
/// never the suzerain's own lieges (spec § 4.1, `feudal.json:vassal_tribute_percent`).
pub fn tribute_due(
    state: &CampaignState,
    data: &GameData,
    vassal: &FactionId,
) -> Option<(FactionId, i64)> {
    let liege = liege_of(state, data, vassal)?;
    let income = state.factions.get(vassal)?.income_last_turn;
    let amount = (income * data.feudal_rules.vassal_tribute_percent / 100).max(0);
    Some((liege, amount))
}
