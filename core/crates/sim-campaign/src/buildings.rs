//! Buildings: construction, effects and the province "city" view (spec § 1.2).
//!
//! Lot C4: buildings stand in settlements; each settlement holds at most one
//! [`Construction`] at a time and only accepts the buildings whose
//! `settlement_kinds` include its kind. Effects are summed into an
//! [`EffectTotals`] per province (every settlement's buildings,
//! [`CampaignState::province_effects`]: taxes, population, religion) or per
//! settlement ([`CampaignState::settlement_effects`]: garrison, walls,
//! recruitment), which the rest of the crate reads instead of touching
//! `Building::effects` directly.

use std::collections::BTreeMap;

use data_model::{
    BuildingId, Effect, EffectKind, EffectMode, GameData, ProvinceId, ResourceId, SettlementId,
    SocialClass, UnitCategory,
};
use serde::{Deserialize, Serialize};

use crate::events::{EventKind, GameEvent};
use crate::state::{CampaignState, Construction, SettlementState};

/// Population count a province can sustain before health suffers (spec § 1.1).
pub const BASE_CAPACITY: u64 = 40_000;
/// Share of a construction's money cost refunded by `cancel_build`.
pub const CANCEL_REFUND_PERCENT: u32 = 50;
/// B7b: most a settlement's `ConstructionSpeed` can shorten a build (+100 %:
/// half the time).
pub const MAX_CONSTRUCTION_SPEED_PERCENT: f64 = 100.0;

/// One `flat` + `percent` pair of a summed [`EffectKind`].
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct EffectValue {
    pub flat: f64,
    pub percent: f64,
}

impl EffectValue {
    fn add(&mut self, effect_mode: EffectMode, value: f64) {
        match effect_mode {
            EffectMode::Add => self.flat += value,
            EffectMode::Percent => self.percent += value,
        }
    }

    /// Applies `flat` then `percent` on top of `base`.
    pub fn apply(&self, base: f64) -> f64 {
        (base + self.flat) * (1.0 + self.percent / 100.0)
    }

    fn merge(&mut self, other: EffectValue) {
        self.flat += other.flat;
        self.percent += other.percent;
    }
}

/// Sum of every building effect of a province, one entry per [`EffectKind`]
/// the simulation reads (spec § 1.2): `TaxIncome`, `TradeIncome`, `Health`,
/// `Unrest`, `Wealth`, `GoodsSatisfaction`, `Growth`, `Garrison`,
/// `FortificationLevel`, `RecruitCost`, `Supply`. Other kinds are ignored.
///
/// M4 (spec § 2) extends this with the kinds carried by character traits and
/// skills (general in battle, governor in a province): `ArmyMorale`,
/// `ArmyExperience`, `Piety`, `Prestige`, `Loyalty`, `Movement`, plus the
/// battle/siege/construction/court kinds `data_model::EffectKind` adds for M4
/// (`BattleCharge`, `BattleRanged`, `BattleDefense`, `SiegeSpeed`,
/// `ConstructionSpeed`, `Diplomacy`, `Intrigue`, `Fertility`); see
/// `skills::character_effects`, which reuses [`EffectTotals::add_effect`] on
/// `Trait`/`Skill` effects the same way this module uses it on buildings.
///
/// F1: an effect with a `class` only reaches that social class
/// ([`EffectTotals::classes`], read by the population model) and an effect
/// with a `unit_category` only that unit family
/// ([`EffectTotals::unit_categories`]: recruitment cost, movement, recruits'
/// experience, upkeep). The top-level fields only hold untargeted effects.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct EffectTotals {
    pub tax_income: EffectValue,
    pub trade_income: EffectValue,
    pub health: EffectValue,
    pub unrest: EffectValue,
    pub wealth: EffectValue,
    pub goods_satisfaction: EffectValue,
    pub growth: EffectValue,
    pub garrison: EffectValue,
    pub fortification_level: EffectValue,
    pub recruit_cost: EffectValue,
    pub supply: EffectValue,
    pub army_morale: EffectValue,
    pub army_experience: EffectValue,
    pub piety: EffectValue,
    pub prestige: EffectValue,
    pub loyalty: EffectValue,
    pub movement: EffectValue,
    pub battle_charge: EffectValue,
    pub battle_ranged: EffectValue,
    pub battle_defense: EffectValue,
    pub siege_speed: EffectValue,
    pub construction_speed: EffectValue,
    pub diplomacy: EffectValue,
    pub intrigue: EffectValue,
    pub fertility: EffectValue,
    // ----- F1: kinds that used to be displayed without effect -------------
    #[serde(default)]
    pub production: EffectValue,
    #[serde(default)]
    pub army_upkeep: EffectValue,
    #[serde(default)]
    pub siege_resistance: EffectValue,
    #[serde(default)]
    pub attrition_resistance: EffectValue,
    #[serde(default)]
    pub research_civil: EffectValue,
    #[serde(default)]
    pub research_military: EffectValue,
    /// H4: `PlagueResistance`, `WoundRecovery`, `DietHealth`.
    #[serde(default)]
    pub plague_resistance: EffectValue,
    #[serde(default)]
    pub wound_recovery: EffectValue,
    #[serde(default)]
    pub diet_health: EffectValue,
    /// G1: extra simultaneous recruitments per province (`RecruitSlots`).
    #[serde(default)]
    pub recruit_slots: EffectValue,
    /// Effects restricted to one social class (`Effect::class`).
    #[serde(default)]
    pub classes: ClassEffectTotals,
    /// Effects restricted to one unit family (`Effect::unit_category`).
    #[serde(default)]
    pub unit_categories: CategoryEffectTotals,
}

