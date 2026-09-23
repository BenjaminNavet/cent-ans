//! `CampaignSim`: Godot-facing handle on a [`CampaignState`] (spec M2 § 2).
//!
//! The game data is loaded once per data directory and shared between
//! instances so that `load_from_string` works on a fresh object.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use data_model::{CharacterId, FactionId, GameData, ProvinceId};
use godot::classes::RefCounted;
use godot::prelude::*;
use sim_campaign::{Army, ArmyId, CampaignState, GameEvent, Order, Unit};

use crate::convert::variant_to_json;

/// Last successfully loaded game data, shared by every `CampaignSim`.
type CachedData = Option<(PathBuf, Arc<GameData>)>;
static SHARED_DATA: OnceLock<Mutex<CachedData>> = OnceLock::new();

fn shared_data(data_dir: Option<&PathBuf>) -> Option<Arc<GameData>> {
    let cache = SHARED_DATA.get_or_init(|| Mutex::new(None));
    let mut guard = cache.lock().ok()?;
    if let Some((cached_dir, data)) = guard.as_ref() {
        if data_dir.is_none_or(|dir| dir == cached_dir) {
            return Some(Arc::clone(data));
        }
    }
    let dir = data_dir?;
    match GameData::load(dir) {
        Ok((data, warnings)) => {
            for warning in &warnings {
                godot_warn!("CampaignSim data: {warning}");
            }
            let data = Arc::new(data);
            *guard = Some((dir.clone(), Arc::clone(&data)));
            Some(data)
        }
        Err(error) => {
            godot_error!(
                "CampaignSim: cannot load data from {}: {error}",
                dir.display()
            );
            None
        }
    }
}

/// Godot-facing handle on a campaign simulation.
#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct CampaignSim {
    data: Option<Arc<GameData>>,
    state: Option<CampaignState>,
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for CampaignSim {
    fn init(base: Base<RefCounted>) -> Self {
        CampaignSim {
            data: None,
            state: None,
            base,
        }
    }
}

#[godot_api]
impl CampaignSim {
    /// Loads `data_dir` and builds the spring-1337 start for `player`.
    #[func]
    fn new_campaign(&mut self, data_dir: GString, player: GString, seed: i64) -> bool {
        let Some(data) = shared_data(Some(&PathBuf::from(data_dir.to_string()))) else {
            return false;
        };
        let player = match FactionId::new(player.to_string()) {
            Ok(id) => id,
            Err(error) => {
                godot_error!("CampaignSim.new_campaign: invalid faction id: {error}");
                return false;
            }
        };
        match CampaignState::new_1337(&data, player, seed as u64) {
            Ok(state) => {
                self.data = Some(data);
                self.state = Some(state);
                true
            }
            Err(error) => {
                godot_error!("CampaignSim.new_campaign failed: {error}");
                false
            }
        }
    }

    /// Serialises the whole campaign as JSON (empty before `new_campaign`).
    #[func]
    fn save_to_string(&self) -> GString {
        self.state
            .as_ref()
            .map_or_else(GString::new, |state| GString::from(&state.save_json()))
    }

    /// Restores a campaign saved with `save_to_string`. The game data must
    /// have been loaded by any `CampaignSim` of this process beforehand.
    #[func]
    fn load_from_string(&mut self, json: GString) -> bool {
        let Some(data) = self.data.clone().or_else(|| shared_data(None)) else {
            godot_error!("CampaignSim.load_from_string: game data not loaded yet");
            return false;
        };
        match CampaignState::load_json(&json.to_string()) {
            Ok(state) => {
                self.data = Some(data);
                self.state = Some(state);
                true
            }
            Err(error) => {
                godot_error!("CampaignSim.load_from_string failed: {error}");
                false
            }
        }
    }

    /// Zero-based turn counter (-1 before `new_campaign`).
    #[func]
    fn get_turn(&self) -> i64 {
        self.state.as_ref().map_or(-1, |s| i64::from(s.turn()))
    }

    /// French date label, e.g. `"Printemps 1337"` (empty before `new_campaign`).
    #[func]
    fn get_date_label(&self) -> GString {
        self.state
            .as_ref()
            .map_or_else(GString::new, |s| GString::from(&s.date_label()))
    }

