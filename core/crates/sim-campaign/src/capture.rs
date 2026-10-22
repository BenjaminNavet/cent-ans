//! Fate of a captured place (spec
//! `docs/design/2026-09-28-tw2-mecaniques-total-war.md` § T1, ADR 0101).
//!
//! Every capture goes through [`crate::siege::capture`], which hands the
//! place over (occupation: capture unrest) and then calls [`on_captured`]:
//! - an AI captor picks an outcome at once ([`ai_choice`], deterministic
//!   scores from `data/rules/capture.json`);
//! - the player gets a [`PendingCapture`], answered with the
//!   `choose_capture_outcome` order; unanswered at the end of the turn, the
//!   place is simply occupied (nothing more happens).
//!
//! Occupy adds nothing to the occupation. Ransom, sack and raze add their
//! effects on top of it (gold, unrest, population, devastation, buildings,
//! experience, piety, papal favour, opinions); raze also costs the place a
//! fortification level, or leaves it a ruin (no recruitment, no
//! construction for `ruin_turns` turns). Razing is refused on province cities
//! and on the emblematic places listed in the data.

use data_model::key_enum;
use std::collections::BTreeMap;

use data_model::{
    BuildingId, CaptureRules, FactionId, GameData, OutcomeRules, OutcomeScores, ProvinceId,
    SettlementId, SettlementKind,
};
use serde::{Deserialize, Serialize};

use crate::diplomacy::REBELS_FACTION;
use crate::economy::province_base_income;
use crate::economy_balance::signed_livres;
use crate::events::{EventKind, GameEvent};
use crate::state::{ArmyId, CampaignState};

key_enum! {
/// The four fates of a captured place, in the order of the UI (the first is
/// the default).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureOutcome {
    /// « Occuper »: the place changes hands (capture unrest only).
    Occupy => "occupy",
    /// « Mettre à rançon » (appatis): moderate gold, more unrest.
    Ransom => "ransom",
    /// « Piller »: much gold, people and buildings lost, piety and
    /// reputation lost, the troops gain experience.
    Sack => "sack",
    /// « Raser »: little gold, the place loses a level or becomes a ruin,
    /// heavy diplomatic malus.
    Raze => "raze",
    /// « Remettre la place » (M2): a feudal gift to an ally or vassal.
    Gift => "gift",
    /// « Libérer la place » (M2): back to its rightful holder.
    Liberate => "liberate",
}
}

impl CaptureOutcome {
    /// French label of the choice.
    pub fn label_fr(self) -> &'static str {
        match self {
            CaptureOutcome::Occupy => "Occuper la place",
            CaptureOutcome::Ransom => "Mettre la place à rançon",
            CaptureOutcome::Sack => "Livrer la place au pillage",
            CaptureOutcome::Raze => "Raser et brûler la place",
            CaptureOutcome::Gift => "Remettre la place à un allié",
            CaptureOutcome::Liberate => "Libérer la place",
        }
    }

    fn score(self, scores: &OutcomeScores) -> i32 {
        match self {
            CaptureOutcome::Occupy => scores.occupy,
            CaptureOutcome::Ransom => scores.ransom,
            CaptureOutcome::Sack => scores.sack,
            CaptureOutcome::Raze => scores.raze,
            CaptureOutcome::Gift => scores.gift,
            CaptureOutcome::Liberate => scores.liberate,
        }
    }
}

/// A capture waiting for the player's choice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingCapture {
    pub id: u32,
    pub settlement: SettlementId,
    /// The captor (the player).
    pub faction: FactionId,
    /// Controller of the place before the capture.
    pub previous: FactionId,
    /// Armies of the captor on the place at the capture (experience of a sack).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub armies: Vec<ArmyId>,
    pub turn: u32,
}

/// Capture part of the campaign state (`#[serde(default)]`: absent from
/// older saves, no change of [`crate::state::STATE_VERSION`]).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CaptureState {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending: Vec<PendingCapture>,
    #[serde(default)]
    pub next_id: u32,
    /// M2: banditry after a sack or a raze, by province.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub banditry: BTreeMap<ProvinceId, Banditry>,
    /// Razed places left in ruins, until the given turn (exclusive).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub ruins: BTreeMap<SettlementId, u32>,
}

/// Temporary extra devastation of a province after a sack or a raze (M2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Banditry {
    /// Devastation points to take back at the end.
    pub points: u8,
    /// Turn at which the banditry ends (exclusive).
    pub until: u32,
}

/// Why a `choose_capture_outcome` order was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CaptureError {
    #[error("aucune place prise n'attend ce choix : {0}")]
    UnknownDecision(u32),
    #[error("la place n'est plus entre vos mains")]
    PlaceLost,
    #[error("impossible de raser cette place : {0}")]
    RazeForbidden(String),
    #[error("impossible de céder cette place : {0}")]
    CessionForbidden(String),
}

