//! Named rule values quoted by the interface (lot SV4).
//!
//! Help texts, tooltips and the encyclopedia explain the rules with numbers
//! ("+10 % per missed installment", "revolt above 75 for three seasons").
//! Copying those numbers into GDScript let them drift from the core; the UI
//! now reads them here, by name, through `GameDataStore.get_rule_constants`.
//! Every value comes from a core constant or from the loaded `data/rules`
//! files, never from a copy.
//!
//! Percentages are given in percent (8 for 8 %), not as fractions.

use std::collections::BTreeMap;

use data_model::GameData;

use crate::state::Season;
use crate::{
    agents, chronicle, diplomacy, economy, population, ransom, religion, research, retinue, table,
};

/// Every rule value the interface may quote, by name.
pub fn rule_constants(data: &GameData) -> BTreeMap<&'static str, f64> {
    let mut values = BTreeMap::new();
    let economy_rules = &data.economy_rules;
    let percent = |fraction: f64| (fraction * 100.0 * 1e6).round() / 1e6;

    // Economy and administration (`data/rules/economy.json`).
    values.insert(
        "administration_base_percent",
        percent(economy_rules.administration_base),
    );
    values.insert(
        "administration_per_province_percent",
        percent(economy_rules.administration_per_province),
    );
    values.insert(
        "administration_max_percent",
        percent(economy_rules.administration_max),
    );
    values.insert("opulence_seasons", economy_rules.opulence_seasons as f64);
    values.insert("opulence_percent", economy_rules.opulence_percent as f64);
    values.insert(
        "bankruptcy_morale_penalty",
        f64::from(economy_rules.bankruptcy_morale_penalty),
    );
    values.insert(
        "resource_import_multiplier",
        f64::from(economy_rules.resource_import_multiplier),
    );

    // Supply (`data/rules/economy.json`).
    values.insert("supply_loss", f64::from(economy_rules.supply_loss));
    values.insert(
        "supply_loss_winter",
        f64::from(economy_rules.supply_loss_winter),
    );
    values.insert("supply_recovery", f64::from(economy_rules.supply_recovery));
    values.insert(
        "starvation_loss_percent",
        f64::from(economy_rules.starvation_loss_percent),
    );
    values.insert(
        "supply_devastation_loss_percent",
        economy_rules.supply_devastation_loss_percent,
    );
    values.insert(
        "supply_devastation_recovery_cut_percent",
        economy_rules.supply_devastation_recovery_cut_percent,
    );
    values.insert(
        "devastation_decay",
        f64::from(economy_rules.devastation_decay),
    );

    // Public order and population (`data/rules/population.json`, `population.rs`).
    let population_rules = &data.population_rules;
    values.insert(
        "revolt_unrest_threshold",
        population_rules.revolt_unrest_threshold,
    );
    values.insert("revolt_seasons", f64::from(population_rules.revolt_seasons));
    values.insert(
        "revolt_control_threshold",
        population_rules.revolt_control_threshold,
    );
    values.insert("health_neutral", population::HEALTH_NEUTRAL);
    values.insert(
        "plague_health_threshold",
        f64::from(population::PLAGUE_HEALTH_THRESHOLD),
    );
    values.insert("goods_target_base", population::GOODS_TARGET_BASE);
    values.insert(
        "goods_target_per_category",
        population::GOODS_TARGET_PER_CATEGORY,
    );
    values.insert(
        "growth_devastation_cap",
        f64::from(population::GROWTH_DEVASTATION_CAP),
    );

    // Research.
    values.insert(
        "anachronism_surcharge_percent",
        f64::from(research::ANACHRONISM_SURCHARGE_PERCENT),
    );
    values.insert("anachronism_years", f64::from(research::ANACHRONISM_YEARS));

    // Ransoms.
    values.insert(
        "ransom_installment_surcharge_percent",
        ransom::INSTALLMENT_SURCHARGE_PERCENT as f64,
    );
    values.insert(
        "ransom_default_surcharge_percent",
        ransom::DEFAULT_SURCHARGE_PERCENT as f64,
    );

    // Religion and diplomacy.
    values.insert(
        "donation_livres_per_favor",
        religion::DONATION_LIVRES_PER_FAVOR as f64,
    );
    values.insert("mediation_cost", diplomacy::MEDIATION_COST as f64);

    // Table (diets, H9).
    values.insert("lent_piety_penalty", f64::from(table::LENT_PIETY_PENALTY));
    values.insert("lent_fish_piety", f64::from(table::LENT_FISH_PIETY));
    values.insert("lent_clergy_unrest", f64::from(table::LENT_CLERGY_UNREST));
    values.insert(
        "winter_fresh_cost_factor",
        table::WINTER_FRESH_COST_PERCENT / 100.0,
    );

    // Movement (`data/movement/rules.json`, `data/settlements/rules.json`).
    let movement = data.movement_rules();
    let season_km = |season: Season| {
        (f64::from(season.movement_steps()) * movement.points_per_step * movement.season_scale)
            .round()
    };
    values.insert("march_km_season", season_km(Season::Summer));
    values.insert("march_km_winter", season_km(Season::Winter));
    values.insert("zoc_radius_km", data.free_movement_rules().zoc_radius_km);

    // Characters and agents.
    values.insert(
        "retinue_max_per_character",
        retinue::max_per_character(data) as f64,
    );
    let agent_rules = agents::rules(data);
    values.insert("agent_xp_success", f64::from(agent_rules.xp_success));
    values.insert("agent_xp_failure", f64::from(agent_rules.xp_failure));

    // Chronicle decisions.
    values.insert("decision_turns", f64::from(chronicle::DECISION_TURNS));

    // Tax brackets (spec § 1.4).
    values.insert("tax_multiplier_low", economy::TaxRate::Low.multiplier());
    values.insert(
        "tax_multiplier_normal",
        economy::TaxRate::Normal.multiplier(),
    );
    values.insert("tax_multiplier_high", economy::TaxRate::High.multiplier());

    // Economy constants still in code.
    values.insert(
        "upkeep_months_per_season",
        economy::UPKEEP_MONTHS_PER_SEASON as f64,
    );
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_core_and_data_values() {
        let data = GameData::default();
        let values = rule_constants(&data);
        assert_eq!(values["supply_loss_winter"], 35.0);
        assert_eq!(values["administration_base_percent"], 8.0);
        assert_eq!(values["ransom_default_surcharge_percent"], 10.0);
        assert_eq!(values["winter_fresh_cost_factor"], 1.5);
        assert_eq!(values["donation_livres_per_favor"], 200.0);
    }
}
