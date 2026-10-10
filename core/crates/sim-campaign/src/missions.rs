//! Short-term campaign missions (ADR 0127).
//!
//! The player's faction (never the AI) receives 1 or 2 missions drawn from
//! `data/missions.json` according to its situation (a neighbour at war to
//! take, a building it can afford, a threatened border place...), resolved
//! at the end of each turn, once the new season has begun: success grants
//! the reward, the deadline passing is a failure with a small prestige loss
//! at most. Offers are deterministic: their generator is seeded by the
//! campaign seed and the turn, and never draws from [`CampaignState::rng`]
//! (the rest of the simulation is left untouched).

use data_model::key_enum;
use std::collections::BTreeSet;

use data_model::{
    BuildingId, FactionId, GameData, MissionCounter, MissionGoal, MissionReward, MissionTarget,
    MissionTemplate, ProvinceId, SettlementId,
};
use serde::{Deserialize, Serialize};

use crate::events::{EventKind, GameEvent};
use crate::rng::CampaignRng;
use crate::state::{CampaignState, Season, Unit};

/// Missions of the player's faction (absent from older saves: empty).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissionsState {
    /// Faction the missions belong to (the player's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faction: Option<FactionId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active: Vec<Mission>,
    #[serde(default)]
    pub next_id: u32,
    /// Turn of the last success or failure (offer cooldown).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_closed_turn: Option<u32>,
    /// Ids of the templates that succeeded (chains of faction missions).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub done: Vec<String>,
    /// Notices of the last resolution, for the interface's toasts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notices: Vec<MissionNotice>,
    /// WR turn (ADR 0304): the offer awaiting the player's choice.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offer: Option<MissionOffer>,
}

/// An offer of several candidate missions: the player picks one
/// (`Order::ChooseMission`) or refuses; ignored, it lapses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissionOffer {
    /// Fully built missions (ids are given on acceptance).
    pub candidates: Vec<Mission>,
    pub offered_turn: u32,
    /// Turn at the start of which the offer lapses.
    pub expires_turn: u32,
}

/// Why `Order::ChooseMission` was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MissionChoiceError {
    #[error("aucune offre de mission en attente")]
    NoOffer,
    #[error("cette mission n'est pas proposée")]
    UnknownCandidate,
    #[error("deux missions sont déjà en cours")]
    NoFreeSlot,
}

/// One active mission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mission {
    pub id: u32,
    /// Id of the [`MissionTemplate`] it was drawn from (progress labels).
    pub template: String,
    pub goal: MissionGoal,
    /// What a `count` goal tallies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counter: Option<MissionCounter>,
    pub title: String,
    pub objective: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub province: Option<ProvinceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settlement: Option<SettlementId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub building: Option<BuildingId>,
    /// Number asked for (battles, units; seasons for `hold`).
    pub count: u32,
    /// Battles won or units recruited since the mission was given.
    #[serde(default)]
    pub progress: u32,
    pub issued_turn: u32,
    /// Turn at the start of which the mission is failed (or, for
    /// `hold`, fulfilled).
    pub deadline_turn: u32,
    pub reward: MissionReward,
    /// `treaty`: the faction's treaties when the mission was given.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub baseline: Vec<String>,
}

key_enum! {
/// What happened to a mission, for a toast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoticeKind {
    Offered => "offered",
    Succeeded => "succeeded",
    Failed => "failed",
    /// The offer lapsed unanswered.
    Expired => "expired",
}
}

/// A toast for the interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissionNotice {
    pub kind: NoticeKind,
    pub mission: u32,
    pub text: String,
}

/// A mission as shown by the objectives panel (French labels).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MissionView {
    pub id: u32,
    pub kind: String,
    pub title: String,
    pub objective: String,
    pub progress: String,
    /// 0-1 share of the objective met, for a bar.
    pub progress_ratio: f64,
    pub turns_left: u32,
    pub deadline: String,
    pub reward: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub province: Option<ProvinceId>,
    /// Faction mission (own to the player's faction, possibly chained).
    #[serde(default)]
    pub faction_mission: bool,
    /// Historical note of a faction mission.
    #[serde(default)]
    pub source: String,
}

/// One candidate of the pending offer, for the choice window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OfferCandidateView {
    pub index: usize,
    pub title: String,
    pub objective: String,
    /// Turns allowed once accepted.
    pub duration: u32,
    pub reward: String,
    #[serde(default)]
    pub faction_mission: bool,
    #[serde(default)]
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub province: Option<ProvinceId>,
}

