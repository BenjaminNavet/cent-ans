//! Scripted historical battles (port of `BattleSim.setup_historical` of the Godot bridge,
//! `godot-bridge/src/historical_battles.rs`): orders of battle regiment by regiment, the real
//! relief and decor of the site, the weather, the hour, the waves and the posts of the scenario
//! come from `data/battle_maps/<id>.json` through [`sim_battle::HistoricalMap::start`].

use std::path::Path;

use data_model::{BattleAbility, BattleOrder, GameData};
use sim_battle::{BattleSim, HistoricalMap, SideId};

/// Parses `data/battle_maps/<id>.json`.
pub fn load_map(data_dir: &Path, id: &str) -> Result<HistoricalMap, String> {
    let path = data_dir.join("battle_maps").join(format!("{id}.json"));
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    HistoricalMap::from_json(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Builds the battle of map `id`; `side` is `attacker`, `defender` or anything else (AI against
/// AI, nobody commanding).
pub fn build_sim(
    game: &GameData,
    data_dir: &Path,
    id: &str,
    side: &str,
    seed: u64,
) -> Result<(BattleSim, HistoricalMap), String> {
    super::super::scene::setup::install_battle_rules(data_dir);
    let map = load_map(data_dir, id)?;
    let orders: Vec<BattleOrder> = game.battle_orders.values().cloned().collect();
    let abilities: Vec<BattleAbility> = game.battle_abilities.values().cloned().collect();
    let standards = std::fs::read_to_string(data_dir.join("rules/battle_standards.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok());
    let mut setup = map.battle_setup(&game.unit_types, orders, standards, SideId::parse(side))?;
    setup.abilities = abilities;
    let sim = map.start(setup, seed)?;
    Ok((sim, map))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths;

    #[test]
    fn crecy_builds_with_its_orders_of_battle() {
        let root = paths::find_root().expect("repo root");
        let dir = paths::data_dir(&root);
        let (game, _) = GameData::load(&dir).expect("game data");
        let (mut sim, map) = build_sim(&game, &dir, "crecy", "defender", 7).expect("crecy builds");
        assert_eq!(map.id, "crecy");
        assert!(sim.units().len() > 10);
        assert_eq!(sim.setup().player_side, Some(SideId::Defender));
        assert!(sim.field().width > 1000.0);
        for _ in 0..20 {
            sim.step();
        }
        assert_eq!(sim.ticks(), 20);
    }
}
