//! Per-season population dynamics: growth, health, wealth, goods
//! satisfaction, unrest and revolt (spec § 1.1).

use data_model::{FactionId, GameData, PopulationClass, ProvinceId, SocialClass};

use crate::buildings::{capacity, EffectTotals};
use crate::events::{EventKind, GameEvent};
use crate::state::CampaignState;

/// Base growth rate per season, before health/devastation modifiers.
fn base_growth(class: SocialClass) -> f64 {
    match class {
        SocialClass::Peasants => 0.004,
        SocialClass::Burghers => 0.003,
        SocialClass::Clergy => 0.0005,
        SocialClass::Nobility => 0.001,
    }
}

/// Base wealth every class tends towards absent any modifier.
///
/// F1: raised by [`CLASS_TARGETING_WEALTH_OFFSET`] when class-targeted
/// building effects stopped reaching every class (they used to be summed
/// province-wide, which inflated every class's wealth and the M3 income
/// calibration relied on it).
fn base_wealth(class: SocialClass) -> f64 {
    CLASS_TARGETING_WEALTH_OFFSET
        + match class {
            SocialClass::Peasants => 30.0,
            SocialClass::Burghers => 55.0,
            SocialClass::Clergy => 50.0,
            SocialClass::Nobility => 70.0,
        }
}

/// See [`base_wealth`] (F1 rebalancing).
pub const CLASS_TARGETING_WEALTH_OFFSET: f64 = 8.0;

/// Devastation above which growth stalls entirely (spec § 1.1).
pub const GROWTH_DEVASTATION_CAP: u8 = 50;
/// Fraction of the gap to the target closed each season, per gauge.
const HEALTH_SPEED: f64 = 0.20;
const WEALTH_SPEED: f64 = 0.15;
const GOODS_SPEED: f64 = 0.25;
const UNREST_SPEED: f64 = 0.20;
/// Weighted-average unrest above which a province is at risk of revolt.
pub const REVOLT_UNREST_THRESHOLD: u8 = 75;
/// Consecutive seasons above [`REVOLT_UNREST_THRESHOLD`] before it revolts.
pub const REVOLT_SEASONS: u32 = 2;
/// Weighted-average unrest above which a revolting province changes hands.
pub const REVOLT_CONTROL_THRESHOLD: u8 = 90;
/// Garrison lost to a revolt.
pub const REVOLT_GARRISON_LOSS_PERCENT: u32 = 25;
/// Average health below which a plague may strike.
pub const PLAGUE_HEALTH_THRESHOLD: u8 = 30;
/// Share of the population a local plague kills (before resistance).
pub const PLAGUE_LOSS: f64 = 0.1;
/// Unrest a local plague adds to every class (before resistance).
pub const PLAGUE_UNREST: i32 = 20;
/// Devastation above which a winter famine may strike.
pub const FAMINE_DEVASTATION_THRESHOLD: u8 = 70;

/// Weighted-average unrest of a province, by population count.
pub fn weighted_unrest(classes: &data_model::PopulationClasses) -> f64 {
    let total = classes.total();
    if total == 0 {
        return 0.0;
    }
    classes
        .iter()
        .map(|(_, entry)| entry.count as f64 * f64::from(entry.unrest))
        .sum::<f64>()
        / total as f64
}

fn goods_categories(
    data: &GameData,
    goods: &std::collections::BTreeMap<data_model::ResourceId, u32>,
    class: SocialClass,
) -> usize {
    // `ResourceCategory` has no `Ord`; a small `Vec` scan is fine (≤ 4 kinds).
    let mut categories: Vec<data_model::ResourceCategory> = Vec::new();
    for (resource_id, count) in goods {
        if *count == 0 {
            continue;
        }
        let Some(resource) = data.resources.get(resource_id) else {
            continue;
        };
        if !resource.satisfies_classes.is_empty() && !resource.satisfies_classes.contains(&class) {
            continue;
        }
        if !categories.contains(&resource.category) {
            categories.push(resource.category);
        }
    }
    categories.len().min(5)
}

fn move_towards(current: u8, target: f64, speed: f64) -> u8 {
    let next = f64::from(current) + (target - f64::from(current)) * speed;
    next.round().clamp(0.0, 100.0) as u8
}