/// The pending offer as shown to the player.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MissionOfferView {
    pub turns_left: u32,
    pub candidates: Vec<OfferCandidateView>,
}

/// Where a mission points to, before its texts are filled in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Target {
    province: Option<ProvinceId>,
    settlement: Option<SettlementId>,
    building: Option<BuildingId>,
    /// `{cible}`.
    target_name: String,
    /// `{lieu}`.
    place_name: String,
}

// ----- end of turn -----------------------------------------------------------

/// End-of-turn step, once the new season has begun: success (reward),
/// deadline (failure), then at most one new offer.
pub fn resolve_missions(state: &mut CampaignState, data: &GameData, events: &mut Vec<GameEvent>) {
    state.missions.notices.clear();
    let player = state.player_faction.clone();
    if state.missions.faction.as_ref() != Some(&player) {
        // New campaign, or a save of an older version: missions follow the
        // player's faction.
        state.missions = MissionsState {
            faction: Some(player.clone()),
            ..MissionsState::default()
        };
    }
    if !state.factions.get(&player).is_some_and(|f| f.alive) {
        state.missions.active.clear();
        return;
    }
    let active = std::mem::take(&mut state.missions.active);
    let mut kept = Vec::with_capacity(active.len());
    for mission in active {
        match verdict(state, &player, &mission) {
            Verdict::Success => {
                if !state.missions.done.contains(&mission.template) {
                    state.missions.done.push(mission.template.clone());
                }
                apply_reward(state, data, &player, &mission);
                let text = format!(
                    "Mission accomplie : {}. Récompense : {}.",
                    mission.title,
                    reward_label(&mission.reward)
                );
                close(
                    state,
                    &player,
                    &mission,
                    NoticeKind::Succeeded,
                    text,
                    events,
                );
            }
            Verdict::Failure(reason) => {
                let penalty = data.mission_rules.failure_prestige;
                if penalty != 0 {
                    state.change_ruler_prestige(&player, penalty);
                }
                let mut text = format!("Mission échouée : {} ({reason}).", mission.title);
                if penalty != 0 {
                    text.push_str(&format!(" Prestige {penalty}."));
                }
                close(state, &player, &mission, NoticeKind::Failed, text, events);
            }
            Verdict::Ongoing => kept.push(mission),
        }
    }
    state.missions.active = kept;
    expire_offer(state, events);
    offer_mission(state, data, &player, events);
}

/// An offer left unanswered lapses at its expiry turn (cooldown as a refusal).
fn expire_offer(state: &mut CampaignState, events: &mut Vec<GameEvent>) {
    let Some(offer) = state.missions.offer.as_ref() else {
        return;
    };
    if state.turn < offer.expires_turn {
        return;
    }
    state.missions.offer = None;
    state.missions.last_closed_turn = Some(state.turn);
    let text = "L'offre de mission est restée sans réponse : elle est retirée.".to_owned();
    let player = state.player_faction.clone();
    events.push(GameEvent::new(EventKind::Mission, text.clone()).faction(&player));
    state.missions.notices.push(MissionNotice {
        kind: NoticeKind::Expired,
        mission: 0,
        text,
    });
}

