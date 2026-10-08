//! Lot TW2-T3 (ADR 0103): mercenary companies, Total War style.
//!
//! Every province belongs to a `region` (`data/provinces`). Each company of
//! `data/rules/mercenaries.json` (`bands`) is offered in some regions over a
//! period (the Great Companies after Brétigny, Genoese crossbowmen,
//! Brabançons, Scots...) from a reserve per region, capped and refilled
//! every season, like the recruitment pools of the settlements (ADR 0102).
//!
//! - An army hires a company where it stands, anywhere in the region
//!   (never on enemy lands, never shut in a besieged place): the company
//!   joins at once, at a steep price (`hire_cost_percent` of the unit's
//!   cost), within a limit per army and per faction each turn.
//! - Companies cost `upkeep_percent` of their type's upkeep: the economy
//!   bills the ordinary upkeep of every unit, this module the premium above
//!   it, after the season's income and upkeep.
//! - Unpaid companies (treasury below zero after the premium) desert, or
//!   else pillage the province they stand in (unrest, devastation).
//!
//! Units marked `mercenary` are raised this way only: the recruitment
//! blocker of the settlements refuses them (`orders`).
//!
//! Storage: [`MercenaryState::pools`] keeps only the reserves below their
//! cap (band → region → thousandths); an absent reserve is full, so old
//! saves and the 1337 start begin with full reserves.

use std::collections::BTreeMap;

use data_model::{FactionId, GameData, MercenaryBand, ProvinceId, UnitTypeId};
use serde::{Deserialize, Serialize};

use crate::events::{EventKind, GameEvent};
use crate::orders::OrderError;
use crate::recruit_pool::{PoolView, MILLI};
use crate::replenish::Territory;
use crate::state::{ArmyId, CampaignState, Unit};

/// Mercenary reserves and this turn's hires (absent from older saves).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MercenaryState {
    /// Reserves below their cap: band id → region → thousandths of a
    /// company. Absent = full.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub pools: BTreeMap<String, BTreeMap<String, u32>>,
    /// Turn the two hire counters below belong to (older turns count 0).
    #[serde(default)]
    pub hires_turn: u32,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub army_hires: BTreeMap<ArmyId, u32>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub faction_hires: BTreeMap<FactionId, u32>,
    /// Premium (upkeep above the ordinary one) each faction paid last season.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub premium_last_turn: BTreeMap<FactionId, i64>,
}

/// One company an army can hire (a line of the « Mercenaires » panel).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MercenaryOption {
    /// Band id (`data/rules/mercenaries.json`).
    pub band: String,
    /// French name of the band.
    pub band_name: String,
    pub unit_type: UnitTypeId,
    /// French name of the unit type.
    pub name: String,
    /// Hiring price, livres.
    pub cost: u32,
    /// Monthly upkeep with the premium (same measure as the recruitment
    /// panel's `upkeep`).
    pub upkeep: u32,
    /// Experience of the hired company.
    pub experience: u8,
    pub available: bool,
    /// French reason when `available` is `false`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// The region's reserve of this band.
    pub pool: PoolView,
}

/// What an army may hire where it stands.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MercenaryMarket {
    /// Region the army stands in (`None` off the map).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// French name of the region.
    pub region_name: String,
    /// Companies the army may still hire this turn (its own limit and its
    /// faction's).
    pub hires_left: u32,
    /// French reason why the army hires nothing here (enemy lands, besieged,
    /// limit reached...).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked: Option<String>,
    /// Companies of the region in this period.
    pub options: Vec<MercenaryOption>,
}

impl CampaignState {
    /// Region (`data/provinces` `region`) `army_id` stands in.
    pub fn army_region(&self, data: &GameData, army_id: &ArmyId) -> Option<String> {
        let army = self.armies.get(army_id)?;
        let province = self.army_province(data, army)?;
        data.provinces.get(&province).map(|p| p.region.clone())
    }