/// Updates one class in place given the province's shared modifiers.
#[allow(clippy::too_many_arguments)]
fn update_class(
    entry: &mut PopulationClass,
    class: SocialClass,
    devastation: u8,
    tax_burden: f64,
    goods_category_count: usize,
    occupied: bool,
    foreign_religion: bool,
    garrison_strength: u32,
    effects: &EffectTotals,
    population_total: u64,
    cap: u64,
) {
    // F1: effects aimed at this class only (`Effect::class`).
    let class_fx = effects.classes.get(class);
    let class_points = |value: crate::buildings::EffectValue| value.flat + value.percent;

    // ----- growth ----------------------------------------------------
    let health_factor = (f64::from(entry.health) - 50.0) / 50.0;
    let devastation_factor = if devastation > GROWTH_DEVASTATION_CAP {
        0.0
    } else {
        1.0
    };
    let mut growth_rate = base_growth(class) * health_factor * devastation_factor;
    growth_rate += (effects.growth.flat + class_fx.growth.flat) / 100.0;
    growth_rate *= 1.0 + (effects.growth.percent + class_fx.growth.percent) / 100.0;
    let new_count = (entry.count as f64 * (1.0 + growth_rate)).max(0.0).round() as u64;
    entry.count = new_count;

    // ----- health ------------------------------------------------------
    let overpopulation = if cap > 0 {
        ((population_total as f64 / cap as f64) - 1.0).max(0.0) * 20.0
    } else {
        0.0
    };
    let health_target = 50.0
        + effects.health.flat
        + effects.health.percent
        + class_points(class_fx.health)
        + (f64::from(entry.goods_satisfaction) - 50.0) / 4.0
        - overpopulation;
    entry.health = move_towards(entry.health, health_target, HEALTH_SPEED);

    // ----- wealth --------------------------------------------------------
    let wealth_target = base_wealth(class)
        + effects.wealth.flat
        + effects.wealth.percent
        + class_points(class_fx.wealth)
        + effects.trade_income.flat
        - tax_burden * 30.0
        - f64::from(devastation) / 2.0;
    entry.wealth = move_towards(entry.wealth, wealth_target, WEALTH_SPEED);

    // ----- goods satisfaction -------------------------------------------
    let goods_target = 40.0
        + 10.0 * goods_category_count as f64
        + effects.goods_satisfaction.flat
        + effects.goods_satisfaction.percent
        + class_points(class_fx.goods_satisfaction);
    entry.goods_satisfaction = move_towards(entry.goods_satisfaction, goods_target, GOODS_SPEED);

    // ----- unrest ----------------------------------------------------------
    let garrison_relief = f64::from(garrison_strength / 100).min(20.0);
    let mut unrest_target = tax_burden * 40.0
        + f64::from(devastation) / 2.0
        + (50.0 - f64::from(entry.goods_satisfaction)) / 3.0
        + (50.0 - f64::from(entry.health)) / 4.0
        - garrison_relief;
    if occupied {
        unrest_target += 25.0;
    }
    if foreign_religion {
        unrest_target += 10.0;
    }
    // Building `Unrest` effects: negative values are appeasement.
    unrest_target += effects.unrest.flat + effects.unrest.percent + class_points(class_fx.unrest);
    // F1 `Loyalty` (castles, a loyal governor): the local nobility holds
    // to its lord.
    if class == SocialClass::Nobility {
        unrest_target -= effects.loyalty.apply(0.0);
    }
    unrest_target = unrest_target.clamp(0.0, 100.0);
    entry.unrest = move_towards(entry.unrest, unrest_target, UNREST_SPEED);
}