/// `Order::ChooseMission`: takes candidate `choice` of the pending offer
/// (`None`: refuses it, the cooldown applies). The mission's clock starts now.
pub(crate) fn choose_mission(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    choice: Option<usize>,
) -> Result<(), MissionChoiceError> {
    if faction != &state.player_faction || state.missions.faction.as_ref() != Some(faction) {
        return Err(MissionChoiceError::NoOffer);
    }
    let Some(offer) = state.missions.offer.as_ref() else {
        return Err(MissionChoiceError::NoOffer);
    };
    let turn = state.turn;
    let mut events = Vec::new();
    let Some(index) = choice else {
        state.missions.offer = None;
        state.missions.last_closed_turn = Some(turn);
        events.push(
            GameEvent::new(EventKind::Mission, "Offre de mission refusée.".to_owned())
                .faction(faction),
        );
        state.pending_events.extend(events);
        return Ok(());
    };
    let Some(candidate) = offer.candidates.get(index).cloned() else {
        return Err(MissionChoiceError::UnknownCandidate);
    };
    if state.missions.active.len() >= data.mission_rules.max_active.min(2) as usize {
        return Err(MissionChoiceError::NoFreeSlot);
    }
    state.missions.offer = None;
    let duration = candidate.deadline_turn - candidate.issued_turn;
    let id = state.missions.next_id;
    state.missions.next_id += 1;
    let mission = Mission {
        id,
        issued_turn: turn,
        deadline_turn: turn + duration,
        progress: 0,
        baseline: if candidate.goal == MissionGoal::Treaty {
            treaty_tokens(state, faction)
        } else {
            Vec::new()
        },
        ..candidate
    };
    let text = format!(
        "Mission acceptée : {}. {} Échéance : {}. Récompense : {}.",
        mission.title,
        mission.objective,
        deadline_label(state, mission.deadline_turn),
        reward_label(&mission.reward)
    );
    let mut event = GameEvent::new(EventKind::Mission, text).faction(faction);
    if let Some(p) = &mission.province {
        event = event.province(p);
    }
    events.push(event);
    state.missions.active.push(mission);
    state.pending_events.extend(events);
    Ok(())
}

/// AI: takes the candidate with the best reward (gold + 20 per prestige,
/// +100 for a free company, +10 per point of order) among those whose deadline
/// is not shorter than its own time to act; the nearest deadline breaks ties.
pub fn ai_choose_mission(state: &CampaignState, data: &GameData) -> Option<crate::Order> {
    let offer = state.missions.offer.as_ref()?;
    if state.missions.active.len() >= data.mission_rules.max_active.min(2) as usize {
        return None;
    }
    let worth = |m: &Mission| {
        i64::from(m.reward.prestige) * 20
            + m.reward.gold
            + i64::from(m.reward.public_order) * 10
            + if m.reward.free_unit { 100 } else { 0 }
    };
    let best = offer
        .candidates
        .iter()
        .enumerate()
        .max_by_key(|(i, m)| {
            (
                worth(m),
                std::cmp::Reverse(m.deadline_turn - m.issued_turn),
                std::cmp::Reverse(*i),
            )
        })
        .map(|(i, _)| i);
    Some(crate::Order::ChooseMission { choice: best })
}

