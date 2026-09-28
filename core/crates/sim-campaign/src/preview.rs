//! IB5 (ADR 0109, spec IB § 2.2): "before → after" values of the statistics an
//! effect moves, and the state of each requirement, for the rich tooltips.
//!
//! Every value is read on the real state and on a copy where the thing is
//! applied the way the turn applies it (a building completed, a technology
//! acquired), with the same functions the turn uses: nothing is re-derived.
//! An effect whose statistic has no exact value in its context (battle
//! modifiers, garrison levies, piety, growth...) gets no entry.
//!
//! Statistics read, by effect kind:
//! - `tax_income`, `trade_income`, `production`: seasonal income in livres —
//!   of the province for a building ([`CampaignState::province_gross_income`]),
//!   of the faction for a technology
//!   ([`CampaignState::faction_income_effective`]);
//! - `health`, `wealth`, `goods_satisfaction`, `unrest`: the value the gauge
//!   tends towards ([`crate::population::equilibrium`]), averaged over the
//!   population (the province's for a building, the faction's provinces' for
//!   a technology), or of one class for an effect aimed at it;
//! - `research_points`: the faction's research points per turn;
//! - buildings only: `fortification_level` and `recruit_slots` of the
//!   settlement, `plague_resistance` (%) of the province.

use std::collections::BTreeMap;

use data_model::{
    BuildingId, Effect, FactionId, GameData, ProvinceId, SettlementId, SocialClass, TechnologyId,
    UnitTypeId,
};

use crate::state::CampaignState;

/// `{effect key: [before, after]}` (see [`effect_key`]).
pub type BeforeAfter = BTreeMap<String, [f64; 2]>;

/// State of one requirement of a building, a unit or a technology.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requirement {
    /// The entity required (building, technology, resource id), or a tag:
    /// `coastal`, `river`, `enabling_building`.
    pub id: String,
    pub met: bool,
}

impl Requirement {
    fn new(id: impl Into<String>, met: bool) -> Self {
        Requirement { id: id.into(), met }
    }
}

fn json_key<T: serde::Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// Key of an effect in [`BeforeAfter`]: its kind (`"health"`), followed by
/// `":<class>"` or `":<unit category>"` when it targets one.
pub fn effect_key(effect: &Effect) -> String {
    let kind = json_key(effect.effect);
    if let Some(class) = effect.class {
        format!("{kind}:{}", class.key())
    } else if let Some(category) = effect.unit_category {
        format!("{kind}:{}", json_key(category))
    } else {
        kind
    }
}

/// Where the statistics of a preview are read.
enum Context<'a> {
    /// A building in a settlement: its province, its controller.
    Settlement {
        settlement: &'a SettlementId,
        province: &'a ProvinceId,
        faction: &'a FactionId,
    },
    /// A technology: the whole faction.
    Faction(&'a FactionId),
}

impl Context<'_> {
    fn faction(&self) -> &FactionId {
        match self {
            Context::Settlement { faction, .. } | Context::Faction(faction) => faction,
        }
    }
}

const GAUGES: [&str; 4] = ["health", "wealth", "goods_satisfaction", "unrest"];

fn gauge(targets: &crate::population::GaugeTargets, name: &str) -> f64 {
    match name {
        "health" => targets.health,
        "wealth" => targets.wealth,
        "goods_satisfaction" => targets.goods_satisfaction,
        _ => targets.unrest,
    }
}

/// Population-weighted gauge targets of `provinces`: `(whole population,
/// per class)` sums of `count × target` and the counts.
#[derive(Default)]
struct GaugeSums {
    all: BTreeMap<&'static str, (f64, f64)>,
    classes: BTreeMap<(&'static str, SocialClass), (f64, f64)>,
}

impl GaugeSums {
    fn read(state: &CampaignState, data: &GameData, provinces: &[ProvinceId]) -> Self {
        let mut sums = GaugeSums::default();
        for province in provinces {
            for (class, count, targets) in
                crate::population::equilibrium(state, data, province).unwrap_or_default()
            {
                let count = count as f64;
                for name in GAUGES {
                    let value = gauge(&targets, name);
                    let all = sums.all.entry(name).or_default();
                    all.0 += count * value;
                    all.1 += count;
                    let one = sums.classes.entry((name, class)).or_default();
                    one.0 += count * value;
                    one.1 += count;
                }
            }
        }
        sums
    }

