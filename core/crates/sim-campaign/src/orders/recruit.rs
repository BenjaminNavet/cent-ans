//! Recruitment: the panel lines, the price and the `Recruit` order.
//!
//! Everything about recruiting in one settlement goes through a
//! [`RecruitContext`], built once per (faction, settlement): the effects
//! (buildings, governor, edict, technologies), the slots and the prices are
//! computed a single time and shared by the player's order, the panel, the
//! AI's planning and the missions that probe the options.

use std::collections::BTreeMap;

use data_model::{EffectKind, FactionId, GameData, ResourceId, SettlementId, UnitType, UnitTypeId};
use serde::{Deserialize, Serialize};

use super::OrderError;
use crate::buildings::EffectTotals;
use crate::state::{CampaignState, FactionState, QueuedRecruit, SettlementState};

/// G1: recruitments every settlement can queue per turn before buildings.
pub const BASE_RECRUIT_SLOTS: usize = 2;

/// One line of the recruitment panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecruitOption {
    pub unit_type: UnitTypeId,
    pub name: String,
    pub cost: u32,
    pub upkeep: u32,
    pub available: bool,
    /// French explanation when `available` is `false`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// SV2: resource units the unit needs (`cost.resources`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub resources: BTreeMap<ResourceId, u32>,
    /// SV2: part of `cost` paid to import the missing resource units
    /// (B7c rule, ADR 0053).
    #[serde(default)]
    pub import_cost: u32,
    /// SV2: resource units the faction lacks and must import.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub imported: BTreeMap<ResourceId, u32>,
    /// TW2-T2: the settlement's reserve of this unit type.
    #[serde(default)]
    pub pool: crate::recruit_pool::PoolView,
}

/// U13: where a recruitment line belongs in the settlement panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecruitGroup {
    /// Can be recruited now.
    Ready,
    /// Blocked for a passing reason (treasury, full queue, empty reserve...).
    Blocked,
    /// Waits for a technology, a building or a date: shown under « Bientôt ».
    Soon,
    /// Reserved to other factions or cultures: not shown.
    Elsewhere,
}

impl RecruitGroup {
    /// Stable name for the interface.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Blocked => "blocked",
            Self::Soon => "soon",
            Self::Elsewhere => "elsewhere",
        }
    }
}

impl RecruitOption {
    /// U13: the panel group of this line, from its blocker.
    pub fn group(&self) -> RecruitGroup {
        let Some(reason) = self.reason.as_deref().filter(|_| !self.available) else {
            return RecruitGroup::Ready;
        };
        if reason.starts_with("réservé à") || reason.starts_with("culture locale") {
            RecruitGroup::Elsewhere
        } else if reason.starts_with("bâtiment requis")
            || reason.starts_with("technologie requise")
            || reason.starts_with("disponible à partir")
        {
            RecruitGroup::Soon
        } else {
            RecruitGroup::Blocked
        }
    }
}

/// SV2: full price of one recruit — money cost plus the import of the
/// resource units the faction's free supply lacks.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RecruitPrice {
    /// Total livres (recruitment cost plus imports, at current prices).
    pub cost: u32,
    /// Part of `cost` paid for imports.
    pub import_cost: u32,
    /// Units drawn from the supply and imported.
    pub draw: crate::buildings::ResourceDraw,
}

/// Everything recruiting in one settlement for one faction depends on,
/// computed once: the settlement and faction state, the merged effects, the
/// slots, the prices and the difficulty.
pub struct RecruitContext<'a> {
    state: &'a CampaignState,
    data: &'a GameData,
    faction: &'a FactionId,
    settlement_id: &'a SettlementId,
    settlement: &'a SettlementState,
    faction_state: &'a FactionState,
    /// Settlement effects merged with the technologies of `faction`.
    effects: EffectTotals,
    /// `recruit_slots` points of the settlement alone (feeds the reserves).
    slot_points: u32,
    slots: usize,
    ordered_this_turn: usize,
    price_factor: f64,
    difficulty_percent: u32,
}