/// Population effects aimed at one social class (F1).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ClassEffects {
    pub wealth: EffectValue,
    pub health: EffectValue,
    pub unrest: EffectValue,
    pub goods_satisfaction: EffectValue,
    pub growth: EffectValue,
}

impl ClassEffects {
    fn merge(&mut self, other: &ClassEffects) {
        for (mine, theirs) in [
            (&mut self.wealth, other.wealth),
            (&mut self.health, other.health),
            (&mut self.unrest, other.unrest),
            (&mut self.goods_satisfaction, other.goods_satisfaction),
            (&mut self.growth, other.growth),
        ] {
            mine.merge(theirs);
        }
    }
}

/// [`ClassEffects`] of the four social classes.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct ClassEffectTotals {
    pub peasants: ClassEffects,
    pub burghers: ClassEffects,
    pub clergy: ClassEffects,
    pub nobility: ClassEffects,
}

impl ClassEffectTotals {
    pub fn get(&self, class: SocialClass) -> &ClassEffects {
        match class {
            SocialClass::Peasants => &self.peasants,
            SocialClass::Burghers => &self.burghers,
            SocialClass::Clergy => &self.clergy,
            SocialClass::Nobility => &self.nobility,
        }
    }

    fn get_mut(&mut self, class: SocialClass) -> &mut ClassEffects {
        match class {
            SocialClass::Peasants => &mut self.peasants,
            SocialClass::Burghers => &mut self.burghers,
            SocialClass::Clergy => &mut self.clergy,
            SocialClass::Nobility => &mut self.nobility,
        }
    }
}

/// Effects aimed at one unit family (F1).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct UnitCategoryEffects {
    pub recruit_cost: EffectValue,
    pub movement: EffectValue,
    pub army_experience: EffectValue,
    pub army_upkeep: EffectValue,
}

impl UnitCategoryEffects {
    fn merge(&mut self, other: &UnitCategoryEffects) {
        for (mine, theirs) in [
            (&mut self.recruit_cost, other.recruit_cost),
            (&mut self.movement, other.movement),
            (&mut self.army_experience, other.army_experience),
            (&mut self.army_upkeep, other.army_upkeep),
        ] {
            mine.merge(theirs);
        }
    }
}

/// [`UnitCategoryEffects`] of the four unit families.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct CategoryEffectTotals {
    pub infantry: UnitCategoryEffects,
    pub ranged: UnitCategoryEffects,
    pub cavalry: UnitCategoryEffects,
    pub siege: UnitCategoryEffects,
}

impl CategoryEffectTotals {
    pub fn get(&self, category: UnitCategory) -> &UnitCategoryEffects {
        match category {
            UnitCategory::Infantry => &self.infantry,
            UnitCategory::Ranged => &self.ranged,
            UnitCategory::Cavalry => &self.cavalry,
            UnitCategory::Siege => &self.siege,
        }
    }

    fn get_mut(&mut self, category: UnitCategory) -> &mut UnitCategoryEffects {
        match category {
            UnitCategory::Infantry => &mut self.infantry,
            UnitCategory::Ranged => &mut self.ranged,
            UnitCategory::Cavalry => &mut self.cavalry,
            UnitCategory::Siege => &mut self.siege,
        }
    }
}

