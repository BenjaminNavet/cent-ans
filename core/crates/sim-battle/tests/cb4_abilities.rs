//! CB4: active abilities of the regiments (plan
//! `docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` § CB4,
//! `docs/research/cb4-capacites.md`): catalogue, cooldown, conditions, end
//! with its reason, each effect, replay determinism, AI.

mod common;

use common::*;
use data_model::AbilityKind;

#[test]
fn the_catalogue_has_the_five_abilities_of_the_historian() {
    let data = data();
    let mut kinds: Vec<AbilityKind> = data.battle_abilities.values().map(|a| a.kind).collect();
    kinds.sort_by_key(|k| format!("{k:?}"));
    assert_eq!(
        kinds,
        vec![
            AbilityKind::AimedShot,
            AbilityKind::BannerRally,
            AbilityKind::CloseRanks,
            AbilityKind::Pavise,
            AbilityKind::PlantedPikes,
        ]
    );
    assert!(!data.battle_orders.contains_key("order_pavise"));
}
