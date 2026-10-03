//! Siege engines built on the spot (lot NT5, N7, ADR 0128;
//! `data/rules/siege_engines.json`).
//!
//! Every turn a siege progresses, the men of the besieging armies put
//! `men / men_per_work_point` work points (at least `min_work_per_turn`,
//! raised by the general's `SiegeSpeed` and the army's traditions) into the
//! engines, built one after the other in the order of the file (ladders,
//! ram, siege tower). Behind standing walls an assault needs at least one
//! ready engine; the ready engines are brought to the siege battle, and a
//! ready siege tower counts as one for the auto-resolved assault.

use data_model::{BuiltEngineKind, GameData, SettlementId};
use serde::{Deserialize, Serialize};

use crate::state::{ArmyId, CampaignState};

/// State of one engine of a siege (UI: « Engins de siège »).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineStatus {
    pub id: String,
    pub kind: BuiltEngineKind,
    /// French display name.
    pub name: String,
    pub ready: bool,
    /// Turns of work left at the present pace (0 when ready).
    pub turns_left: u32,
}

/// Work points the `men` of the besiegers put into the engines each turn,
/// with the besieging general's and traditions' `siege_speed_percent`.
pub fn work_per_turn(data: &GameData, men: u32, siege_speed_percent: f64) -> u32 {
    let rules = &data.siege_engine_rules;
    let base = (men / rules.men_per_work_point.max(1)).max(rules.min_work_per_turn);
    (f64::from(base) * (1.0 + siege_speed_percent.max(0.0) / 100.0)).round() as u32
}

/// Engines of a siege with `work` points done, at `rate` points per turn.
///
/// A6-L2: against walls of `walls` level the engines cost more work (see
/// `SiegeEngineRule::cost`).
pub fn statuses(data: &GameData, work: u32, rate: u32, walls: u32) -> Vec<EngineStatus> {
    let min_level = data.siege_engine_rules.scaling_min_wall_level;
    let mut needed = 0u32;
    data.siege_engine_rules
        .engines
        .iter()
        .map(|engine| {
            needed = needed.saturating_add(engine.cost(min_level, walls));
            let left = needed.saturating_sub(work);
            EngineStatus {
                id: engine.id.clone(),
                kind: engine.kind,
                name: engine.name.clone(),
                ready: left == 0,
                turns_left: left.div_ceil(rate.max(1)),
            }
        })
        .collect()
}

/// The engine kinds ready with `work` points done.
pub fn ready_kinds(data: &GameData, work: u32, walls: u32) -> Vec<BuiltEngineKind> {
    statuses(data, work, 1, walls)
        .into_iter()
        .filter(|s| s.ready)
        .map(|s| s.kind)
        .collect()
}

/// Men of the besieging `armies` (the workforce of the engines).
pub(crate) fn men(state: &CampaignState, armies: &[ArmyId]) -> u32 {
    armies
        .iter()
        .filter_map(|id| state.armies.get(id))
        .flat_map(|a| a.units.iter())
        .map(|u| u.strength)
        .sum()
}

impl CampaignState {
    /// Engine work of the siege of `settlement` (0 without a siege).
    fn engine_work(&self, settlement: &SettlementId) -> u32 {
        self.settlements
            .get(settlement)
            .and_then(|s| s.siege.as_ref())
            .map_or(0, |s| s.engine_work)
    }

    /// Work points per turn the present besiegers of `settlement` put into
    /// the engines (0 without a siege).
    pub fn engine_rate(&self, data: &GameData, settlement: &SettlementId) -> u32 {
        let Some(settlement_state) = self.settlements.get(settlement) else {
            return 0;
        };
        let Some(siege) = settlement_state.siege.as_ref() else {
            return 0;
        };
        let besiegers = crate::siege::besiegers(self, settlement, &settlement_state.controller);
        let lead = besiegers
            .iter()
            .find(|id| self.armies[*id].faction == siege.attacker)
            .or(besiegers.first());
        let speed = lead.map_or(0.0, |id| crate::siege::siege_speed_percent(self, data, id));
        work_per_turn(data, men(self, &besiegers), speed)
    }

    /// NT5: the engines of the siege of `settlement`, ready or with the
    /// turns left at the present pace (empty without a siege).
    pub fn siege_engines(&self, data: &GameData, settlement: &SettlementId) -> Vec<EngineStatus> {
        if self
            .settlements
            .get(settlement)
            .is_none_or(|s| s.siege.is_none())
        {
            return Vec::new();
        }
        statuses(
            data,
            self.engine_work(settlement),
            self.engine_rate(data, settlement),
            self.fortification_level(data, settlement),
        )
    }