impl<'a> RecruitContext<'a> {
    /// The context of `faction` recruiting in `settlement`; `None` when
    /// either is unknown.
    pub fn new(
        state: &'a CampaignState,
        data: &'a GameData,
        faction: &'a FactionId,
        settlement_id: &'a SettlementId,
    ) -> Option<Self> {
        let settlement = state.settlements.get(settlement_id)?;
        let faction_state = state.factions.get(faction)?;
        let own_effects = state.settlement_effects(data, settlement_id);
        let slot_points = own_effects[EffectKind::RecruitSlots].flat.max(0.0) as u32;
        let mut effects = own_effects;
        effects.merge(&crate::research::faction_tech_effects(state, data, faction));
        // The slots follow the controller's technologies.
        let slot_effects = if &settlement.controller == faction {
            effects
        } else {
            let mut controller_effects = own_effects;
            controller_effects.merge(&crate::research::faction_tech_effects(
                state,
                data,
                &settlement.controller,
            ));
            controller_effects
        };
        let capital = state.is_capital_city(settlement_id);
        let slots = BASE_RECRUIT_SLOTS
            + usize::from(capital)
            + slot_effects[EffectKind::RecruitSlots].flat.max(0.0) as usize;
        Some(Self {
            state,
            data,
            faction,
            settlement_id,
            settlement,
            faction_state,
            effects,
            slot_points,
            slots,
            ordered_this_turn: state.recruits_ordered_this_turn(settlement),
            price_factor: crate::coinage::price_factor(state, faction),
            difficulty_percent: state.difficulty_recruit_percent(data, faction),
        })
    }

    /// The context of the settlement's own controller (the player's view).
    pub fn for_controller(
        state: &'a CampaignState,
        data: &'a GameData,
        settlement_id: &'a SettlementId,
    ) -> Option<Self> {
        let controller = &state.settlements.get(settlement_id)?.controller;
        Self::new(state, data, controller, settlement_id)
    }

    /// G1 `RecruitSlots`: recruitments the settlement can queue per turn —
    /// [`BASE_RECRUIT_SLOTS`], one more in the city of the controller's
    /// capital, plus the flat `recruit_slots` of its buildings, governor and
    /// the controller's technologies.
    pub fn slots(&self) -> usize {
        self.slots
    }

    /// Slots still free this turn (B7b: only the recruits ordered this turn
    /// use one; those still training from an earlier turn do not).
    pub fn slots_free(&self) -> usize {
        self.slots.saturating_sub(self.ordered_this_turn)
    }

    /// The controller has used every slot of the turn.
    fn queue_full(&self) -> bool {
        &self.settlement.controller == self.faction && self.ordered_this_turn >= self.slots
    }

    /// The reserve of `unit_type` in the settlement.
    fn pool(&self, unit_type: &UnitTypeId) -> crate::recruit_pool::PoolView {
        self.state
            .recruit_pool_with(self.data, self.settlement_id, unit_type, self.slot_points)
    }

    /// Money cost of recruiting `unit_type` (F1 `RecruitCost`): the
    /// settlement's buildings and governor (stables: cavalry −10 %) and the
    /// faction's technologies (francs-archers: ranged −10 %), global or per
    /// unit family. Never below a quarter of the base cost.
    pub fn cost(&self, unit_type: &UnitType) -> u32 {
        let targeted = self
            .effects
            .unit_categories
            .get(unit_type.category)
            .recruit_cost;
        let flat = self.effects[EffectKind::RecruitCost].flat + targeted.flat;
        let percent = (self.effects[EffectKind::RecruitCost].percent + targeted.percent).max(-75.0);
        let base = f64::from(unit_type.cost.money);
        let cost = ((base + flat) * (1.0 + percent / 100.0))
            .round()
            .max(base / 4.0)
            .max(0.0);
        // DF1: the AI recruits cheaper at higher difficulty.
        let cost = if self.difficulty_percent == 100 {
            cost
        } else {
            (cost * f64::from(self.difficulty_percent) / 100.0).round()
        };
        // H5: prices follow the coinage.
        (cost * self.price_factor) as u32
    }

