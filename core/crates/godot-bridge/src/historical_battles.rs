//! EP7: historical battle maps on the GDExtension side (ADR 0035).
//!
//! - `BattleSim.list_historical(data_dir)` lists `data/battle_maps/*.json`
//!   for the menu « Batailles historiques » ;
//! - `BattleSim.setup_historical(data_dir, id, player_side, seed)` builds
//!   the battle of a map (site, orders of battle, weather, hour, scenario);
//! - a campaign battle fought in the province of a map, within its years,
//!   is fought on the historical site: `CampaignSim.get_battle_setup` adds
//!   the map (`historical_site`, JSON text) and `BattleSim.setup` lays it.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use data_model::load::load_entities;
use data_model::{BattleAbility, BattleOrder, BattleStandardRules, UnitType, UnitTypeId};
use godot::prelude::*;
use sim_battle::{BattleSetup, HistoricalMap, SideId};

use crate::battle_sim::BattleSim;

/// A historical map with its JSON text.
pub(crate) type MapEntry = (HistoricalMap, String);

/// Maps found in a data folder (read once per folder, shared).
type MapCache = Option<(PathBuf, Arc<[MapEntry]>)>;
static MAPS: OnceLock<Mutex<MapCache>> = OnceLock::new();

/// Every historical map of `data_dir` with its JSON text, sorted by year
/// (cached: a call on the same folder only clones the `Arc`).
pub(crate) fn maps_in(data_dir: &Path) -> Arc<[MapEntry]> {
    let cache = MAPS.get_or_init(|| Mutex::new(None));
    let Ok(mut guard) = cache.lock() else {
        return Arc::new([]);
    };
    if let Some((dir, maps)) = guard.as_ref() {
        if dir == data_dir {
            return Arc::clone(maps);
        }
    }
    let mut maps = Vec::new();
    if let Ok(entries) = std::fs::read_dir(data_dir.join("battle_maps")) {
        let mut paths: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
        paths.sort();
        for path in paths {
            let is_map = path.extension().is_some_and(|e| e == "json")
                && !path
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("decor_plan"));
            if !is_map {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            match HistoricalMap::from_json(&text) {
                Ok(map) => maps.push((map, text)),
                Err(error) => {
                    godot_warn!("battle map {}: {error}", path.display());
                }
            }
        }
    }
    maps.sort_by_key(|(m, _)| m.year);
    let maps: Arc<[MapEntry]> = maps.into();
    *guard = Some((data_dir.to_path_buf(), Arc::clone(&maps)));
    maps
}

/// The map a campaign battle in `province` in `year` is fought on, if any
/// (never a siege).
pub(crate) fn campaign_site(data_dir: &Path, setup: &BattleSetup, year: i32) -> Option<MapEntry> {
    if setup.siege.is_some() {
        return None;
    }
    maps_in(data_dir)
        .iter()
        .find(|(map, _)| map.matches_campaign(&setup.province, year))
        .cloned()
}

/// Unit types, leader's orders, standards rules and (CB4) regiments'
/// abilities of a data folder.
pub(crate) type BattleData = (
    std::collections::BTreeMap<UnitTypeId, UnitType>,
    Vec<BattleOrder>,
    Option<BattleStandardRules>,
    Vec<BattleAbility>,
);