impl EffectTotals {
    /// Merges `other`'s flat/percent totals into `self` (used to combine
    /// building effects with a governor's `character_effects`).
    pub fn merge(&mut self, other: &EffectTotals) {
        macro_rules! merge_field {
            ($($field:ident),* $(,)?) => {
                $(self.$field.merge(other.$field);)*
            };
        }
        merge_field!(
            tax_income,
            trade_income,
            health,
            unrest,
            wealth,
            goods_satisfaction,
            growth,
            garrison,
            fortification_level,
            recruit_cost,
            supply,
            army_morale,
            army_experience,
            piety,
            prestige,
            loyalty,
            movement,
            battle_charge,
            battle_ranged,
            battle_defense,
            siege_speed,
            construction_speed,
            diplomacy,
            intrigue,
            fertility,
            production,
            army_upkeep,
            siege_resistance,
            attrition_resistance,
            research_civil,
            research_military,
            plague_resistance,
            wound_recovery,
            diet_health,
            recruit_slots,
        );
        for class in SocialClass::ALL {
            self.classes.get_mut(class).merge(other.classes.get(class));
        }
        for category in UNIT_CATEGORIES {
            self.unit_categories
                .get_mut(category)
                .merge(other.unit_categories.get(category));
        }
    }

    /// Adds one [`Effect`]'s worth of value, honouring its `class` and
    /// `unit_category` targets (F1). Shared by building effects
    /// ([`effects_of`]), character trait/skill effects
    /// (`skills::character_effects`) and technologies
    /// (`research::faction_tech_effects`).
    pub(crate) fn add_effect(&mut self, effect: &Effect) {
        if let Some(class) = effect.class {
            let slot = self.classes.get_mut(class);
            let target = match effect.effect {
                EffectKind::Wealth => Some(&mut slot.wealth),
                EffectKind::Health => Some(&mut slot.health),
                EffectKind::Unrest => Some(&mut slot.unrest),
                EffectKind::GoodsSatisfaction => Some(&mut slot.goods_satisfaction),
                EffectKind::Growth => Some(&mut slot.growth),
                _ => None,
            };
            if let Some(target) = target {
                target.add(effect.mode, effect.value);
                return;
            }
        }
        if let Some(category) = effect.unit_category {
            let slot = self.unit_categories.get_mut(category);
            let target = match effect.effect {
                EffectKind::RecruitCost => Some(&mut slot.recruit_cost),
                EffectKind::Movement => Some(&mut slot.movement),
                EffectKind::ArmyExperience => Some(&mut slot.army_experience),
                EffectKind::ArmyUpkeep => Some(&mut slot.army_upkeep),
                // Per-category battle bonuses are read from the raw effects
                // by `research::tech_unit_bonus`.
                _ => return,
            };
            if let Some(target) = target {
                target.add(effect.mode, effect.value);
            }
            return;
        }
        self.add(effect.effect, effect.mode, effect.value);
    }

    /// Adds an untargeted value to the matching slot; unmapped kinds are
    /// ignored.
    pub(crate) fn add(&mut self, kind: EffectKind, mode: EffectMode, value: f64) {
        let slot = match kind {
            EffectKind::TaxIncome => &mut self.tax_income,
            EffectKind::TradeIncome => &mut self.trade_income,
            EffectKind::Health => &mut self.health,
            EffectKind::Unrest => &mut self.unrest,
            EffectKind::Wealth => &mut self.wealth,
            EffectKind::GoodsSatisfaction => &mut self.goods_satisfaction,
            EffectKind::Growth => &mut self.growth,
            EffectKind::Garrison => &mut self.garrison,
            EffectKind::FortificationLevel => &mut self.fortification_level,
            EffectKind::RecruitCost => &mut self.recruit_cost,
            EffectKind::Supply => &mut self.supply,
            EffectKind::ArmyMorale => &mut self.army_morale,
            EffectKind::ArmyExperience => &mut self.army_experience,
            EffectKind::Piety => &mut self.piety,
            EffectKind::Prestige => &mut self.prestige,
            EffectKind::Loyalty => &mut self.loyalty,
            EffectKind::Movement => &mut self.movement,
            EffectKind::BattleCharge => &mut self.battle_charge,
            EffectKind::BattleRanged => &mut self.battle_ranged,
            EffectKind::BattleDefense => &mut self.battle_defense,
            EffectKind::SiegeSpeed => &mut self.siege_speed,
            EffectKind::ConstructionSpeed => &mut self.construction_speed,
            EffectKind::Diplomacy => &mut self.diplomacy,
            EffectKind::Intrigue => &mut self.intrigue,
            EffectKind::Fertility => &mut self.fertility,
            EffectKind::Production => &mut self.production,
            EffectKind::ArmyUpkeep => &mut self.army_upkeep,
            EffectKind::SiegeResistance => &mut self.siege_resistance,
            EffectKind::AttritionResistance => &mut self.attrition_resistance,
            EffectKind::ResearchCivil => &mut self.research_civil,
            EffectKind::ResearchMilitary => &mut self.research_military,
            EffectKind::PlagueResistance => &mut self.plague_resistance,
            EffectKind::WoundRecovery => &mut self.wound_recovery,
            EffectKind::DietHealth => &mut self.diet_health,
            EffectKind::RecruitSlots => &mut self.recruit_slots,
            _ => return,
        };
        slot.add(mode, value);
    }
}