    /// The reserve of `band` in `region`.
    pub fn mercenary_pool(&self, band: &MercenaryBand, region: &str) -> PoolView {
        let full = band.cap * MILLI;
        let milli = self
            .mercenaries
            .pools
            .get(&band.id)
            .and_then(|regions| regions.get(region).copied())
            .map_or(full, |m| m.min(full));
        let rate_milli = band.refill_milli;
        let seasons_to_next = if milli >= full || rate_milli == 0 {
            None
        } else {
            let next = (milli / MILLI + 1) * MILLI;
            Some((next - milli).div_ceil(rate_milli))
        };
        PoolView {
            available: milli / MILLI,
            cap: band.cap,
            milli,
            rate_milli,
            seasons_to_next,
        }
    }

    /// Hiring price of `unit_type` for `faction`: `hire_cost_percent` of its
    /// cost, at the faction's prices (coinage) and, for the AI, its
    /// difficulty's recruitment percentage.
    pub fn mercenary_hire_cost(
        &self,
        data: &GameData,
        faction: &FactionId,
        unit_type: &UnitTypeId,
    ) -> u32 {
        let Some(unit) = data.unit_types.get(unit_type) else {
            return 0;
        };
        let rules = &data.mercenary_rules;
        let base = f64::from(unit.cost.money) * f64::from(rules.hire_cost_percent) / 100.0;
        let difficulty = f64::from(self.difficulty_recruit_percent(data, faction)) / 100.0;
        let prices = crate::coinage::price_factor(self, faction);
        (base * difficulty * prices).round().max(0.0) as u32
    }

    /// Hires already made this turn by `army_id` and by `faction`.
    fn hires_this_turn(&self, army_id: &ArmyId, faction: &FactionId) -> (u32, u32) {
        let hires = &self.mercenaries;
        if hires.hires_turn != self.turn {
            return (0, 0);
        }
        (
            hires.army_hires.get(army_id).copied().unwrap_or(0),
            hires.faction_hires.get(faction).copied().unwrap_or(0),
        )
    }

    /// Companies `army_id` may hire where it stands (« Mercenaires » panel).
    /// `None` for an unknown army.
    pub fn mercenary_market(&self, data: &GameData, army_id: &ArmyId) -> Option<MercenaryMarket> {
        let army = self.armies.get(army_id)?;
        let rules = &data.mercenary_rules;
        let faction = &army.faction;
        let region = self.army_region(data, army_id);
        let mut market = MercenaryMarket {
            region_name: region
                .as_deref()
                .map_or_else(String::new, |r| rules.region_name(r).to_owned()),
            region: region.clone(),
            ..Default::default()
        };
        let Some(region) = region else {
            market.blocked = Some("l'armée n'est dans aucune région".to_owned());
            return Some(market);
        };
        let (army_hires, faction_hires) = self.hires_this_turn(army_id, faction);
        market.hires_left = rules
            .hires_per_army_per_turn
            .saturating_sub(army_hires)
            .min(
                rules
                    .hires_per_faction_per_turn
                    .saturating_sub(faction_hires),
            );
        let shut_in = army.settlement().is_some_and(|s| {
            self.settlements
                .get(s)
                .is_some_and(|s| s.siege.is_some() && self.is_allied(faction, &s.controller))
        });
        market.blocked = if !rules.hostile_territory_allowed
            && self.army_territory(data, army_id) == Territory::Hostile
        {
            Some("aucune compagnie ne traite en terre ennemie".to_owned())
        } else if army.units.len() >= data.army_rules.cap() {
            // NT5 (N6): no company joins a full army.
            Some(format!(
                "armée complète ({} unités au plus)",
                data.army_rules.max_units
            ))
        } else if shut_in {
            Some("l'armée est enfermée dans une place assiégée".to_owned())
        } else if army_hires >= rules.hires_per_army_per_turn {
            Some(format!(
                "cette armée a déjà engagé {army_hires} compagnie(s) ce tour"
            ))
        } else if faction_hires >= rules.hires_per_faction_per_turn {
            Some(format!(
                "le royaume a déjà engagé {faction_hires} compagnie(s) ce tour"
            ))
        } else {
            None
        };
        let treasury = self.factions.get(faction).map_or(0, |f| f.treasury);
        for band in &rules.bands {
            if !band.active(self.year) || !band.regions.iter().any(|r| r == &region) {
                continue;
            }
            let Some(unit) = data.unit_types.get(&band.unit) else {
                continue;
            };
            let cost = self.mercenary_hire_cost(data, faction, &band.unit);
            let pool = self.mercenary_pool(band, &region);
            let reason = if let Some(blocked) = &market.blocked {
                Some(blocked.clone())
            } else if pool.available == 0 {
                Some(match pool.seasons_to_next {
                    Some(1) => "réserve épuisée (+1 dans 1 saison)".to_owned(),
                    Some(k) => format!("réserve épuisée (+1 dans {k} saisons)"),
                    None => "réserve épuisée".to_owned(),
                })
            } else if treasury < i64::from(cost) {
                Some(format!("trésor insuffisant ({cost} livres nécessaires)"))
            } else {
                None
            };
            market.options.push(MercenaryOption {
                band: band.id.clone(),
                band_name: band.name.clone(),
                unit_type: band.unit.clone(),
                name: unit.name.display.clone(),
                cost,
                upkeep: unit.upkeep * rules.upkeep_percent / 100,
                experience: band.experience,
                available: reason.is_none(),
                reason,
                pool,
            });
        }
        Some(market)
    }

