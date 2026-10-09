//! JR4b fit of the 1337 garrisons, precomputed (ADR 0233).
//!
//! A great realm whose starting forces outrun its receipts sends home its
//! costliest units and buildings (lot JR4b, LR-15, A6-L3). The result no
//! longer depends on the data at start: `data/rules/starting_fit.json`
//! lists what is removed and [`apply_starting_fit`] applies it. The
//! computation lives in the tests, which check the file equals it and
//! regenerate it with `CENT_ANS_REGEN_STARTING_FIT=1 cargo test -p
//! sim-campaign starting_fit`.

use data_model::GameData;

use crate::state::CampaignState;

/// Removes the positions `indices` (ascending) of `list`.
fn remove_positions<T>(list: &mut Vec<T>, indices: &[usize]) {
    for index in indices.iter().rev() {
        if *index < list.len() {
            list.remove(*index);
        }
    }
}

/// Applies the precomputed fit of `data` to a freshly built start.
pub(crate) fn apply_starting_fit(state: &mut CampaignState, data: &GameData) {
    let Some(fit) = data.starting_fit.as_ref() else {
        return;
    };
    for (faction, treasury) in &fit.treasuries {
        if let Some(f) = state.factions.get_mut(faction) {
            f.treasury = f.treasury.min(*treasury);
        }
    }
    for (id, indices) in &fit.garrisons {
        if let Some(place) = state.settlements.get_mut(id) {
            remove_positions(&mut place.garrison, indices);
        }
    }
    for (id, indices) in &fit.armies {
        if let Some(army) = state.armies.iter_mut().find(|(k, _)| k.to_string() == *id) {
            remove_positions(&mut army.1.units, indices);
        }
    }
    for (id, indices) in &fit.buildings {
        if let Some(place) = state.settlements.get_mut(id) {
            remove_positions(&mut place.buildings, indices);
        }
    }
}

#[cfg(test)]
pub(crate) mod compute {
    use data_model::{FactionId, GameData};

    use crate::state::CampaignState;

    /// Balance of the season `faction` would pay from the start, without the
    /// idle hoard's share of the court (it melts with the treasury), and its
    /// receipts.
    pub(crate) fn structural_balance(
        state: &CampaignState,
        data: &GameData,
        faction: &FactionId,
    ) -> (i64, i64) {
        let Some(economy) = state.faction_economy(data, faction) else {
            return (0, 0);
        };
        let rules = &data.economy_rules;
        let income = state.faction_income(data, faction);
        let treasury = state.factions.get(faction).map_or(0, |f| f.treasury);
        let opulence = (treasury - rules.opulence_seasons * income.max(0)).max(0)
            * rules.opulence_percent
            / 100;
        (
            economy.net_income() + opulence,
            economy.projected_income + economy.trade_income,
        )
    }

    /// Lot JR4b (`settlement_rules.starting_budget`): a great realm whose
    /// starting forces outrun its receipts sends home its costliest garrison
    /// units, one at a time, until the deficit is within the allowed share —
    /// never a settlement's last unit nor the capital's garrison; then (LR-15)
    /// field units down to `min_field_units`, and last the capital's garrison
    /// down to `min_capital_units`. (The Mamluks paid 4 000 livres of upkeep on
    /// 4 000 of receipts and their AI dismissed its whole field army by the
    /// eighth season.)
    pub(crate) fn fit_starting_garrisons(state: &mut CampaignState, data: &GameData) {
        let Some(rule) = data
            .settlement_rules
            .as_ref()
            .and_then(|r| r.starting_budget.clone())
        else {
            return;
        };
        // JR5: the reference balance ignores the difficulty (whatever level the
        // campaign is or will be set to): measured at the neutral level, where
        // neither the AI nor the player gets any favour.
        let level = state.difficulty;
        state.difficulty = crate::difficulty::Difficulty::Normal;
        let factions: Vec<FactionId> = state.factions.keys().cloned().collect();
        for faction in factions {
            if state.controlled_provinces(&faction).count() < rule.min_provinces
                || crate::crusade::starting_army(state, data, &faction).is_some()
            {
                continue;
            }
            // A6-L3 (ADR 0183): the treasury is capped at a few seasons of income.
            if let Some(seasons) = rule.treasury_max_income_seasons {
                let (_, receipts) = structural_balance(state, data, &faction);
                if let Some(f) = state.factions.get_mut(&faction) {
                    f.treasury = f.treasury.min(seasons * receipts.max(0));
                }
            }
            let capital = state.faction_capital_city(&faction).cloned();
            let is_capital = |id: &data_model::SettlementId| Some(id) == capital.as_ref();
            // 1. Garrisons other than the capital's, down to one unit each.
            send_home_garrisons(state, data, &faction, &rule, false, |id| {
                if is_capital(id) {
                    usize::MAX
                } else {
                    1
                }
            });
            // 2. LR-15: the starting field armies.
            if let Some(min_units) = rule.min_field_units {
                fit_starting_field_armies(state, data, &faction, min_units);
            }
            // 3. LR-15: the capital's garrison, as a last resort.
            if let Some(min_units) = rule.min_capital_units {
                send_home_garrisons(state, data, &faction, &rule, true, |id| {
                    if is_capital(id) {
                        min_units
                    } else {
                        usize::MAX
                    }
                });
            }
            // 4. A6-L3: then the dearest building left unpaid for (a realm whose
            // upkeep alone outruns its receipts).
            while in_deficit(state, data, &faction) {
                let dearest = state
                    .settlements
                    .iter()
                    .filter(|(_, s)| s.controller == faction)
                    .flat_map(|(id, s)| {
                        let percent = crate::economy::building_upkeep_percent(data, s.kind);
                        s.buildings.iter().enumerate().map(move |(index, b)| {
                            let upkeep = data
                                .buildings
                                .get(b)
                                .map_or(0, |t| i64::from(t.upkeep.unwrap_or(0)));
                            (upkeep * percent, id.clone(), index)
                        })
                    })
                    .filter(|(paid, _, _)| *paid > 0)
                    .max_by(|a, b| {
                        a.0.cmp(&b.0)
                            .then_with(|| b.1.cmp(&a.1))
                            .then_with(|| b.2.cmp(&a.2))
                    });
                let Some((_, id, index)) = dearest else {
                    break;
                };
                if let Some(place) = state.settlements.get_mut(&id) {
                    place.buildings.remove(index);
                }
            }
        }
        state.difficulty = level;
    }