/// The four unit families, in schema order.
pub const UNIT_CATEGORIES: [UnitCategory; 4] = [
    UnitCategory::Infantry,
    UnitCategory::Ranged,
    UnitCategory::Cavalry,
    UnitCategory::Siege,
];

/// Sums the effects of every completed building of `buildings` (data-only:
/// no [`CampaignState`] needed, so `orders.rs` and the population/economy
/// modules can call it on any building list).
pub fn effects_of(data: &GameData, buildings: &[BuildingId]) -> EffectTotals {
    let mut totals = EffectTotals::default();
    for id in buildings {
        let Some(building) = data.buildings.get(id) else {
            continue;
        };
        for effect in &building.effects {
            totals.add_effect(effect);
        }
    }
    totals
}

/// G1: flat `(armor, ranged)` bonus the buildings of a province grant the
/// units of `category` levied there (armoury +3 armour, archery butts +3
/// ranged for archers); an effect without `unit_category` applies to every
/// family. Stored on the unit at recruitment (`Unit::levy_armor`), so it
/// follows the regiment wherever it fights. Capped at 10 each.
pub fn levy_bonus(data: &GameData, buildings: &[BuildingId], category: UnitCategory) -> (u8, u8) {
    let (mut armor, mut ranged) = (0.0, 0.0);
    for building in buildings.iter().filter_map(|id| data.buildings.get(id)) {
        for effect in &building.effects {
            if effect.mode != EffectMode::Add || effect.unit_category.is_some_and(|c| c != category)
            {
                continue;
            }
            match effect.effect {
                EffectKind::ArmyArmor => armor += effect.value,
                EffectKind::ArmyRanged => ranged += effect.value,
                _ => {}
            }
        }
    }
    let cap = |v: f64| v.round().clamp(0.0, 10.0) as u8;
    (cap(armor), cap(ranged))
}

/// B7c: buildings whose `enables_units` list `unit_type` — the only source of
/// the "building required to recruit" rule; empty when no building gates it.
pub fn enabling_buildings<'a>(
    data: &'a GameData,
    unit_type: &'a data_model::UnitTypeId,
) -> impl Iterator<Item = &'a data_model::Building> + 'a {
    data.buildings
        .values()
        .filter(move |b| b.enables_units.contains(unit_type))
}

/// Population capacity of a province: [`BASE_CAPACITY`] times one plus the
/// sum of the tiers of its `production`-category buildings (spec § 1.1).
pub fn capacity(data: &GameData, province: &ProvinceId, buildings: &[BuildingId]) -> u64 {
    let bonus_tiers: u64 = buildings
        .iter()
        .filter_map(|id| data.buildings.get(id))
        .filter(|b| b.category == data_model::BuildingCategory::Production)
        .map(|b| u64::from(b.tier))
        .sum();
    capacity_with_tiers_percent(data, province, bonus_tiers * 100)
}

/// [`capacity`] for production tiers counted in hundredths (lot DC3: weighed tiers).
fn capacity_with_tiers_percent(data: &GameData, province: &ProvinceId, tiers_percent: u64) -> u64 {
    // The 1337 population is the reference: a province can grow ~25 % above
    // it before crowding hurts health, plus 10 % per production tier.
    let base = data
        .provinces
        .get(province)
        .map_or(BASE_CAPACITY, |p| p.population.classes.total())
        .max(BASE_CAPACITY);
    base * (12_500 + 10 * tiers_percent) / 10_000
}

/// Weight, in per cent, of the buildings of a settlement of `kind` in the effects on
/// its whole province (lot DC3, `rules.json` `province_effect_percent`; 100 when absent).
pub fn province_effect_percent(data: &GameData, kind: data_model::SettlementKind) -> u32 {
    data.settlement_rules
        .as_ref()
        .and_then(|rules| rules.province_effect_percent.get(&kind).copied())
        .unwrap_or(100)
}

/// Weight, in per cent, of the buildings of a settlement of `kind` in its controller's
/// research points (lot DC6b, `rules.json` `research_percent`; 100 when absent).
pub fn research_percent(data: &GameData, kind: data_model::SettlementKind) -> u32 {
    data.settlement_rules
        .as_ref()
        .and_then(|rules| rules.research_percent.get(&kind).copied())
        .unwrap_or(100)
}