    /// SV2: full price of `unit_type` when `supply` resource units are
    /// still free ([`CampaignState::free_supply`]): [`Self::cost`] plus the
    /// import of the missing `cost.resources` at
    /// `base_price × resource_import_multiplier` (B7c rule, ADR 0053), at the
    /// faction's prices. The AI passes its own running supply to price
    /// several recruits of one turn.
    pub fn price(
        &self,
        unit_type: &UnitTypeId,
        supply: &BTreeMap<ResourceId, u32>,
    ) -> Option<RecruitPrice> {
        let unit_type = self.data.unit_types.get(unit_type)?;
        Some(self.price_of(unit_type, supply))
    }

    fn price_of(&self, unit_type: &UnitType, supply: &BTreeMap<ResourceId, u32>) -> RecruitPrice {
        let draw = crate::buildings::resource_draw(self.data, supply, &unit_type.cost.resources);
        let import_cost =
            crate::coinage::priced(self.state, self.faction, draw.import_cost).max(0) as u32;
        RecruitPrice {
            cost: self.cost(unit_type).saturating_add(import_cost),
            import_cost,
            draw,
        }
    }

    /// Availability of one unit type for `faction` in the settlement.
    pub fn option(
        &self,
        unit_type_id: &UnitTypeId,
        supply: &BTreeMap<ResourceId, u32>,
    ) -> Option<RecruitOption> {
        let unit_type = self.data.unit_types.get(unit_type_id)?;
        let price = self.price_of(unit_type, supply);
        Some(self.option_of(unit_type, &price))
    }

    fn option_of(&self, unit_type: &UnitType, price: &RecruitPrice) -> RecruitOption {
        let reason = self.blocker(unit_type, price.cost);
        RecruitOption {
            unit_type: unit_type.id.clone(),
            name: unit_type.name.display.clone(),
            cost: price.cost,
            upkeep: unit_type.upkeep,
            available: reason.is_none(),
            reason,
            resources: unit_type.cost.resources.clone(),
            import_cost: price.import_cost,
            imported: price.draw.imported.clone(),
            pool: self.pool(&unit_type.id),
        }
    }

    /// Every unit type of the game, available or not.
    pub fn options(&self, supply: &BTreeMap<ResourceId, u32>) -> Vec<RecruitOption> {
        self.data
            .unit_types
            .values()
            .map(|unit_type| self.option_of(unit_type, &self.price_of(unit_type, supply)))
            .collect()
    }

    /// SV2: `options` priced against the free resource `supply` left by the
    /// recruits already planned this turn — a second trebuchet imports the
    /// wood the first one drew. Options without resources keep their price.
    pub fn reprice(
        &self,
        options: &[RecruitOption],
        supply: &BTreeMap<ResourceId, u32>,
    ) -> Vec<RecruitOption> {
        options
            .iter()
            .map(|option| {
                let mut option = option.clone();
                if !option.resources.is_empty() {
                    if let Some(price) = self.price(&option.unit_type, supply) {
                        option.cost = price.cost;
                        option.import_cost = price.import_cost;
                        option.imported = price.draw.imported;
                    }
                }
                option
            })
            .collect()
    }