    /// `true` while `faction` is beyond the deficit `rule` allows (A6-L3: the
    /// share may be negative, a surplus is then asked).
    pub(crate) fn over_budget(
        state: &CampaignState,
        data: &GameData,
        faction: &FactionId,
        rule: &data_model::StartingBudget,
    ) -> bool {
        let (net, receipts) = structural_balance(state, data, faction);
        net * 100 < -rule.max_deficit_percent * receipts.max(0)
    }

    /// `true` while `faction` is still in deficit. The fallbacks below (field
    /// armies, the capital's garrison, buildings) only bring a deficit to zero:
    /// the surplus asked of the others stops at their garrisons.
    pub(crate) fn in_deficit(state: &CampaignState, data: &GameData, faction: &FactionId) -> bool {
        structural_balance(state, data, faction).0 < 0
    }

    /// Sends home the costliest garrison units of `faction`, one at a time,
    /// while it is over budget; a place keeps at least `keep(place)` units.
    fn send_home_garrisons(
        state: &mut CampaignState,
        data: &GameData,
        faction: &FactionId,
        rule: &data_model::StartingBudget,
        fallback: bool,
        keep: impl Fn(&data_model::SettlementId) -> usize,
    ) {
        while if fallback {
            in_deficit(state, data, faction)
        } else {
            over_budget(state, data, faction, rule)
        } {
            let costliest = state
                .settlements
                .iter()
                .filter(|(id, s)| &s.controller == faction && s.garrison.len() > keep(id))
                .flat_map(|(id, s)| {
                    let percent = crate::economy::garrison_upkeep_percent(data, s.kind);
                    s.garrison.iter().enumerate().map(move |(index, unit)| {
                        (
                            crate::economy::unit_upkeep(data, unit) * percent,
                            id.clone(),
                            index,
                        )
                    })
                })
                .filter(|(paid, _, _)| *paid > 0)
                .max_by(|a, b| {
                    a.0.cmp(&b.0)
                        .then_with(|| b.1.cmp(&a.1))
                        .then_with(|| b.2.cmp(&a.2))
                });
            let Some((_, settlement, index)) = costliest else {
                break;
            };
            if let Some(place) = state.settlements.get_mut(&settlement) {
                place.garrison.remove(index);
            }
        }
    }