    /// `HireMercenary`: `army_id` of `faction` hires a company of
    /// `unit_type` from its region's reserve; it joins at once.
    pub(crate) fn order_hire_mercenary(
        &mut self,
        data: &GameData,
        faction: &FactionId,
        army_id: &ArmyId,
        unit_type: &UnitTypeId,
    ) -> Result<(), OrderError> {
        let army = self
            .armies
            .get(army_id)
            .ok_or_else(|| OrderError::UnknownArmy(army_id.clone()))?;
        if &army.faction != faction {
            return Err(OrderError::NotYourArmy(faction.clone()));
        }
        let unit = data
            .unit_types
            .get(unit_type)
            .ok_or_else(|| OrderError::UnknownUnitType(unit_type.clone()))?;
        let market = self
            .mercenary_market(data, army_id)
            .ok_or_else(|| OrderError::UnknownArmy(army_id.clone()))?;
        let Some(option) = market.options.iter().find(|o| &o.unit_type == unit_type) else {
            return Err(OrderError::MercenaryUnavailable(
                market.blocked.unwrap_or_else(|| {
                    format!("aucune compagnie de ce type en {}", market.region_name)
                }),
            ));
        };
        if !option.available {
            let reason = option.reason.clone().unwrap_or_default();
            if reason.starts_with("trésor") {
                return Err(OrderError::InsufficientFunds {
                    needed: i64::from(option.cost),
                    available: self.factions.get(faction).map_or(0, |f| f.treasury),
                });
            }
            return Err(OrderError::MercenaryUnavailable(reason));
        }
        let region = market.region.clone().expect("options need a region");
        let band = option.band.clone();
        let cost = option.cost;
        let experience = option.experience;
        let pool = option.pool;
        // Pay, draw from the reserve, count the hire, join the army.
        self.factions
            .get_mut(faction)
            .expect("army's faction")
            .treasury -= i64::from(cost);
        self.mercenaries
            .pools
            .entry(band)
            .or_default()
            .insert(region, pool.milli.saturating_sub(MILLI));
        if self.mercenaries.hires_turn != self.turn {
            self.mercenaries.hires_turn = self.turn;
            self.mercenaries.army_hires.clear();
            self.mercenaries.faction_hires.clear();
        }
        *self
            .mercenaries
            .army_hires
            .entry(army_id.clone())
            .or_default() += 1;
        *self
            .mercenaries
            .faction_hires
            .entry(faction.clone())
            .or_default() += 1;
        let mut company = Unit::fresh(unit);
        company.experience = experience.min(10);
        self.armies
            .get_mut(army_id)
            .expect("checked")
            .units
            .push(company);
        if faction == &self.player_faction {
            self.pending_events.push(
                GameEvent::new(
                    EventKind::Recruited,
                    format!(
                        "{} rejoignent l'ost pour {cost} livres ({}).",
                        unit.name.display, option.band_name
                    ),
                )
                .army(army_id)
                .faction(faction),
            );
        }
        Ok(())
    }

