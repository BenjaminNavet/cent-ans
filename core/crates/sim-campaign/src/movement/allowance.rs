//! Movement points an army receives at the start of a turn.

use super::points_per_step;
use crate::research;
use crate::skills;
use crate::state::{Army, CampaignState};
use data_model::EffectKind;
use data_model::GameData;

impl CampaignState {
    /// Movement points `army` receives at the start of a turn (F1, lot C4):
    /// the season's steps scaled by the pace of its slowest unit family
    /// (technologies' `Movement` percents, global or per family; siege
    /// trains `ArmyRules::siege_train_pace_percent`), plus the flat `Movement` steps
    /// of its general (admiral, chevauchée) and of the faction's
    /// technologies, the whole times `MovementRules::points_per_step`.
    pub fn army_movement_allowance(&self, data: &GameData, army: &Army) -> u32 {
        let base = f64::from(self.season.movement_steps());
        let tech = research::faction_tech_effects(self, data, &army.faction);
        let general = army
            .general
            .as_ref()
            .map(|id| skills::character_effects(self, data, id))
            .unwrap_or_default();
        let common = tech[EffectKind::Movement].percent + general[EffectKind::Movement].percent;
        let pace = army
            .units
            .iter()
            .filter_map(|unit| data.unit_types.get(&unit.unit_type))
            .map(|unit_type| {
                let mut percent = common
                    + tech
                        .unit_categories
                        .get(unit_type.category)
                        .movement
                        .percent;
                if unit_type.category == data_model::UnitCategory::Siege {
                    percent += data.army_rules.siege_train_pace_percent;
                }
                percent
            })
            .fold(common, f64::min);
        let steps = (base * (1.0 + pace / 100.0) + 1e-9).floor()
            + tech[EffectKind::Movement].flat
            + general[EffectKind::Movement].flat;
        // TW2-T5: march traditions of the army, on the points themselves (a
        // few percent of three or four steps would be floored away).
        let traditions =
            f64::from(crate::traditions::army_tradition_effects(data, army).movement_percent);
        // Lot C7a: the season covers `season_scale` of the v1 steps.
        (steps.max(1.0)
            * points_per_step(data)
            * data.movement_rules().season_scale
            * (1.0 + traditions / 100.0))
            .round() as u32
    }

    /// Movement points of a fresh army at full pace this season (no
    /// technology, general or siege train): the season's steps times
    /// `points_per_step` and `season_scale` (lot C7a).
    pub fn season_movement_points(&self, data: &GameData) -> u32 {
        let rules = data.movement_rules();
        (f64::from(self.season.movement_steps()) * rules.points_per_step * rules.season_scale)
            .round() as u32
    }
}
