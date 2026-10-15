//! Building the 3D battle of a pending campaign battle.
//!
//! Godot's `battle_scene.gd` `begin()` asks the bridge for `get_battle_setup`, builds a
//! `BattleSim` from it with the campaign seed and sets the starting hour drawn by the campaign
//! (EP8). Here `game_api::flow::battle_launch` gives the same package and the battle scene
//! (`battle::scene`) reads [`CampaignBattle`] when it opens.
//!
//! EP7: in the province and years of a historical map, a field battle is fought on the real site
//! (`HistoricalMap::apply_site`): [`campaign_site`] picks the map, [`start_of`] builds the
//! [`ReplayStart`] so the live battle and its replay start alike.

use std::path::Path;

use bevy::prelude::*;
use game_api::flow::BattleLaunch;
use sim_battle::{BattleSetup, BattleSim, HistoricalMap, ReplayStart};

use crate::battle::scene::setup::install_battle_rules;
use crate::paths;

/// Set by flow right before entering `AppState::Battle`; consumed (kept until the battle ends)
/// by the battle scene, which builds the simulation from the core's `BattleSetup`.
#[derive(Resource, Clone)]
pub struct CampaignBattle {
    pub launch: BattleLaunch,
    /// The historical site the battle is fought on (EP7), if the province and year match one.
    pub site: Option<HistoricalMap>,
}

impl CampaignBattle {
    /// A campaign battle on no historical site.
    pub fn plain(launch: BattleLaunch) -> Self {
        CampaignBattle { launch, site: None }
    }
}

/// Every historical map of `data_dir` (`data/battle_maps/*.json`, the decor example excluded),
/// sorted by year.
pub fn maps_in(data_dir: &Path) -> Vec<HistoricalMap> {
    let Ok(entries) = std::fs::read_dir(data_dir.join("battle_maps")) else {
        return Vec::new();
    };
    let mut paths: Vec<_> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    paths.sort();
    let mut maps: Vec<HistoricalMap> = paths
        .into_iter()
        .filter(|p| {
            p.extension().is_some_and(|e| e == "json")
                && !p
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("decor_plan"))
        })
        .filter_map(|p| {
            let text = std::fs::read_to_string(&p).ok()?;
            match HistoricalMap::from_json(&text) {
                Ok(map) => Some(map),
                Err(error) => {
                    warn!("battle map {}: {error}", p.display());
                    None
                }
            }
        })
        .collect();
    maps.sort_by_key(|m| m.year);
    maps
}

/// The map a campaign battle in `setup.province` in `year` is fought on, if any (never a siege).
pub fn campaign_site(data_dir: &Path, setup: &BattleSetup, year: i32) -> Option<HistoricalMap> {
    if setup.siege.is_some() {
        return None;
    }
    maps_in(data_dir)
        .into_iter()
        .find(|map| map.matches_campaign(&setup.province, year))
}

/// The province whose real-relief horizon tile the battle uses: the battle's province when the
/// horizon index (`game/assets/horizon/relief/index.json`) has a tile for it, else `None` (the
/// decor then picks a tile by terrain and seed).
pub fn horizon_province(province: &str) -> Option<String> {
    let root = paths::find_root().ok()?;
    let path = paths::asset_root(&root).join("horizon/relief/index.json");
    let index: serde_json::Value = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    index["provinces"]
        .get(province)
        .map(|_| province.to_owned())
}

/// How the battle of `launch` starts: a battle on its historical site when there is one.
pub fn start_of(launch: &BattleLaunch, site: Option<&HistoricalMap>) -> ReplayStart {
    let start = ReplayStart::new(launch.setup.clone(), launch.seed);
    match site {
        Some(map) => start.on_site(map.clone()),
        None => start,
    }
}

/// The simulation of a campaign battle (same construction path as the Godot bridge's `setup`),
/// the start it was built from and the hour the campaign drew. The hour is NOT applied: the
/// caller starts the replay recording first, then applies it as a recorded input
/// (`ReplayAction::StartHour`).
pub fn build(battle: &CampaignBattle) -> Result<(BattleSim, ReplayStart, f64), String> {
    let root = paths::find_root()?;
    install_battle_rules(&paths::data_dir(&root));
    let start = start_of(&battle.launch, battle.site.as_ref());
    let sim = start.build()?;
    Ok((sim, start, battle.launch.hour))
}

/// The simulation only, hour applied (tests, tools that do not record).
pub fn build_sim(launch: &BattleLaunch) -> Result<BattleSim, String> {
    let (mut sim, _, hour) = build(&CampaignBattle::plain(launch.clone()))?;
    sim.set_start_hour(hour);
    Ok(sim)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_field_battle_in_a_site_province_and_year_is_fought_on_the_site() {
        let root = paths::find_root().expect("repo root");
        let dir = paths::data_dir(&root);
        let (game, _) = data_model::GameData::load(&dir).expect("game data");
        crate::battle::scene::setup::install_battle_rules(&dir);
        let custom = crate::battle::scene::setup::default_custom_battle(&game);
        let (plain, _, _) =
            crate::battle::scene::setup::build_start(&game, &dir, &custom, 1).unwrap();
        let mut setup = plain.setup().clone();
        let maps = maps_in(&dir);
        let found = maps.iter().find_map(|map| {
            setup.province = map.province.clone();
            (1330..1460)
                .find(|year| map.matches_campaign(&setup.province, *year))
                .map(|year| (map.id.clone(), year, setup.clone()))
        });
        let (id, year, setup) = found.expect("a map is open to campaign battles");
        let site = campaign_site(&dir, &setup, year).expect("site found");
        assert_eq!(site.province, setup.province, "{id}");
        // Never a siege, never outside its years.
        assert!(campaign_site(&dir, &setup, 1000).is_none());
        let launch = BattleLaunch {
            setup,
            seed: 5,
            hour: 9.0,
            index: 0,
        };
        let on_site = start_of(&launch, Some(&site)).build().expect("site battle");
        let plain = start_of(&launch, None).build().expect("plain battle");
        assert_ne!(on_site.field().width, plain.field().width);
    }

    #[test]
    fn historical_maps_are_found_and_matched() {
        let root = paths::find_root().expect("repo root");
        let dir = paths::data_dir(&root);
        let maps = maps_in(&dir);
        assert!(maps.len() >= 3, "{}", maps.len());
        assert!(maps.iter().all(|m| !m.id.starts_with("decor_plan")));
    }
}