/// Total upkeep of the completed buildings of a province (spec § 1.2).
pub fn province_building_upkeep(data: &GameData, buildings: &[BuildingId]) -> i64 {
    buildings
        .iter()
        .filter_map(|id| data.buildings.get(id))
        .map(|b| i64::from(b.upkeep.unwrap_or(0)))
        .sum()
}

/// One line of the "what can I build here" panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildOption {
    pub building: BuildingId,
    pub name: String,
    pub cost: u32,
    pub turns: u32,
    pub available: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// B7c: part of `cost` paid to import the missing resource units.
    #[serde(default)]
    pub import_cost: u32,
    /// B7c: resource units the faction lacks and must import.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub imported: BTreeMap<ResourceId, u32>,
}

/// B7c: how a construction's `cost.resources` are met — units drawn from the
/// faction's producing provinces and units imported, with their price.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResourceDraw {
    pub drawn: BTreeMap<ResourceId, u32>,
    pub imported: BTreeMap<ResourceId, u32>,
    /// Import price in livres (before coinage).
    pub import_cost: i64,
}

/// B7c: splits `needed` between the free `supply` and imports priced at
/// `base_price × resource_import_multiplier` a unit.
pub fn resource_draw(
    data: &GameData,
    supply: &BTreeMap<ResourceId, u32>,
    needed: &BTreeMap<ResourceId, u32>,
) -> ResourceDraw {
    let mut draw = ResourceDraw::default();
    let multiplier = i64::from(data.economy_rules.resource_import_multiplier);
    for (resource, &amount) in needed {
        let own = supply.get(resource).copied().unwrap_or(0).min(amount);
        if own > 0 {
            draw.drawn.insert(resource.clone(), own);
        }
        let missing = amount - own;
        if missing > 0 {
            draw.imported.insert(resource.clone(), missing);
            let price = data
                .resources
                .get(resource)
                .map_or(0, |r| i64::from(r.base_price));
            draw.import_cost += i64::from(missing) * price * multiplier;
        }
    }
    draw
}

/// Snapshot of a province's city panel (bridge input, spec § 2): the
/// province's population, resources and effects, the buildings,
/// construction and build options of its city (lot C4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProvinceCity {
    pub classes: data_model::PopulationClasses,
    pub buildings: Vec<BuildingId>,
    pub construction: Option<Construction>,
    pub fortification_level: u32,
    pub capacity: u64,
    pub buildable: Vec<BuildOption>,
    pub resources: Vec<ResourceId>,
    pub effects: EffectTotals,
}

impl CampaignState {
    /// Sum of the building effects of every settlement of `province`, plus
    /// its governor's trait/skill effects and its active regional edict
    /// (spec § 2, lot C4).
    pub fn province_effects(&self, data: &GameData, province: &ProvinceId) -> EffectTotals {
        let mut totals = self.province_building_effects(data, province);
        totals.merge(&self.governor_effects(data, province));
        totals.merge(&crate::edicts::edict_effects(self, data, province));
        totals
    }

    /// Effects of the buildings of every settlement of `province` on the whole province,
    /// each settlement's weighing its kind's `province_effect_percent` (lot DC3, ADR 0082:
    /// the secondary places count for half, so that twice as many of them do not double
    /// the appeasement of their churches and abbeys).
    pub fn province_building_effects(
        &self,
        data: &GameData,
        province: &ProvinceId,
    ) -> EffectTotals {
        let mut totals = EffectTotals::default();
        for (_, settlement) in self.settlements_of(province) {
            let percent = province_effect_percent(data, settlement.kind);
            for building in settlement
                .buildings
                .iter()
                .filter_map(|id| data.buildings.get(id))
            {
                for effect in &building.effects {
                    if percent == 100 {
                        totals.add_effect(effect);
                    } else {
                        let mut weighed = effect.clone();
                        weighed.value = effect.value * f64::from(percent) / 100.0;
                        totals.add_effect(&weighed);
                    }
                }
            }
        }
        totals
    }

    /// Population capacity of `province` ([`capacity`]), its places' production
    /// buildings weighing their kind's `province_effect_percent` (lot DC3).
    pub fn province_capacity(&self, data: &GameData, province: &ProvinceId) -> u64 {
        // Production tiers in hundredths, so that full weights give exactly `capacity`.
        let tiers_percent: u64 = self
            .settlements_of(province)
            .map(|(_, settlement)| {
                let percent = u64::from(province_effect_percent(data, settlement.kind));
                settlement
                    .buildings
                    .iter()
                    .filter_map(|id| data.buildings.get(id))
                    .filter(|b| b.category == data_model::BuildingCategory::Production)
                    .map(|b| u64::from(b.tier) * percent)
                    .sum::<u64>()
            })
            .sum();
        capacity_with_tiers_percent(data, province, tiers_percent)
    }