/// Figures of one outcome for one place (the preview shown to the player,
/// and exactly what [`apply_outcome`] does).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CaptureEffects {
    /// Livres taken.
    pub gold: i64,
    /// Unrest of the province: the occupation's plus the outcome's.
    pub unrest: u8,
    /// Part of `unrest` added by the outcome (the occupation's is applied
    /// at the capture).
    pub extra_unrest: u8,
    pub population_percent: i32,
    /// Heads lost (estimate at the preview).
    pub population_lost: u64,
    pub devastation: u8,
    /// M2: temporary devastation of the banditry (included in `devastation`).
    pub banditry_devastation: u8,
    pub banditry_turns: u32,
    pub buildings_lost: Vec<BuildingId>,
    /// Fortification levels lost (raze).
    pub fortification_loss: u8,
    /// The place becomes a ruin (raze of a place left at level 0).
    pub becomes_ruin: bool,
    pub ruin_turns: u32,
    pub unit_experience: u8,
    pub ruler_piety: i32,
    pub papal_favor: i32,
    pub victim_opinion: i32,
    pub others_opinion: i32,
    /// M2 (gift, liberate): opinion of the beneficiary (the recipient, or
    /// the former holder) towards the giver.
    pub recipient_opinion: i32,
    /// M2 (liberate): opinion of the beneficiary's allies.
    pub allies_opinion: i32,
    /// M2 (liberate): prestige of the giver's ruler.
    pub prestige: i32,
}

/// One choice of a pending capture, for the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureOptionView {
    pub outcome: CaptureOutcome,
    pub label: String,
    /// French summary of the effects, one per line.
    pub effects_text: String,
    pub allowed: bool,
    /// Why the choice is refused (raze of a city or an emblematic place).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub effects: CaptureEffects,
    /// M2 (gift, liberate): the beneficiary proposed by default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipient: Option<FactionId>,
    /// M2 (gift): every faction the place can be given to.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recipients: Vec<FactionId>,
}

/// A pending capture, for the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureDecisionView {
    pub id: u32,
    pub settlement: SettlementId,
    pub settlement_name: String,
    pub province: ProvinceId,
    pub province_name: String,
    pub kind: SettlementKind,
    pub previous_name: String,
    pub title: String,
    pub text: String,
    pub options: Vec<CaptureOptionView>,
}

fn scaled_u8(value: u8, share: f64) -> u8 {
    (f64::from(value) * share).round().clamp(0.0, 100.0) as u8
}

/// Scale of the outcome's figures for this kind of place.
fn place_share(rules: &CaptureRules, kind: SettlementKind) -> f64 {
    rules.place_share.get(&kind).copied().unwrap_or(1.0)
}

/// `true` when `settlement` is the city of its province.
fn is_city(state: &CampaignState, settlement: &SettlementId) -> bool {
    state
        .settlements
        .get(settlement)
        .is_some_and(|s| state.province_city_id(&s.province) == Some(settlement))
}

/// M2: the culture of the place's province is not the controller's (language,
/// customary law): the occupation stirs more unrest.
pub fn is_foreign_culture(state: &CampaignState, data: &GameData, settlement: &SettlementId) -> bool {
    let Some(place) = state.settlements.get(settlement) else {
        return false;
    };
    let province_culture = data.provinces.get(&place.province).map(|p| &p.culture);
    let faction_culture = data.factions.get(&place.controller).map(|f| &f.culture);
    province_culture.is_some() && faction_culture.is_some() && province_culture != faction_culture
}

/// Extra unrest of a foreign culture for `outcome`.
fn foreign_extra(data: &GameData, outcome: CaptureOutcome) -> u8 {
    let extra = &data.capture_rules.foreign_culture_extra;
    match outcome {
        CaptureOutcome::Occupy => extra.occupy,
        CaptureOutcome::Ransom => extra.ransom,
        CaptureOutcome::Sack => extra.sack,
        CaptureOutcome::Raze => extra.raze,
        CaptureOutcome::Gift | CaptureOutcome::Liberate => 0,
    }
}

/// Unrest of the occupation (applied by every capture): the base of the
/// kind of place plus, for a foreign culture, `foreign_culture_extra.occupy`.
pub fn occupation_unrest(state: &CampaignState, data: &GameData, settlement: &SettlementId) -> u8 {
    let occupy = &data.capture_rules.occupy;
    let base = if is_city(state, settlement) {
        occupy.unrest_city
    } else {
        occupy.unrest_place
    };
    if is_foreign_culture(state, data, settlement) {
        base.saturating_add(foreign_extra(data, CaptureOutcome::Occupy))
            .min(100)
    } else {
        base
    }
}

/// Why `settlement` cannot be razed (`None`: it can).
pub fn raze_refusal(
    state: &CampaignState,
    data: &GameData,
    settlement: &SettlementId,
) -> Option<String> {
    let rules = &data.capture_rules;
    let kind = state.settlement_kind(settlement);
    if is_city(state, settlement) || rules.raze_forbidden_kinds.contains(&kind) {
        return Some("on ne rase pas la cité d'une province".to_owned());
    }
    if rules.raze_forbidden_settlements.contains(settlement) {
        return Some("lieu trop vénéré pour être livré aux flammes".to_owned());
    }
    None
}

