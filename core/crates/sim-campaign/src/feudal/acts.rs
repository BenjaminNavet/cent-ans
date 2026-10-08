//! Deliberate acts of a vassal (lot FE5, spec § 4.2): revolt and change of
//! allegiance, issued as orders by the feudal AI (`crates/ai`).

use data_model::{FactionId, GameData};

use super::{liege_of, primary_rank, FeudalError};
use crate::events::{EventKind, GameEvent};
use crate::state::CampaignState;

fn alive(state: &CampaignState, faction: &FactionId) -> bool {
    state.factions.get(faction).is_some_and(|f| f.alive)
}

/// `vassal` revolts against its direct suzerain: a felony case opens (the
/// suzerain may declare forfeiture), the tie is cut and war begins.
pub fn revolt(
    state: &mut CampaignState,
    data: &GameData,
    vassal: &FactionId,
) -> Result<FactionId, FeudalError> {
    if !alive(state, vassal) {
        return Err(FeudalError::DeadFaction(vassal.clone()));
    }
    let liege = liege_of(state, data, vassal)
        .ok_or_else(|| FeudalError::War("aucun suzerain contre qui se révolter".to_owned()))?;
    // The case first: once the tie is cut, `vassal` no longer holds of it.
    super::on_revolt(state, data, vassal, &liege);
    state.cut_vassal_tie(vassal, &liege);
    if !state.is_at_war(vassal, &liege) {
        state.start_war(vassal, &liege);
    }
    state.push_order_event(
        GameEvent::new(
            EventKind::VassalRebellion,
            format!(
                "{} se révolte contre son suzerain {} et proclame son indépendance.",
                data.faction_name(vassal),
                data.faction_name(&liege)
            ),
        )
        .faction(vassal),
    );
    Ok(liege)
}

/// `vassal` pays homage to `lord` (spec § 4.2: a lord of higher rank than
/// its own). A vassal leaving its suzerain commits felony (revolt) towards
/// it; a sovereign faction simply seeks a protector.
pub fn switch_allegiance(
    state: &mut CampaignState,
    data: &GameData,
    vassal: &FactionId,
    lord: &FactionId,
) -> Result<(), FeudalError> {
    if !alive(state, vassal) {
        return Err(FeudalError::DeadFaction(vassal.clone()));
    }
    if !alive(state, lord) || lord == vassal || lord == &state.player_faction {
        return Err(FeudalError::DeadFaction(lord.clone()));
    }
    if state.is_at_war(vassal, lord) {
        return Err(FeudalError::War(
            "on ne prête pas hommage à un ennemi".to_owned(),
        ));
    }
    let (Some(own), Some(above)) = (
        primary_rank(state, data, vassal),
        primary_rank(state, data, lord),
    ) else {
        return Err(FeudalError::NotVassal(vassal.clone(), lord.clone()));
    };
    if above <= own {
        return Err(FeudalError::NotVassal(vassal.clone(), lord.clone()));
    }
    let former = liege_of(state, data, vassal);
    if former.as_ref() == Some(lord) {
        return Ok(());
    }
    if let Some(former) = &former {
        super::on_revolt(state, data, vassal, former);
        state.cut_vassal_tie(vassal, former);
    }
    if !super::pay_homage(state, data, vassal, lord) {
        return Err(FeudalError::NotVassal(vassal.clone(), lord.clone()));
    }
    let homage_start = data.feudal_rules.loyalty.homage_start;
    if let Some(v) = state.factions.get_mut(vassal) {
        v.loyalty = homage_start;
    }
    // ADR 0114: the feudal tie stands for an alliance.
    super::drop_alliance(state, vassal, lord);
    let text = match &former {
        Some(former) => format!(
            "{} renie {} et prête hommage à {}.",
            data.faction_name(vassal),
            data.faction_name(former),
            data.faction_name(lord)
        ),
        None => format!(
            "{} se place sous la protection de {} et lui prête hommage.",
            data.faction_name(vassal),
            data.faction_name(lord)
        ),
    };
    state.push_order_event(GameEvent::new(EventKind::Vassalage, text).faction(vassal));
    Ok(())
}