    /// Sum of the building effects of `settlement`, plus the trait/skill
    /// effects of its province's governor (lot C4: garrison, walls,
    /// recruitment) and its province's active regional edict.
    pub fn settlement_effects(&self, data: &GameData, settlement: &SettlementId) -> EffectTotals {
        let Some(state) = self.settlements.get(settlement) else {
            return EffectTotals::default();
        };
        let mut totals = effects_of(data, &state.buildings);
        totals.merge(&self.governor_effects(data, &state.province));
        totals.merge(&crate::edicts::edict_effects(self, data, &state.province));
        totals
    }

    /// Trait/skill effects of the living governor of `province` (M4), or none.
    pub fn governor_effects(&self, data: &GameData, province: &ProvinceId) -> EffectTotals {
        self.province_governor(province)
            .map(|governor| crate::skills::character_effects(self, data, governor))
            .unwrap_or_default()
    }

    /// Fortification level used by sieges: the settlement's base level
    /// (data) plus its building effects (spec § 1.2).
    pub fn fortification_level(&self, data: &GameData, settlement: &SettlementId) -> u32 {
        let Some(state) = self.settlements.get(settlement) else {
            return 0;
        };
        let base = u32::from(state.fortification_level);
        let effects = self.settlement_effects(data, settlement);
        let walls = effects.fortification_level.apply(f64::from(base)).max(0.0);
        // F1: the controller's masonry techniques strengthen existing walls
        // (an open town gains nothing).
        let tech = crate::research::faction_tech_effects(self, data, &state.controller)
            .fortification_level
            .flat;
        let level = if walls >= 1.0 { walls + tech } else { walls };
        level.max(0.0) as u32
    }

    /// Fortification level of the city of `province`.
    pub fn province_fortification_level(&self, data: &GameData, province: &ProvinceId) -> u32 {
        self.province_city_id(province)
            .map_or(0, |city| self.fortification_level(data, city))
    }

    /// Siege resistance (0-80 %) of `settlement` against `attacker` (F1): its
    /// buildings (walls, castle, artillery bastion), the controller's
    /// fortification technologies (positive `SiegeResistance`) and the
    /// attacker's siegecraft (negative `SiegeResistance`: engineering,
    /// bombards). It shrinks the wall damage of each siege turn.
    pub fn siege_resistance(
        &self,
        data: &GameData,
        settlement: &SettlementId,
        attacker: &data_model::FactionId,
    ) -> f64 {
        let Some(controller) = self
            .settlements
            .get(settlement)
            .map(|s| s.controller.clone())
        else {
            return 0.0;
        };
        let buildings = self
            .settlement_effects(data, settlement)
            .siege_resistance
            .apply(0.0);
        let (defence, _) = crate::research::tech_siege_resistance(self, data, &controller);
        let (_, siegecraft) = crate::research::tech_siege_resistance(self, data, attacker);
        (buildings + defence + siegecraft).clamp(0.0, 80.0)
    }

    /// Buildable options of `settlement` for its controller (spec § 1.2,
    /// lot C4).
    pub fn buildable(&self, data: &GameData, settlement: &SettlementId) -> Vec<BuildOption> {
        let Some(state) = self.settlements.get(settlement) else {
            return Vec::new();
        };
        let supply = self.free_supply(data, &state.controller);
        self.buildable_with_supply(data, settlement, &supply)
    }

    /// [`CampaignState::buildable`] with the controller's
    /// [`CampaignState::free_supply`] already computed (PB3f: the AI values
    /// every settlement of a realm against the same supply).
    pub fn buildable_with_supply(
        &self,
        data: &GameData,
        settlement: &SettlementId,
        supply: &BTreeMap<ResourceId, u32>,
    ) -> Vec<BuildOption> {
        let Some(state) = self.settlements.get(settlement) else {
            return Vec::new();
        };
        let Some(province_data) = data.provinces.get(&state.province) else {
            return Vec::new();
        };
        let Some(faction) = self.factions.get(&state.controller) else {
            return Vec::new();
        };
        // PB3f: the settlement's construction speed, once for all buildings.
        let speed_percent = self.construction_speed_percent(data, settlement);
        data.buildings
            .values()
            .filter(|building| building.allowed_in(state.kind))
            .map(|building| {
                let draw = resource_draw(data, supply, &building.cost.resources);
                let priced = |livres: i64| crate::coinage::priced(self, &state.controller, livres);
                let cost = priced(i64::from(building.cost.money) + draw.import_cost);
                let mut option = BuildOption {
                    building: building.id.clone(),
                    name: building.name.display.clone(),
                    cost: cost as u32,
                    turns: build_time_at(speed_percent, building.build_time_turns),
                    available: true,
                    reason: None,
                    import_cost: priced(draw.import_cost) as u32,
                    imported: draw.imported,
                };
                if let Some(reason) = self.build_blocker(
                    data,
                    settlement,
                    state,
                    province_data,
                    faction,
                    building,
                    cost,
                ) {
                    option.available = false;
                    option.reason = Some(reason);
                }
                option
            })
            .collect()
    }

