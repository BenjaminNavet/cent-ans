//! Battle construction from a custom battle composition (port of the `setup_custom` path of
//! `godot-bridge/src/custom_battle.rs`; the rules stay in `sim_battle::custom`).

use std::path::Path;

use data_model::{
    BattleAbility, BattleOrder, BattleStandardRules, FactionId, GameData, UnitCategory, UnitType,
    UnitTypeId,
};
use sim_battle::custom::{CustomBattle, CustomBattleRules, CustomData, CustomSide, roster};
use sim_battle::{BattleSim, FireRules, ReplayStart, SideId};

/// Default factions of the custom battle screen (`custom_battle_screen.gd` `DEFAULT_FACTIONS`).
pub const DEFAULT_ATTACKER: &str = "fac_france";
pub const DEFAULT_DEFENDER: &str = "fac_england";

/// Installs the siege fire rules of the data folder (done once per process, like
/// `install_battle_rules` in the bridge); built-in rules are kept when the file is unusable.
pub fn install_battle_rules(data_dir: &Path) {
    match FireRules::load(data_dir) {
        Ok(rules) => FireRules::install(Some(rules)),
        Err(error) => {
            bevy::log::warn!("siege fire rules: {error} (built-in rules kept)");
            FireRules::install(None);
        }
    }
}

/// An army bought from `faction`'s roster: units of the three field categories in turn
/// (cheapest of each first, cycling), mercenaries and siege engines left out, until the budget or
/// the unit cap is reached. The screen of the Godot client leaves this to the player.
pub fn compose_army(
    game: &GameData,
    faction: &str,
    budget: u32,
    max_units: usize,
    year: i32,
) -> CustomSide {
    let mut side = CustomSide {
        faction: faction.to_owned(),
        budget,
        units: Vec::new(),
        technologies: Vec::new(),
    };
    let Some(faction) = FactionId::new(faction)
        .ok()
        .and_then(|id| game.factions.get(&id))
    else {
        return side;
    };
    let techs = sim_battle::custom::known_technologies(faction, &[]);
    let available = roster(&game.unit_types, faction, year, &techs);
    let groups: Vec<Vec<&UnitType>> = [
        UnitCategory::Infantry,
        UnitCategory::Ranged,
        UnitCategory::Cavalry,
    ]
    .iter()
    .map(|category| {
        available
            .iter()
            .copied()
            .filter(|u| u.category == *category && !u.mercenary && u.soldiers > 0)
            .collect()
    })
    .collect();
    // Infantry twice as often as the other categories: a line of battle.
    let pattern = [0usize, 1, 0, 2];
    let mut cursor = [0usize; 3];
    let mut remaining = budget;
    let mut idle = 0;
    let mut step = 0;
    while side.units.len() < max_units && idle < pattern.len() {
        let group = &groups[pattern[step % pattern.len()]];
        step += 1;
        if group.is_empty() {
            idle += 1;
            continue;
        }
        let slot = &mut cursor[pattern[(step - 1) % pattern.len()]];
        let unit = group[*slot % group.len()];
        if unit.cost.money <= remaining {
            remaining -= unit.cost.money;
            side.units.push(unit.id.to_string());
            *slot += 1;
            idle = 0;
        } else if *slot % group.len() == 0 {
            idle += 1;
        } else {
            *slot = 0; // too dear: back to the cheapest of the category
        }
    }
    side
}

/// The battle shown when the screen opens with no pending setup: France against England, plains
/// in summer, each side with the default budget, the attacker commanded by the player.
pub fn default_custom_battle(game: &GameData) -> CustomBattle {
    let rules = CustomBattleRules::bundled();
    let year = rules.default_year;
    CustomBattle {
        attacker: compose_army(
            game,
            DEFAULT_ATTACKER,
            rules.default_budget,
            rules.max_units_per_side,
            year,
        ),
        defender: compose_army(
            game,
            DEFAULT_DEFENDER,
            rules.default_budget,
            rules.max_units_per_side,
            year,
        ),
        terrain: "plains".into(),
        season: "summer".into(),
        weather: String::new(),
        hour: String::new(),
        siege: false,
        fortification: None,
        player_side: "attacker".into(),
        year: Some(year),
        engines: None,
    }
}