    #[func]
    fn get_player_faction(&self) -> GString {
        self.state
            .as_ref()
            .map_or_else(GString::new, |s| GString::from(s.player_faction().as_str()))
    }

    /// `{treasury, income, at_war_with, allies, provinces_count, armies_count, alive}`.
    #[func]
    fn get_faction_summary(&self, id: GString) -> VarDictionary {
        let Some(state) = &self.state else {
            return VarDictionary::new();
        };
        let Some(summary) = FactionId::new(id.to_string())
            .ok()
            .and_then(|id| state.faction_summary(&id))
        else {
            return VarDictionary::new();
        };
        vdict! {
            "treasury" => summary.treasury,
            "income" => summary.income,
            "at_war_with" => &ids(summary.at_war_with.iter()),
            "allies" => &ids(summary.allies.iter()),
            "provinces_count" => summary.provinces_count as i64,
            "armies_count" => summary.armies_count as i64,
            "alive" => summary.alive,
        }
    }

    /// `{owner, controller, garrison[], siege?, unrest, devastation, population_total}`.
    #[func]
    fn get_province_state(&self, id: GString) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let Some(province) = ProvinceId::new(id.to_string())
            .ok()
            .and_then(|id| state.province_state(&id))
        else {
            return VarDictionary::new();
        };
        let mut dict = vdict! {
            "owner" => province.owner.as_str(),
            "controller" => province.controller.as_str(),
            "garrison" => &units_array(data, &province.garrison),
            "unrest" => i64::from(province.unrest),
            "devastation" => i64::from(province.devastation),
            "population_total" => province.population.total() as i64,
        };
        if let Some(siege) = &province.siege {
            let siege_dict = vdict! {
                "attacker" => siege.attacker.as_str(),
                "turns_left" => i64::from(siege.turns_left),
            };
            dict.set("siege", &siege_dict);
        }
        dict
    }

    /// Sorted army ids.
    #[func]
    fn get_army_ids(&self) -> PackedStringArray {
        self.state
            .as_ref()
            .map(|s| ids(s.armies().keys()))
            .unwrap_or_default()
    }

    /// `{faction, general, general_name, location, units[], movement_points, supply, stance, path[]}`.
    #[func]
    fn get_army(&self, id: GString) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let Some(army) = ArmyId::parse(&id.to_string()).and_then(|id| state.army(&id)) else {
            return VarDictionary::new();
        };
        army_dict(data, army)
    }

    /// `{province_id: cost}` for every province the army can reach this turn.
    #[func]
    fn get_reachable(&self, army_id: GString) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let Some(army) = ArmyId::parse(&army_id.to_string()) else {
            return VarDictionary::new();
        };
        if state.army(&army).is_none() {
            return VarDictionary::new();
        }
        let mut dict = VarDictionary::new();
        for (province, cost) in state.reachable(data, &army) {
            dict.set(province.as_str(), i64::from(cost));
        }
        dict
    }

    /// Provinces to cross to reach `target` (empty when unreachable or already there).
    #[func]
    fn find_path(&self, army_id: GString, target: GString) -> PackedStringArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return PackedStringArray::new();
        };
        let (Some(army), Ok(target)) = (
            ArmyId::parse(&army_id.to_string()),
            ProvinceId::new(target.to_string()),
        ) else {
            return PackedStringArray::new();
        };
        state
            .find_path(data, &army, &target)
            .map(|path| ids(path.iter()))
            .unwrap_or_default()
    }

    /// Recruitment options of a province: `[{unit_type, name, cost, upkeep, available, reason}]`.
    #[func]
    fn get_recruitable(&self, province_id: GString) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        let Ok(province) = ProvinceId::new(province_id.to_string()) else {
            return VarArray::new();
        };
        state
            .recruitable(data, &province)
            .iter()
            .map(|option| {
                vdict! {
                    "unit_type" => option.unit_type.as_str(),
                    "name" => option.name.as_str(),
                    "cost" => i64::from(option.cost),
                    "upkeep" => i64::from(option.upkeep),
                    "available" => option.available,
                    "reason" => option.reason.as_deref().unwrap_or(""),
                }
                .to_variant()
            })
            .collect()
    }

    /// Validates and records an order given as `{"type": "move_army", ...}`.
    /// Returns `{ok, error}`; `error` is a French message when `ok` is false.
    #[func]
    fn submit_order(&mut self, order: VarDictionary) -> VarDictionary {
        let (Some(state), Some(data)) = (&mut self.state, &self.data) else {
            return order_result(Err("aucune campagne en cours".to_owned()));
        };
        let parsed = variant_to_json(&order.to_variant())
            .and_then(|json| serde_json::from_value::<Order>(json).map_err(|e| e.to_string()));
        let order = match parsed {
            Ok(order) => order,
            Err(error) => return order_result(Err(format!("ordre invalide : {error}"))),
        };
        order_result(state.submit_order(data, order).map_err(|e| e.to_string()))
    }

    /// Resolves the turn and returns its events.
    #[func]
    fn end_turn(&mut self) -> VarArray {
        let (Some(state), Some(data)) = (&mut self.state, &self.data) else {
            godot_warn!("CampaignSim.end_turn called before new_campaign");
            return VarArray::new();
        };
        events_array(&state.end_turn(data))
    }

    /// Events of the last resolved turn.
    #[func]
    fn get_events(&self) -> VarArray {
        self.state
            .as_ref()
            .map(|s| events_array(s.events()))
            .unwrap_or_default()
    }
}