    /// Seasonal premium `faction` pays for its companies: the upkeep above
    /// the ordinary one of every mercenary unit (field armies in full,
    /// garrisons at the garrison share), at its prices and difficulty.
    pub fn mercenary_premium(&self, data: &GameData, faction: &FactionId) -> i64 {
        let extra = i64::from(data.mercenary_rules.upkeep_percent.saturating_sub(100));
        let premium = |units: &[Unit]| -> i64 {
            units
                .iter()
                .filter(|u| is_mercenary(data, &u.unit_type))
                .map(|u| crate::economy::unit_upkeep(data, u) * extra / 100)
                .sum()
        };
        let field: i64 = self
            .armies
            .values()
            .filter(|a| &a.faction == faction)
            .map(|a| premium(&a.units))
            .sum();
        let garrisons: i64 = self
            .settlements
            .values()
            .filter(|s| &s.controller == faction)
            .map(|s| {
                premium(&s.garrison) * crate::economy::garrison_upkeep_percent(data, s.kind) / 100
            })
            .sum();
        let difficulty = i64::from(self.difficulty_upkeep_percent(data, faction));
        let total = (field + garrisons) * difficulty / 100;
        crate::coinage::priced(self, faction, total)
    }
}

/// `true` when `unit_type` is marked `mercenary`.
pub fn is_mercenary(data: &GameData, unit_type: &UnitTypeId) -> bool {
    data.unit_types.get(unit_type).is_some_and(|t| t.mercenary)
}

/// End of the season, after the economy: every faction pays its companies'
/// premium; unpaid companies (treasury below zero) desert or pillage; the
/// reserves refill.
pub fn resolve_mercenaries(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let factions: Vec<FactionId> = state
        .factions
        .iter()
        .filter(|(_, f)| f.alive)
        .map(|(id, _)| id.clone())
        .collect();
    state.mercenaries.premium_last_turn.clear();
    for faction in factions {
        let premium = state.mercenary_premium(data, &faction);
        if premium == 0 {
            continue;
        }
        let treasury = {
            let f = state.factions.get_mut(&faction).expect("listed");
            f.treasury -= premium;
            f.treasury
        };
        state
            .mercenaries
            .premium_last_turn
            .insert(faction.clone(), premium);
        if treasury < 0 {
            resolve_arrears(state, data, &faction, events);
        }
    }
    refill_pools(state, data);
}