/// Phase: growth, health, wealth, goods satisfaction, unrest, revolt.
pub(crate) fn resolve_population(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let winter = state.season == crate::state::Season::Winter;
    let ids: Vec<ProvinceId> = state.provinces.keys().cloned().collect();
    for id in ids {
        let (controller, owner, devastation, buildings, garrison_strength) = {
            let p = &state.provinces[&id];
            let Some(city) = state.settlements.get(&p.city) else {
                continue;
            };
            (
                city.controller.clone(),
                city.owner.clone(),
                p.devastation,
                state.province_buildings(&id),
                state.province_garrison_strength(&id),
            )
        };
        let Some(province_data) = data.provinces.get(&id) else {
            continue;
        };
        let tax_burden = state
            .factions
            .get(&controller)
            .map_or(0.35, |f| f.tax_rate.burden());
        let goods = state
            .factions
            .get(&controller)
            .map(|f| f.goods.clone())
            .unwrap_or_default();
        let occupied = controller != owner;
        // Obediences of one church are not foreign to each other (M5).
        let foreign_religion = crate::religion::faction_religion(state, data, &controller)
            .is_some_and(|r| !crate::religion::same_faith(data, &r, &province_data.religion));
        // Buildings plus governor (M4), technologies (M6), plus regency,
        // excommunication, embargo and heresy unrest (M5).
        let mut effects = state.province_effects(data, &id);
        effects.merge(&crate::research::faction_province_tech_effects(
            state,
            data,
            &controller,
        ));
        effects.unrest.flat += state.political_unrest(&id);
        let cap = capacity(data, &id, &buildings);
        // H3: the province's diet, possibly aimed at one class.
        let class_effects: Vec<EffectTotals> = SocialClass::ALL
            .iter()
            .map(|class| {
                let mut merged = effects;
                merged.merge(&crate::table::diet_class_effects(state, data, &id, *class));
                // H5: inflation and the coinage itself.
                merged.merge(&crate::coinage::class_effects(state, &controller, *class));
                merged
            })
            .collect();
        // H4: a resistant province may be spared, and suffers less.
        let resistance = crate::medicine::plague_resistance(state, data, &id);

        let province = state.provinces.get_mut(&id).expect("exists");
        let population_total = province.population.total();
        for (class, class_effects) in SocialClass::ALL.into_iter().zip(&class_effects) {
            let goods_categories_count = goods_categories(data, &goods, class);
            let entry = match class {
                SocialClass::Peasants => &mut province.population.peasants,
                SocialClass::Burghers => &mut province.population.burghers,
                SocialClass::Clergy => &mut province.population.clergy,
                SocialClass::Nobility => &mut province.population.nobility,
            };
            update_class(
                entry,
                class,
                devastation,
                tax_burden,
                goods_categories_count,
                occupied,
                foreign_religion,
                garrison_strength,
                class_effects,
                population_total,
                cap,
            );
        }

        // ----- revolt tracking ---------------------------------------------
        let weighted = weighted_unrest(&province.population);
        if weighted > f64::from(REVOLT_UNREST_THRESHOLD) {
            province.revolt_seasons += 1;
        } else {
            province.revolt_seasons = 0;
        }
        if province.revolt_seasons >= REVOLT_SEASONS {
            let province_name = province_data.name.display.clone();
            // Lot C4: the rebels rise at the city.
            let city_id = province.city.clone();
            let Some(city) = state.settlements.get_mut(&city_id) else {
                continue;
            };
            let lost = city.garrison_strength() * REVOLT_GARRISON_LOSS_PERCENT / 100;
            let mut removed = 0u32;
            city.garrison.retain_mut(|unit| {
                if removed >= lost {
                    return true;
                }
                if unit.strength <= lost - removed {
                    removed += unit.strength;
                    false
                } else {
                    unit.strength -= lost - removed;
                    removed = lost;
                    true
                }
            });
            events.push(
                GameEvent::new(
                    EventKind::Revolt,
                    format!("Révolte à {province_name} : la garnison est affaiblie."),
                )
                .province(&id)
                .faction(&controller),
            );
            if weighted > f64::from(REVOLT_CONTROL_THRESHOLD) {
                if let Ok(rebels) = FactionId::new("fac_rebels") {
                    if state.factions.contains_key(&rebels) {
                        let province = state.provinces.get_mut(&id).expect("exists");
                        province.revolt_seasons = 0;
                        let city = state.settlements.get_mut(&city_id).expect("exists");
                        city.controller = rebels.clone();
                        city.siege = None;
                        events.push(
                            GameEvent::new(
                                EventKind::Revolt,
                                format!("{province_name} passe aux mains des rebelles."),
                            )
                            .province(&id)
                            .faction(&rebels),
                        );
                    }
                }
            }
        }

        // ----- plague / famine -----------------------------------------------
        let province = state.provinces.get_mut(&id).expect("exists");
        let average_health = province
            .population
            .iter()
            .map(|(_, e)| f64::from(e.health))
            .sum::<f64>()
            / 4.0;
        let plague = average_health < f64::from(PLAGUE_HEALTH_THRESHOLD);
        // The roll only happens with some resistance, so that factions
        // without medicine keep the exact random stream of before (H4).
        let spared = plague && resistance > 0.0 && state.rng.unit_f64() < resistance;
        let province = state.provinces.get_mut(&id).expect("exists");
        if spared {
            if controller == state.player_faction {
                let province_name = province_data.name.display.clone();
                events.push(
                    GameEvent::new(
                        EventKind::Medicine,
                        format!(
                            "La peste menaçait {province_name} : simples, fumigations et isolement l'ont tenue à distance."
                        ),
                    )
                    .province(&id)
                    .faction(&controller),
                );
            }
        } else if plague {
            let survival = 1.0 - PLAGUE_LOSS * (1.0 - resistance);
            let unrest =
                crate::medicine::mitigated(-PLAGUE_UNREST, resistance).unsigned_abs() as u8;
            for (_, entry) in [
                (SocialClass::Peasants, &mut province.population.peasants),
                (SocialClass::Burghers, &mut province.population.burghers),
                (SocialClass::Clergy, &mut province.population.clergy),
                (SocialClass::Nobility, &mut province.population.nobility),
            ] {
                entry.count = (entry.count as f64 * survival).round() as u64;
                entry.unrest = entry.unrest.saturating_add(unrest).min(100);
            }
            let province_name = province_data.name.display.clone();
            events.push(
                GameEvent::new(
                    EventKind::Plague,
                    format!(
                        "La peste frappe {province_name} : population et ordre public en berne."
                    ),
                )
                .province(&id)
                .faction(&controller),
            );
        }
        if winter && province.devastation > FAMINE_DEVASTATION_THRESHOLD {
            province.population.peasants.count =
                (province.population.peasants.count as f64 * 0.95).round() as u64;
            let province_name = province_data.name.display.clone();
            events.push(
                GameEvent::new(
                    EventKind::Famine,
                    format!("Famine hivernale à {province_name} : les paysans meurent de faim."),
                )
                .province(&id)
                .faction(&controller),
            );
        }
    }
}