    fn value(&self, name: &str, class: Option<SocialClass>) -> Option<f64> {
        let (sum, count) = match class {
            Some(class) => self
                .classes
                .iter()
                .find(|((n, c), _)| *n == name && *c == class)
                .map(|(_, v)| *v)?,
            None => self
                .all
                .iter()
                .find(|(n, _)| **n == name)
                .map(|(_, v)| *v)?,
        };
        (count > 0.0).then(|| (sum / count * 10.0).round() / 10.0)
    }
}

/// Reads the statistic of `effect` in `context`, `None` when it has no exact
/// value there. `gauges` is computed lazily (the population model is the
/// costly part).
fn read_stat(
    state: &CampaignState,
    data: &GameData,
    context: &Context,
    effect: &Effect,
    gauges: &mut Option<GaugeSums>,
) -> Option<f64> {
    use data_model::EffectKind as K;
    let faction = context.faction();
    // Effects aimed at a unit family move no statistic of a place or realm.
    if effect.unit_category.is_some() {
        return None;
    }
    let kind = json_key(effect.effect);
    if GAUGES.contains(&kind.as_str()) {
        let sums = gauges.get_or_insert_with(|| {
            let provinces: Vec<ProvinceId> = match context {
                Context::Settlement { province, .. } => vec![(*province).clone()],
                Context::Faction(faction) => state
                    .provinces
                    .keys()
                    .filter(|p| state.province_controller(p) == Some(*faction))
                    .cloned()
                    .collect(),
            };
            GaugeSums::read(state, data, &provinces)
        });
        return sums.value(&kind, effect.class);
    }
    if effect.class.is_some() {
        return None;
    }
    match (effect.effect, context) {
        (K::TaxIncome | K::TradeIncome | K::Production, Context::Settlement { province, .. }) => {
            let tax_rate = state.factions.get(faction)?.tax_rate;
            let tech = crate::research::faction_province_tech_effects(state, data, faction);
            Some(state.province_gross_income(data, province, faction, tax_rate, &tech) as f64)
        }
        (K::TaxIncome | K::TradeIncome | K::Production, Context::Faction(_)) => {
            Some(state.faction_income_effective(data, faction) as f64)
        }
        (K::ResearchPoints, _) => Some(f64::from(state.research_points_per_turn(data, faction))),
        (K::FortificationLevel, Context::Settlement { settlement, .. }) => {
            Some(f64::from(state.fortification_level(data, settlement)))
        }
        (K::RecruitSlots, Context::Settlement { settlement, .. }) => {
            Some(state.recruit_slots(data, settlement) as f64)
        }
        (K::PlagueResistance, Context::Settlement { province, .. }) => {
            let percent = crate::medicine::plague_resistance(state, data, province) * 100.0;
            Some((percent * 10.0).round() / 10.0)
        }
        _ => None,
    }
}

/// Before/after of each effect of `effects` whose key is unique among them
/// (two effects of one key would share a line's value).
fn before_after(
    before: &CampaignState,
    after: &CampaignState,
    data: &GameData,
    context: &Context,
    effects: &[Effect],
) -> BeforeAfter {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for effect in effects {
        *counts.entry(effect_key(effect)).or_default() += 1;
    }
    let (mut gauges_before, mut gauges_after) = (None, None);
    let mut out = BeforeAfter::new();
    for effect in effects {
        let key = effect_key(effect);
        if counts.get(&key) != Some(&1) {
            continue;
        }
        let old = read_stat(before, data, context, effect, &mut gauges_before);
        let new = read_stat(after, data, context, effect, &mut gauges_after);
        if let (Some(old), Some(new)) = (old, new) {
            out.insert(key, [old, new]);
        }
    }
    out
}

impl CampaignState {
    /// Statistics of `settlement` and its province moved by completing
    /// `building` there (empty when already built or unknown).
    pub fn building_before_after(
        &self,
        data: &GameData,
        settlement: &SettlementId,
        building: &BuildingId,
    ) -> BeforeAfter {
        let (Some(place), Some(definition)) = (
            self.settlements.get(settlement),
            data.buildings.get(building),
        ) else {
            return BeforeAfter::new();
        };
        if data.has_building(&place.buildings, building) {
            return BeforeAfter::new();
        }
        let context = Context::Settlement {
            settlement,
            province: &place.province,
            faction: &place.controller,
        };
        let mut after = self.clone();
        if let Some(place) = after.settlements.get_mut(settlement) {
            crate::buildings::complete_building(data, place, building);
        }
        before_after(self, &after, data, &context, &definition.effects)
    }

