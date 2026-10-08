//! Lot CT1: the record of the AI turn, for the Total War-style replay of the
//! AI armies' marches on the campaign map (ADR 0073).
//!
//! The AI turn is resolved at once inside `end_turn` (lot M3). When the
//! interface asks for it ([`CampaignState::set_ai_replay_recording`]), each
//! AI army move of that turn is recorded as an [`AiMoveRecord`]: the path it
//! walked (map pixels, start included), what the move ended in (march,
//! siege, capture, battle, landing), whether the player may see it (his
//! vision before or after the AI turn, lot M5a) and whether it concerns him
//! (its notability, which the camera follows). Nothing here changes the
//! game: recording only reads the state, and costs nothing when it is off.
//!
//! Records are rebuilt at each `end_turn` and never saved.

use data_model::key_enum;
use std::collections::{BTreeMap, BTreeSet};

use data_model::{FactionId, GameData, ProvinceId, SettlementId};
use serde::Serialize;

use crate::march::{px_per_km, MoveReport, StopReason};
use crate::orders::{Order, OrderOutcome};
use crate::state::{ArmyId, CampaignState};
use crate::vision::VisionMask;

key_enum! {
/// What an AI army move ended in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AiMoveKind {
    /// A plain march (stopped in the field, out of points or at a zone of control).
    March => "march",
    /// The army entered a friendly or neutral settlement.
    Stationed => "stationed",
    /// The army laid siege to a settlement.
    SiegeStarted => "siege_started",
    /// The army took a settlement without a fight.
    SettlementTaken => "settlement_taken",
    /// The army attacked another army (the battle is auto-resolved).
    Battle => "battle",
    /// The army crossed the sea and landed in a port.
    Landing => "landing",
}
}

impl AiMoveKind {
    fn from_stop(stop: &StopReason) -> (Self, Option<SettlementId>, Option<ArmyId>) {
        match stop {
            StopReason::Stationed { settlement } => {
                (AiMoveKind::Stationed, Some(settlement.clone()), None)
            }
            StopReason::SiegeStarted { settlement } => {
                (AiMoveKind::SiegeStarted, Some(settlement.clone()), None)
            }
            StopReason::SettlementTaken { settlement } => {
                (AiMoveKind::SettlementTaken, Some(settlement.clone()), None)
            }
            StopReason::Engaged { army } => (AiMoveKind::Battle, None, Some(army.clone())),
            StopReason::EnemyZoneOfControl { army } => {
                (AiMoveKind::March, None, Some(army.clone()))
            }
            _ => (AiMoveKind::March, None, None),
        }
    }
}

/// Why a move concerns the player (the camera follows it), by increasing
/// priority. Moves of the player's allies and vassals only concern him when
/// they fight or besiege him (never, in practice).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AiMoveNotability {
    /// Ends near a player army or settlement.
    NearPlayer = 1,
    /// Walks on land the player controlled before the AI turn.
    PlayerTerritory = 2,
    /// Besieges or takes a settlement the player controlled.
    Siege = 3,
    /// Attacks a player army.
    Battle = 4,
}

impl AiMoveNotability {
    pub fn as_str(self) -> &'static str {
        match self {
            AiMoveNotability::NearPlayer => "near_player",
            AiMoveNotability::PlayerTerritory => "player_territory",
            AiMoveNotability::Siege => "siege",
            AiMoveNotability::Battle => "battle",
        }
    }

    /// 1 (lowest) to 4 (highest).
    pub fn priority(self) -> u8 {
        self as u8
    }
}

/// One AI army move of the last AI turn (lot CT1).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AiMoveRecord {
    /// Order of the move in the AI turn (0, 1, …): factions by id, each
    /// faction's resumed marches then its orders.
    pub sequence: u32,
    pub army: ArmyId,
    pub faction: FactionId,
    /// Map-pixel points walked, the start included (at least two).
    pub path: Vec<[f32; 2]>,
    pub kind: AiMoveKind,
    /// Settlement entered, besieged, taken or landed in.
    pub settlement: Option<SettlementId>,
    /// Army attacked, or whose zone of control stopped the march.
    pub target_army: Option<ArmyId>,
    /// Faction of `target_army` when the move was made.
    pub target_faction: Option<FactionId>,
    /// Controller of `settlement` before the move.
    pub settlement_controller: Option<FactionId>,
    /// The player sees part of the path (his vision before or after the AI
    /// turn); `visible_from..=visible_to` is the seen span of `path`.
    pub visible: bool,
    pub visible_from: usize,
    pub visible_to: usize,
    /// Why the move concerns the player, if it does.
    pub notable: Option<AiMoveNotability>,
}

/// Recording switch and tuning, set by the interface (never saved).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AiReplayLog {
    pub enabled: bool,
    /// Radius (km) around player armies and settlements within which the
    /// end of a move is notable.
    pub notable_radius_km: f64,
    pub records: Vec<AiMoveRecord>,
    /// Snapshot of the player's side before the AI turn.
    before: Option<PlayerSnapshot>,
}