    /// B7c: resource units `faction` can still draw for a new construction
    /// or recruit: one per producing province it controls or its allies
    /// control ([`goods_map`]), less what its ongoing constructions and its
    /// recruits in training (SV2) have reserved.
    pub fn free_supply(
        &self,
        data: &GameData,
        faction: &data_model::FactionId,
    ) -> BTreeMap<ResourceId, u32> {
        let mut supply = goods_map(data, self, faction);
        for settlement in self
            .settlements
            .values()
            .filter(|s| &s.controller == faction)
        {
            // SV2: recruits in training hold what they drew, like the
            // constructions.
            let reserved = settlement
                .construction
                .iter()
                .map(|c| &c.drawn)
                .chain(settlement.recruit_queue.iter().map(|r| &r.drawn));
            for drawn in reserved {
                for (resource, amount) in drawn {
                    if let Some(left) = supply.get_mut(resource) {
                        *left = left.saturating_sub(*amount);
                    }
                }
            }
        }
        supply
    }

    /// B7b `ConstructionSpeed` of `settlement` in percent: its buildings and
    /// its province's edict, plus the best of its province's governor and
    /// its controller's ruler (a builder on the throne or in the province
    /// hurries the works; the two do not stack).
    pub fn construction_speed_percent(&self, data: &GameData, settlement: &SettlementId) -> f64 {
        let Some(state) = self.settlements.get(settlement) else {
            return 0.0;
        };
        let mut local = effects_of(data, &state.buildings);
        local.merge(&crate::edicts::edict_effects(self, data, &state.province));
        let governor = self
            .governor_effects(data, &state.province)
            .construction_speed
            .percent;
        let ruler = self
            .factions
            .get(&state.controller)
            .and_then(|f| f.ruler.as_ref())
            .filter(|r| self.characters.get(*r).is_some_and(|c| c.alive))
            .map_or(0.0, |r| {
                crate::skills::character_effects(self, data, r)
                    .construction_speed
                    .percent
            });
        local.construction_speed.percent + governor.max(ruler)
    }

    /// Turns needed to build something of `base_turns` in `settlement`
    /// (B7b): `base × 100 / (100 + speed %)`, rounded, at least one turn
    /// (+15 % turns 4 into 3, +50 % turns 8 into 5).
    pub fn build_time(&self, data: &GameData, settlement: &SettlementId, base_turns: u32) -> u32 {
        build_time_at(
            self.construction_speed_percent(data, settlement),
            base_turns,
        )
    }

    /// Build options of the city of `province` (v1 signature).
    pub fn buildable_in_province(
        &self,
        data: &GameData,
        province: &ProvinceId,
    ) -> Vec<BuildOption> {
        self.province_city_id(province)
            .map(|city| self.buildable(data, city))
            .unwrap_or_default()
    }

    #[allow(clippy::too_many_arguments)]
    fn build_blocker(
        &self,
        data: &GameData,
        settlement: &SettlementId,
        state: &SettlementState,
        province_data: &data_model::Province,
        faction: &crate::state::FactionState,
        building: &data_model::Building,
        cost: i64,
    ) -> Option<String> {
        if !building.allowed_in(state.kind) {
            return Some("impossible dans ce type de colonie".to_owned());
        }
        if state.owner != state.controller {
            return Some("la colonie doit être possédée et contrôlée".to_owned());
        }
        if state.construction.is_some() {
            return Some("une construction est déjà en cours".to_owned());
        }
        if data.has_building(&state.buildings, &building.id) {
            return Some("déjà construit".to_owned());
        }
        if let Some(from) = &building.upgrades_from {
            if !state.buildings.contains(from) {
                let name = data
                    .buildings
                    .get(from)
                    .map_or_else(|| from.to_string(), |b| b.name.display.clone());
                return Some(format!("nécessite {name}"));
            }
        }
        if let Some(required) = &building.required_building {
            if !data.has_building(&state.buildings, required) {
                let name = data
                    .buildings
                    .get(required)
                    .map_or_else(|| required.to_string(), |b| b.name.display.clone());
                return Some(format!("bâtiment requis : {name}"));
            }
        }
        if let Some(tech) = &building.required_technology {
            if !faction.technologies.contains(tech) {
                let name = data
                    .technologies
                    .get(tech)
                    .map_or_else(|| tech.to_string(), |t| t.name.display.clone());
                return Some(format!("technologie requise : {name}"));
            }
        }
        if let Some(resource) = &building.required_resource {
            if !province_data.resources.contains(resource) {
                return Some("ressource requise absente".to_owned());
            }
        }
        if building.requires_coastal {
            let is_city = self.province_city_id(&state.province) == Some(settlement);
            let port = data.settlements.get(settlement).is_some_and(|s| s.port);
            if !province_data.coastal || !(port || is_city) {
                return Some("nécessite une côte".to_owned());
            }
        }
        if building.requires_river && province_data.rivers.is_empty() {
            return Some("nécessite une rivière".to_owned());
        }
        if building.unique_per_faction
            && self.settlements.iter().any(|(id, s)| {
                s.controller == state.controller
                    && id != settlement
                    && s.buildings.contains(&building.id)
            })
        {
            return Some("unique pour la faction (déjà construit ailleurs)".to_owned());
        }
        if faction.treasury < cost {
            return Some(format!("trésor insuffisant ({cost} livres nécessaires)"));
        }
        None
    }