enum Verdict {
    Ongoing,
    Success,
    /// French reason.
    Failure(&'static str),
}

fn verdict(state: &CampaignState, player: &FactionId, mission: &Mission) -> Verdict {
    if mission.goal == MissionGoal::Hold && !controls_target(state, player, mission) {
        return Verdict::Failure("la place est perdue");
    }
    let (done, of) = measure(state, player, mission);
    if done >= of {
        Verdict::Success
    } else if state.turn >= mission.deadline_turn {
        Verdict::Failure("l'échéance est passée")
    } else {
        Verdict::Ongoing
    }
}

fn controls_target(state: &CampaignState, player: &FactionId, mission: &Mission) -> bool {
    mission
        .province
        .as_ref()
        .is_some_and(|p| state.controls_province(player, p))
}

/// How far a mission is: `(done, of)`, fulfilled when `done >= of`. Steps of
/// a `build` goal: 0 not begun, 1 under construction, 2 standing.
fn measure(state: &CampaignState, player: &FactionId, mission: &Mission) -> (u32, u32) {
    match mission.goal {
        MissionGoal::Control => (u32::from(controls_target(state, player, mission)), 1),
        MissionGoal::Count => (mission.progress, mission.count),
        MissionGoal::Hold => (
            state.turn.saturating_sub(mission.issued_turn),
            mission.deadline_turn - mission.issued_turn,
        ),
        MissionGoal::Build => {
            let settlement = mission
                .settlement
                .as_ref()
                .and_then(|s| state.settlements.get(s))
                .filter(|s| &s.controller == player);
            let step = match (settlement, mission.building.as_ref()) {
                (Some(s), Some(b)) if s.buildings.contains(b) => 2,
                (Some(s), Some(b)) if s.construction.as_ref().is_some_and(|c| &c.building == b) => {
                    1
                }
                _ => 0,
            };
            (step, 2)
        }
        MissionGoal::Treaty => {
            let baseline: BTreeSet<&String> = mission.baseline.iter().collect();
            let new = treaty_tokens(state, player)
                .iter()
                .any(|t| !baseline.contains(t));
            (u32::from(new), 1)
        }
    }
}

fn close(
    state: &mut CampaignState,
    player: &FactionId,
    mission: &Mission,
    kind: NoticeKind,
    text: String,
    events: &mut Vec<GameEvent>,
) {
    state.missions.last_closed_turn = Some(state.turn);
    let mut event = GameEvent::new(EventKind::Mission, text.clone()).faction(player);
    if let Some(p) = &mission.province {
        event = event.province(p);
    }
    events.push(event);
    state.missions.notices.push(MissionNotice {
        kind,
        mission: mission.id,
        text,
    });
}

/// Gold, prestige, public order (target province, else the capital), a free
/// company in the capital's garrison.
fn apply_reward(state: &mut CampaignState, data: &GameData, player: &FactionId, mission: &Mission) {
    let reward = &mission.reward;
    if reward.gold != 0 {
        if let Some(f) = state.factions.get_mut(player) {
            f.treasury += reward.gold;
        }
    }
    if reward.prestige != 0 {
        state.change_ruler_prestige(player, reward.prestige);
    }
    if reward.public_order > 0 {
        let province = mission
            .province
            .clone()
            .filter(|p| state.controls_province(player, p))
            .or_else(|| {
                let capital = state.factions.get(player)?.capital.clone();
                state.controls_province(player, &capital).then_some(capital)
            });
        if let Some(p) = province.and_then(|p| state.provinces.get_mut(&p)) {
            p.unrest = p.unrest.saturating_sub(reward.public_order);
        }
    }
    if reward.free_unit {
        if let Some(city) = reward_city(state, player) {
            if let Some(unit_type) = free_unit_type(state, data, &city) {
                if let Some(s) = state.settlements.get_mut(&city) {
                    s.garrison.push(Unit::fresh(unit_type));
                }
            }
        }
    }
}

/// The player's seat ([`CampaignState::faction_seat`]; JR1: a host based in
/// a town holds no city, its reward lands in its base).
fn reward_city(state: &CampaignState, player: &FactionId) -> Option<SettlementId> {
    state.faction_seat(player)
}

/// The dearest unit raised in `city` (available first, else any listed).
fn free_unit_type<'a>(
    state: &CampaignState,
    data: &'a GameData,
    city: &SettlementId,
) -> Option<&'a data_model::UnitType> {
    let options = state.recruitable(data, city);
    let pick = |available_only: bool| {
        options
            .iter()
            .filter(|o| o.available || !available_only)
            .filter(|o| data.unit_types.contains_key(&o.unit_type))
            .max_by(|a, b| a.cost.cmp(&b.cost).then(b.unit_type.cmp(&a.unit_type)))
            .map(|o| o.unit_type.clone())
    };
    pick(true)
        .or_else(|| pick(false))
        .and_then(|id| data.unit_types.get(&id))
}

// ----- hooks -----------------------------------------------------------------

/// A battle was won by `faction` (hook of the battle outcome).
pub(crate) fn note_battle_won(state: &mut CampaignState, faction: &FactionId) {
    bump(state, faction, MissionCounter::BattleWon, 1);
}

/// `units` were recruited or hired by `faction` (hook of the player's orders).
pub(crate) fn note_units_recruited(state: &mut CampaignState, faction: &FactionId, units: u32) {
    bump(state, faction, MissionCounter::UnitsRecruited, units);
}

fn bump(state: &mut CampaignState, faction: &FactionId, counter: MissionCounter, amount: u32) {
    if faction != &state.player_faction || state.missions.faction.as_ref() != Some(faction) {
        return;
    }
    for mission in state
        .missions
        .active
        .iter_mut()
        .filter(|m| m.counter == Some(counter))
    {
        mission.progress = mission.progress.saturating_add(amount);
    }
}

// ----- offers ----------------------------------------------------------------