#[derive(Debug, Clone, PartialEq)]
struct PlayerSnapshot {
    vision: VisionMask,
    provinces: BTreeSet<ProvinceId>,
    points: Vec<[f32; 2]>,
}

impl CampaignState {
    /// Turns the AI turn record on or off (lot CT1); `notable_radius_km`
    /// tunes [`AiMoveNotability::NearPlayer`].
    pub fn set_ai_replay_recording(&mut self, enabled: bool, notable_radius_km: f64) {
        self.ai_replay.enabled = enabled;
        self.ai_replay.notable_radius_km = notable_radius_km.max(0.0);
        if !enabled {
            self.ai_replay.records.clear();
            self.ai_replay.before = None;
        }
    }

    /// The AI army moves of the last `end_turn`, in the order they were
    /// played (empty when recording is off).
    pub fn ai_turn_moves(&self) -> &[AiMoveRecord] {
        &self.ai_replay.records
    }

    /// Player vision, provinces and army/settlement points (map pixels).
    fn player_snapshot(&self, data: &GameData) -> PlayerSnapshot {
        let player = &self.player_faction;
        let vision = self.vision(data, player).mask;
        let provinces = self
            .provinces
            .keys()
            .filter(|p| self.province_controller(p) == Some(player))
            .cloned()
            .collect();
        let mut points: Vec<[f32; 2]> = self
            .armies
            .values()
            .filter(|a| &a.faction == player)
            .map(|a| self.army_point(data, a))
            .collect();
        points.extend(
            self.settlements
                .iter()
                .filter(|(_, s)| &s.controller == player)
                .filter_map(|(id, _)| data.settlement_point(id)),
        );
        PlayerSnapshot {
            vision,
            provinces,
            points,
        }
    }

    /// Start of the AI turns: clears the record and snapshots the player.
    pub(crate) fn ai_replay_begin(&mut self, data: &GameData) {
        self.ai_replay.records.clear();
        self.ai_replay.before = None;
        if self.ai_replay.enabled {
            self.ai_replay.before = Some(self.player_snapshot(data));
        }
    }

    /// Applies `order` for AI `faction`, recording the move it makes.
    pub(crate) fn apply_ai_order(&mut self, data: &GameData, faction: &FactionId, order: Order) {
        if !self.ai_replay.enabled {
            let _ = self.apply_order(data, faction, order);
            return;
        }
        let (army, target) = match &order {
            Order::MoveArmy { army, .. } | Order::Embark { army, .. } => (army.clone(), None),
            Order::Attack { army, target_army } => (army.clone(), Some(target_army.clone())),
            _ => {
                let _ = self.apply_order(data, faction, order);
                return;
            }
        };
        let Some(start) = self.armies.get(&army).map(|a| self.army_point(data, a)) else {
            let _ = self.apply_order(data, faction, order);
            return;
        };
        let target_faction = target
            .as_ref()
            .and_then(|t| self.armies.get(t))
            .map(|a| a.faction.clone());
        let embark_to = match &order {
            Order::Embark { to_port, .. } => Some(to_port.clone()),
            _ => None,
        };
        let controllers = self.controllers();
        match self.apply_order_outcome(data, faction, order) {
            Ok(OrderOutcome::Moved(report)) => {
                self.record_report(faction, start, &report, target_faction, &controllers);
            }
            Ok(OrderOutcome::Done) => {
                let Some(port) = embark_to else {
                    return;
                };
                let end = self
                    .armies
                    .get(&army)
                    .map(|a| self.army_point(data, a))
                    .or_else(|| data.settlement_point(&port));
                if let Some(end) = end {
                    let controller = controllers.get(&port).cloned();
                    self.push_record(AiMoveRecord {
                        sequence: 0,
                        army,
                        faction: faction.clone(),
                        path: vec![start, end],
                        kind: AiMoveKind::Landing,
                        settlement: Some(port),
                        target_army: None,
                        target_faction: None,
                        settlement_controller: controller,
                        visible: false,
                        visible_from: 0,
                        visible_to: 0,
                        notable: None,
                    });
                }
            }
            Err(_) => {}
        }
    }

    /// Resumes the marches of AI `faction`, recording them.
    pub(crate) fn continue_ai_marches(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        events: &mut Vec<crate::events::GameEvent>,
    ) {
        if !self.ai_replay.enabled {
            crate::march::continue_marches(self, data, Some(faction), events);
            return;
        }
        let starts: BTreeMap<ArmyId, [f32; 2]> = self
            .armies
            .iter()
            .filter(|(_, a)| &a.faction == faction)
            .map(|(id, a)| (id.clone(), self.army_point(data, a)))
            .collect();
        let controllers = self.controllers();
        let reports = crate::march::continue_marches(self, data, Some(faction), events);
        for report in reports {
            if let Some(start) = starts.get(&report.army) {
                self.record_report(faction, *start, &report, None, &controllers);
            }
        }
    }

