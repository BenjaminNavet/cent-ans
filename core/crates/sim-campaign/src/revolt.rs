//! Graded revolts (B4, ADR 0731): the tension gauge of each province, the
//! warnings before a revolt, the rebel army raised by an uprising and its
//! outcome (put down by force, or the city handed to the rebels).
//!
//! Every value is in `data/rules/revolt.json` ([`data_model::RevoltRules`]).

use data_model::{FactionId, GameData, ProvinceId, UnitTypeId};

use crate::events::{EventKind, GameEvent};
use crate::state::{Army, ArmyPosition, CampaignState, Unit, Uprising};

fn rebels() -> Option<FactionId> {
    FactionId::new(FactionId::REBELS).ok()
}

/// Sets or lifts the war between `a` and `b` (both directions).
fn set_war(state: &mut CampaignState, a: &FactionId, b: &FactionId, war: bool) {
    for (x, y) in [(a, b), (b, a)] {
        if let Some(faction) = state.factions.get_mut(x) {
            if war {
                faction.at_war_with.insert(y.clone());
            } else {
                faction.at_war_with.remove(y);
                faction.war_scores.remove(y);
            }
        }
    }
}

/// Lifts the war with the rebels when no rebel army is left in the field.
fn end_rebel_war(state: &mut CampaignState, controller: &FactionId) {
    let Some(rebels) = rebels() else {
        return;
    };
    let any_rebel_army = state.armies.values().any(|a| a.faction == rebels);
    if !any_rebel_army && controller != &rebels {
        set_war(state, controller, &rebels, false);
    }
}

/// Raises the rebel army: regiments among the peasants and the burghers
/// in proportion to the province's people. Empty when the unit types are
/// unknown.
fn rebel_regiments(state: &CampaignState, data: &GameData, id: &ProvinceId) -> Vec<Unit> {
    let levy = &data.revolt_rules.rebel_levy;
    let Some(province) = state.provinces.get(id) else {
        return Vec::new();
    };
    let population = province.population.total();
    let count = ((population as f64 / levy.population_per_regiment.max(1) as f64).round() as u32)
        .clamp(
            levy.min_regiments,
            levy.max_regiments.max(levy.min_regiments),
        );
    let burgher_share = if population == 0 {
        0.0
    } else {
        province.population.burghers.count as f64 / population as f64
    };
    let burghers = ((f64::from(count) * burgher_share).round() as u32).min(count);
    let kind = |raw: &str| {
        UnitTypeId::new(raw)
            .ok()
            .and_then(|type_id| data.unit_types.get(&type_id))
    };
    let mut units = Vec::new();
    for (raw, n) in [
        (&levy.burgher_unit_type, burghers),
        (&levy.peasant_unit_type, count - burghers),
    ] {
        if let Some(unit_type) = kind(raw) {
            units.extend((0..n).map(|_| Unit::fresh(unit_type)));
        }
    }
    units
}

/// The tension of `id` for this season (called by the population phase with
/// the class-weighted unrest and the city's controller). Warns, raises the
/// uprising at full tension, and follows a uprising already under way.
pub(crate) fn resolve(
    state: &mut CampaignState,
    data: &GameData,
    id: &ProvinceId,
    weighted: f64,
    controller: &FactionId,
    events: &mut Vec<GameEvent>,
) {
    let rules = &data.revolt_rules;
    let threshold = data.population_rules.revolt_unrest_threshold;
    let Some(province) = state.provinces.get_mut(id) else {
        return;
    };
    // Informative counter: consecutive seasons above the threshold.
    if weighted > threshold && !controller.is_rebels() {
        province.revolt_seasons += 1;
    } else {
        province.revolt_seasons = 0;
    }
    if province.uprising.is_some() {
        resolve_uprising(state, data, id, controller, events);
        return;
    }
    let before = province.tension;
    let rise = if controller.is_rebels() {
        0.0
    } else {
        rules.rise(weighted, threshold)
    };
    let after = if rise > 0.0 {
        (before + rise.round() as u32).min(rules.tension_max)
    } else {
        before.saturating_sub(rules.tension_decay.round() as u32)
    };
    province.tension = after;
    let name = data.province_name(id);
    if after >= rules.tension_max {
        rise_up(state, data, id, controller, &name, events);
        return;
    }
    for &level in &rules.warning_tensions {
        if before < level && after >= level {
            events.push(
                GameEvent::new(
                    EventKind::Unrest,
                    format!(
                        "Grogne à {name} : la tension monte ({after} sur {}).",
                        rules.tension_max
                    ),
                )
                .province(id)
                .faction(controller)
                .loss(),
            );
        }
    }
}

