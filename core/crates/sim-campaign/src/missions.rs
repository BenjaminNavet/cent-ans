//! Short-term campaign missions (lot NT3, ADR 0127).
//!
//! The player's faction (never the AI) receives 1 or 2 missions drawn from
//! `data/missions.json` according to its situation (a neighbour at war to
//! take, a building it can afford, a threatened border place...), resolved
//! at the end of each turn, once the new season has begun: success grants
//! the reward, the deadline passing is a failure with a small prestige loss
//! at most. Offers are deterministic: their generator is seeded by the
//! campaign seed and the turn, and never draws from [`CampaignState::rng`]
//! (the rest of the simulation is left untouched).

use std::collections::BTreeSet;

use data_model::{
    BuildingId, FactionId, GameData, MissionKind, MissionReward, MissionTemplate, ProvinceId,
    SettlementId,
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
    /// Notices of the last resolution, for the interface's toasts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notices: Vec<MissionNotice>,
}

/// One active mission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mission {
    pub id: u32,
    pub template: String,
    pub kind: MissionKind,
    pub title: String,
    pub objective: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub province: Option<ProvinceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settlement: Option<SettlementId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub building: Option<BuildingId>,
    /// Number asked for (battles, units; seasons for `hold_place`).
    pub count: u32,
    /// Battles won or units recruited since the mission was given.
    #[serde(default)]
    pub progress: u32,
    pub issued_turn: u32,
    /// Turn at the start of which the mission is failed (or, for
    /// `hold_place`, fulfilled).
    pub deadline_turn: u32,
    pub reward: MissionReward,
    /// `conclude_treaty`: the faction's treaties when the mission was given.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub baseline: Vec<String>,
}

/// What happened to a mission, for a toast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NoticeKind {
    Offered,
    Succeeded,
    Failed,
}

impl NoticeKind {
    pub fn key(self) -> &'static str {
        match self {
            NoticeKind::Offered => "offered",
            NoticeKind::Succeeded => "succeeded",
            NoticeKind::Failed => "failed",
        }
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
    offer_mission(state, data, &player, events);
}