fn order_result(result: Result<(), String>) -> VarDictionary {
    match result {
        Ok(()) => vdict! { "ok" => true, "error" => "" },
        Err(error) => vdict! { "ok" => false, "error" => error.as_str() },
    }
}

fn ids<'a>(iter: impl Iterator<Item = &'a (impl AsRef<str> + 'a)>) -> PackedStringArray {
    iter.map(|id| GString::from(id.as_ref())).collect()
}

fn unit_name(data: &GameData, unit: &Unit) -> String {
    data.unit_types.get(&unit.unit_type).map_or_else(
        || unit.unit_type.as_str().to_owned(),
        |t| t.name.display.clone(),
    )
}

fn units_array(data: &GameData, units: &[Unit]) -> VarArray {
    units
        .iter()
        .map(|unit| {
            vdict! {
                "unit_type" => unit.unit_type.as_str(),
                "name" => unit_name(data, unit).as_str(),
                "strength" => i64::from(unit.strength),
                "max_strength" => i64::from(unit.max_strength),
                "morale" => i64::from(unit.morale),
            }
            .to_variant()
        })
        .collect()
}

fn character_name(data: &GameData, id: Option<&CharacterId>) -> String {
    id.and_then(|id| data.characters.get(id))
        .map_or_else(String::new, |c| c.name.display.clone())
}

fn army_dict(data: &GameData, army: &Army) -> VarDictionary {
    vdict! {
        "faction" => army.faction.as_str(),
        "general" => army.general.as_ref().map_or("", |id| id.as_str()),
        "general_name" => character_name(data, army.general.as_ref()).as_str(),
        "location" => army.location.as_str(),
        "units" => &units_array(data, &army.units),
        "movement_points" => i64::from(army.movement_points),
        "supply" => i64::from(army.supply),
        "stance" => stance_key(army.stance),
        "path" => &ids(army.path.iter()),
    }
}

fn stance_key(stance: sim_campaign::Stance) -> &'static str {
    match stance {
        sim_campaign::Stance::Normal => "normal",
        sim_campaign::Stance::Raid => "raid",
        sim_campaign::Stance::Siege => "siege",
    }
}

fn events_array(events: &[GameEvent]) -> VarArray {
    events
        .iter()
        .map(|event| {
            let kind = serde_json::to_value(event.kind)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default();
            vdict! {
                "kind" => kind.as_str(),
                "text_fr" => event.text_fr.as_str(),
                "province" => event.province.as_ref().map_or("", |id| id.as_str()),
                "army" => event.army.as_ref().map_or("", |id| id.as_str()),
                "faction" => event.faction.as_ref().map_or("", |id| id.as_str()),
            }
            .to_variant()
        })
        .collect()
}