/// At most one pending offer, when no offer waits, a slot is free and the
/// cooldown has elapsed: up to `offer_candidates` distinct templates drawn by
/// weight among those with a plausible target, each with a random target.
fn offer_mission(
    state: &mut CampaignState,
    data: &GameData,
    player: &FactionId,
    events: &mut Vec<GameEvent>,
) {
    let rules = &data.mission_rules;
    let turn = state.turn;
    if state.missions.offer.is_some()
        || turn < rules.first_turn
        || state.missions.active.len() >= rules.max_active.min(2) as usize
    {
        return;
    }
    if state
        .missions
        .last_closed_turn
        .is_some_and(|closed| turn < closed + rules.offer_cooldown_turns)
    {
        return;
    }
    let active_templates: BTreeSet<&str> = state
        .missions
        .active
        .iter()
        .map(|m| m.template.as_str())
        .collect();
    let mut pool: Vec<(&MissionTemplate, Vec<Target>)> = rules
        .templates
        .iter()
        .filter(|t| !active_templates.contains(t.id.as_str()))
        .filter(|t| template_open(state, player, t))
        .map(|t| (t, candidates(state, data, player, t)))
        .filter(|(_, c)| !c.is_empty())
        .collect();
    let mut rng = mission_rng(state.seed, turn);
    let mut built = Vec::new();
    while built.len() < rules.offer_candidates.max(1) as usize && !pool.is_empty() {
        let total: u32 = pool.iter().map(|(t, _)| t.weight.max(1)).sum();
        let mut roll = rng.below(total);
        let at = pool
            .iter()
            .position(|(t, _)| {
                let w = t.weight.max(1);
                if roll < w {
                    true
                } else {
                    roll -= w;
                    false
                }
            })
            .unwrap_or(0);
        let (template, targets) = pool.remove(at);
        let target = targets[rng.below(targets.len() as u32) as usize].clone();
        built.push(build_mission(state, template, &target, turn));
    }
    if built.is_empty() {
        return;
    }
    let list = built
        .iter()
        .map(|m| format!("{} ({})", m.title, reward_label(&m.reward)))
        .collect::<Vec<_>>()
        .join(" ; ");
    let text = format!("Offre de mission, à choisir ou refuser : {list}.");
    events.push(GameEvent::new(EventKind::Mission, text.clone()).faction(player));
    state.missions.notices.push(MissionNotice {
        kind: NoticeKind::Offered,
        mission: 0,
        text,
    });
    state.missions.offer = Some(MissionOffer {
        candidates: built,
        offered_turn: turn,
        expires_turn: turn + rules.offer_expiry_turns.max(1),
    });
}

/// A candidate mission (id 0 until accepted).
fn build_mission(
    state: &CampaignState,
    template: &MissionTemplate,
    target: &Target,
    turn: u32,
) -> Mission {
    let count = if template.goal == MissionGoal::Hold {
        template.duration
    } else {
        template.count.max(1)
    };
    let fill = |text: &str| {
        text.replace("{cible}", &target.target_name)
            .replace("{lieu}", &target.place_name)
            .replace("{n}", &count.to_string())
    };
    Mission {
        id: 0,
        template: template.id.clone(),
        goal: template.goal,
        counter: template.counter,
        title: fill(&template.title),
        objective: fill(&template.objective),
        province: target.province.clone(),
        settlement: target.settlement.clone(),
        building: target.building.clone(),
        count,
        progress: 0,
        issued_turn: turn,
        deadline_turn: turn + template.duration,
        reward: template.reward.clone(),
        baseline: if template.goal == MissionGoal::Treaty {
            treaty_tokens(state, &state.player_faction)
        } else {
            Vec::new()
        },
    }
}

/// Faction missions go to their faction only, once, and chained ones after
/// their predecessor succeeded.
fn template_open(state: &CampaignState, player: &FactionId, template: &MissionTemplate) -> bool {
    if template.faction.as_ref().is_some_and(|f| f != player) {
        return false;
    }
    if template
        .after
        .as_ref()
        .is_some_and(|a| !state.missions.done.contains(a))
    {
        return false;
    }
    template.faction.is_none() || !state.missions.done.contains(&template.id)
}

/// Generator of the offers of `turn`, independent of the campaign's own.
fn mission_rng(seed: u64, turn: u32) -> CampaignRng {
    CampaignRng::from_seed(
        seed ^ 0x4D49_5353_494F_4E53 ^ (u64::from(turn) + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15),
    )
}

