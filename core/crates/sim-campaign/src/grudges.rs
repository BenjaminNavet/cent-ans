//! Dated grudges (ADR 0127 extension, ADR 0720): missions anchored in a past affront.
//!
//! A grudge is remembered when the player's faction suffers an affront (a place
//! lost, an army defeated, a province raided). While the war lasts it feeds the
//! mission offer; its reward grows with its age (`data/missions.json`,
//! `grudge`). It is cancelled without penalty when the war ends or when the
//! place falls to a third party. Only the player's faction keeps grudges.

use data_model::{FactionId, GameData, GrudgeKindRules, MissionReward, ProvinceId, SettlementId};
use serde::{Deserialize, Serialize};

use crate::missions::Mission;
use crate::state::CampaignState;

/// Nature of the affront a grudge remembers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrudgeKind {
    /// A place was taken from us.
    LostSettlement,
    /// An army of ours was destroyed.
    DefeatedArmy,
    /// A province of ours was raided.
    RaidedProvince,
}

/// A remembered affront.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grudge {
    pub kind: GrudgeKind,
    /// Faction at fault.
    pub culprit: FactionId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settlement: Option<SettlementId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub province: Option<ProvinceId>,
    /// Campaign turn of the affront.
    pub turn: u32,
}

impl GrudgeKind {
    fn rules(self, data: &GameData) -> &GrudgeKindRules {
        let rules = &data.mission_rules.grudge;
        match self {
            GrudgeKind::LostSettlement => &rules.lost_settlement,
            GrudgeKind::DefeatedArmy => &rules.defeated_army,
            GrudgeKind::RaidedProvince => &rules.raided_province,
        }
    }

    /// Template id given to the missions of this kind.
    fn template_id(self) -> &'static str {
        match self {
            GrudgeKind::LostSettlement => "grudge_lost_settlement",
            GrudgeKind::DefeatedArmy => "grudge_defeated_army",
            GrudgeKind::RaidedProvince => "grudge_raided_province",
        }
    }
}

impl Grudge {
    /// Seasons since the affront.
    pub fn age(&self, turn: u32) -> u32 {
        turn.saturating_sub(self.turn)
    }

    /// Emblematic place lost (`emblematic_settlements` of the data).
    pub fn is_emblematic(&self, data: &GameData) -> bool {
        self.kind == GrudgeKind::LostSettlement
            && self
                .settlement
                .as_ref()
                .is_some_and(|s| data.mission_rules.grudge.emblematic_settlements.contains(s))
    }
}

/// Remembers `grudge` if the victim is the player's faction. An identical
/// grudge already kept stays (the older one is the worth more); beyond
/// `max_grudges` the oldest are forgotten.
pub(crate) fn record(
    state: &mut CampaignState,
    data: &GameData,
    victim: &FactionId,
    grudge: Grudge,
) {
    if victim != &state.player_faction || &grudge.culprit == victim {
        return;
    }
    let kept = &mut state.missions.grudges;
    if kept.iter().any(|g| {
        g.kind == grudge.kind
            && g.culprit == grudge.culprit
            && g.settlement == grudge.settlement
            && g.province == grudge.province
    }) {
        return;
    }
    kept.push(grudge);
    let max = data.mission_rules.grudge.max_grudges.max(1) as usize;
    if kept.len() > max {
        kept.sort_by_key(|g| g.turn);
        let excess = kept.len() - max;
        kept.drain(..excess);
    }
}

/// `capture::on_captured` hook: `previous` lost `settlement` to `captor`.
pub(crate) fn note_settlement_lost(
    state: &mut CampaignState,
    data: &GameData,
    settlement: &SettlementId,
    captor: &FactionId,
    previous: &FactionId,
) {
    let province = state.settlement_province(settlement).cloned();
    let turn = state.turn;
    record(
        state,
        data,
        previous,
        Grudge {
            kind: GrudgeKind::LostSettlement,
            culprit: captor.clone(),
            settlement: Some(settlement.clone()),
            province,
            turn,
        },
    );
}

/// Battle hook: `victim` was beaten by `culprit` in `province`.
pub(crate) fn note_army_defeated(
    state: &mut CampaignState,
    data: &GameData,
    victim: &FactionId,
    culprit: &FactionId,
    province: &ProvinceId,
) {
    let turn = state.turn;
    record(
        state,
        data,
        victim,
        Grudge {
            kind: GrudgeKind::DefeatedArmy,
            culprit: culprit.clone(),
            settlement: None,
            province: Some(province.clone()),
            turn,
        },
    );
}

/// Raid hook: `culprit` devastates `province` (the player's, else ignored).
pub(crate) fn note_province_raided(
    state: &mut CampaignState,
    data: &GameData,
    culprit: &FactionId,
    province: &ProvinceId,
) {
    let Some(victim) = state.province_owner(province).cloned() else {
        return;
    };
    let turn = state.turn;
    record(
        state,
        data,
        &victim,
        Grudge {
            kind: GrudgeKind::RaidedProvince,
            culprit: culprit.clone(),
            settlement: None,
            province: Some(province.clone()),
            turn,
        },
    );
}