/// `true` while a razed place lies in ruins (no recruitment, no building).
pub fn is_ruined(state: &CampaignState, settlement: &SettlementId) -> bool {
    state
        .captures
        .ruins
        .get(settlement)
        .is_some_and(|until| *until > state.turn)
}

/// The rules of an outcome's extra effects (`None` for occupy).
fn outcome_rules(rules: &CaptureRules, outcome: CaptureOutcome) -> Option<&OutcomeRules> {
    match outcome {
        CaptureOutcome::Occupy => None,
        CaptureOutcome::Ransom => Some(&rules.ransom),
        CaptureOutcome::Sack => Some(&rules.sack),
        CaptureOutcome::Raze => Some(&rules.raze.effects),
        CaptureOutcome::Gift | CaptureOutcome::Liberate => None,
    }
}

/// What `outcome` would do to `settlement` now (preview and application).
pub fn preview(
    state: &CampaignState,
    data: &GameData,
    settlement: &SettlementId,
    outcome: CaptureOutcome,
) -> CaptureEffects {
    let rules = &data.capture_rules;
    let occupation = occupation_unrest(state, data, settlement);
    let Some(place) = state.settlements.get(settlement) else {
        return CaptureEffects::default();
    };
    match outcome {
        // Neither a gift nor a liberation stirs unrest: the occupation's is undone.
        CaptureOutcome::Gift => {
            return CaptureEffects {
                recipient_opinion: rules.gift.recipient_opinion,
                ..CaptureEffects::default()
            };
        }
        CaptureOutcome::Liberate => {
            return CaptureEffects {
                recipient_opinion: rules.liberate.former_holder_opinion,
                allies_opinion: rules.liberate.allies_opinion,
                prestige: rules.liberate.prestige,
                ..CaptureEffects::default()
            };
        }
        _ => {}
    }
    let Some(effects) = outcome_rules(rules, outcome) else {
        return CaptureEffects {
            unrest: occupation,
            ..CaptureEffects::default()
        };
    };
    let share = place_share(rules, place.kind);
    let (income, heads) = state.provinces.get(&place.province).map_or((0.0, 0), |p| {
        (
            province_base_income(data, p),
            p.population.iter().map(|(_, c)| c.count).sum::<u64>(),
        )
    });
    let gold =
        ((income * effects.gold_income_share).max(effects.min_gold as f64) * share).round() as i64;
    // The occupation already counts `foreign_culture_extra.occupy`.
    let foreign = if is_foreign_culture(state, data, settlement) {
        foreign_extra(data, outcome).saturating_sub(foreign_extra(data, CaptureOutcome::Occupy))
    } else {
        0
    };
    let extra_unrest = scaled_u8(effects.extra_unrest.saturating_add(foreign), share);
    let population_percent = (f64::from(effects.population_percent) * share).round() as i32;
    let population_lost = (heads as f64 * f64::from(-population_percent.min(0)) / 100.0) as u64;
    let lost = (effects.buildings_destroyed as usize).min(place.buildings.len());
    let buildings_lost: Vec<BuildingId> =
        place.buildings.iter().rev().take(lost).cloned().collect();
    let (fortification_loss, becomes_ruin, ruin_turns) = if outcome == CaptureOutcome::Raze {
        let loss = rules.raze.fortification_loss;
        let ruin = place.fortification_level <= loss;
        (
            loss.min(place.fortification_level),
            ruin,
            if ruin { rules.raze.ruin_turns } else { 0 },
        )
    } else {
        (0, false, 0)
    };
    let banditry_devastation = scaled_u8(effects.banditry_devastation, share);
    CaptureEffects {
        gold,
        unrest: occupation.saturating_add(extra_unrest).min(100),
        extra_unrest,
        population_percent,
        population_lost,
        devastation: scaled_u8(effects.devastation, share)
            .saturating_add(banditry_devastation)
            .min(100),
        banditry_devastation,
        banditry_turns: if banditry_devastation > 0 {
            effects.banditry_turns
        } else {
            0
        },
        buildings_lost,
        fortification_loss,
        becomes_ruin,
        ruin_turns,
        unit_experience: effects.unit_experience,
        ruler_piety: effects.ruler_piety,
        papal_favor: effects.papal_favor,
        victim_opinion: effects.victim_opinion,
        others_opinion: effects.others_opinion,
        ..CaptureEffects::default()
    }
}