    fn controllers(&self) -> BTreeMap<SettlementId, FactionId> {
        self.settlements
            .iter()
            .map(|(id, s)| (id.clone(), s.controller.clone()))
            .collect()
    }

    fn record_report(
        &mut self,
        faction: &FactionId,
        start: [f32; 2],
        report: &MoveReport,
        target_faction: Option<FactionId>,
        controllers: &BTreeMap<SettlementId, FactionId>,
    ) {
        if report.walked.is_empty() {
            return;
        }
        let (kind, settlement, target_army) = AiMoveKind::from_stop(&report.stop);
        let target_faction = target_faction.or_else(|| {
            target_army
                .as_ref()
                .and_then(|t| self.armies.get(t))
                .map(|a| a.faction.clone())
        });
        let settlement_controller = settlement
            .as_ref()
            .and_then(|s| controllers.get(s).cloned());
        let mut path = Vec::with_capacity(report.walked.len() + 1);
        path.push(start);
        path.extend(report.walked.iter().copied());
        self.push_record(AiMoveRecord {
            sequence: 0,
            army: report.army.clone(),
            faction: faction.clone(),
            path,
            kind,
            settlement,
            target_army,
            target_faction,
            settlement_controller,
            visible: false,
            visible_from: 0,
            visible_to: 0,
            notable: None,
        });
    }

    /// Appends `record`, or extends the previous record of the same army
    /// when it continues it (a resumed march followed by a new order).
    fn push_record(&mut self, mut record: AiMoveRecord) {
        let records = &mut self.ai_replay.records;
        if let Some(last) = records.last_mut() {
            let continues = last.army == record.army
                && matches!(last.kind, AiMoveKind::March | AiMoveKind::Stationed)
                && record.kind != AiMoveKind::Landing
                && last.path.last() == record.path.first();
            if continues {
                last.path.extend(record.path.drain(1..));
                last.kind = record.kind;
                last.settlement = record.settlement;
                last.target_army = record.target_army;
                last.target_faction = record.target_faction;
                last.settlement_controller = record.settlement_controller;
                return;
            }
        }
        record.sequence = records.len() as u32;
        records.push(record);
    }

    /// End of the turn: visibility (the player's vision before or after the
    /// AI turn) and notability of each record.
    pub(crate) fn ai_replay_finish(&mut self, data: &GameData) {
        let Some(before) = self.ai_replay.before.take() else {
            return;
        };
        if self.ai_replay.records.is_empty() {
            return;
        }
        let after = self.player_snapshot(data).vision;
        let player = self.player_faction.clone();
        let radius = self.ai_replay.notable_radius_km as f32 * px_per_km(data);
        let mut records = std::mem::take(&mut self.ai_replay.records);
        for record in &mut records {
            let seen: Vec<usize> = record
                .path
                .iter()
                .enumerate()
                .filter(|(_, p)| before.vision.sees_point(data, **p) || after.sees_point(data, **p))
                .map(|(i, _)| i)
                .collect();
            if let (Some(first), Some(last)) = (seen.first(), seen.last()) {
                record.visible = true;
                record.visible_from = *first;
                record.visible_to = *last;
            }
            let allied = self.is_allied(&record.faction, &player);
            record.notable = notability(data, record, &player, allied, &before, radius);
        }
        self.ai_replay.records = records;
    }
}

fn notability(
    data: &GameData,
    record: &AiMoveRecord,
    player: &FactionId,
    allied: bool,
    before: &PlayerSnapshot,
    radius_px: f32,
) -> Option<AiMoveNotability> {
    if record.kind == AiMoveKind::Battle && record.target_faction.as_ref() == Some(player) {
        return Some(AiMoveNotability::Battle);
    }
    if matches!(
        record.kind,
        AiMoveKind::SiegeStarted | AiMoveKind::SettlementTaken | AiMoveKind::Landing
    ) && record.settlement_controller.as_ref() == Some(player)
    {
        return Some(AiMoveNotability::Siege);
    }
    // An ally's (or vassal's) army on the player's land or next to his
    // armies is no news: the camera keeps to threats.
    if allied {
        return None;
    }
    let on_player_land = record.path.iter().any(|p| {
        data.province_at_point(p[0], p[1])
            .is_some_and(|province| before.provinces.contains(province))
    });
    if on_player_land {
        return Some(AiMoveNotability::PlayerTerritory);
    }
    let end = *record.path.last()?;
    let near = before.points.iter().any(|p| {
        let dx = p[0] - end[0];
        let dy = p[1] - end[1];
        dx * dx + dy * dy <= radius_px * radius_px
    });
    near.then_some(AiMoveNotability::NearPlayer)
}

#[cfg(test)]
mod key_enum_tests {
    use super::*;
    use data_model::key_enum::assert_keys_match_serde;

    #[test]
    fn keys_match_serde_names() {
        assert_keys_match_serde::<AiMoveKind>();
    }
}