/// Full tension: the garrison is weakened and a rebel army stands in the
/// province.
fn rise_up(
    state: &mut CampaignState,
    data: &GameData,
    id: &ProvinceId,
    controller: &FactionId,
    name: &str,
    events: &mut Vec<GameEvent>,
) {
    let rules = &data.revolt_rules;
    let Some(province) = state.provinces.get_mut(id) else {
        return;
    };
    province.tension = rules.tension_after_revolt;
    province.revolt_seasons = 0;
    let city_id = province.city.clone();
    let Some(city) = state.settlements.get_mut(&city_id) else {
        return;
    };
    let lost = city.garrison_strength() * rules.garrison_loss_percent / 100;
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
    let units = rebel_regiments(state, data, id);
    let rebels = rebels().filter(|r| state.factions.contains_key(r));
    let Some(rebels) = rebels.filter(|_| !units.is_empty()) else {
        events.push(
            GameEvent::new(
                EventKind::Revolt,
                format!("Révolte à {name} : la garnison est affaiblie."),
            )
            .province(id)
            .faction(controller),
        );
        return;
    };
    let city_point = data.settlement_point(&city_id).unwrap_or([0.0, 0.0]);
    let offset = crate::march::px_per_km(data) * data.free_movement_rules().engage_radius_km as f32;
    let army_id = state.allocate_army_id();
    let raised_strength: u32 = units.iter().map(|u| u.strength).sum();
    state.armies.insert(
        army_id.clone(),
        Army::new(
            rebels.clone(),
            ArmyPosition::field([city_point[0] + offset * 0.6, city_point[1]]),
            units,
        ),
    );
    set_war(state, controller, &rebels, true);
    if let Some(province) = state.provinces.get_mut(id) {
        province.uprising = Some(Uprising {
            army: army_id.clone(),
            raised_strength,
            seasons: 0,
        });
    }
    events.push(
        GameEvent::new(
            EventKind::Revolt,
            format!(
                "Révolte à {name} : la garnison est affaiblie et {raised_strength} rebelles prennent les armes."
            ),
        )
        .province(id)
        .army(&army_id)
        .faction(controller),
    );
}

/// One season of an uprising: put down when the rebel army is beaten,
/// otherwise the province falls to the rebels once it has held long enough.
fn resolve_uprising(
    state: &mut CampaignState,
    data: &GameData,
    id: &ProvinceId,
    controller: &FactionId,
    events: &mut Vec<GameEvent>,
) {
    let rules = &data.revolt_rules;
    let Some(uprising) = state.provinces.get(id).and_then(|p| p.uprising.clone()) else {
        return;
    };
    let name = data.province_name(id);
    let strength = state
        .armies
        .get(&uprising.army)
        .map_or(0, |army| army.total_strength());
    let beaten = strength * 100 < uprising.raised_strength * rules.rebel_rout_percent
        || strength == 0
        || controller.is_rebels();
    if beaten {
        state.armies.remove(&uprising.army);
        let relief = u8::try_from(rules.repression_disorder_relief).unwrap_or(u8::MAX);
        if let Some(province) = state.provinces.get_mut(id) {
            province.uprising = None;
            province.tension = province.tension.min(rules.repression_tension);
            province.unrest = province.unrest.saturating_sub(relief);
        }
        end_rebel_war(state, controller);
        if !controller.is_rebels() {
            events.push(
                GameEvent::new(
                    EventKind::Unrest,
                    format!("La révolte de {name} est matée : les rebelles sont dispersés."),
                )
                .province(id)
                .faction(controller),
            );
        }
        return;
    }
    // A province changing hands keeps the rebels at war with its new lord.
    if let Some(rebels) = rebels() {
        if !state.is_at_war(controller, &rebels) {
            set_war(state, controller, &rebels, true);
        }
    }
    let seasons = uprising.seasons + 1;
    if seasons < rules.rebel_siege_seasons {
        if let Some(province) = state.provinces.get_mut(id) {
            if let Some(up) = province.uprising.as_mut() {
                up.seasons = seasons;
            }
        }
        return;
    }
    // The rebels hold the province: its city goes over to them.
    let Some(rebels) = rebels().filter(|r| state.factions.contains_key(r)) else {
        return;
    };
    state.armies.remove(&uprising.army);
    let city_id = match state.provinces.get_mut(id) {
        Some(province) => {
            province.uprising = None;
            province.revolt_seasons = 0;
            province.tension = rules.tension_after_revolt;
            province.city.clone()
        }
        None => return,
    };
    if let Some(city) = state.settlements.get_mut(&city_id) {
        city.hand_over(&rebels);
    }
    events.push(
        GameEvent::new(
            EventKind::Revolt,
            format!("{name} passe aux mains des rebelles."),
        )
        .province(id)
        .faction(&rebels),
    );
}