/// French lines describing `effects` (tooltips and the decision window).
pub fn describe(data: &GameData, effects: &CaptureEffects, previous: &FactionId) -> String {
    let mut lines = Vec::new();
    if effects.gold != 0 {
        lines.push(format!("Trésor {}", signed_livres(effects.gold)));
    }
    if effects.unrest > 0 {
        lines.push(format!("Mécontentement +{}", effects.unrest));
    }
    if effects.population_percent != 0 {
        lines.push(format!(
            "Population {} % (≈ {} âmes)",
            effects.population_percent, effects.population_lost
        ));
    }
    if effects.devastation > 0 {
        lines.push(format!("Dévastation +{}", effects.devastation));
    }
    if effects.banditry_devastation > 0 {
        lines.push(format!(
            "dont banditisme +{} pendant {} saisons",
            effects.banditry_devastation, effects.banditry_turns
        ));
    }
    if !effects.buildings_lost.is_empty() {
        let names: Vec<String> = effects
            .buildings_lost
            .iter()
            .map(|b| data.building_name(b))
            .collect();
        lines.push(format!("Bâtiments détruits : {}", names.join(", ")));
    }
    if effects.becomes_ruin {
        lines.push(format!(
            "La place devient une ruine ({} saisons sans recrutement ni chantier)",
            effects.ruin_turns
        ));
    } else if effects.fortification_loss > 0 {
        lines.push(format!(
            "Fortification −{} niveau{}",
            effects.fortification_loss,
            if effects.fortification_loss > 1 {
                "x"
            } else {
                ""
            }
        ));
    }
    if effects.unit_experience > 0 {
        lines.push(format!(
            "Expérience des troupes +{}",
            effects.unit_experience
        ));
    }
    if effects.ruler_piety != 0 {
        lines.push(format!("Piété du souverain {:+}", effects.ruler_piety));
    }
    if effects.papal_favor != 0 {
        lines.push(format!("Faveur pontificale {:+}", effects.papal_favor));
    }
    if effects.victim_opinion != 0 {
        lines.push(format!(
            "Attitude de {} {:+}",
            data.faction_name(previous),
            effects.victim_opinion
        ));
    }
    if effects.others_opinion != 0 {
        lines.push(format!(
            "Attitude des autres royaumes {:+}",
            effects.others_opinion
        ));
    }
    lines.join("\n")
}

/// Living, real factions other than `captor` (the rebels are no one's heir).
fn is_candidate(state: &CampaignState, captor: &FactionId, faction: &FactionId) -> bool {
    let rebels = FactionId::new(REBELS_FACTION).ok();
    faction != captor
        && Some(faction) != rebels.as_ref()
        && state.factions.get(faction).is_some_and(|f| f.alive)
        && !state.is_at_war(captor, faction)
}

/// M2: an ally (or suzerain, or vassal) of `captor` with an army on the
/// place, a joint siege (`None` when no such army).
fn allied_on_place(
    state: &CampaignState,
    settlement: &SettlementId,
    captor: &FactionId,
) -> Option<FactionId> {
    state
        .armies
        .values()
        .filter(|a| a.settlement() == Some(settlement))
        .map(|a| &a.faction)
        .find(|f| is_candidate(state, captor, f) && state.is_allied(captor, f))
        .cloned()
}

/// M2: the factions `settlement` can be given to (« Remettre »): allies and
/// vassals, at peace with `captor`; those standing on the place come first.
pub fn gift_recipients(
    state: &CampaignState,
    settlement: &SettlementId,
    captor: &FactionId,
) -> Vec<FactionId> {
    let mut out: Vec<FactionId> = state
        .factions
        .keys()
        .filter(|f| is_candidate(state, captor, f) && state.is_allied(captor, f))
        .cloned()
        .collect();
    if let Some(present) = allied_on_place(state, settlement, captor) {
        out.retain(|f| f != &present);
        out.insert(0, present);
    }
    out
}

/// M2: the rightful holder `settlement` can be given back to (« Libérer »):
/// its de jure owner when that is neither the captor nor an enemy.
pub fn liberation_recipient(
    state: &CampaignState,
    settlement: &SettlementId,
    captor: &FactionId,
) -> Option<FactionId> {
    let owner = &state.settlements.get(settlement)?.owner;
    is_candidate(state, captor, owner).then(|| owner.clone())
}

/// Default beneficiary of a cession (`None` for other outcomes or when no
/// one qualifies).
fn default_recipient(
    state: &CampaignState,
    settlement: &SettlementId,
    captor: &FactionId,
    outcome: CaptureOutcome,
) -> Option<FactionId> {
    match outcome {
        CaptureOutcome::Gift => gift_recipients(state, settlement, captor)
            .into_iter()
            .next(),
        CaptureOutcome::Liberate => liberation_recipient(state, settlement, captor),
        _ => None,
    }
}

/// Why `outcome` cannot go to `recipient` (`None`: it can).
fn cession_refusal(
    state: &CampaignState,
    settlement: &SettlementId,
    captor: &FactionId,
    outcome: CaptureOutcome,
    recipient: Option<&FactionId>,
) -> Option<String> {
    let allowed = match outcome {
        CaptureOutcome::Gift => {
            let list = gift_recipients(state, settlement, captor);
            if list.is_empty() {
                return Some("aucun allié ni vassal à qui la remettre".to_owned());
            }
            list
        }
        CaptureOutcome::Liberate => match liberation_recipient(state, settlement, captor) {
            Some(owner) => vec![owner],
            None => return Some("aucun tenant de droit à qui la rendre".to_owned()),
        },
        _ => return None,
    };
    match recipient {
        Some(r) if !allowed.contains(r) => Some("ce bénéficiaire ne convient pas".to_owned()),
        _ => None,
    }
}

