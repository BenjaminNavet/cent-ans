//! Fixtures partagées par les tests d'intégration de `sim-campaign` et de `ai`
//! (feature `test-support`) : campagne de départ, planificateur à vide,
//! armée principale, identifiants courts.

use data_model::{BuildingId, FactionId, GameData, ProvinceId, SettlementId, UnitTypeId};

use crate::{ArmyId, CampaignState, Order};

/// Campagne de 1337 jouée par `faction`, graine `seed`.
pub fn start(data: &GameData, faction: &str, seed: u64) -> CampaignState {
    CampaignState::new_1337(data, FactionId::new(faction).unwrap(), seed).expect("1337 start")
}

/// Comme [`start`], sans événements de chronique (pas de hasard parasite).
pub fn start_quiet(data: &GameData, faction: &str, seed: u64) -> CampaignState {
    let mut state = start(data, faction, seed);
    state.chronicle.disabled = true;
    state
}

/// Planificateur qui ne fait rien : seuls les ordres soumis par le test s'appliquent.
pub fn idle(_: &CampaignState, _: &GameData, _: &FactionId) -> Vec<Order> {
    Vec::new()
}

/// Armée la plus nombreuse de `faction` (départage par identifiant).
pub fn main_army(state: &CampaignState, faction: &str) -> ArmyId {
    let faction = FactionId::new(faction).unwrap();
    state
        .armies()
        .iter()
        .filter(|(_, army)| army.faction == faction)
        .max_by_key(|(id, army)| (army.units.len(), std::cmp::Reverse((*id).clone())))
        .map(|(id, _)| id.clone())
        .expect("faction has an army")
}

/// Première armée (ordre des identifiants) de `faction`.
pub fn first_army(state: &CampaignState, faction: &str) -> ArmyId {
    let faction = FactionId::new(faction).unwrap();
    state
        .armies()
        .iter()
        .find(|(_, army)| army.faction == faction)
        .map(|(id, _)| id.clone())
        .expect("faction has an army")
}

/// Ville de la province `province`.
pub fn city(state: &CampaignState, province: &str) -> SettlementId {
    state
        .province_city_id(&ProvinceId::new(province).unwrap())
        .expect("province has a city")
        .clone()
}

/// Ville de la capitale de `faction`.
pub fn capital_city(state: &CampaignState, faction: &str) -> SettlementId {
    let capital = state.factions[&FactionId::new(faction).unwrap()]
        .capital
        .clone();
    state
        .province_city_id(&capital)
        .cloned()
        .expect("capital city")
}

/// Identifiant de bâtiment.
pub fn bld(id: &str) -> BuildingId {
    BuildingId::new(id).unwrap()
}

/// Identifiant de type d'unité.
pub fn unit_type(id: &str) -> UnitTypeId {
    UnitTypeId::new(id).unwrap()
}
