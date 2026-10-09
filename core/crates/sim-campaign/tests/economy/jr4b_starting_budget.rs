//! Lot JR4b: the great realms start with garrisons they can pay
//! (`data/settlements/rules.json` § `starting_budget`).

use data_model::{FactionId, GameData};
use sim_campaign::CampaignState;

use data_model::test_support::{fac, game_data};

/// Seasonal balance without the idle hoard's share of the court, and the
/// receipts.
fn structural(state: &CampaignState, data: &GameData, faction: &FactionId) -> (i64, i64) {
    let e = state.faction_economy(data, faction).expect("economy");
    let rules = &data.economy_rules;
    let income = state.faction_income(data, faction);
    let opulence = (state.factions[faction].treasury - rules.opulence_seasons * income.max(0))
        .max(0)
        * rules.opulence_percent
        / 100;
    (
        e.net_income() + opulence,
        e.projected_income + e.trade_income,
    )
}

#[test]
fn the_great_realms_start_within_their_means_and_the_others_untouched() {
    let data = game_data();
    let rule = data
        .settlement_rules
        .as_ref()
        .and_then(|r| r.starting_budget.clone())
        .expect("starting_budget rule");
    let fitted = CampaignState::new_1337(data, fac("fac_france"), 1).expect("start");
    let mut raw_data = data.clone();
    raw_data
        .settlement_rules
        .as_mut()
        .expect("settlement rules")
        .starting_budget = None;
    raw_data.starting_fit = None;
    let raw = CampaignState::new_1337(&raw_data, fac("fac_france"), 1).expect("start");

    // The Mamluks were 21 % short; now within the allowed share.
    let mamluks = fac("fac_mamluks");
    let (net, receipts) = structural(&fitted, data, &mamluks);
    // A6-L3: the rule now asks for a surplus (negative max_deficit_percent);
    // the garrisons alone reach it only for some realms, none stays in deficit.
    assert!(net >= 0, "net {net} on {receipts}");
    let (raw_net, _) = structural(&raw, &raw_data, &mamluks);
    assert!(raw_net < net, "{raw_net} -> {net}");

    for (id, place) in &raw.settlements {
        let now = &fitted.settlements[id];
        // Never a settlement's last unit; the capital's garrison only as a
        // last resort, down to `min_capital_units` (LR-15).
        if !place.garrison.is_empty() {
            assert!(!now.garrison.is_empty(), "{id}");
        }
        // A6-L3: only a realm still in deficit loses its capital's garrison,
        // and then down to `min_capital_units` (LR-15) at most.
        let (capital_net, _) = structural(&raw, &raw_data, &place.controller);
        if fitted.faction_capital_city(&place.controller) == Some(id) {
            let floor = rule.min_capital_units.unwrap_or(usize::MAX);
            assert!(
                now.garrison == place.garrison || now.garrison.len() >= floor,
                "{id}"
            );
            if capital_net >= 0 {
                assert_eq!(now.garrison, place.garrison, "{id}");
            }
        }
        // Sound or small realms keep their garrisons.
        let faction = &place.controller;
        let small = raw.controlled_provinces(faction).count() < rule.min_provinces;
        let (net, receipts) = structural(&raw, &raw_data, faction);
        if small || net * 100 >= -rule.max_deficit_percent * receipts {
            assert_eq!(now.garrison, place.garrison, "{id} of {faction}");
        }
    }
}