/// Called by [`crate::siege::capture`] once `settlement` is handed over to
/// `captor`: the player gets a pending choice, an AI decides at once.
pub(crate) fn on_captured(
    state: &mut CampaignState,
    data: &GameData,
    settlement: &SettlementId,
    captor: &FactionId,
    previous: &FactionId,
    events: &mut Vec<GameEvent>,
) {
    crate::grudges::note_settlement_lost(state, data, settlement, captor, previous);
    let armies: Vec<ArmyId> = state
        .armies
        .iter()
        .filter(|(_, a)| &a.faction == captor && a.settlement() == Some(settlement))
        .map(|(id, _)| id.clone())
        .collect();
    // A place captured again before the choice: the old question lapses.
    state
        .captures
        .pending
        .retain(|p| &p.settlement != settlement);
    if captor == &state.player_faction {
        let id = state.captures.next_id.max(1);
        state.captures.next_id = id + 1;
        state.captures.pending.push(PendingCapture {
            id,
            settlement: settlement.clone(),
            faction: captor.clone(),
            previous: previous.clone(),
            armies,
            turn: state.turn,
        });
        return;
    }
    let outcome = ai_choice(state, data, settlement, captor);
    apply_outcome(
        state, data, settlement, captor, previous, &armies, outcome, None, events,
    );
}

/// The AI's choice: the highest score (ties in the order of
/// [`CaptureOutcome::ALL`]), without randomness.
pub fn ai_choice(
    state: &CampaignState,
    data: &GameData,
    settlement: &SettlementId,
    captor: &FactionId,
) -> CaptureOutcome {
    let ai = &data.capture_rules.ai;
    let Some(place) = state.settlements.get(settlement) else {
        return CaptureOutcome::Occupy;
    };
    let mut bonuses = vec![*ai.factions.get(captor).unwrap_or(&ai.default)];
    let treasury = state.factions.get(captor).map_or(0, |f| f.treasury);
    if treasury < ai.poor_treasury {
        bonuses.push(ai.poor_bonus);
    }
    let province_culture = data.provinces.get(&place.province).map(|p| &p.culture);
    let faction_culture = data.factions.get(captor).map(|f| &f.culture);
    if province_culture.is_some() && province_culture != faction_culture {
        bonuses.push(ai.foreign_culture_bonus);
    }
    if &place.owner == captor {
        bonuses.push(ai.own_land_bonus);
    }
    let exposed = !is_city(state, settlement)
        && state
            .province_city_id(&place.province)
            .and_then(|city| state.settlements.get(city))
            .is_some_and(|city| state.is_at_war(captor, &city.controller));
    if exposed {
        bonuses.push(ai.exposed_bonus);
    }
    let raze_allowed = raze_refusal(state, data, settlement).is_none();
    let can_gift = !gift_recipients(state, settlement, captor).is_empty();
    let can_liberate = liberation_recipient(state, settlement, captor).is_some();
    if can_gift && allied_on_place(state, settlement, captor).is_some() {
        bonuses.push(ai.allied_present_bonus);
    }
    if can_liberate {
        bonuses.push(ai.liberate_bonus);
    }
    let mut best = (CaptureOutcome::Occupy, i32::MIN);
    for outcome in CaptureOutcome::ALL {
        if (outcome == CaptureOutcome::Raze && !raze_allowed)
            || (outcome == CaptureOutcome::Gift && !can_gift)
            || (outcome == CaptureOutcome::Liberate && !can_liberate)
        {
            continue;
        }
        let score: i32 = bonuses.iter().map(|b| outcome.score(b)).sum();
        if score > best.1 {
            best = (outcome, score);
        }
    }
    best.0
}