/// Unit types, leader's orders and standards of `data_dir` (a historical
/// battle is built without a campaign).
pub(crate) fn battle_data(data_dir: &Path) -> Result<BattleData, String> {
    crate::campaign_sim::install_battle_rules(data_dir);
    let units = load_entities(&data_dir.join("unit_types"), |u: &UnitType| &u.id)
        .map_err(|e| e.to_string())?;
    let orders: Vec<BattleOrder> =
        load_entities(&data_dir.join("battle_orders"), |o: &BattleOrder| &o.id)
            .map(|m| m.into_values().collect())
            .unwrap_or_default();
    let standards = std::fs::read_to_string(data_dir.join("rules/battle_standards.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<BattleStandardRules>(&t).ok());
    let abilities: Vec<BattleAbility> =
        load_entities(&data_dir.join("battle_abilities"), |a: &BattleAbility| {
            &a.id
        })
        .map(|m| m.into_values().collect())
        .unwrap_or_default();
    Ok((units, orders, standards, abilities))
}

/// The menu line of a map.
pub(crate) fn map_summary(map: &HistoricalMap) -> VarDictionary {
    let army = |side: SideId| {
        let a = map.armies.side(side);
        let soldiers: u32 = a
            .regiments
            .iter()
            .map(|b| b.count.max(1) * b.soldiers.unwrap_or(100))
            .sum();
        vdict! {
            "faction" => a.faction.as_str(),
            "faction_name" => a.faction_name.as_str(),
            "army" => a.army.as_str(),
            "general" => a.general.as_ref().map_or("", |g| g.name.as_str()),
            "soldiers" => i64::from(soldiers),
        }
    };
    let waves = |side: SideId| -> VarArray {
        map.armies
            .side(side)
            .waves
            .iter()
            .map(|w| w.label.to_variant())
            .collect()
    };
    let mut dict = vdict! {
        "id" => map.id.as_str(),
        "name" => map.name.as_str(),
        "date" => map.date.as_str(),
        "date_fr" => map.date_fr(),
        "year" => i64::from(map.year),
        "place" => map.site.place.as_str(),
        "province" => map.province.as_str(),
        "province_name" => map.province_name.as_str(),
        "summary" => map.summary.as_str(),
        "weather_label" => map.weather.label.as_str(),
        "weather_start" => map.weather.start.key(),
        "weather_end" => map.final_weather().key(),
        "start_hour" => map.start_hour,
        "horizon" => map.horizon_key(),
        "historical_winner" => map.historical_winner.key(),
    };
    dict.set("attacker", &army(SideId::Attacker));
    dict.set("defender", &army(SideId::Defender));
    dict.set("attacker_waves", &waves(SideId::Attacker));
    dict.set("defender_waves", &waves(SideId::Defender));
    dict
}

#[godot_api(secondary)]
impl BattleSim {
    /// EP7: the historical maps of `data_dir` (menu « Batailles
    /// historiques »): `[{id, name, date, year, place, province,
    /// province_name, summary, weather_label, start_hour, horizon,
    /// historical_winner, attacker: {faction, faction_name, army, general,
    /// soldiers}, defender, attacker_waves, defender_waves}]`.
    #[func]
    fn list_historical(&self, data_dir: GString) -> VarArray {
        maps_in(&PathBuf::from(data_dir.to_string()))
            .iter()
            .map(|(map, _)| map_summary(map).to_variant())
            .collect()
    }

    /// EP7: builds the historical battle `id` of `data_dir` (site, orders
    /// of battle, weather, hour, scenario). `player_side`: `attacker`,
    /// `defender`, or empty for AI against AI.
    #[func]
    fn setup_historical(
        &mut self,
        data_dir: GString,
        id: GString,
        player_side: GString,
        seed: i64,
    ) -> bool {
        let dir = PathBuf::from(data_dir.to_string());
        let id = id.to_string();
        type Built = (
            HistoricalMap,
            sim_battle::BattleSim,
            sim_battle::ReplayStart,
        );
        let built = (|| -> Result<Built, String> {
            let map = maps_in(&dir)
                .iter()
                .find(|(m, _)| m.id == id)
                .map(|(m, _)| m.clone())
                .ok_or_else(|| format!("unknown historical map {id}"))?;
            let (units, orders, standards, abilities) = battle_data(&dir)?;
            let side = SideId::parse(&player_side.to_string());
            let mut setup = map.battle_setup(&units, orders, standards, side)?;
            setup.abilities = abilities;
            // EP13: built through the replay start, recorded.
            let start = sim_battle::ReplayStart::historical(setup, seed as u64, map.clone());
            let sim = start.build()?;
            Ok((map, sim, start))
        })();
        self.player = None;
        self.recorder = None;
        self.touch_poses();
        match built {
            Ok((map, sim, start)) => {
                self.recorder = Some(sim_battle::ReplayRecorder::new(start, &sim));
                self.sim = Some(sim);
                self.historical = Some(map);
                true
            }
            Err(error) => {
                godot_error!("BattleSim.setup_historical({id}): {error}");
                self.sim = None;
                self.historical = None;
                false
            }
        }
    }

    /// EP7: the historical map of the battle (`list_historical` line, plus
    /// `site_only`: a campaign battle on the site), empty otherwise.
    #[func]
    fn get_historical(&self) -> VarDictionary {
        let Some(map) = &self.historical else {
            return VarDictionary::new();
        };
        let mut dict = map_summary(map);
        let scripted = self.sim.as_ref().is_some_and(|s| s.is_historical());
        dict.set("site_only", !scripted);
        dict
    }

    /// EP7: the successive "battles" of `side` and whether each has been
    /// released: `[{label, released}]`.
    #[func]
    fn get_waves(&self, side: GString) -> VarArray {
        let (Some(sim), Some(side)) = (&self.sim, SideId::parse(&side.to_string())) else {
            return VarArray::new();
        };
        sim.scenario_waves(side)
            .into_iter()
            .map(|(label, released)| {
                vdict! { "label" => label, "released" => released }.to_variant()
            })
            .collect()
    }
}
