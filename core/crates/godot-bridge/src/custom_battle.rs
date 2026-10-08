//! NT2: custom battle of the main menu on the GDExtension side.
//!
//! - `BattleSim.custom_rules()` : budget bounds, unit cap, fortification;
//! - `BattleSim.custom_factions(data_dir)` : the playable factions;
//! - `BattleSim.custom_roster(data_dir, faction, year, techs)` : the units a faction buys
//!   that year (NT11; `0`: the default year) with its starting technologies plus `techs` (LR-12);
//! - `BattleSim.custom_technologies(data_dir, faction, year)` : the technologies it may be granted;
//! - `BattleSim.validate_custom(data_dir, config)` : costs and French errors;
//! - `BattleSim.setup_custom(data_dir, config, seed)` : builds the battle.
//!
//! Rules (roster, budget, cap, setup) live in `sim_battle::custom`.

use crate::convert::{from_dict, to_dict};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use data_model::load::load_entities;
use data_model::{Faction, FactionId, Technology, TechnologyId, UnitType, UnitTypeId};
use godot::prelude::*;
use sim_battle::custom::{
    faction_name, grantable_technologies, known_technologies, roster, CustomBattle,
    CustomBattleRules, CustomData,
};

use crate::battle_sim::BattleSim;

/// Unit types and factions of a data folder.
type Catalog = (
    BTreeMap<UnitTypeId, UnitType>,
    BTreeMap<FactionId, Faction>,
    BTreeMap<TechnologyId, Technology>,
);

type CatalogCache = Option<(PathBuf, Arc<Catalog>)>;
static CATALOG: OnceLock<Mutex<CatalogCache>> = OnceLock::new();

/// Unit types and factions of `data_dir` (read once per folder).
fn catalog(data_dir: &Path) -> Result<Arc<Catalog>, String> {
    let cache = CATALOG.get_or_init(|| Mutex::new(None));
    let mut guard = cache.lock().map_err(|e| e.to_string())?;
    if let Some((dir, catalog)) = guard.as_ref() {
        if dir == data_dir {
            return Ok(Arc::clone(catalog));
        }
    }
    let units = load_entities(&data_dir.join("unit_types"), |u: &UnitType| &u.id)
        .map_err(|e| e.to_string())?;
    let factions = load_entities(&data_dir.join("factions"), |f: &Faction| &f.id)
        .map_err(|e| e.to_string())?;
    let technologies = load_entities(&data_dir.join("technologies"), |t: &Technology| &t.id)
        .map_err(|e| e.to_string())?;
    let catalog = Arc::new((units, factions, technologies));
    *guard = Some((data_dir.to_path_buf(), Arc::clone(&catalog)));
    Ok(catalog)
}

fn parse_config(config: &VarDictionary) -> Result<CustomBattle, String> {
    from_dict::<CustomBattle>(config)
}

#[godot_api(secondary)]
impl BattleSim {
    /// NT2: `data/rules/custom_battle.json` (`default_budget`, `min_budget`,
    /// `max_budget`, `budget_step`, `max_units_per_side`, `general_command`,
    /// `default_fortification`, `max_fortification`).
    #[func]
    fn custom_rules(&self) -> VarDictionary {
        to_dict(CustomBattleRules::bundled())
    }

    /// NT2: playable factions `[{id, name, full_name, culture}]`, by name.
    #[func]
    fn custom_factions(&self, data_dir: GString) -> VarArray {
        let Ok(catalog) = catalog(&PathBuf::from(data_dir.to_string())) else {
            return VarArray::new();
        };
        let mut factions: Vec<&Faction> = catalog.1.values().filter(|f| f.playable).collect();
        factions.sort_by_key(|f| faction_name(f));
        factions
            .into_iter()
            .map(|f| {
                vdict! {
                    "id" => f.id.as_str(),
                    "name" => faction_name(f),
                    "full_name" => f.name.display.as_str(),
                    "culture" => f.culture.as_str(),
                }
                .to_variant()
            })
            .collect()
    }

    /// NT2: units `faction` can buy in `year` (NT11; `0`: the default year)
    /// `[{id, name, category, cost, soldiers, mounted, mercenary}]` (cost:
    /// recruitment price in livres).
    #[func]
    fn custom_roster(
        &self,
        data_dir: GString,
        faction: GString,
        year: i64,
        techs: PackedStringArray,
    ) -> VarArray {
        let Ok(catalog) = catalog(&PathBuf::from(data_dir.to_string())) else {
            return VarArray::new();
        };
        let Some(faction) = FactionId::new(faction.to_string())
            .ok()
            .and_then(|id| catalog.1.get(&id))
        else {
            return VarArray::new();
        };
        let rules = CustomBattleRules::bundled();
        let year = match i32::try_from(year) {
            Ok(y) if y > 0 => y.clamp(rules.min_year, rules.max_year),
            _ => rules.default_year,
        };
        let extra: Vec<String> = techs.as_slice().iter().map(ToString::to_string).collect();
        roster(
            &catalog.0,
            faction,
            year,
            &known_technologies(faction, &extra),
        )
        .into_iter()
        .map(|u| {
            let category = serde_json::to_value(u.category)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default();
            vdict! {
                "id" => u.id.as_str(),
                "name" => u.name.display.as_str(),
                "category" => category,
                "cost" => i64::from(u.cost.money),
                "soldiers" => i64::from(u.soldiers),
                "mounted" => u.mounted,
                "mercenary" => u.mercenary,
            }
            .to_variant()
        })
        .collect()
    }