/// Applies the extra effects of `outcome` (the occupation is already done).
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_outcome(
    state: &mut CampaignState,
    data: &GameData,
    settlement: &SettlementId,
    captor: &FactionId,
    previous: &FactionId,
    armies: &[ArmyId],
    outcome: CaptureOutcome,
    recipient: Option<&FactionId>,
    events: &mut Vec<GameEvent>,
) {
    if outcome == CaptureOutcome::Occupy {
        return;
    }
    if matches!(outcome, CaptureOutcome::Gift | CaptureOutcome::Liberate) {
        apply_cession(state, data, settlement, captor, outcome, recipient, events);
        return;
    }
    let outcome =
        if outcome == CaptureOutcome::Raze && raze_refusal(state, data, settlement).is_some() {
            CaptureOutcome::Sack
        } else {
            outcome
        };
    let effects = preview(state, data, settlement, outcome);
    let Some(place) = state.settlements.get_mut(settlement) else {
        return;
    };
    let province_id = place.province.clone();
    let kind = place.kind;
    for building in &effects.buildings_lost {
        if let Some(position) = place.buildings.iter().rposition(|b| b == building) {
            place.buildings.remove(position);
        }
    }
    place.fortification_level = place
        .fortification_level
        .saturating_sub(effects.fortification_loss);
    if effects.becomes_ruin {
        place.construction = None;
        place.build_queue.clear();
        place.recruit_queue.clear();
        let until = state.turn + effects.ruin_turns;
        state.captures.ruins.insert(settlement.clone(), until);
    }
    if let Some(faction) = state.factions.get_mut(captor) {
        faction.treasury += effects.gold;
    }
    if let Some(province) = state.provinces.get_mut(&province_id) {
        province.unrest = province
            .unrest
            .saturating_add(effects.extra_unrest)
            .min(100);
        province.devastation = province
            .devastation
            .saturating_add(effects.devastation)
            .min(100);
        if effects.banditry_devastation > 0 {
            let entry = state
                .captures
                .banditry
                .entry(province_id.clone())
                .or_insert(Banditry {
                    points: 0,
                    until: 0,
                });
            entry.points = entry.points.saturating_add(effects.banditry_devastation);
            entry.until = entry.until.max(state.turn + effects.banditry_turns);
        }
        if effects.population_percent != 0 {
            let factor = f64::from((100 + effects.population_percent).max(0)) / 100.0;
            for class in [
                &mut province.population.peasants,
                &mut province.population.burghers,
                &mut province.population.clergy,
                &mut province.population.nobility,
            ] {
                class.count = (class.count as f64 * factor).round() as u64;
            }
        }
    }
    if effects.unit_experience > 0 {
        for army_id in armies {
            if let Some(army) = state
                .armies
                .get_mut(army_id)
                .filter(|a| &a.faction == captor)
            {
                for unit in &mut army.units {
                    unit.experience = unit
                        .experience
                        .saturating_add(effects.unit_experience)
                        .min(10);
                }
            }
        }
    }
    if effects.ruler_piety != 0 {
        let ruler = state.factions.get(captor).and_then(|f| f.ruler.clone());
        if let Some(c) = ruler.and_then(|r| state.characters.get_mut(&r)) {
            c.piety = (i32::from(c.piety) + effects.ruler_piety).clamp(0, 100) as u8;
        }
    }
    if effects.papal_favor != 0 {
        crate::religion::change_favor(state, captor, effects.papal_favor);
    }
    let place_name = data.settlement_name(settlement);
    let turns = outcome_rules(&data.capture_rules, outcome).map_or(0, |r| r.opinion_turns);
    let reason = match outcome {
        CaptureOutcome::Ransom => format!("A rançonné {place_name}"),
        CaptureOutcome::Sack => format!("A pillé {place_name}"),
        _ => format!("A rasé {place_name}"),
    };
    let rebels = FactionId::new(REBELS_FACTION).ok();
    if effects.victim_opinion != 0
        && previous != captor
        && Some(previous) != rebels.as_ref()
        && state.factions.get(previous).is_some_and(|f| f.alive)
    {
        state.add_modifier(previous, captor, effects.victim_opinion, &reason, turns);
    }
    if effects.others_opinion != 0 {
        let others: Vec<FactionId> = state
            .factions
            .iter()
            .filter(|(id, f)| {
                f.alive && *id != captor && *id != previous && Some(*id) != rebels.as_ref()
            })
            .map(|(id, _)| id.clone())
            .collect();
        for other in others {
            state.add_modifier(&other, captor, effects.others_opinion, &reason, turns);
        }
    }
    let who = data.faction_name(captor);
    let place_label = if kind == SettlementKind::City {
        data.province_name(&province_id)
    } else {
        place_name.clone()
    };
    let text = match outcome {
        CaptureOutcome::Ransom => format!(
            "{who} met {place_label} à rançon : {} livres d'appatis.",
            effects.gold
        ),
        CaptureOutcome::Sack => format!(
            "{who} livre {place_label} au pillage : {} livres de butin, {} âmes perdues.",
            effects.gold, effects.population_lost
        ),
        _ if effects.becomes_ruin => {
            format!("{who} rase et brûle {place_label} : il n'en reste qu'une ruine.")
        }
        _ => format!("{who} rase les défenses de {place_label} et y met le feu."),
    };
    events.push(
        GameEvent::new(EventKind::ProvinceCaptured, text)
            .province(&province_id)
            .faction(captor),
    );
}

