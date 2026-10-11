//! RG9 (ADR 0343): river navigation between riverside places.
//!
//! `data/movement/rivers.json` lists, for each navigable river, its places
//! from the source to the mouth with their position along the course. An
//! army standing in one of them may sail to another place of the same river
//! (order `SailRiver`), spending movement points: the river distance in
//! kilometres, priced like a road (`road_cost_factor`) times
//! `river_downstream_factor` (0.5) or `river_upstream_factor` (0.8), times
//! the general's `RiverMovement` effects (Batelier). The voyage fits in the
//! army's remaining movement of the turn; it neither enters nor passes a
//! place held by an enemy or holding an enemy army, nor changes river.

use std::collections::BTreeMap;

use data_model::util::dist;
use data_model::{EffectKind, GameData, SettlementId};
use serde::{Deserialize, Serialize};

use crate::events::GameEvent;
use crate::march::{enter_settlement, km_to_grid_points, MoveReport};
use crate::orders::OrderError;
use crate::state::{Army, ArmyId, ArmyPosition, CampaignState};

/// A river voyage of an army from the place it stands in.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RiverVoyage {
    /// Id of the river (`riv_loire`).
    pub river: String,
    /// Display name of the river.
    pub river_name: String,
    /// Places on the way, destination last.
    pub stops: Vec<SettlementId>,
    /// Distance along the course.
    pub km: f64,
    /// `true` when sailing with the current.
    pub downstream: bool,
    /// Movement points the voyage costs this army.
    pub cost: u32,
    /// Cost of the same distance on a road, for the tooltip.
    pub road_cost: u32,
}

impl RiverVoyage {
    /// The destination.
    pub fn destination(&self) -> &SettlementId {
        self.stops.last().expect("a voyage has a destination")
    }
}

impl CampaignState {
    /// Movement points of `km` of river for `army`: the road price of the
    /// distance, times the current factor and the general's `RiverMovement`.
    pub fn river_cost(
        &self,
        data: &GameData,
        army: &Army,
        km: f64,
        downstream: bool,
    ) -> (u32, u32) {
        let rules = data.free_movement_rules();
        let road = km_to_grid_points(data, km) as f64 * rules.road_cost_factor;
        let current = if downstream {
            rules.river_downstream_factor
        } else {
            rules.river_upstream_factor
        };
        let boatman = army.general.as_ref().map_or(0.0, |id| {
            crate::skills::character_effects(self, data, id)[EffectKind::RiverMovement].percent
        });
        let cost = road * current * (1.0 + boatman / 100.0).max(0.1);
        ((cost.round() as u32).max(1), (road.round() as u32).max(1))
    }

    /// Places an army may not sail through or into.
    fn river_blocked(&self, army: &Army, place: &SettlementId) -> bool {
        self.is_hostile_settlement(&army.faction, place)
            || !self.hostile_armies_at(&army.faction, place).is_empty()
    }

    /// Every river voyage `army` could make from the place it stands in,
    /// whatever its movement left (see [`RiverVoyage::cost`]).
    pub fn river_voyages_all(
        &self,
        data: &GameData,
        army_id: &ArmyId,
    ) -> BTreeMap<SettlementId, RiverVoyage> {
        let mut out: BTreeMap<SettlementId, RiverVoyage> = BTreeMap::new();
        let Some(army) = self.armies.get(army_id) else {
            return out;
        };
        let Some(from) = army.settlement() else {
            return out;
        };
        for river in &data.navigable_rivers.rivers {
            let Some(start) = river.places.iter().position(|p| &p.settlement == from) else {
                continue;
            };
            for downstream in [true, false] {
                let mut stops = Vec::new();
                let mut index = start;
                loop {
                    let next = if downstream {
                        index + 1
                    } else {
                        match index.checked_sub(1) {
                            Some(previous) => previous,
                            None => break,
                        }
                    };
                    let Some(place) = river.places.get(next) else {
                        break;
                    };
                    if self.river_blocked(army, &place.settlement)
                        || !self.settlements.contains_key(&place.settlement)
                    {
                        break;
                    }
                    stops.push(place.settlement.clone());
                    let km = (place.km - river.places[start].km).abs();
                    let (cost, road_cost) = self.river_cost(data, army, km, downstream);
                    let voyage = RiverVoyage {
                        river: river.id.clone(),
                        river_name: river.name.display.clone(),
                        stops: stops.clone(),
                        km,
                        downstream,
                        cost,
                        road_cost,
                    };
                    let better = out
                        .get(&place.settlement)
                        .is_none_or(|known| known.cost > voyage.cost);
                    if better {
                        out.insert(place.settlement.clone(), voyage);
                    }
                    index = next;
                }
            }
        }
        out.remove(from);
        out
    }

    /// The river voyages `army` can afford with the movement it has left.
    pub fn river_voyages(
        &self,
        data: &GameData,
        army_id: &ArmyId,
    ) -> BTreeMap<SettlementId, RiverVoyage> {
        let left = self.armies.get(army_id).map_or(0, |a| a.movement_left);
        let mut voyages = self.river_voyages_all(data, army_id);
        voyages.retain(|_, voyage| voyage.cost <= left);
        voyages
    }

    /// Order `SailRiver`: `army`, standing in a riverside place, sails to
    /// `to`, another place of the same river, spending the voyage's cost
    /// from its movement left.
    pub(crate) fn order_sail_river(
        &mut self,
        data: &GameData,
        faction: &data_model::FactionId,
        army: &ArmyId,
        to: &SettlementId,
        events: &mut Vec<GameEvent>,
    ) -> Result<MoveReport, OrderError> {
        let entry = self.own_army(faction, army)?;
        let from = entry
            .settlement()
            .cloned()
            .ok_or(OrderError::NotInSettlement)?;
        if !data.settlements.contains_key(to) {
            return Err(OrderError::UnknownSettlement(to.clone()));
        }
        if entry.movement_left == 0 {
            return Err(OrderError::NoMovementLeft);
        }
        let voyage = self
            .river_voyages_all(data, army)
            .remove(to)
            .ok_or_else(|| OrderError::NoRiverRoute {
                from: data.settlement_name(&from),
                to: data.settlement_name(to),
            })?;
        if voyage.cost > entry.movement_left {
            return Err(OrderError::RiverTooFar {
                cost: voyage.cost,
                left: entry.movement_left,
            });
        }
        let besieged = self
            .armies
            .get(army)
            .filter(|a| a.stance == crate::state::Stance::Siege)
            .and_then(|a| a.settlement().cloned());
        crate::posture::leave_static_stance(self, army);
        if let Some(settlement) = besieged {
            crate::siege::leave_siege(self, data, army, &settlement, events);
        }
        if let Some(a) = self.armies.get_mut(army) {
            a.movement_left -= voyage.cost;
            a.clear_plan();
        }
        let mut walked: Vec<[f32; 2]> = Vec::new();
        for stop in &voyage.stops {
            if let Some(point) = data.settlement_point(stop) {
                if walked.last().is_none_or(|last| dist(*last, point) > 0.0) {
                    walked.push(point);
                }
            }
        }
        let stop = enter_settlement(self, data, army, to, events);
        if let Some(a) = self.armies.get_mut(army) {
            a.position = ArmyPosition::Settlement(to.clone());
        }
        Ok(MoveReport {
            army: army.clone(),
            walked,
            cost: voyage.cost,
            stop,
            planned_path: Vec::new(),
        })
    }
}