    /// Lot LR-15: the second step of [`fit_starting_garrisons`] — with its
    /// garrisons at the floor, a realm still beyond the allowed deficit sends
    /// home the costliest units of its starting field armies, keeping at least
    /// `min_units` of them and never emptying an army. (Serbia and Lithuania
    /// paid 1 080 livres for the three coded units on ~1 800 of receipts; their
    /// AI dismissed them all within five seasons.)
    fn fit_starting_field_armies(
        state: &mut CampaignState,
        data: &GameData,
        faction: &FactionId,
        min_units: usize,
    ) {
        while in_deficit(state, data, faction) {
            let units: usize = state
                .armies
                .values()
                .filter(|a| &a.faction == faction)
                .map(|a| a.units.len())
                .sum();
            if units <= min_units {
                break;
            }
            let costliest = state
                .armies
                .iter()
                .filter(|(_, a)| &a.faction == faction && a.units.len() > 1)
                .flat_map(|(id, a)| {
                    a.units.iter().enumerate().map(move |(index, unit)| {
                        (crate::economy::unit_upkeep(data, unit), id.clone(), index)
                    })
                })
                .filter(|(paid, _, _)| *paid > 0)
                .max_by(|a, b| {
                    a.0.cmp(&b.0)
                        .then_with(|| b.1.cmp(&a.1))
                        .then_with(|| b.2.cmp(&a.2))
                });
            let Some((_, army, index)) = costliest else {
                break;
            };
            if let Some(army) = state.armies.get_mut(&army) {
                army.units.remove(index);
            }
        }
    }

    /// Positions of `before` that `after` (a subsequence of it) lacks.
    fn removed_positions<T>(
        before: &[T],
        after: &[T],
        same: impl Fn(&T, &T) -> bool,
    ) -> Vec<usize> {
        let mut kept = after.iter().peekable();
        let mut removed = Vec::new();
        for (index, item) in before.iter().enumerate() {
            if kept.peek().is_some_and(|next| same(item, next)) {
                kept.next();
            } else {
                removed.push(index);
            }
        }
        assert!(kept.next().is_none(), "the fit only removes");
        removed
    }

    /// The fit of `data`, computed by running the JR4b algorithm.
    pub(crate) fn compute_starting_fit(
        data: &GameData,
        player: &FactionId,
    ) -> data_model::StartingFit {
        let before = CampaignState::new_1337_unfitted(data, player.clone(), 1).unwrap();
        let mut after = before.clone();
        fit_starting_garrisons(&mut after, data);
        let mut fit = data_model::StartingFit::default();
        for (id, f) in &after.factions {
            if f.treasury != before.factions[id].treasury {
                fit.treasuries.insert(id.clone(), f.treasury);
            }
        }
        for (id, s) in &after.settlements {
            let old = &before.settlements[id];
            let units = removed_positions(&old.garrison, &s.garrison, |a, b| a == b);
            if !units.is_empty() {
                fit.garrisons.insert(id.clone(), units);
            }
            let buildings = removed_positions(&old.buildings, &s.buildings, |a, b| a == b);
            if !buildings.is_empty() {
                fit.buildings.insert(id.clone(), buildings);
            }
        }
        for (id, a) in &after.armies {
            let units = removed_positions(&before.armies[id].units, &a.units, |x, y| x == y);
            if !units.is_empty() {
                fit.armies.insert(id.to_string(), units);
            }
        }
        fit
    }
}

#[cfg(test)]
mod tests {
    use data_model::test_support::game_data;
    use data_model::FactionId;

    use super::compute::compute_starting_fit;
    use super::*;

    const FILE: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../data/rules/starting_fit.json"
    );

    #[test]
    fn the_starting_fit_data_equals_the_computation() {
        let player = FactionId::new("fac_france").unwrap();
        let mut computed = compute_starting_fit(game_data(), &player);
        computed.description = game_data()
            .starting_fit
            .as_ref()
            .and_then(|f| f.description.clone());
        if std::env::var("CENT_ANS_REGEN_STARTING_FIT").is_ok() {
            if computed.description.is_none() {
                computed.description = Some(
                    "Ajustement JR4b des garnisons de 1337, précalculé (ADR 0233) : unités, \
                     bâtiments et trésoreries que le budget de départ retire. Régénéré par \
                     CENT_ANS_REGEN_STARTING_FIT=1 cargo test -p sim-campaign starting_fit ; \
                     un test vérifie que ce fichier égale le calcul."
                        .into(),
                );
            }
            let text = serde_json::to_string_pretty(&computed).unwrap();
            std::fs::write(FILE, text + "\n").unwrap();
            return;
        }
        assert_eq!(
            game_data().starting_fit.as_ref(),
            Some(&computed),
            "data/rules/starting_fit.json is stale; regenerate it (see module docs)"
        );
    }

    #[test]
    fn the_fit_is_applied_to_the_start() {
        let data = game_data();
        let player = FactionId::new("fac_france").unwrap();
        let mut expected = CampaignState::new_1337_unfitted(data, player.clone(), 1).unwrap();
        compute::fit_starting_garrisons(&mut expected, data);
        let mut applied = CampaignState::new_1337_unfitted(data, player, 1).unwrap();
        apply_starting_fit(&mut applied, data);
        assert_eq!(
            serde_json::to_string(&applied).unwrap(),
            serde_json::to_string(&expected).unwrap()
        );
    }
}