/// M2: « Remettre » / « Libérer »: the place goes (owner and controller) to
/// the beneficiary, the occupation's unrest is undone. Without a valid
/// beneficiary the place is simply occupied.
fn apply_cession(
    state: &mut CampaignState,
    data: &GameData,
    settlement: &SettlementId,
    captor: &FactionId,
    outcome: CaptureOutcome,
    recipient: Option<&FactionId>,
    events: &mut Vec<GameEvent>,
) {
    if cession_refusal(state, settlement, captor, outcome, recipient).is_some() {
        return;
    }
    let Some(beneficiary) = recipient
        .cloned()
        .or_else(|| default_recipient(state, settlement, captor, outcome))
    else {
        return;
    };
    let rules = &data.capture_rules;
    let occupation = occupation_unrest(state, data, settlement);
    let Some(place) = state.settlements.get_mut(settlement) else {
        return;
    };
    let province_id = place.province.clone();
    let kind = place.kind;
    place.owner = beneficiary.clone();
    place.hand_over(&beneficiary);
    place.garrison.clear();
    if let Some(province) = state.provinces.get_mut(&province_id) {
        province.unrest = province.unrest.saturating_sub(occupation);
    }
    let place_name = data.settlement_name(settlement);
    let (opinion, turns, reason) = match outcome {
        CaptureOutcome::Gift => (
            rules.gift.recipient_opinion,
            rules.gift.opinion_turns,
            format!("A remis {place_name}"),
        ),
        _ => (
            rules.liberate.former_holder_opinion,
            rules.liberate.opinion_turns,
            format!("A libéré {place_name}"),
        ),
    };
    if opinion != 0 {
        state.add_modifier(&beneficiary, captor, opinion, &reason, turns);
    }
    let who = data.faction_name(captor);
    let to = data.faction_name(&beneficiary);
    let place_label = if kind == SettlementKind::City {
        data.province_name(&province_id)
    } else {
        place_name.clone()
    };
    let text = if outcome == CaptureOutcome::Gift {
        format!("{who} remet {place_label} à {to}, en don féodal.")
    } else {
        let allies: Vec<FactionId> = state
            .factions
            .keys()
            .filter(|f| {
                *f != captor
                    && **f != beneficiary
                    && state.factions[*f].alive
                    && state.is_allied(f, &beneficiary)
            })
            .cloned()
            .collect();
        for ally in allies {
            state.add_modifier(&ally, captor, rules.liberate.allies_opinion, &reason, turns);
        }
        state.change_ruler_prestige(captor, rules.liberate.prestige);
        crate::grudges::forget_settlement(state, settlement);
        format!("{who} libère {place_label} et la rend à {to}.")
    };
    events.push(
        GameEvent::new(EventKind::ProvinceCaptured, text)
            .province(&province_id)
            .faction(captor),
    );
}

impl CampaignState {
    /// Captures waiting for `faction`'s choice, for the UI.
    pub fn capture_decision_views(
        &self,
        data: &GameData,
        faction: &FactionId,
    ) -> Vec<CaptureDecisionView> {
        self.captures
            .pending
            .iter()
            .filter(|p| &p.faction == faction)
            .filter_map(|pending| {
                let place = self.settlements.get(&pending.settlement)?;
                let settlement_name = data.settlement_name(&pending.settlement);
                let province_name = data.province_name(&place.province);
                let previous_name = data.faction_name(&pending.previous);
                let refusal = raze_refusal(self, data, &pending.settlement);
                let options = CaptureOutcome::ALL
                    .into_iter()
                    .map(|outcome| {
                        let effects = preview(self, data, &pending.settlement, outcome);
                        let cession = cession_refusal(
                            self,
                            &pending.settlement,
                            &pending.faction,
                            outcome,
                            None,
                        );
                        let reason = match outcome {
                            CaptureOutcome::Raze => refusal.clone(),
                            CaptureOutcome::Gift | CaptureOutcome::Liberate => cession,
                            _ => None,
                        };
                        let recipient = default_recipient(
                            self,
                            &pending.settlement,
                            &pending.faction,
                            outcome,
                        );
                        let recipients = if outcome == CaptureOutcome::Gift {
                            gift_recipients(self, &pending.settlement, &pending.faction)
                        } else {
                            Vec::new()
                        };
                        let mut effects_text = describe(data, &effects, &pending.previous);
                        if let (Some(to), true) = (&recipient, reason.is_none()) {
                            let line = if outcome == CaptureOutcome::Gift {
                                format!("Remise à {} (troubles annulés)", data.faction_name(to))
                            } else {
                                format!(
                                    "Rendue à {} (troubles annulés, rancune oubliée, prestige {:+})",
                                    data.faction_name(to),
                                    effects.prestige
                                )
                            };
                            effects_text = format!("{line}\nAttitude de {} {:+}", data.faction_name(to), effects.recipient_opinion);
                        }
                        if outcome == CaptureOutcome::Occupy {
                            effects_text = format!(
                                "La place change de mains, sans plus.\n{effects_text} (déjà compté)"
                            );
                        }
                        CaptureOptionView {
                            outcome,
                            label: outcome.label_fr().to_owned(),
                            effects_text,
                            allowed: reason.is_none(),
                            reason,
                            effects,
                            recipient,
                            recipients,
                        }
                    })
                    .collect();
                // RJ-c (ADR 0175): taking a place occupies it; possession
                // changes only by treaty.
                let is_city = self.province_city_id(&place.province) == Some(&pending.settlement);
                let owner_name = data.faction_name(&place.owner);
                let reminder = occupation_reminder(
                    is_city,
                    &province_name,
                    (place.owner != pending.faction).then_some(owner_name.as_str()),
                );
                Some(CaptureDecisionView {
                    id: pending.id,
                    settlement: pending.settlement.clone(),
                    title: format!("{settlement_name} est prise"),
                    text: format!(
                        "La place de {settlement_name} ({province_name}) est tombée ; ses \
                         défenseurs, qui tenaient pour {previous_name}, attendent votre bon \
                         plaisir. Vos capitaines demandent quel sort lui réserver : l'occuper \
                         telle quelle, en tirer rançon, la livrer au pillage ou la raser. \
                         Sans ordre avant la fin de la saison, on l'occupera.\n\n{reminder}"
                    ),
                    settlement_name,
                    province: place.province.clone(),
                    province_name,
                    kind: place.kind,
                    previous_name,
                    options,
                })
            })
            .collect()
    }