    /// Why `unit_type` cannot be recruited now, in French (`None`: it can).
    fn blocker(&self, unit_type: &UnitType, cost: u32) -> Option<String> {
        let (data, settlement) = (self.data, self.settlement);
        let province_state = self.state.provinces.get(&settlement.province)?;
        let province = data.provinces.get(&settlement.province)?;
        // LR-08: a razed place lies in ruins, the order is refused upfront.
        if crate::capture::is_ruined(self.state, self.settlement_id) {
            return Some("la colonie est en ruines".to_owned());
        }
        if &settlement.owner != self.faction || &settlement.controller != self.faction {
            return Some("la colonie doit être possédée et contrôlée".to_owned());
        }
        if settlement.siege.is_some() {
            return Some("la colonie est assiégée".to_owned());
        }
        // TW2-T3 (ADR 0103): companies are hired by an army from its region's
        // reserve (`HireMercenary`), never levied in a town.
        if unit_type.mercenary {
            return Some("compagnie de mercenaires : à engager depuis une armée".to_owned());
        }
        if self.ordered_this_turn >= self.slots {
            return Some(format!(
                "file de recrutement pleine ({} par tour)",
                self.slots
            ));
        }
        // B7c: a unit listed in some building's `enables_units` needs one of
        // those buildings (or an upgrade of it) in the settlement.
        let enablers: Vec<&data_model::Building> =
            crate::buildings::enabling_buildings(data, &unit_type.id).collect();
        if !enablers.is_empty()
            && !enablers
                .iter()
                .any(|b| data.has_building(&settlement.buildings, &b.id))
        {
            let names: Vec<&str> = enablers.iter().map(|b| b.name.display.as_str()).collect();
            return Some(format!("bâtiment requis : {}", names.join(" ou ")));
        }
        if let Some(tech) = &unit_type.required_technology {
            if !self.faction_state.technologies.contains(tech) {
                let name = data.tech_name(tech);
                return Some(format!("technologie requise : {name}"));
            }
        }
        // Lot UR1: period units (compagnies d'ordonnance from 1445, routiers
        // until the bands are hired away to Castile...).
        if let Some(from) = unit_type.available_from {
            if self.state.year < from {
                return Some(format!("disponible à partir de {from}"));
            }
        }
        if let Some(until) = unit_type.available_until {
            if self.state.year > until {
                return Some(format!("plus levée après {until}"));
            }
        }
        if !unit_type.required_faction.is_empty()
            && !unit_type.required_faction.contains(self.faction)
        {
            return Some("réservé à d'autres factions".to_owned());
        }
        if !unit_type.required_culture.is_empty()
            && !unit_type.required_culture.contains(&province.culture)
        {
            return Some("culture locale inadaptée".to_owned());
        }
        // Lot C4: the province provides the men, capped by the settlement's share.
        let class = province_state.population.get(unit_type.source_class);
        let share = crate::settlements::weight_share(data, self.settlement_id);
        if (class.count as f64 * share) < f64::from(unit_type.soldiers) * 10.0 {
            return Some("classe sociale trop peu nombreuse".to_owned());
        }
        // TW2-T2: the settlement's reserve of the unit type.
        let pool = self.pool(&unit_type.id);
        if pool.available == 0 {
            return Some(match pool.seasons_to_next {
                Some(1) => "réserve épuisée (+1 dans 1 saison)".to_owned(),
                Some(k) => format!("réserve épuisée (+1 dans {k} saisons)"),
                None => "réserve épuisée".to_owned(),
            });
        }
        if self.faction_state.treasury < i64::from(cost) {
            return Some(format!("trésor insuffisant ({cost} livres nécessaires)"));
        }
        None
    }

    /// Validates a `Recruit` order and returns what to apply: the price and
    /// the unit type. Refusals keep the order of the panel: full queue,
    /// unknown type, then the first blocker.
    fn check(&self, unit_type_id: &UnitTypeId) -> Result<RecruitPrice, OrderError> {
        if self.queue_full() {
            return Err(OrderError::RecruitQueueFull { slots: self.slots });
        }
        let supply = self.state.free_supply(self.data, self.faction);
        let unit_type = self
            .data
            .unit_types
            .get(unit_type_id)
            .ok_or_else(|| OrderError::UnknownUnitType(unit_type_id.clone()))?;
        let price = self.price_of(unit_type, &supply);
        match self.blocker(unit_type, price.cost) {
            None => Ok(price),
            Some(reason) if reason.starts_with("trésor") => Err(OrderError::InsufficientFunds {
                needed: i64::from(price.cost),
                available: self.faction_state.treasury,
            }),
            Some(reason) => Err(OrderError::RecruitUnavailable(reason)),
        }
    }
}