    /// LR-12: technologies `faction` may be granted in `year`
    /// `[{id, name, units: [noms]}]`, by name: those that gate units it
    /// could field but does not know from the start.
    #[func]
    fn custom_technologies(&self, data_dir: GString, faction: GString, year: i64) -> VarArray {
        let Ok(catalog) = catalog(&PathBuf::from(data_dir.to_string())) else {
            return VarArray::new();
        };
        let Some(faction) = FactionId::new(faction.to_string())
            .ok()
            .and_then(|id| catalog.1.get(&id))
        else {
            return VarArray::new();
        };
        let rules = CustomBattleRules::bundled();
        let year = match i32::try_from(year) {
            Ok(y) if y > 0 => y.clamp(rules.min_year, rules.max_year),
            _ => rules.default_year,
        };
        let mut out: Vec<(String, String, Vec<String>)> =
            grantable_technologies(&catalog.0, faction, year)
                .into_iter()
                .map(|(id, units)| {
                    let name = catalog
                        .2
                        .get(&id)
                        .map_or_else(|| id.to_string(), |t| t.name.display.clone());
                    let names = units.iter().map(|u| u.name.display.clone()).collect();
                    (name, id.to_string(), names)
                })
                .collect();
        out.sort();
        out.into_iter()
            .map(|(name, id, units)| {
                let units: VarArray = units.iter().map(|u| u.to_variant()).collect();
                vdict! { "id" => id, "name" => name, "units" => &units }.to_variant()
            })
            .collect()
    }

    /// NT2: checks a composition: `{ok, errors: [texte], attacker: {cost,
    /// budget, units, max_units}, defender}`.
    #[func]
    fn validate_custom(&self, data_dir: GString, config: VarDictionary) -> VarDictionary {
        let result = (|| -> Result<VarDictionary, String> {
            let catalog = catalog(&PathBuf::from(data_dir.to_string()))?;
            let custom = parse_config(&config)?;
            let data = CustomData {
                unit_types: &catalog.0,
                factions: &catalog.1,
            };
            Ok(to_dict(
                &custom.validate(&data, CustomBattleRules::bundled()),
            ))
        })();
        result.unwrap_or_else(|error| {
            let mut dict = VarDictionary::new();
            dict.set("ok", false);
            let errors: VarArray = [format!("configuration illisible : {error}").to_variant()]
                .into_iter()
                .collect();
            dict.set("errors", &errors);
            dict
        })
    }

    /// NT2: builds the custom battle `config` (same dictionary as
    /// `validate_custom`), recorded for the replay; false if invalid.
    #[func]
    fn setup_custom(&mut self, data_dir: GString, config: VarDictionary, seed: i64) -> bool {
        let dir = PathBuf::from(data_dir.to_string());
        let built = (|| -> Result<(sim_battle::BattleSim, sim_battle::ReplayStart), String> {
            let catalog = catalog(&dir)?;
            let custom = parse_config(&config)?;
            let data = CustomData {
                unit_types: &catalog.0,
                factions: &catalog.1,
            };
            let (_, orders, standards, abilities) = crate::historical_battles::battle_data(&dir)?;
            let setup = custom.battle_setup(
                &data,
                CustomBattleRules::bundled(),
                orders,
                standards,
                abilities,
            )?;
            let start =
                sim_battle::ReplayStart::plain(setup, seed as u64).with_weather(custom.weather()?);
            let sim = start.build()?;
            Ok((sim, start))
        })();
        self.player = None;
        self.recorder = None;
        self.historical = None;
        self.touch_poses();
        match built {
            Ok((sim, start)) => {
                self.recorder = Some(sim_battle::ReplayRecorder::new(start, &sim));
                self.sim = Some(sim);
                if let Ok(Some(hour)) = parse_config(&config).and_then(|c| c.start_hour()) {
                    self.drive(sim_battle::ReplayAction::StartHour { hour });
                }
                true
            }
            Err(error) => {
                godot_error!("BattleSim.setup_custom: {error}");
                self.sim = None;
                false
            }
        }
    }
}
