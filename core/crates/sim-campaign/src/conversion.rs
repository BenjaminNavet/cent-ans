//! Progressive conversion of a province to the faith of its lord (ADR 0326,
//! Medieval II's religion percentages). The province keeps its faith from the
//! data until `conversion_progress` reaches 100 under a lord of another faith;
//! then [`ProvinceState::faith_override`] changes and the « foreign religion »
//! unrest disappears. Everything is read from `data/rules/religion.json`.

use data_model::{GameData, ProvinceId, ReligionId};

use crate::events::{EventKind, GameEvent};
use crate::religion::{self, FaithRelation};
use crate::state::CampaignState;

impl CampaignState {
    /// Faith of a province: its conversion, else the faith of the data.
    pub fn province_faith(&self, data: &GameData, province: &ProvinceId) -> Option<ReligionId> {
        self.provinces
            .get(province)
            .and_then(|p| p.faith_override.clone())
            .or_else(|| data.provinces.get(province).map(|p| p.religion.clone()))
    }

    /// Faith the lord of `province` would convert it to, with the relation
    /// between the two faiths; `None` when nothing is to be converted (no
    /// lord, same faith, obediences of one church).
    pub fn conversion_goal(
        &self,
        data: &GameData,
        province: &ProvinceId,
    ) -> Option<(ReligionId, FaithRelation)> {
        let controller = self.province_controller(province)?;
        let lord_faith = religion::faction_religion(self, data, controller)?;
        let own = self.province_faith(data, province)?;
        match religion::religions_relation(data, &lord_faith, &own) {
            FaithRelation::Same | FaithRelation::RivalObedience => None,
            relation => Some((lord_faith, relation)),
        }
    }

    /// Points a season of the lord's rule adds to the conversion of
    /// `province` (0 when stalled or nothing to convert).
    pub fn conversion_speed(&self, data: &GameData, province: &ProvinceId) -> u32 {
        let Some(rules) = data.religion_rules.as_ref().map(|r| &r.conversion) else {
            return 0;
        };
        let Some((_, relation)) = self.conversion_goal(data, province) else {
            return 0;
        };
        let (Some(p), Some(controller)) = (
            self.provinces.get(province),
            self.province_controller(province),
        ) else {
            return 0;
        };
        if p.unrest >= rules.stall_unrest {
            return 0;
        }
        let piety = self
            .province_governor(province)
            .map(|g| religion::effective_piety(self, data, g))
            .unwrap_or_else(|| religion::ruler_piety(self, data, controller));
        let mut points = f64::from(rules.base_per_season)
            + f64::from(piety) / f64::from(rules.piety_divisor.max(1))
            + rules.per_religious_building
                * religion::weighted_religious_buildings(self, data, province);
        if relation == FaithRelation::Kindred {
            points = points * f64::from(rules.kindred_percent) / 100.0;
        }
        if self.province_owner(province) != Some(controller) {
            points = points * f64::from(rules.occupier_percent) / 100.0;
        }
        points.round().max(0.0) as u32
    }

    /// A preacher's sermon in `province`: adds progress to a conversion to
    /// `faith` (the preacher's own). Returns the points actually added.
    pub(crate) fn push_conversion(
        &mut self,
        data: &GameData,
        province: &ProvinceId,
        gain: u32,
    ) -> u32 {
        let Some((goal, _)) = self.conversion_goal(data, province) else {
            return 0;
        };
        let Some(p) = self.provinces.get_mut(province) else {
            return 0;
        };
        if p.conversion_to.as_ref() != Some(&goal) {
            p.conversion_to = Some(goal);
            p.conversion_progress = 0;
        }
        let before = p.conversion_progress;
        p.conversion_progress = (u32::from(before) + gain).min(100) as u8;
        u32::from(p.conversion_progress - before)
    }
}

/// Phase (after the heresies): each province under a lord of another faith
/// moves towards his faith; at 100 it converts. Provinces whose lord shares
/// their faith lose any leftover progress.
pub(crate) fn resolve_conversion(
    state: &mut CampaignState,
    data: &GameData,
    events: &mut Vec<GameEvent>,
) {
    let Some(rules) = data.religion_rules.as_ref().map(|r| r.conversion.clone()) else {
        return;
    };
    let ids: Vec<ProvinceId> = state.provinces.keys().cloned().collect();
    for id in ids {
        let goal = state.conversion_goal(data, &id);
        let Some((goal, _)) = goal else {
            if let Some(p) = state.provinces.get_mut(&id) {
                if p.conversion_progress > 0 {
                    p.conversion_progress =
                        p.conversion_progress.saturating_sub(rules.decay_per_season);
                    if p.conversion_progress == 0 {
                        p.conversion_to = None;
                    }
                }
            }
            continue;
        };
        let speed = state.conversion_speed(data, &id);
        let Some(p) = state.provinces.get_mut(&id) else {
            continue;
        };
        if p.conversion_to.as_ref() != Some(&goal) {
            p.conversion_to = Some(goal.clone());
            p.conversion_progress = 0;
        }
        p.conversion_progress = (u32::from(p.conversion_progress) + speed).min(100) as u8;
        if p.conversion_progress < 100 {
            continue;
        }
        // The faith changes.
        let static_faith = data.provinces.get(&id).map(|d| d.religion.clone());
        p.faith_override = if static_faith.as_ref() == Some(&goal) {
            None
        } else {
            Some(goal.clone())
        };
        p.conversion_progress = 0;
        p.conversion_to = None;
        for class in [
            &mut p.population.peasants,
            &mut p.population.burghers,
            &mut p.population.clergy,
        ] {
            class.unrest = class
                .unrest
                .saturating_add(rules.unrest_on_conversion)
                .min(100);
        }
        let controller = state.province_controller(&id).cloned();
        let text = format!(
            "{} se convertit : la province suit désormais {}.",
            data.province_name(&id),
            religion::religion_display(state, data, &goal)
        );
        let mut event = GameEvent::new(EventKind::Heresy, text).province(&id);
        if let Some(c) = controller {
            event = event.faction(&c);
        }
        events.push(event);
    }
}