/// Builds the simulation of `custom` with `seed` (same path as `setup_custom`), hour applied.
pub fn build_sim(
    game: &GameData,
    data_dir: &Path,
    custom: &CustomBattle,
    seed: u64,
) -> Result<BattleSim, String> {
    let (mut sim, _, hour) = build_start(game, data_dir, custom, seed)?;
    if let Some(hour) = hour {
        sim.set_start_hour(hour);
    }
    Ok(sim)
}

/// The simulation of `custom`, the start it was built from and the starting hour it asks for
/// (not applied: the replay recorder starts between the build and the hour).
pub fn build_start(
    game: &GameData,
    data_dir: &Path,
    custom: &CustomBattle,
    seed: u64,
) -> Result<(BattleSim, ReplayStart, Option<f64>), String> {
    let data = CustomData {
        unit_types: &game.unit_types,
        factions: &game.factions,
    };
    let orders: Vec<BattleOrder> = game.battle_orders.values().cloned().collect();
    let abilities: Vec<BattleAbility> = game.battle_abilities.values().cloned().collect();
    let standards = std::fs::read_to_string(data_dir.join("rules/battle_standards.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<BattleStandardRules>(&text).ok());
    let setup = custom.battle_setup(
        &data,
        CustomBattleRules::bundled(),
        orders,
        standards,
        abilities,
    )?;
    let start = ReplayStart::new(setup, seed).with_weather(custom.weather()?);
    let sim = start.build()?;
    Ok((sim, start, custom.start_hour()?))
}

/// Side commanded by the player in `custom` (`None`: AI against AI).
pub fn player_side(custom: &CustomBattle) -> Option<SideId> {
    SideId::parse(&custom.player_side)
}

/// `#RRGGBB` as linear-agnostic sRGB floats.
pub fn parse_hex_color(text: &str) -> Option<[f32; 3]> {
    let hex = text.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let channel = |i: usize| {
        u8::from_str_radix(&hex[i..i + 2], 16)
            .ok()
            .map(|v| f32::from(v) / 255.0)
    };
    Some([channel(0)?, channel(2)?, channel(4)?])
}

/// Colour of a side: its faction's primary heraldic colour, else blue / red.
pub fn side_color(game: &GameData, faction: &str, side: SideId) -> [f32; 3] {
    FactionId::new(faction)
        .ok()
        .and_then(|id| game.factions.get(&id))
        .and_then(|f| parse_hex_color(&f.heraldry.primary_color))
        .unwrap_or(match side {
            SideId::Attacker => [0.2, 0.3, 0.75],
            SideId::Defender => [0.75, 0.15, 0.12],
        })
}

/// Unit type lookup used by tests and the dump.
pub fn unit_type<'a>(game: &'a GameData, id: &str) -> Option<&'a UnitType> {
    UnitTypeId::new(id)
        .ok()
        .and_then(|id| game.unit_types.get(&id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths;

    fn load() -> (GameData, std::path::PathBuf) {
        let root = paths::find_root().expect("repo root");
        let dir = paths::data_dir(&root);
        let (game, _) = GameData::load(&dir).expect("game data");
        install_battle_rules(&dir);
        (game, dir)
    }

    #[test]
    fn default_battle_is_valid_and_steps() {
        let (game, dir) = load();
        let custom = default_custom_battle(&game);
        assert!(
            custom.attacker.units.len() > 5,
            "{:?}",
            custom.attacker.units
        );
        assert!(custom.defender.units.len() > 5);
        let mut sim = build_sim(&game, &dir, &custom, 42).expect("battle builds");
        assert!(!sim.units().is_empty());
        let before = sim.strength(SideId::Attacker);
        assert!(before > 0);
        for _ in 0..30 {
            sim.step();
        }
        assert_eq!(sim.ticks(), 30);
        let moved = sim
            .units()
            .iter()
            .any(|u| u.state != sim_battle::UnitState::Idle);
        assert!(moved, "after 3 s someone is marching");
        let poses: usize = sim
            .units()
            .iter()
            .map(|u| sim.soldier_poses(u, 1.0).len())
            .sum();
        assert!(poses > 500, "{poses}");
    }

    #[test]
    fn hex_colors_parse() {
        assert_eq!(parse_hex_color("#FF0000"), Some([1.0, 0.0, 0.0]));
        assert_eq!(parse_hex_color("red"), None);
    }
}