impl CampaignState {
    pub(super) fn order_recruit(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        settlement: &SettlementId,
        unit_type: &UnitTypeId,
    ) -> Result<(), OrderError> {
        // SV2: the resources come from the faction's producing provinces
        // (reserved while the recruit trains), the rest is imported and
        // paid (B7c rule, ADR 0053).
        let (price, turns_left) = {
            let context = RecruitContext::new(self, data, faction, settlement)
                .ok_or_else(|| OrderError::UnknownSettlement(settlement.clone()))?;
            let price = context.check(unit_type)?;
            let turns = data.unit_types[unit_type]
                .recruit_time_turns
                .unwrap_or(1)
                .max(1);
            (price, turns)
        };
        self.factions.get_mut(faction).expect("checked").treasury -= i64::from(price.cost);
        let ordered_turn = self.turn;
        // TW2-T2: the recruit leaves the settlement's reserve.
        self.draw_recruit_pool(data, settlement, unit_type);
        self.settlements
            .get_mut(settlement)
            .expect("checked")
            .recruit_queue
            .push(QueuedRecruit {
                unit_type: unit_type.clone(),
                turns_left,
                ordered_turn,
                drawn: price.draw.drawn,
            });
        Ok(())
    }

    /// Recruitment options of `settlement` for its controller (the player's view).
    pub fn recruitable(&self, data: &GameData, settlement: &SettlementId) -> Vec<RecruitOption> {
        let Some(controller) = self.settlements.get(settlement).map(|s| &s.controller) else {
            return Vec::new();
        };
        let supply = self.free_supply(data, controller);
        self.recruitable_with_supply(data, settlement, &supply)
    }

    /// [`CampaignState::recruitable`] with the controller's
    /// [`CampaignState::free_supply`] already computed (PB3f: the AI weighs
    /// every recruitment site of a realm against the same supply).
    pub fn recruitable_with_supply(
        &self,
        data: &GameData,
        settlement: &SettlementId,
        supply: &BTreeMap<ResourceId, u32>,
    ) -> Vec<RecruitOption> {
        RecruitContext::for_controller(self, data, settlement)
            .map(|context| context.options(supply))
            .unwrap_or_default()
    }

    /// Recruitment options of the city of `province` (v1 signature).
    pub fn recruitable_in_province(
        &self,
        data: &GameData,
        province: &data_model::ProvinceId,
    ) -> Vec<RecruitOption> {
        self.province_city_id(province)
            .map(|city| self.recruitable(data, city))
            .unwrap_or_default()
    }

    /// Availability of one unit type in one settlement for `faction`.
    pub fn recruit_option(
        &self,
        data: &GameData,
        faction: &FactionId,
        settlement: &SettlementId,
        unit_type_id: &UnitTypeId,
    ) -> Option<RecruitOption> {
        let context = RecruitContext::new(self, data, faction, settlement)?;
        context.option(unit_type_id, &self.free_supply(data, faction))
    }

    /// Recruitments `settlement` can queue per turn
    /// ([`RecruitContext::slots`]); 0 for an unknown settlement.
    pub fn recruit_slots(&self, data: &GameData, settlement: &SettlementId) -> usize {
        RecruitContext::for_controller(self, data, settlement).map_or(0, |c| c.slots())
    }

    /// Recruitment slots still free this turn in `settlement`.
    pub fn recruit_slots_free(&self, data: &GameData, settlement: &SettlementId) -> usize {
        RecruitContext::for_controller(self, data, settlement).map_or(0, |c| c.slots_free())
    }

    /// Recruits of `settlement`'s queue ordered during the current turn.
    pub fn recruits_ordered_this_turn(&self, settlement: &SettlementState) -> usize {
        settlement
            .recruit_queue
            .iter()
            .filter(|r| r.ordered_during(self.turn))
            .count()
    }

    /// [`RecruitContext::price`] for one-off callers.
    pub fn recruit_price(
        &self,
        data: &GameData,
        faction: &FactionId,
        settlement: &SettlementId,
        unit_type: &UnitTypeId,
        supply: &BTreeMap<ResourceId, u32>,
    ) -> Option<RecruitPrice> {
        RecruitContext::new(self, data, faction, settlement)?.price(unit_type, supply)
    }

    /// [`RecruitContext::cost`] for one-off callers (0 for an unknown
    /// settlement or faction).
    pub fn recruit_cost(
        &self,
        data: &GameData,
        faction: &FactionId,
        settlement: &SettlementId,
        unit_type: &UnitType,
    ) -> u32 {
        RecruitContext::new(self, data, faction, settlement).map_or(0, |c| c.cost(unit_type))
    }
}
