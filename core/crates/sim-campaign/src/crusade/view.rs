//! The fervour panel.

use super::*;

/// The fervour panel of `faction` (`None` unless it is the crusaders).
pub fn crusade_view(
    state: &CampaignState,
    data: &GameData,
    faction: &FactionId,
) -> Option<CrusadeView> {
    let rules = active(state, data).filter(|rules| &rules.faction == faction)?;
    let crusade = state.crusade.as_ref()?;
    let blocker = preach_blocker(state, data, faction);
    // A contingent is sized after the call has lifted the fervour.
    let after_call = (i32::from(crusade.fervor) + rules.fervor.preach).clamp(0, 100) as u8;
    Some(CrusadeView {
        fervor: crusade.fervor,
        floor: floor(rules, crusade),
        alms: alms_at(rules, crusade.fervor),
        alms_last_turn: crusade.alms_last_turn,
        changes: crusade
            .last_changes
            .iter()
            .map(|(cause, delta)| FervorChange {
                cause: cause.clone(),
                delta: *delta,
            })
            .collect(),
        zeal_morale: zeal_at(rules, crusade.fervor),
        desertion_percent: rules.desertion.percent_at(crusade.fervor),
        zeal_high_threshold: rules.zeal.high_threshold,
        zeal_low_threshold: rules.zeal.low_threshold,
        zeal_high_morale: rules.zeal.high_morale,
        zeal_low_morale: rules.zeal.low_morale,
        desertion_threshold: rules.desertion.threshold,
        desertion_men_percent: rules.desertion.men_percent_per_turn,
        target_taken: crusade.target_taken,
        target_name: target_name(state, data, rules),
        passage_cost: passage_cost(state, data, faction),
        passage_cooldown: crusade.preach_cooldown,
        passage_available: blocker.is_none(),
        passage_blocker: blocker.map(|b| b.to_string()).unwrap_or_default(),
        passage_units: rules.passage.units_at(after_call),
        passage_delay: rules.passage.delay_turns,
        pending: crusade
            .pending_passages
            .iter()
            .map(|p| PendingPassageView {
                turns_left: p.arrival_turn.saturating_sub(state.turn),
                port: p.port.clone(),
                port_name: data.settlement_name(&p.port),
                units: p.units,
            })
            .collect(),
    })
}