/// Unpaid companies of `faction`: each deserts (`desert_percent`) or else
/// pillages the province it stands in (once per province and season).
fn resolve_arrears(
    state: &mut CampaignState,
    data: &GameData,
    faction: &FactionId,
    events: &mut Vec<GameEvent>,
) {
    let arrears = &data.mercenary_rules.arrears;
    let mut deserted = 0u32;
    let mut pillaged: BTreeMap<ProvinceId, u32> = BTreeMap::new();
    // Field armies.
    let army_ids: Vec<ArmyId> = state
        .armies
        .iter()
        .filter(|(_, a)| &a.faction == faction)
        .map(|(id, _)| id.clone())
        .collect();
    for army_id in army_ids {
        let Some(army) = state.armies.get(&army_id) else {
            continue;
        };
        let province = state.army_province(data, army);
        let mercenaries: Vec<usize> = army
            .units
            .iter()
            .enumerate()
            .filter(|(_, u)| is_mercenary(data, &u.unit_type))
            .map(|(i, _)| i)
            .collect();
        let mut leaving = Vec::new();
        for index in mercenaries {
            if state.rng.chance_permille(arrears.desert_percent * 10) {
                leaving.push(index);
            } else if let Some(province) = &province {
                *pillaged.entry(province.clone()).or_default() += 1;
            }
        }
        if leaving.is_empty() {
            continue;
        }
        deserted += leaving.len() as u32;
        let army = state.armies.get_mut(&army_id).expect("listed");
        for index in leaving.into_iter().rev() {
            army.units.remove(index);
        }
        if army.units.is_empty() {
            let general = army.general.clone();
            if let Some(general) = general {
                state.detach_general(&general);
            }
            state.armies.remove(&army_id);
        }
    }
    // Garrisons.
    let settlement_ids: Vec<data_model::SettlementId> = state
        .settlements
        .iter()
        .filter(|(_, s)| &s.controller == faction)
        .filter(|(_, s)| s.garrison.iter().any(|u| is_mercenary(data, &u.unit_type)))
        .map(|(id, _)| id.clone())
        .collect();
    for id in settlement_ids {
        let province = state.settlements[&id].province.clone();
        let garrison = state.settlements[&id].garrison.clone();
        let mut kept = Vec::with_capacity(garrison.len());
        for unit in garrison {
            if !is_mercenary(data, &unit.unit_type) {
                kept.push(unit);
            } else if state.rng.chance_permille(arrears.desert_percent * 10) {
                deserted += 1;
            } else {
                *pillaged.entry(province.clone()).or_default() += 1;
                kept.push(unit);
            }
        }
        state.settlements.get_mut(&id).expect("listed").garrison = kept;
    }
    let is_player = faction == &state.player_faction;
    if deserted > 0 {
        events.push(
            GameEvent::new(
                EventKind::Attrition,
                format!(
                    "Solde impayée : {deserted} compagnie(s) de mercenaires désertent faute d'argent."
                ),
            )
            .faction(faction),
        );
    }
    for (province, companies) in pillaged {
        if let Some(p) = state.provinces.get_mut(&province) {
            p.unrest = p.unrest.saturating_add(arrears.pillage_unrest).min(100);
            p.devastation = p
                .devastation
                .saturating_add(arrears.pillage_devastation)
                .min(100);
        }
        let name = data.province_name(&province);
        if is_player || state.provinces.contains_key(&province) {
            events.push(
                GameEvent::new(
                    EventKind::Raid,
                    format!(
                        "Solde impayée : {companies} compagnie(s) de mercenaires se paient sur le pays en {name} (troubles, dévastation)."
                    ),
                )
                .province(&province)
                .faction(faction),
            );
        }
    }
}

/// Every reserve below its cap regains its band's `refill_milli`; a full
/// reserve leaves the map, as does one of a band out of its period.
fn refill_pools(state: &mut CampaignState, data: &GameData) {
    let rules = &data.mercenary_rules;
    let year = state.year;
    let pools = std::mem::take(&mut state.mercenaries.pools);
    for (band_id, regions) in pools {
        let Some(band) = rules.band(&band_id) else {
            continue;
        };
        if !band.active(year) {
            continue;
        }
        let full = band.cap * MILLI;
        let refilled: BTreeMap<String, u32> = regions
            .into_iter()
            .map(|(region, milli)| (region, milli + band.refill_milli))
            .filter(|(_, milli)| *milli < full)
            .collect();
        if !refilled.is_empty() {
            state.mercenaries.pools.insert(band_id, refilled);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_reserve_is_full() {
        let state = CampaignState::new(1);
        let band = MercenaryBand {
            id: "b".to_owned(),
            name: "B".to_owned(),
            unit: UnitTypeId::new("unit_routiers").unwrap(),
            regions: vec!["r".to_owned()],
            from: 1337,
            until: 1400,
            cap: 3,
            refill_milli: 400,
            experience: 0,
            note: None,
        };
        let full = state.mercenary_pool(&band, "r");
        assert_eq!(
            (full.available, full.cap, full.seasons_to_next),
            (3, 3, None)
        );
        let mut state = state;
        state
            .mercenaries
            .pools
            .entry("b".to_owned())
            .or_default()
            .insert("r".to_owned(), 1_200);
        let view = state.mercenary_pool(&band, "r");
        assert_eq!((view.available, view.seasons_to_next), (1, Some(2)));
    }
}
