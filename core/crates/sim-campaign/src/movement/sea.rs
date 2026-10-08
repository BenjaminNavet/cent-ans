//! Landing on a hostile shore.

use crate::events::{EventKind, GameEvent};
use crate::state::{ArmyId, CampaignState};
use data_model::{GameData, ProvinceId};

/// A landing on a hostile shore (M10 balance): the army is spent for the turn
/// and pays in men and morale for the disembarkation.
pub(crate) fn land_on_hostile_shore(
    state: &mut CampaignState,
    data: &GameData,
    army_id: &ArmyId,
    province: Option<&ProvinceId>,
    events: &mut Vec<GameEvent>,
) {
    let landing = &data.army_rules.landing;
    let percent = if state.season == crate::state::Season::Winter {
        landing.winter_factor * landing.loss_percent
    } else {
        landing.loss_percent
    };
    let name = state.army_name(data, army_id);
    let Some(army) = state.armies.get_mut(army_id) else {
        return;
    };
    army.movement_left = 0;
    let mut lost = 0;
    for unit in &mut army.units {
        let casualties = (unit.strength * percent)
            .div_ceil(100)
            .min(unit.strength.saturating_sub(1));
        unit.strength -= casualties;
        unit.morale = unit.morale.saturating_sub(landing.morale_loss);
        lost += casualties;
    }
    let faction = army.faction.clone();
    let mut event = GameEvent::new(
        EventKind::Attrition,
        format!("Débarquement en terre hostile : {name} perd {lost} hommes."),
    )
    .army(army_id)
    .faction(&faction);
    if let Some(province) = province {
        event = event.province(province);
    }
    events.push(event);
}