/// A grudge lives while the war with its culprit lasts and, for a lost
/// place, while the culprit still holds it.
pub(crate) fn is_live(state: &CampaignState, grudge: &Grudge) -> bool {
    let player = &state.player_faction;
    if !state.factions.get(&grudge.culprit).is_some_and(|f| f.alive)
        || !state.is_at_war(player, &grudge.culprit)
    {
        return false;
    }
    match (&grudge.kind, &grudge.settlement) {
        (GrudgeKind::LostSettlement, Some(settlement)) => state
            .settlements
            .get(settlement)
            .is_some_and(|s| s.controller == grudge.culprit),
        _ => true,
    }
}

/// M2 (ADR 0723): the player liberated `settlement`, the affront it
/// recalls is settled; its grudges are forgotten without penalty.
pub(crate) fn forget_settlement(state: &mut CampaignState, settlement: &SettlementId) {
    state
        .missions
        .grudges
        .retain(|g| g.settlement.as_ref() != Some(settlement));
}

/// Forgets the grudges that are no longer live (no penalty).
pub(crate) fn prune(state: &mut CampaignState) {
    let kept: Vec<Grudge> = std::mem::take(&mut state.missions.grudges)
        .into_iter()
        .filter(|g| is_live(state, g))
        .collect();
    state.missions.grudges = kept;
}

/// Failure penalty of a mission from `grudge`: the base penalty times the
/// age multiplier, capped.
fn failure_penalty(data: &GameData, multiplier: f64) -> i32 {
    let rules = &data.mission_rules;
    let capped = multiplier.min(rules.grudge.failure_multiplier_cap);
    (f64::from(rules.failure_prestige) * capped).round() as i32
}

fn scale(value: f64, factor: f64) -> f64 {
    (value * factor).round()
}

/// The reward of a mission from `grudge`, scaled by its age.
fn scaled_reward(
    data: &GameData,
    grudge: &Grudge,
    base: &MissionReward,
    turn: u32,
) -> MissionReward {
    let rules = &data.mission_rules.grudge;
    let age_factor = rules.age_multiplier(grudge.age(turn));
    let prestige_factor = if grudge.is_emblematic(data) {
        age_factor * rules.emblematic_prestige_multiplier
    } else {
        age_factor
    };
    MissionReward {
        gold: scale(base.gold as f64, age_factor) as i64,
        prestige: scale(f64::from(base.prestige), prestige_factor) as i32,
        public_order: scale(f64::from(base.public_order), age_factor).min(255.0) as u8,
        free_unit: false,
    }
}

/// Candidate missions of the live grudges: for each kind the oldest grudge
/// not already taken by an active mission, with the kind's draw weight.
pub fn candidates(state: &CampaignState, data: &GameData) -> Vec<(u32, Mission)> {
    let mut out = Vec::new();
    for kind in [
        GrudgeKind::DefeatedArmy,
        GrudgeKind::LostSettlement,
        GrudgeKind::RaidedProvince,
    ] {
        if state
            .missions
            .active
            .iter()
            .any(|m| m.template == kind.template_id())
        {
            continue;
        }
        let Some(grudge) = state
            .missions
            .grudges
            .iter()
            .filter(|g| g.kind == kind && is_live(state, g))
            .min_by_key(|g| g.turn)
        else {
            continue;
        };
        out.push((kind.rules(data).weight.max(1), build(state, data, grudge)));
    }
    out
}

fn build(state: &CampaignState, data: &GameData, grudge: &Grudge) -> Mission {
    let rules = grudge.kind.rules(data);
    let turn = state.turn;
    let place = grudge
        .settlement
        .as_ref()
        .map(|s| data.settlement_name(s))
        .or_else(|| grudge.province.as_ref().map(|p| data.province_name(p)))
        .unwrap_or_default();
    let culprit = data.faction_name(&grudge.culprit);
    let count = 1_u32;
    let fill = |text: &str| {
        text.replace("{cible}", &place)
            .replace("{auteur}", &culprit)
            .replace("{n}", &count.to_string())
    };
    let multiplier = data.mission_rules.grudge.age_multiplier(grudge.age(turn));
    Mission {
        id: 0,
        template: grudge.kind.template_id().to_owned(),
        goal: rules.goal,
        counter: rules.counter,
        title: fill(&rules.title),
        objective: fill(&rules.objective),
        province: grudge.province.clone(),
        settlement: grudge.settlement.clone(),
        building: None,
        count,
        progress: 0,
        issued_turn: turn,
        deadline_turn: turn + rules.duration,
        reward: scaled_reward(data, grudge, &rules.reward, turn),
        baseline: Vec::new(),
        grudge: Some(grudge.clone()),
        failure_prestige: Some(failure_penalty(data, multiplier)),
    }
}