    /// NT9: the attacker's damage bonus (percent) of the engines ready at
    /// the siege of `settlement` in an auto-resolved assault behind
    /// standing walls (the ram breaks the gate).
    pub(crate) fn engine_assault_bonus(&self, data: &GameData, settlement: &SettlementId) -> u32 {
        let work = self.engine_work(settlement);
        let walls = self.fortification_level(data, settlement);
        let min_level = data.siege_engine_rules.scaling_min_wall_level;
        let mut needed = 0u32;
        let mut bonus = 0u32;
        for engine in &data.siege_engine_rules.engines {
            needed = needed.saturating_add(engine.cost(min_level, walls));
            if work < needed {
                break;
            }
            bonus = bonus.saturating_add(engine.auto_assault_bonus_percent);
        }
        bonus
    }

    /// A6-L2: share (percent) of the engines of the siege of `settlement`
    /// that are ready against its walls.
    pub(crate) fn engines_ready_percent(&self, data: &GameData, settlement: &SettlementId) -> u32 {
        let work = self.engine_work(settlement);
        let walls = self.fortification_level(data, settlement);
        let statuses = statuses(data, work, 1, walls);
        let ready = statuses.iter().filter(|s| s.ready).count();
        (100 * ready / statuses.len().max(1)) as u32
    }

    /// A6-L2: the walls of `settlement` for the assault resolution.
    pub(crate) fn wall_stand(
        &self,
        data: &GameData,
        settlement: &SettlementId,
    ) -> crate::battle_auto::WallStand {
        crate::battle_auto::WallStand {
            level: self.fortification_level(data, settlement),
            breach_percent: self
                .settlements
                .get(settlement)
                .and_then(|s| s.siege.as_ref())
                .map_or(0, |s| u32::from(s.breach)),
            engines_ready_percent: self.engines_ready_percent(data, settlement),
        }
    }

    /// Siege towers built at the siege of `settlement`.
    pub(crate) fn built_towers(&self, data: &GameData, settlement: &SettlementId) -> usize {
        ready_kinds(
            data,
            self.engine_work(settlement),
            self.fortification_level(data, settlement),
        )
        .into_iter()
        .filter(|k| *k == BuiltEngineKind::Tower)
        .count()
    }

    /// NT5: why `army` may not storm the place it besieges yet (French), or
    /// `None`: behind standing walls (fortified, breach under 50, no siege
    /// tower in the army) at least one engine must be ready.
    pub fn assault_blocker(&self, data: &GameData, army: &ArmyId) -> Option<String> {
        let place = self.armies.get(army)?.settlement()?.clone();
        if !crate::siege::walls_stand(self, data, army, &place) {
            return None;
        }
        let engines = self.siege_engines(data, &place);
        if engines.is_empty() || engines.iter().any(|e| e.ready) {
            return None;
        }
        let next = engines.iter().min_by_key(|e| e.turns_left)?;
        Some(format!(
            "murailles intactes et aucun engin prêt ({} dans {} tour{})",
            next.name.to_lowercase(),
            next.turns_left,
            if next.turns_left > 1 { "s" } else { "" }
        ))
    }

    /// The engines built at the siege of `settlement`, for its siege battle.
    pub(crate) fn battle_engines(
        &self,
        data: &GameData,
        settlement: &SettlementId,
    ) -> sim_battle::SiegeEngineSetup {
        let work = self.engine_work(settlement);
        let walls = self.fortification_level(data, settlement);
        let min_level = data.siege_engine_rules.scaling_min_wall_level;
        let mut setup = sim_battle::SiegeEngineSetup::default();
        let mut needed = 0u32;
        for engine in &data.siege_engine_rules.engines {
            needed = needed.saturating_add(engine.cost(min_level, walls));
            if work < needed {
                break;
            }
            match engine.kind {
                BuiltEngineKind::Ladders => setup.ladders = true,
                BuiltEngineKind::Ram => setup.ram = true,
                BuiltEngineKind::Tower => {
                    let unit_type = engine
                        .tower_unit_type
                        .as_ref()
                        .and_then(|id| data.unit_types.get(id));
                    if let Some(unit_type) = unit_type {
                        setup.towers.push(sim_battle::UnitSetup::from_unit_type(
                            unit_type,
                            unit_type.soldiers,
                            unit_type.stats.morale,
                            0,
                        ));
                    }
                }
            }
        }
        setup
    }
}