enum Verdict {
    Ongoing,
    Success,
    /// French reason.
    Failure(&'static str),
}

fn verdict(state: &CampaignState, player: &FactionId, mission: &Mission) -> Verdict {
    let turn = state.turn;
    let achieved = match mission.kind {
        MissionKind::TakeProvince => mission
            .province
            .as_ref()
            .is_some_and(|p| state.controls_province(player, p)),
        MissionKind::WinBattle | MissionKind::RecruitUnits => mission.progress >= mission.count,
        MissionKind::ConstructBuilding => {
            match (mission.settlement.as_ref(), mission.building.as_ref()) {
                (Some(s), Some(b)) => state
                    .settlements
                    .get(s)
                    .is_some_and(|s| &s.controller == player && s.buildings.contains(b)),
                _ => false,
            }
        }
        MissionKind::ConcludeTreaty => {
            let baseline: BTreeSet<&String> = mission.baseline.iter().collect();
            treaty_tokens(state, player)
                .iter()
                .any(|t| !baseline.contains(t))
        }
        MissionKind::HoldPlace => {
            let held = mission
                .province
                .as_ref()
                .is_some_and(|p| state.controls_province(player, p));
            if !held {
                return Verdict::Failure("la place est perdue");
            }
            turn >= mission.deadline_turn
        }
    };
    if achieved {
        Verdict::Success
    } else if turn >= mission.deadline_turn {
        Verdict::Failure("l'échéance est passée")
    } else {
        Verdict::Ongoing
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

/// The capital's city when the player controls it, else his first city,
/// else the first place he holds (JR1: a host based in a town holds no
/// city, its reward lands in its base).
fn reward_city(state: &CampaignState, player: &FactionId) -> Option<SettlementId> {
    let capital = state.factions.get(player)?.capital.clone();
    if state.controls_province(player, &capital) {
        return state.province_city_id(&capital).cloned();
    }
    state
        .provinces
        .keys()
        .find(|p| state.controls_province(player, p))
        .and_then(|p| state.province_city_id(p).cloned())
        .or_else(|| {
            state
                .settlements
                .iter()
                .find(|(_, s)| &s.owner == player && &s.controller == player)
                .map(|(id, _)| id.clone())
        })
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
    bump(state, faction, MissionKind::WinBattle, 1);
}

/// `units` were recruited or hired by `faction` (hook of the player's orders).
pub(crate) fn note_units_recruited(state: &mut CampaignState, faction: &FactionId, units: u32) {
    bump(state, faction, MissionKind::RecruitUnits, units);
}

fn bump(state: &mut CampaignState, faction: &FactionId, kind: MissionKind, amount: u32) {
    if faction != &state.player_faction || state.missions.faction.as_ref() != Some(faction) {
        return;
    }
    for mission in state.missions.active.iter_mut().filter(|m| m.kind == kind) {
        mission.progress = mission.progress.saturating_add(amount);
    }
}

// ----- offers ----------------------------------------------------------------

/// At most one new mission per turn, when a slot is free and the cooldown has
/// elapsed; the template is drawn by weight among those with a plausible
/// target, the target at random among the plausible ones.
fn offer_mission(
    state: &mut CampaignState,
    data: &GameData,
    player: &FactionId,
    events: &mut Vec<GameEvent>,
) {
    let rules = &data.mission_rules;
    let turn = state.turn;
    if turn < rules.first_turn || state.missions.active.len() >= rules.max_active.min(2) as usize {
        return;
    }
    if state
        .missions
        .last_closed_turn
        .is_some_and(|closed| turn < closed + rules.offer_cooldown_turns)
    {
        return;
    }
    let active_kinds: BTreeSet<MissionKind> =
        state.missions.active.iter().map(|m| m.kind).collect();
    let pool: Vec<(&MissionTemplate, Vec<Target>)> = rules
        .templates
        .iter()
        .filter(|t| !active_kinds.contains(&t.kind))
        .map(|t| (t, candidates(state, data, player, t)))
        .filter(|(_, c)| !c.is_empty())
        .collect();
    let total: u32 = pool.iter().map(|(t, _)| t.weight.max(1)).sum();
    if total == 0 {
        return;
    }
    let mut rng = mission_rng(state.seed, turn);
    let mut roll = rng.below(total);
    let Some((template, targets)) = pool.iter().find(|(t, _)| {
        let w = t.weight.max(1);
        if roll < w {
            true
        } else {
            roll -= w;
            false
        }
    }) else {
        return;
    };
    let target = targets[rng.below(targets.len() as u32) as usize].clone();
    let count = if template.kind == MissionKind::HoldPlace {
        template.duration
    } else {
        template.count.max(1)
    };
    let fill = |text: &str| {
        text.replace("{cible}", &target.target_name)
            .replace("{lieu}", &target.place_name)
            .replace("{n}", &count.to_string())
    };
    let id = state.missions.next_id;
    state.missions.next_id += 1;
    let mission = Mission {
        id,
        template: template.id.clone(),
        kind: template.kind,
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
        baseline: if template.kind == MissionKind::ConcludeTreaty {
            treaty_tokens(state, player)
        } else {
            Vec::new()
        },
    };
    let text = format!(
        "Nouvelle mission : {}. {} Échéance : {}. Récompense : {}.",
        mission.title,
        mission.objective,
        deadline_label(state, mission.deadline_turn),
        reward_label(&mission.reward)
    );
    let mut event = GameEvent::new(EventKind::Mission, text.clone()).faction(player);
    if let Some(p) = &mission.province {
        event = event.province(p);
    }
    events.push(event);
    state.missions.notices.push(MissionNotice {
        kind: NoticeKind::Offered,
        mission: id,
        text,
    });
    state.missions.active.push(mission);
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
    let province_name = |p: &ProvinceId| {
        data.provinces
            .get(p)
            .map_or_else(|| p.to_string(), |p| p.name.display.clone())
    };
    let enemy_held = |p: &ProvinceId| {
        state
            .province_controller(p)
            .is_some_and(|c| enemies.contains(c))
    };
    match template.kind {
        MissionKind::TakeProvince => {
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
        MissionKind::HoldPlace => held
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
        MissionKind::WinBattle => {
            if enemies.is_empty() {
                Vec::new()
            } else {
                vec![Target::default()]
            }
        }
        MissionKind::RecruitUnits => {
            let can_recruit = held.iter().any(|p| {
                state
                    .province_city_id(p)
                    .is_some_and(|city| state.recruitable(data, city).iter().any(|o| o.available))
            });
            if can_recruit {
                vec![Target::default()]
            } else {
                Vec::new()
            }
        }
        MissionKind::ConstructBuilding => {
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
                            place_name: crate::siege::settlement_name(data, city),
                            province: Some((*p).clone()),
                            settlement: Some(city.clone()),
                            building: Some(option.building.clone()),
                        });
                    }
                }
            }
            targets
        }
        MissionKind::ConcludeTreaty => {
            let partner = state.factions.iter().any(|(id, f)| {
                id != player
                    && f.alive
                    && !faction.allies.contains(id)
                    && id != &state.player_faction
            });
            if partner {
                vec![Target::default()]
            } else {
                Vec::new()
            }
        }
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

fn progress_of(state: &CampaignState, data: &GameData, mission: &Mission) -> (String, f64) {
    let player = &state.player_faction;
    let ratio = |done: u32, of: u32| f64::from(done.min(of)) / f64::from(of.max(1));
    match mission.kind {
        MissionKind::TakeProvince => {
            let holder = mission
                .province
                .as_ref()
                .and_then(|p| state.province_controller(p));
            match holder {
                Some(h) if h == player => ("prise".to_owned(), 1.0),
                Some(h) => {
                    let name = data
                        .factions
                        .get(h)
                        .map_or_else(|| h.to_string(), |f| f.short_or_display_name().to_owned());
                    (format!("tenue par {name}"), 0.0)
                }
                None => ("introuvable".to_owned(), 0.0),
            }
        }
        MissionKind::WinBattle => (
            format!(
                "{}/{} victoire{}",
                mission.progress.min(mission.count),
                mission.count,
                if mission.count > 1 { "s" } else { "" }
            ),
            ratio(mission.progress, mission.count),
        ),
        MissionKind::RecruitUnits => (
            format!(
                "{}/{} unités",
                mission.progress.min(mission.count),
                mission.count
            ),
            ratio(mission.progress, mission.count),
        ),
        MissionKind::ConstructBuilding => {
            let settlement = mission
                .settlement
                .as_ref()
                .and_then(|s| state.settlements.get(s));
            let building = mission.building.as_ref();
            match (settlement, building) {
                (Some(s), Some(b)) if s.buildings.contains(b) => ("achevé".to_owned(), 1.0),
                (Some(s), Some(b)) if s.construction.as_ref().is_some_and(|c| &c.building == b) => {
                    ("en chantier".to_owned(), 0.5)
                }
                _ => ("pas encore commencé".to_owned(), 0.0),
            }
        }
        MissionKind::ConcludeTreaty => ("aucun traité nouveau".to_owned(), 0.0),
        MissionKind::HoldPlace => {
            let held = state.turn.saturating_sub(mission.issued_turn);
            (
                format!(
                    "{}/{} saisons tenues",
                    held.min(mission.count),
                    mission.count
                ),
                ratio(held, mission.count),
            )
        }
    }
}

impl CampaignState {
    /// The player's active missions, for the objectives panel.
    pub fn missions(&self, data: &GameData) -> Vec<MissionView> {
        self.missions
            .active
            .iter()
            .map(|m| {
                let (progress, progress_ratio) = progress_of(self, data, m);
                MissionView {
                    id: m.id,
                    kind: m.kind.key().to_owned(),
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

    /// Notices (offered, succeeded, failed) of the last resolution.
    pub fn mission_notices(&self) -> &[MissionNotice] {
        &self.missions.notices
    }
}