/// Plausible targets of `template` for `player` (empty: not offered).
fn candidates(
    state: &CampaignState,
    data: &GameData,
    player: &FactionId,
    template: &MissionTemplate,
) -> Vec<Target> {
    let Some(faction) = state.factions.get(player) else {
        return Vec::new();
    };
    let enemies: BTreeSet<&FactionId> = faction
        .at_war_with
        .iter()
        .filter(|f| state.factions.get(*f).is_some_and(|f| f.alive))
        .collect();
    let held: Vec<&ProvinceId> = state
        .provinces
        .keys()
        .filter(|p| state.controls_province(player, p))
        .collect();
    let province_name = |p: &ProvinceId| data.province_name(p);
    let enemy_held = |p: &ProvinceId| {
        state
            .province_controller(p)
            .is_some_and(|c| enemies.contains(c))
    };
    let single = |ok: bool| {
        if ok {
            vec![Target::default()]
        } else {
            Vec::new()
        }
    };
    match template.target {
        MissionTarget::EnemyNeighbour => {
            let mut targets = BTreeSet::new();
            for p in &held {
                for n in data.provinces.get(*p).map_or(&[][..], |p| &p.neighbors) {
                    if enemy_held(n) {
                        targets.insert(n.clone());
                    }
                }
            }
            targets
                .into_iter()
                .map(|p| Target {
                    target_name: province_name(&p),
                    settlement: state.province_city_id(&p).cloned(),
                    province: Some(p),
                    ..Target::default()
                })
                .collect()
        }
        MissionTarget::ThreatenedOwn => held
            .iter()
            .filter(|p| {
                data.provinces
                    .get(**p)
                    .is_some_and(|d| d.neighbors.iter().any(enemy_held))
            })
            .map(|p| Target {
                target_name: province_name(p),
                settlement: state.province_city_id(p).cloned(),
                province: Some((*p).clone()),
                ..Target::default()
            })
            .collect(),
        MissionTarget::AtWar => single(!enemies.is_empty()),
        MissionTarget::CanRecruit => single(held.iter().any(|p| {
            state
                .province_city_id(p)
                .is_some_and(|city| state.recruitable(data, city).iter().any(|o| o.available))
        })),
        MissionTarget::Buildable => {
            let mut targets = Vec::new();
            for p in &held {
                let Some(city) = state.province_city_id(p) else {
                    continue;
                };
                let Some(settlement) = state.settlements.get(city) else {
                    continue;
                };
                if &settlement.owner != player || settlement.construction.is_some() {
                    continue;
                }
                for option in state.buildable(data, city) {
                    if option.available && option.turns < template.duration {
                        targets.push(Target {
                            target_name: option.name.clone(),
                            place_name: data.settlement_name(city),
                            province: Some((*p).clone()),
                            settlement: Some(city.clone()),
                            building: Some(option.building.clone()),
                        });
                    }
                }
            }
            targets
        }
        MissionTarget::Fixed => {
            let Some(p) = template.province.as_ref() else {
                return Vec::new();
            };
            if !state.provinces.contains_key(p) {
                return Vec::new();
            }
            let held_by_player = state.controls_province(player, p);
            if held_by_player == (template.goal == MissionGoal::Control) {
                return Vec::new();
            }
            vec![Target {
                target_name: province_name(p),
                settlement: state.province_city_id(p).cloned(),
                province: Some(p.clone()),
                ..Target::default()
            }]
        }
        MissionTarget::TreatyPartner => single(state.factions.iter().any(|(id, f)| {
            id != player && f.alive && !faction.allies.contains(id) && id != &state.player_faction
        })),
    }
}

/// The faction's treaties: alliances, truces (a peace), liege and vassals.
pub fn treaty_tokens(state: &CampaignState, player: &FactionId) -> Vec<String> {
    let Some(faction) = state.factions.get(player) else {
        return Vec::new();
    };
    let mut tokens: Vec<String> = faction
        .allies
        .iter()
        .map(|f| format!("ally:{f}"))
        .chain(faction.truces.keys().map(|f| format!("truce:{f}")))
        .chain(faction.suzerain.iter().map(|f| format!("liege:{f}")))
        .chain(
            state
                .factions
                .iter()
                .filter(|(_, f)| f.suzerain.as_ref() == Some(player))
                .map(|(id, _)| format!("vassal:{id}")),
        )
        .collect();
    tokens.sort();
    tokens
}

// ----- labels ----------------------------------------------------------------

/// "600 livres, +15 prestige, −10 agitation, une compagnie gratuite".
pub fn reward_label(reward: &MissionReward) -> String {
    let mut parts = Vec::new();
    if reward.gold != 0 {
        parts.push(format!("{} livres", reward.gold));
    }
    if reward.prestige != 0 {
        parts.push(format!("+{} prestige", reward.prestige));
    }
    if reward.public_order > 0 {
        parts.push(format!("−{} agitation", reward.public_order));
    }
    if reward.free_unit {
        parts.push("une compagnie gratuite".to_owned());
    }
    if parts.is_empty() {
        "aucune".to_owned()
    } else {
        parts.join(", ")
    }
}