    /// Applies the player's (or `faction`'s) choice for a pending capture
    /// (order `choose_capture_outcome`).
    pub fn choose_capture_outcome(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        decision: u32,
        outcome: CaptureOutcome,
        recipient: Option<FactionId>,
    ) -> Result<(), CaptureError> {
        let position = self
            .captures
            .pending
            .iter()
            .position(|p| p.id == decision && &p.faction == faction)
            .ok_or(CaptureError::UnknownDecision(decision))?;
        let settlement = self.captures.pending[position].settlement.clone();
        if self
            .settlements
            .get(&settlement)
            .is_none_or(|s| &s.controller != faction)
        {
            self.captures.pending.remove(position);
            return Err(CaptureError::PlaceLost);
        }
        if outcome == CaptureOutcome::Raze {
            if let Some(reason) = raze_refusal(self, data, &settlement) {
                return Err(CaptureError::RazeForbidden(reason));
            }
        }
        if let Some(reason) =
            cession_refusal(self, &settlement, faction, outcome, recipient.as_ref())
        {
            return Err(CaptureError::CessionForbidden(reason));
        }
        let pending = self.captures.pending.remove(position);
        let mut events = Vec::new();
        apply_outcome(
            self,
            data,
            &settlement,
            faction,
            &pending.previous,
            &pending.armies,
            outcome,
            recipient.as_ref(),
            &mut events,
        );
        for event in events {
            self.push_order_event(event);
        }
        Ok(())
    }
}

/// Start of the end of turn: captures the player left unanswered are
/// occupied (nothing more happens), and ruins past their time are cleared.
pub(crate) fn resolve_unanswered(state: &mut CampaignState) {
    state.captures.pending.clear();
    let turn = state.turn;
    state.captures.ruins.retain(|_, until| *until > turn);
    // M2: the banditry ends, taking its devastation back.
    let ended: Vec<(ProvinceId, Banditry)> = state
        .captures
        .banditry
        .iter()
        .filter(|(_, b)| b.until <= turn)
        .map(|(p, b)| (p.clone(), *b))
        .collect();
    for (province, banditry) in ended {
        state.captures.banditry.remove(&province);
        if let Some(p) = state.provinces.get_mut(&province) {
            p.devastation = p.devastation.saturating_sub(banditry.points);
        }
    }
}

/// Tests and headless UI checks only: `faction` takes `settlement` at once
/// through the common capture path (the garrison is dismissed). Refused
/// when the faction already controls the place.
pub fn debug_capture(
    state: &mut CampaignState,
    data: &GameData,
    settlement: &SettlementId,
    faction: &FactionId,
) -> bool {
    if state
        .settlements
        .get(settlement)
        .is_none_or(|s| &s.controller == faction)
    {
        return false;
    }
    let mut events = Vec::new();
    crate::siege::capture(state, data, settlement, faction, &mut events);
    state.pending_events.extend(events);
    true
}

/// RJ-c (ADR 0175): one line under a capture decision — occupying is not
/// possessing. `owner_name` is the de jure owner of the place, `None` when
/// the taker owns it (a place recovered).
pub fn occupation_reminder(is_city: bool, province_name: &str, owner_name: Option<&str>) -> String {
    let control = if is_city {
        format!("C'est la cité de {province_name} : vous en prenez le contrôle. ")
    } else {
        String::new()
    };
    let Some(owner_name) = owner_name else {
        return format!("{control}La place est à vous de droit : vous la recouvrez.");
    };
    format!(
        "{control}Occuper n'est pas posséder : la place reste de droit à {owner_name} ; \
         elle ne devient vôtre que par cession dans un traité, et vous la rendrez à la paix \
         si elle n'est pas cédée."
    )
}

#[cfg(test)]
mod key_enum_tests {
    use super::*;
    use data_model::key_enum::assert_keys_match_serde;

    #[test]
    fn keys_match_serde_names() {
        assert_keys_match_serde::<CaptureOutcome>();
    }
}