    /// Full city snapshot for the bridge (spec § 2): the province's
    /// population, capacity, resources and effects; the buildings,
    /// construction, walls and build options of its city.
    pub fn province_city(&self, data: &GameData, id: &ProvinceId) -> Option<ProvinceCity> {
        let state = self.provinces.get(id)?;
        let province_data = data.provinces.get(id)?;
        let city = self.settlements.get(&state.city)?;
        Some(ProvinceCity {
            classes: state.population.clone(),
            buildings: city.buildings.clone(),
            construction: city.construction.clone(),
            fortification_level: self.fortification_level(data, &state.city),
            capacity: self.province_capacity(data, id),
            buildable: self.buildable(data, &state.city),
            resources: province_data.resources.clone(),
            effects: self.province_effects(data, id),
        })
    }
}

/// Adds a finished `building` to `settlement`, replacing the building it
/// upgrades (IB5: shared with the tooltip previews).
pub(crate) fn complete_building(
    data: &GameData,
    settlement: &mut SettlementState,
    building: &BuildingId,
) {
    if let Some(from) = data
        .buildings
        .get(building)
        .and_then(|b| b.upgrades_from.clone())
    {
        settlement.buildings.retain(|b| b != &from);
    }
    settlement.buildings.push(building.clone());
}

/// Phase: progresses (and completes) constructions in every settlement.
pub(crate) fn resolve_construction(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let player = state.player_faction.clone();
    let ids: Vec<SettlementId> = state.settlements.keys().cloned().collect();
    for id in ids {
        let settlement = state.settlements.get_mut(&id).expect("exists");
        let Some(construction) = &mut settlement.construction else {
            continue;
        };
        construction.turns_left = construction.turns_left.saturating_sub(1);
        if construction.turns_left > 0 {
            continue;
        }
        let Construction { building, .. } = settlement.construction.take().expect("checked above");
        complete_building(data, settlement, &building);
        let controller = settlement.controller.clone();
        let province = settlement.province.clone();
        if controller == player {
            let name = data
                .buildings
                .get(&building)
                .map_or_else(|| building.to_string(), |b| b.name.display.clone());
            events.push(
                GameEvent::new(
                    EventKind::BuildingCompleted,
                    format!(
                        "{name} achevé à {}.",
                        crate::siege::settlement_name(data, &id)
                    ),
                )
                .province(&province)
                .faction(&controller),
            );
        }
    }
}

/// Resources accessible to `faction`: one point per producing province it
/// controls or is allied with (spec § 1.3).
pub(crate) fn goods_map(
    data: &GameData,
    state: &CampaignState,
    faction: &data_model::FactionId,
) -> BTreeMap<ResourceId, u32> {
    let mut goods: BTreeMap<ResourceId, u32> = BTreeMap::new();
    for id in state.provinces.keys() {
        if !state
            .province_controller(id)
            .is_some_and(|controller| state.is_allied(faction, controller))
        {
            continue;
        }
        let Some(province_data) = data.provinces.get(id) else {
            continue;
        };
        for resource in &province_data.resources {
            *goods.entry(resource.clone()).or_insert(0) += 1;
        }
    }
    goods
}

/// [`CampaignState::build_time`] for a construction speed already computed
/// (`construction_speed_percent`, before clamping).
fn build_time_at(speed_percent: f64, base_turns: u32) -> u32 {
    let percent = speed_percent.clamp(-50.0, MAX_CONSTRUCTION_SPEED_PERCENT);
    let turns = (f64::from(base_turns) * 100.0 / (100.0 + percent)).round();
    (turns as u32).max(1)
}