    /// Statistics of `faction` moved by acquiring `technology` (empty when
    /// already known or unknown).
    pub fn technology_before_after(
        &self,
        data: &GameData,
        faction: &FactionId,
        technology: &TechnologyId,
    ) -> BeforeAfter {
        let (Some(known), Some(definition)) = (
            self.factions.get(faction).map(|f| &f.technologies),
            data.technologies.get(technology),
        ) else {
            return BeforeAfter::new();
        };
        if known.contains(technology) {
            return BeforeAfter::new();
        }
        let mut after = self.clone();
        if let Some(f) = after.factions.get_mut(faction) {
            f.technologies.insert(technology.clone());
        }
        before_after(
            self,
            &after,
            data,
            &Context::Faction(faction),
            &definition.effects,
        )
    }

    /// Requirements of `building` in `settlement` (the checks of
    /// `build_blocker`, each on its own), in the order `upgrades_from`,
    /// `required_building`, `required_technology`, `required_resource`,
    /// coast, river.
    pub fn building_requirements(
        &self,
        data: &GameData,
        settlement: &SettlementId,
        building: &BuildingId,
    ) -> Vec<Requirement> {
        let (Some(place), Some(definition)) = (
            self.settlements.get(settlement),
            data.buildings.get(building),
        ) else {
            return Vec::new();
        };
        let province = data.provinces.get(&place.province);
        let technologies = self
            .factions
            .get(&place.controller)
            .map(|f| &f.technologies);
        let mut out = Vec::new();
        if let Some(from) = &definition.upgrades_from {
            out.push(Requirement::new(
                from.as_str(),
                place.buildings.contains(from),
            ));
        }
        if let Some(required) = &definition.required_building {
            let met = data.has_building(&place.buildings, required);
            out.push(Requirement::new(required.as_str(), met));
        }
        if let Some(tech) = &definition.required_technology {
            let met = technologies.is_some_and(|t| t.contains(tech));
            out.push(Requirement::new(tech.as_str(), met));
        }
        if let Some(resource) = &definition.required_resource {
            let met = province.is_some_and(|p| p.resources.contains(resource));
            out.push(Requirement::new(resource.as_str(), met));
        }
        if definition.requires_coastal {
            let is_city = self.province_city_id(&place.province) == Some(settlement);
            let port = data.settlements.get(settlement).is_some_and(|s| s.port);
            let met = province.is_some_and(|p| p.coastal) && (port || is_city);
            out.push(Requirement::new("coastal", met));
        }
        if definition.requires_river {
            let met = province.is_some_and(|p| !p.rivers.is_empty());
            out.push(Requirement::new("river", met));
        }
        out
    }

    /// Requirements of recruiting `unit_type` in `settlement` (the checks of
    /// `recruit_blocker`): its technology, then one of its enabling
    /// buildings (`enabling_building`).
    pub fn recruit_requirements(
        &self,
        data: &GameData,
        settlement: &SettlementId,
        unit_type: &UnitTypeId,
    ) -> Vec<Requirement> {
        let (Some(place), Some(definition)) = (
            self.settlements.get(settlement),
            data.unit_types.get(unit_type),
        ) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        if let Some(tech) = &definition.required_technology {
            let met = self
                .factions
                .get(&place.controller)
                .is_some_and(|f| f.technologies.contains(tech));
            out.push(Requirement::new(tech.as_str(), met));
        }
        let mut enablers = crate::buildings::enabling_buildings(data, unit_type).peekable();
        if enablers.peek().is_some() {
            let met = enablers.any(|b| data.has_building(&place.buildings, &b.id));
            out.push(Requirement::new("enabling_building", met));
        }
        out
    }

    /// Prerequisites of `technology`, each known or not by `faction`.
    pub fn technology_requirements(
        &self,
        data: &GameData,
        faction: &FactionId,
        technology: &TechnologyId,
    ) -> Vec<Requirement> {
        let Some(definition) = data.technologies.get(technology) else {
            return Vec::new();
        };
        let known = self.factions.get(faction).map(|f| &f.technologies);
        definition
            .prerequisites
            .iter()
            .map(|p| Requirement::new(p.as_str(), known.is_some_and(|k| k.contains(p))))
            .collect()
    }
}

#[cfg(test)]
mod tests;