/// French date of `deadline_turn` ("Hiver 1338").
fn deadline_label(state: &CampaignState, deadline_turn: u32) -> String {
    let mut season = state.season;
    let mut year = state.year;
    for _ in state.turn..deadline_turn {
        if season == Season::Winter {
            year += 1;
        }
        season = season.next();
    }
    format!("{} {year}", season.label_fr())
}

/// The label and the 0-1 ratio of a mission, from its template's texts.
fn progress_of(state: &CampaignState, data: &GameData, mission: &Mission) -> (String, f64) {
    let (done, of) = measure(state, &state.player_faction, mission);
    let done = done.min(of);
    let ratio = f64::from(done) / f64::from(of.max(1));
    let Some(template) = data
        .mission_rules
        .templates
        .iter()
        .find(|t| t.id == mission.template)
    else {
        return (format!("{done}/{of}"), ratio);
    };
    let text = template
        .steps
        .get(done as usize)
        .unwrap_or(&template.progress);
    let holder = || {
        mission
            .province
            .as_ref()
            .and_then(|p| state.province_controller(p))
            .map_or_else(|| "personne".to_owned(), |h| data.faction_name(h))
    };
    let text = if text.contains("{holder}") {
        text.replace("{holder}", &holder())
    } else {
        text.clone()
    };
    (
        text.replace("{done}", &done.to_string())
            .replace("{n}", &of.to_string()),
        ratio,
    )
}

impl CampaignState {
    /// The player's active missions, for the objectives panel.
    pub fn missions(&self, data: &GameData) -> Vec<MissionView> {
        self.missions
            .active
            .iter()
            .map(|m| {
                let (progress, progress_ratio) = progress_of(self, data, m);
                let template = data
                    .mission_rules
                    .templates
                    .iter()
                    .find(|t| t.id == m.template);
                MissionView {
                    faction_mission: template.is_some_and(|t| t.faction.is_some()),
                    source: template.map(|t| t.source.clone()).unwrap_or_default(),
                    id: m.id,
                    kind: m.template.clone(),
                    title: m.title.clone(),
                    objective: m.objective.clone(),
                    progress,
                    progress_ratio,
                    turns_left: m.deadline_turn.saturating_sub(self.turn),
                    deadline: deadline_label(self, m.deadline_turn),
                    reward: reward_label(&m.reward),
                    province: m.province.clone(),
                }
            })
            .collect()
    }

    /// The pending offer (candidates, time left), for the choice window.
    pub fn mission_offer(&self, data: &GameData) -> Option<MissionOfferView> {
        let offer = self.missions.offer.as_ref()?;
        Some(MissionOfferView {
            turns_left: offer.expires_turn.saturating_sub(self.turn),
            candidates: offer
                .candidates
                .iter()
                .enumerate()
                .map(|(index, m)| {
                    let template = data
                        .mission_rules
                        .templates
                        .iter()
                        .find(|t| t.id == m.template);
                    OfferCandidateView {
                        index,
                        title: m.title.clone(),
                        objective: m.objective.clone(),
                        duration: m.deadline_turn - m.issued_turn,
                        reward: reward_label(&m.reward),
                        faction_mission: template.is_some_and(|t| t.faction.is_some()),
                        source: template.map(|t| t.source.clone()).unwrap_or_default(),
                        province: m.province.clone(),
                    }
                })
                .collect(),
        })
    }

    /// Turn the pending offer was made (its identity for the interface; 0
    /// when none).
    pub fn mission_offer_turn(&self) -> u32 {
        self.missions.offer.as_ref().map_or(0, |o| o.offered_turn)
    }

    /// Notices (offered, succeeded, failed) of the last resolution.
    pub fn mission_notices(&self) -> &[MissionNotice] {
        &self.missions.notices
    }
}

#[cfg(test)]
mod key_enum_tests {
    use super::*;
    use data_model::key_enum::assert_keys_match_serde;

    #[test]
    fn keys_match_serde_names() {
        assert_keys_match_serde::<NoticeKind>();
    }
}
