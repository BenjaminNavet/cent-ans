//! `CampaignSim`: Godot-facing handle on a [`CampaignState`] (spec M2 § 2).
//!
//! The game data is loaded once per data directory and shared between
//! instances so that `load_from_string` works on a fresh object.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use data_model::{
    BuildingId, CharacterId, Effect, FactionId, GameData, PopulationClass, ProvinceId,
    ResourceCategory, Role, SettlementId, Skill, SkillBranch,
};
use godot::classes::RefCounted;
use godot::prelude::*;
use sim_campaign::{
    Army, ArmyId, BuildOption, CampaignState, CharacterView, Construction, EffectTotals,
    EffectValue, FactionEconomy, GameEvent, Order, ProvinceCity, TaxRate, Unit,
};

use crate::campaign_sim_turn::TURN_PENDING_FR;
use crate::convert::variant_to_json;

/// Last successfully loaded game data and its load warnings, shared by
/// every `CampaignSim` and `GameDataStore` (one load at start-up).
type CachedData = Option<(PathBuf, Arc<GameData>, Arc<[String]>)>;
static SHARED_DATA: OnceLock<Mutex<CachedData>> = OnceLock::new();

/// Game data of `data_dir` with the warnings of its load: the cached copy
/// when it comes from the same folder, else loaded from disk (the cache is
/// then replaced; a failed load leaves it untouched). The warnings are
/// logged once, when the data is read from disk.
pub(crate) fn load_shared_data(data_dir: &Path) -> Result<(Arc<GameData>, Arc<[String]>), String> {
    let cache = SHARED_DATA.get_or_init(|| Mutex::new(None));
    let mut guard = cache
        .lock()
        .map_err(|_| "cache des données empoisonné".to_owned())?;
    if let Some((cached_dir, data, warnings)) = guard.as_ref() {
        if cached_dir == data_dir {
            return Ok((Arc::clone(data), Arc::clone(warnings)));
        }
    }
    let (data, warnings) = GameData::load(data_dir).map_err(|error| error.to_string())?;
    let warnings: Arc<[String]> = warnings.iter().map(ToString::to_string).collect();
    for warning in warnings.iter() {
        godot_warn!("game data: {warning}");
    }
    let data = Arc::new(data);
    *guard = Some((
        data_dir.to_path_buf(),
        Arc::clone(&data),
        Arc::clone(&warnings),
    ));
    Ok((data, warnings))
}

fn shared_data(data_dir: Option<&PathBuf>) -> Option<Arc<GameData>> {
    let Some(dir) = data_dir else {
        let cache = SHARED_DATA.get_or_init(|| Mutex::new(None));
        let guard = cache.lock().ok()?;
        return guard.as_ref().map(|(_, data, _)| Arc::clone(data));
    };
    match load_shared_data(dir) {
        Ok((data, _)) => Some(data),
        Err(error) => {
            godot_error!(
                "CampaignSim: cannot load data from {}: {error}",
                dir.display()
            );
            None
        }
    }
}

/// EP7: folder of the game data already loaded, if any.
pub(crate) fn loaded_data_dir() -> Option<PathBuf> {
    let cache = SHARED_DATA.get_or_init(|| Mutex::new(None));
    let guard = cache.lock().ok()?;
    guard.as_ref().map(|(dir, _, _)| dir.clone())
}

/// Game data already loaded by any `CampaignSim` of this process, if any
/// (lot DF1: the faction screen lists the difficulty levels before a
/// campaign starts).
pub(crate) fn loaded_data() -> Option<Arc<GameData>> {
    shared_data(None)
}

/// Godot-facing handle on a campaign simulation.
#[derive(GodotClass)]
#[class(base = RefCounted)]
pub struct CampaignSim {
    pub(crate) data: Option<Arc<GameData>>,
    pub(crate) state: Option<CampaignState>,
    /// French message of the last failed `load_from_string`.
    pub(crate) last_load_error: String,
    /// PB3d: end of turn running on its worker thread, if any.
    pub(crate) pending_turn: Option<crate::turn_job::TurnJob>,
    /// PB3d: bumped by every call that may change the state.
    pub(crate) revision: u64,
    base: Base<RefCounted>,
}

#[godot_api]
impl IRefCounted for CampaignSim {
    fn init(base: Base<RefCounted>) -> Self {
        CampaignSim {
            data: None,
            state: None,
            last_load_error: String::new(),
            pending_turn: None,
            revision: 0,
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
                self.cancel_pending_turn();
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
                self.cancel_pending_turn();
                self.data = Some(data);
                self.state = Some(state);
                self.last_load_error = String::new();
                true
            }
            Err(error) => {
                // Lot C4: saves older than the settlements are refused with
                // a French message (« sauvegarde d'une version antérieure à
                // la refonte des colonies »), kept for the UI.
                self.last_load_error = error.to_string();
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
            "projected_income" => summary.projected_income,
            "army_upkeep" => summary.army_upkeep,
            "building_upkeep" => summary.building_upkeep,
            "tax_rate" => tax_rate_key(summary.tax_rate),
        }
    }

    /// Full city panel of a province (spec M3 § 2), or an empty dictionary
    /// for an unknown id.
    #[func]
    fn get_province_city(&self, id: GString) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let Some(city) = ProvinceId::new(id.to_string())
            .ok()
            .and_then(|id| state.province_city(data, &id))
        else {
            return VarDictionary::new();
        };
        province_city_dict(data, &city)
    }

    /// Full economic panel of a faction (spec M3 § 2), or an empty
    /// dictionary for an unknown id.
    #[func]
    fn get_faction_economy(&self, id: GString) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let Ok(faction) = FactionId::new(id.to_string()) else {
            return VarDictionary::new();
        };
        let Some(economy) = state.faction_economy(data, &faction) else {
            return VarDictionary::new();
        };
        let mut dict = faction_economy_dict(&economy);
        // UI audit A3 E1: the booked balance of the last season, from core.
        dict.set(
            "net_income_last_turn",
            state.faction_net_last_turn(&faction).unwrap_or(0),
        );
        budget_into_dict(&mut dict, &economy, state.budget_history(&faction));
        dict
    }

    /// `{owner, controller, garrison[], siege?, unrest, disorder,
    /// revolt_seasons, revolt_threshold, revolt_seasons_needed, devastation,
    /// population_total, city, settlements[]}`. Lot C4: owner, controller,
    /// garrison and siege are those of the province's city (derived).
    #[func]
    fn get_province_state(&self, id: GString) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let Some((province, city)) = ProvinceId::new(id.to_string())
            .ok()
            .and_then(|id| Some((state.province_state(&id)?, state.city_state(&id)?)))
        else {
            return VarDictionary::new();
        };
        let mut dict = vdict! {
            "owner" => city.owner.as_str(),
            "controller" => city.controller.as_str(),
            "garrison" => &units_array(data, &city.garrison),
            "city" => province.city.as_str(),
            "settlements" => &ids(province.settlements.iter()),
            // EQ1: the unrest that drives revolts (class-weighted), not the
            // province's disorder gauge, which is only one of its causes.
            "unrest" => sim_campaign::population::weighted_unrest(&province.population).round() as i64,
            "disorder" => i64::from(province.unrest),
            "revolt_seasons" => i64::from(province.revolt_seasons),
            "revolt_threshold" => data.population_rules.revolt_unrest_threshold.round() as i64,
            "revolt_seasons_needed" => i64::from(data.population_rules.revolt_seasons),
            "devastation" => i64::from(province.devastation),
            "population_total" => province.population.total() as i64,
        };
        if let Some(governor) = ProvinceId::new(id.to_string())
            .ok()
            .and_then(|p| state.province_governor(&p))
        {
            dict.set("governor", governor.as_str());
            dict.set(
                "governor_name",
                state.character_name(data, governor).as_str(),
            );
        }
        if let Some(siege) = &city.siege {
            let siege_dict = vdict! {
                "attacker" => siege.attacker.as_str(),
                "turns_left" => i64::from(siege.turns_left),
                "turns_elapsed" => i64::from(siege.turns_elapsed),
                "supplies" => i64::from(siege.supplies),
                "breach" => i64::from(siege.breach),
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
        army_dict(state, data, army)
    }

    /// `{province_id: cost}` for every province the army can reach this turn
    /// (lot C4: the cheapest reachable settlement of each province other
    /// than the army's own; see `get_reachable_settlements`).
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
        for (province, cost) in state.reachable_provinces(data, &army) {
            dict.set(province.as_str(), i64::from(cost));
        }
        dict
    }

    /// `{settlement_id: cost}` for every settlement the army can reach this turn.
    #[func]
    fn get_reachable_settlements(&self, army_id: GString) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let Some(army) = ArmyId::parse(&army_id.to_string()) else {
            return VarDictionary::new();
        };
        let mut dict = VarDictionary::new();
        for (settlement, cost) in state.reachable(data, &army) {
            dict.set(settlement.as_str(), i64::from(cost));
        }
        dict
    }

    /// `[target]` when `army` can reach `target` — a settlement id, or a
    /// province id standing for its city — on the navigation grid (lot M2:
    /// the march itself is computed by the core), empty when unreachable or
    /// already there. The result feeds a `move_army` order as is.
    #[func]
    fn find_path(&self, army_id: GString, target: GString) -> PackedStringArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return PackedStringArray::new();
        };
        let (Some(army), Some(target)) = (
            ArmyId::parse(&army_id.to_string()),
            settlement_or_city(state, &target.to_string()),
        ) else {
            return PackedStringArray::new();
        };
        if state.army(&army).is_none_or(|a| a.is_at(&target)) {
            return PackedStringArray::new();
        }
        data.settlement_point(&target)
            .and_then(|point| state.find_path(data, &army, point))
            .map(|_| ids(std::iter::once(&target)))
            .unwrap_or_default()
    }

    /// Provinces crossed by `find_path` (lot C4: for the v1 map preview,
    /// which draws province to province).
    #[func]
    fn find_path_provinces(&self, army_id: GString, target: GString) -> PackedStringArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return PackedStringArray::new();
        };
        let (Some(army), Some(target)) = (
            ArmyId::parse(&army_id.to_string()),
            settlement_or_city(state, &target.to_string()),
        ) else {
            return PackedStringArray::new();
        };
        let Some(entry) = state.army(&army) else {
            return PackedStringArray::new();
        };
        let Some(path) = data
            .settlement_point(&target)
            .and_then(|point| state.find_path(data, &army, point))
        else {
            return PackedStringArray::new();
        };
        // Lot M2: the provinces under the cells of the grid path.
        let grid = data.navgrid();
        let mut last = state.army_province(data, entry);
        let mut out = PackedStringArray::new();
        for cell in &path.cells {
            let point = cell.center(grid);
            if let Some(p) = data.province_at_point(point[0], point[1]) {
                if last.as_ref() != Some(p) {
                    out.push(p.as_str());
                    last = Some(p.clone());
                }
            }
        }
        out
    }

    /// Recruitment options of a settlement (or of a province's city):
    /// `[{unit_type, name, cost, upkeep, available, reason, resources,
    /// import_cost, imported, pool_available, pool_cap,
    /// pool_seasons_to_next, pool_label}]` (SV2: `cost` includes
    /// `import_cost`; TW2-T2: the settlement's reserve of the unit type,
    /// `pool_seasons_to_next` -1 when full or never refilled).
    #[func]
    fn get_recruitable(&self, place_id: GString) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        let Some(settlement) = settlement_or_city(state, &place_id.to_string()) else {
            return VarArray::new();
        };
        state
            .recruitable(data, &settlement)
            .iter()
            .map(|option| {
                let amounts = |map: &std::collections::BTreeMap<data_model::ResourceId, u32>| {
                    let mut dict = VarDictionary::new();
                    for (resource, amount) in map {
                        dict.set(resource.as_str(), i64::from(*amount));
                    }
                    dict
                };
                vdict! {
                    "unit_type" => option.unit_type.as_str(),
                    "name" => option.name.as_str(),
                    "cost" => i64::from(option.cost),
                    "upkeep" => i64::from(option.upkeep),
                    "available" => option.available,
                    "reason" => option.reason.as_deref().unwrap_or(""),
                    // SV2: resource units the unit needs, the livres of
                    // `cost` spent importing what the faction lacks, and
                    // the units imported (B7c rule, ADR 0053).
                    "resources" => &amounts(&option.resources),
                    "import_cost" => i64::from(option.import_cost),
                    "imported" => &amounts(&option.imported),
                    "pool_available" => i64::from(option.pool.available),
                    "pool_cap" => i64::from(option.pool.cap),
                    "pool_seasons_to_next" => option.pool.seasons_to_next.map_or(-1, i64::from),
                    "pool_label" => option.pool.label_fr().as_str(),
                }
                .to_variant()
            })
            .collect()
    }

    /// Validates and records an order given as `{"type": "move_army", ...}`.
    /// Returns `{ok, error}`; `error` is a French message when `ok` is false.
    #[func]
    fn submit_order(&mut self, order: VarDictionary) -> VarDictionary {
        if self.refuse_while_turn_pending("submit_order") {
            return order_result(Err(TURN_PENDING_FR.to_owned()));
        }
        let (Some(state), Some(data)) = (&mut self.state, &self.data) else {
            return order_result(Err("aucune campagne en cours".to_owned()));
        };
        let parsed = variant_to_json(&order.to_variant())
            .and_then(|json| serde_json::from_value::<Order>(json).map_err(|e| e.to_string()));
        let order = match parsed {
            Ok(order) => order,
            Err(error) => return order_result(Err(invalid_order_message(&error))),
        };
        order_result(state.submit_order(data, order).map_err(|e| e.to_string()))
    }

    /// Resolves the turn and returns its events (synchronous: tests,
    /// headless runs; the map uses `begin_end_turn` / `poll_end_turn`, PB3d).
    /// An end of turn already running on its thread is waited for and its
    /// events returned instead.
    #[func]
    fn end_turn(&mut self) -> VarArray {
        if let Some(events) = self.finish_pending_turn() {
            return events;
        }
        self.revision += 1;
        let (Some(state), Some(data)) = (&mut self.state, &self.data) else {
            godot_warn!("CampaignSim.end_turn called before new_campaign");
            return VarArray::new();
        };
        events_array(&crate::turn_job::resolve_turn(state, data))
    }

    /// Character sheet (spec M4 § 3), or an empty dictionary for an unknown id.
    #[func]
    fn get_character(&self, id: GString) -> VarDictionary {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarDictionary::new();
        };
        let Some(view) = CharacterId::new(id.to_string())
            .ok()
            .and_then(|id| state.character_view(data, &id))
        else {
            return VarDictionary::new();
        };
        character_dict(state, data, &view)
    }

    /// Living characters of `faction`: ruler, heir, then by age.
    #[func]
    fn get_faction_characters(&self, faction: GString) -> VarArray {
        let Some(state) = &self.state else {
            return VarArray::new();
        };
        let Ok(faction) = FactionId::new(faction.to_string()) else {
            return VarArray::new();
        };
        state
            .faction_characters(&faction)
            .iter()
            .map(|id| GString::from(id.as_str()).to_variant())
            .collect()
    }

    /// The whole skill tree, sorted by branch, tier, then id.
    #[func]
    fn get_skill_tree(&self) -> VarArray {
        let Some(data) = &self.data else {
            return VarArray::new();
        };
        let mut skills: Vec<&Skill> = data.skills.values().collect();
        skills.sort_by_key(|s| (branch_key(s.branch), s.tier, s.id.clone()));
        skills
            .into_iter()
            .map(|skill| skill_dict(skill).to_variant())
            .collect()
    }

    /// Skills `character` could learn now (prerequisites met, not learned).
    #[func]
    fn get_learnable(&self, character: GString) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        let Ok(id) = CharacterId::new(character.to_string()) else {
            return VarArray::new();
        };
        let points = state.character(&id).map_or(0, |c| c.skill_points);
        state
            .learnable_skills(data, &id)
            .iter()
            .filter(|skill| data.skills.get(*skill).is_some_and(|s| s.cost <= points))
            .map(|skill| GString::from(skill.as_str()).to_variant())
            .collect()
    }

    /// `[{id, name, age, faction}]` of valid spouses for `character`.
    #[func]
    fn get_marriage_candidates(&self, character: GString) -> VarArray {
        let (Some(state), Some(data)) = (&self.state, &self.data) else {
            return VarArray::new();
        };
        let Ok(id) = CharacterId::new(character.to_string()) else {
            return VarArray::new();
        };
        state
            .marriage_candidates(data, &id)
            .iter()
            .filter_map(|candidate| state.character_view(data, candidate))
            .map(|view| {
                vdict! {
                    "id" => view.id.as_str(),
                    "name" => view.name.as_str(),
                    "age" => i64::from(view.age),
                    "faction" => view.faction.as_str(),
                }
                .to_variant()
            })
            .collect()
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

/// Provinces crossed by a settlement path starting at `start`: consecutive
/// duplicates and the starting province are dropped (lot C4, v1 UI).
pub(crate) fn provinces_of_path(
    state: &CampaignState,
    start: &SettlementId,
    path: &[SettlementId],
) -> PackedStringArray {
    let mut last = state.settlement_province(start).cloned();
    let mut out = PackedStringArray::new();
    for step in path {
        if let Some(p) = state.settlement_province(step) {
            if last.as_ref() != Some(p) {
                out.push(p.as_str());
                last = Some(p.clone());
            }
        }
    }
    out
}

/// A settlement id, or the city of a province id (lot C4 compatibility).
pub(crate) fn settlement_or_city(state: &CampaignState, raw: &str) -> Option<SettlementId> {
    if let Ok(id) = SettlementId::new(raw) {
        return state.settlement_state(&id).map(|_| id);
    }
    let province = ProvinceId::new(raw).ok()?;
    state.province_city_id(&province).cloned()
}

/// Player-facing message for an order the simulation cannot parse (UI
/// audit A3: no raw serde error on screen). The technical detail goes to
/// the Godot log for developers.
pub(crate) fn invalid_order_message(error: &str) -> String {
    godot_warn!("order rejected by the bridge: {error}");
    "Cet ordre n'est pas reconnu par la simulation : la bibliothèque du jeu n'est sans doute \
     pas à jour. Relancez le jeu après l'avoir réinstallé."
        .to_owned()
}

pub(crate) fn order_result(result: Result<(), String>) -> VarDictionary {
    match result {
        Ok(()) => vdict! { "ok" => true, "error" => "" },
        Err(error) => vdict! { "ok" => false, "error" => error.as_str() },
    }
}

pub(crate) fn ids<'a>(iter: impl Iterator<Item = &'a (impl AsRef<str> + 'a)>) -> PackedStringArray {
    iter.map(|id| GString::from(id.as_ref())).collect()
}

fn unit_name(data: &GameData, unit: &Unit) -> String {
    data.unit_types.get(&unit.unit_type).map_or_else(
        || unit.unit_type.as_str().to_owned(),
        |t| t.name.display.clone(),
    )
}

pub(crate) fn units_array(data: &GameData, units: &[Unit]) -> VarArray {
    units
        .iter()
        .map(|unit| {
            vdict! {
                "unit_type" => unit.unit_type.as_str(),
                "name" => unit_name(data, unit).as_str(),
                "strength" => i64::from(unit.strength),
                "max_strength" => i64::from(unit.max_strength),
                "morale" => i64::from(unit.morale),
                "experience" => i64::from(unit.experience),
            }
            .to_variant()
        })
        .collect()
}

fn army_dict(state: &CampaignState, data: &GameData, army: &Army) -> VarDictionary {
    let general_name = army
        .general
        .as_ref()
        .map_or_else(String::new, |id| state.character_name(data, id));
    // Lot M2: crossings take the whole turn at once (no pending crossing).
    let embarked = false;
    // Lot M2 (compatibility until M4): "location" is the army's settlement,
    // or the nearest one in the field; "path" the destination of a march
    // spanning several turns.
    let location = state.army_anchor(data, army);
    let location_str = location.as_ref().map_or("", |s| s.as_str());
    let location_province = state.army_province(data, army);
    let destination: Vec<SettlementId> = match &army.destination {
        Some(sim_campaign::MoveTarget::Settlement(id)) if !army.planned_path.is_empty() => {
            vec![id.clone()]
        }
        _ => Vec::new(),
    };
    let path_provinces = location
        .as_ref()
        .map_or_else(PackedStringArray::new, |start| {
            provinces_of_path(state, start, &destination)
        });
    let lonlat = location
        .as_ref()
        .and_then(|id| data.settlements.get(id))
        .map_or(Vector2::ZERO, |s| {
            Vector2::new(s.lonlat[0] as f32, s.lonlat[1] as f32)
        });
    // Lot M4: free position on the map (map pixels), the settlement the
    // army stands in ("" in the field), points left and allowance, and the
    // corners of the rest of a multi-turn march.
    let grid = data.navgrid();
    let point = state.army_point(data, army);
    let planned_path: PackedVector2Array = army
        .planned_path
        .iter()
        .map(|cell| {
            let p = cell.center(grid);
            Vector2::new(p[0], p[1])
        })
        .collect();
    let destination_point = army
        .destination
        .as_ref()
        .filter(|_| !army.planned_path.is_empty())
        .and_then(|target| state.target_point(data, target))
        .map_or(Vector2::new(-1.0, -1.0), |p| Vector2::new(p[0], p[1]));
    let mut dict = vdict! {
        "position" => Vector2::new(point[0], point[1]),
        "settlement" => army.settlement().map_or("", |s| s.as_str()),
        "movement_left" => i64::from(army.movement_left),
        "movement_max" => i64::from(state.army_grid_allowance(data, army)),
        // Unit roster: points left as km of plain (10 points = one plain cell).
        "movement_km" => f64::from(army.movement_left) / f64::from(data_model::PLAIN_COST)
            * grid.cell_km,
        "planned_path" => &planned_path,
        "destination_point" => destination_point,
    };
    dict.extend_dictionary(
        &vdict! {
            "embarked" => embarked,
            "faction" => army.faction.as_str(),
            "general" => army.general.as_ref().map_or("", |id| id.as_str()),
            "general_name" => general_name.as_str(),
            "location" => location_str,
            "location_province" => location_province.as_ref().map_or("", |p| p.as_str()),
            "location_lonlat" => lonlat,
            "path_provinces" => &path_provinces,
            "units" => &units_array(data, &army.units),
            "movement_points" => i64::from(army.movement_left),
            "supply" => i64::from(army.supply),
            "stance" => stance_key(army.stance),
            "path" => &ids(destination.iter()),
        },
        true,
    );
    dict
}

fn stance_key(stance: sim_campaign::Stance) -> &'static str {
    stance.key()
}

fn tax_rate_key(rate: TaxRate) -> &'static str {
    match rate {
        TaxRate::Low => "low",
        TaxRate::Normal => "normal",
        TaxRate::High => "high",
    }
}

fn resource_category_key(category: ResourceCategory) -> &'static str {
    match category {
        ResourceCategory::Food => "food",
        ResourceCategory::RawMaterial => "raw_material",
        ResourceCategory::Manufactured => "manufactured",
        ResourceCategory::Luxury => "luxury",
    }
}

fn building_category_key(category: data_model::BuildingCategory) -> &'static str {
    match category {
        data_model::BuildingCategory::Production => "production",
        data_model::BuildingCategory::Commerce => "commerce",
        data_model::BuildingCategory::Military => "military",
        data_model::BuildingCategory::Religious => "religious",
        data_model::BuildingCategory::Sanitary => "sanitary",
        data_model::BuildingCategory::Fortification => "fortification",
    }
}

fn population_class_dict(entry: &PopulationClass) -> VarDictionary {
    vdict! {
        "count" => entry.count as i64,
        "unrest" => i64::from(entry.unrest),
        "health" => i64::from(entry.health),
        "wealth" => i64::from(entry.wealth),
        "goods_satisfaction" => i64::from(entry.goods_satisfaction),
    }
}

fn population_classes_dict(classes: &data_model::PopulationClasses) -> VarDictionary {
    vdict! {
        "peasants" => &population_class_dict(&classes.peasants),
        "burghers" => &population_class_dict(&classes.burghers),
        "clergy" => &population_class_dict(&classes.clergy),
        "nobility" => &population_class_dict(&classes.nobility),
    }
}

fn building_summary_dict(data: &GameData, id: &BuildingId) -> VarDictionary {
    let Some(building) = data.buildings.get(id) else {
        return vdict! {
            "id" => id.as_str(),
            "name" => id.as_str(),
            "category" => "",
            "upkeep" => 0,
        };
    };
    vdict! {
        "id" => id.as_str(),
        "name" => building.name.display.as_str(),
        "category" => building_category_key(building.category),
        "upkeep" => i64::from(building.upkeep.unwrap_or(0)),
    }
}

pub(crate) fn buildings_array(data: &GameData, buildings: &[BuildingId]) -> VarArray {
    buildings
        .iter()
        .map(|id| building_summary_dict(data, id).to_variant())
        .collect()
}

pub(crate) fn construction_dict(data: &GameData, construction: &Construction) -> VarDictionary {
    let name = data.buildings.get(&construction.building).map_or_else(
        || construction.building.to_string(),
        |b| b.name.display.clone(),
    );
    vdict! {
        "building" => construction.building.as_str(),
        "name" => name.as_str(),
        "turns_left" => i64::from(construction.turns_left),
    }
}

fn build_option_dict(data: &GameData, option: &BuildOption) -> VarDictionary {
    let category = data
        .buildings
        .get(&option.building)
        .map_or("", |b| building_category_key(b.category));
    let mut imported = VarDictionary::new();
    for (resource, amount) in &option.imported {
        imported.set(resource.as_str(), i64::from(*amount));
    }
    vdict! {
        "building" => option.building.as_str(),
        "name" => option.name.as_str(),
        "category" => category,
        "cost" => i64::from(option.cost),
        "turns" => i64::from(option.turns),
        "available" => option.available,
        "reason" => option.reason.as_deref().unwrap_or(""),
        // B7c: livres of `cost` spent importing missing resources, and the
        // units imported.
        "import_cost" => i64::from(option.import_cost),
        "imported" => &imported,
    }
}

pub(crate) fn buildable_array(data: &GameData, options: &[BuildOption]) -> VarArray {
    options
        .iter()
        .map(|option| build_option_dict(data, option).to_variant())
        .collect()
}

fn effect_value_dict(value: EffectValue) -> VarDictionary {
    vdict! {
        "flat" => value.flat,
        "percent" => value.percent,
    }
}

fn effect_totals_dict(effects: &EffectTotals) -> VarDictionary {
    vdict! {
        "tax_income" => &effect_value_dict(effects.tax_income),
        "trade_income" => &effect_value_dict(effects.trade_income),
        "health" => &effect_value_dict(effects.health),
        "unrest" => &effect_value_dict(effects.unrest),
        "wealth" => &effect_value_dict(effects.wealth),
        "goods_satisfaction" => &effect_value_dict(effects.goods_satisfaction),
        "growth" => &effect_value_dict(effects.growth),
        "garrison" => &effect_value_dict(effects.garrison),
        "fortification_level" => &effect_value_dict(effects.fortification_level),
        "recruit_cost" => &effect_value_dict(effects.recruit_cost),
        "supply" => &effect_value_dict(effects.supply),
        "production" => &effect_value_dict(effects.production),
        "siege_resistance" => &effect_value_dict(effects.siege_resistance),
        "plague_resistance" => &effect_value_dict(effects.plague_resistance),
        // F1: effects aimed at one social class.
        "by_class" => &vdict! {
            "peasants" => &class_effects_dict(&effects.classes.peasants),
            "burghers" => &class_effects_dict(&effects.classes.burghers),
            "clergy" => &class_effects_dict(&effects.classes.clergy),
            "nobility" => &class_effects_dict(&effects.classes.nobility),
        },
    }
}

fn class_effects_dict(effects: &sim_campaign::buildings::ClassEffects) -> VarDictionary {
    vdict! {
        "wealth" => &effect_value_dict(effects.wealth),
        "health" => &effect_value_dict(effects.health),
        "unrest" => &effect_value_dict(effects.unrest),
        "goods_satisfaction" => &effect_value_dict(effects.goods_satisfaction),
        "growth" => &effect_value_dict(effects.growth),
    }
}

fn province_city_dict(data: &GameData, city: &ProvinceCity) -> VarDictionary {
    let mut dict = vdict! {
        "classes" => &population_classes_dict(&city.classes),
        "buildings" => &buildings_array(data, &city.buildings),
        "fortification_level" => i64::from(city.fortification_level),
        "capacity" => city.capacity as i64,
        "buildable" => &buildable_array(data, &city.buildable),
        "resources" => &ids(city.resources.iter()),
        "effects" => &effect_totals_dict(&city.effects),
    };
    if let Some(construction) = &city.construction {
        dict.set("construction", &construction_dict(data, construction));
    }
    dict
}

fn faction_economy_dict(economy: &FactionEconomy) -> VarDictionary {
    let mut goods = VarDictionary::new();
    for (resource, count) in &economy.goods {
        goods.set(resource.as_str(), i64::from(*count));
    }
    let goods_categories: PackedStringArray = economy
        .goods_categories
        .iter()
        .map(|category| GString::from(resource_category_key(*category)))
        .collect();
    vdict! {
        "treasury" => economy.treasury,
        "income" => economy.income,
        "projected_income" => economy.projected_income,
        "net_income" => economy.net_income(),
        "army_upkeep" => economy.army_upkeep,
        "building_upkeep" => economy.building_upkeep,
        "administration_upkeep" => economy.administration_upkeep,
        "table_upkeep" => economy.table_upkeep,
        "table_upkeep_last_turn" => economy.table_upkeep_last_turn,
        "coinage" => economy.coinage.key(),
        "price_level" => economy.price_level,
        "seigniorage" => economy.seigniorage,
        "recoinage" => economy.recoinage,
        "seigniorage_last_turn" => economy.seigniorage_last_turn,
        "recoinage_last_turn" => economy.recoinage_last_turn,
        "tax_rate" => tax_rate_key(economy.tax_rate),
        "goods" => &goods,
        "goods_categories" => &goods_categories,
        "trade_income" => economy.trade_income,
        "trade_income_last_turn" => economy.trade_income_last_turn,
    }
}

/// UI audit A3, lot U3: signed budget lines (`budget_lines`: `{key,
/// projected, last?, delta?, charge}`), the change of the projected balance
/// against the season just resolved (`net_change`, absent before the first
/// turn) and the purse history (`budget_history`: `{turn, treasury, net,
/// change, other}`, oldest first, at most 12 seasons).
fn budget_into_dict(
    dict: &mut VarDictionary,
    economy: &FactionEconomy,
    history: &[sim_campaign::economy_balance::BudgetRecord],
) {
    let last = history.last();
    let lines: VarArray = economy
        .budget_lines(last)
        .iter()
        .map(|line| {
            let mut row = vdict! {
                "key" => line.kind.key(),
                "projected" => line.projected,
                "charge" => line.kind.is_charge(),
            };
            if let Some(booked) = line.last {
                row.set("last", booked);
            }
            if let Some(delta) = line.delta() {
                row.set("delta", delta);
            }
            row.to_variant()
        })
        .collect();
    dict.set("budget_lines", &lines);
    if let Some(record) = last {
        dict.set("net_change", economy.net_income() - record.net());
    }
    let records: VarArray = history
        .iter()
        .map(|record| {
            vdict! {
                "turn" => i64::from(record.turn),
                "treasury" => record.treasury,
                "net" => record.net(),
                "change" => record.change(),
                "other" => record.other,
            }
            .to_variant()
        })
        .collect();
    dict.set("budget_history", &records);
}

pub(crate) fn events_array(events: &[GameEvent]) -> VarArray {
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

fn branch_key(branch: SkillBranch) -> &'static str {
    match branch {
        SkillBranch::Command => "command",
        SkillBranch::Governance => "governance",
        SkillBranch::Court => "court",
    }
}

fn role_label_fr(role: Role) -> &'static str {
    match role {
        Role::Ruler => "Souverain(e)",
        Role::Consort => "Conjoint(e) royal(e)",
        Role::Heir => "Héritier(ère)",
        Role::Prince => "Prince/Princesse",
        Role::Commander => "Commandant(e)",
        Role::Noble => "Noble",
        Role::Prelate => "Prélat",
        Role::Burgher => "Bourgeois(e)",
        Role::Exile => "Exilé(e)",
        Role::Claimant => "Prétendant(e)",
        Role::Regent => "Régent(e)",
    }
}

fn province_name(data: &GameData, id: &ProvinceId) -> String {
    data.provinces
        .get(id)
        .map_or_else(|| id.to_string(), |p| p.name.display.clone())
}

/// Current activity as shown by the court panel (its filters match on the
/// prefixes "général", "gouverneur" and the exact "à la cour").
fn activity_label(state: &CampaignState, data: &GameData, view: &CharacterView) -> String {
    if view.captive {
        return "Captif(ve)".to_owned();
    }
    if let Some(province) = &view.governor_of {
        return format!("gouverneur de {}", province_name(data, province));
    }
    if let Some(army) = view.army.as_ref().and_then(|a| state.army(a)) {
        let place = state
            .army_province(data, army)
            .map_or_else(String::new, |p| province_name(data, &p));

        return format!("général de l'armée en {place}");
    }
    "à la cour".to_owned()
}

pub(crate) fn effects_array(effects: &[Effect]) -> VarArray {
    effects
        .iter()
        .map(|effect| {
            let kind = serde_json::to_value(effect.effect)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default();
            let mode = serde_json::to_value(effect.mode)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default();
            let unit_category = effect
                .unit_category
                .and_then(|c| serde_json::to_value(c).ok())
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_default();
            vdict! {
                "kind" => kind.as_str(),
                "value" => effect.value,
                "mode" => mode.as_str(),
                "unit_category" => unit_category.as_str(),
                "class" => effect.class.map_or("", |c| c.key()),
            }
            .to_variant()
        })
        .collect()
}

fn skill_dict(skill: &Skill) -> VarDictionary {
    vdict! {
        "id" => skill.id.as_str(),
        "name" => skill.name.display.as_str(),
        "branch" => branch_key(skill.branch),
        "tier" => i64::from(skill.tier),
        "prerequisites" => &ids(skill.prerequisites.iter()),
        "cost" => i64::from(skill.cost),
        "description" => skill.description.as_str(),
        "effects" => &effects_array(&skill.effects),
    }
}

fn character_dict(state: &CampaignState, data: &GameData, view: &CharacterView) -> VarDictionary {
    let traits: VarArray = view
        .traits
        .iter()
        .map(|t| {
            let description = data
                .traits
                .get(&t.id)
                .map_or("", |d| d.description.as_str());
            vdict! {
                "id" => t.id.as_str(),
                "name" => t.name.as_str(),
                "category" => t.category.as_str(),
                "description" => description,
            }
            .to_variant()
        })
        .collect();
    let children: VarArray = view
        .children
        .iter()
        .map(|child| {
            vdict! {
                "id" => child.id.as_str(),
                "name" => child.name.as_str(),
                "age" => i64::from(child.age),
            }
            .to_variant()
        })
        .collect();
    let captor = state
        .characters
        .get(&view.id)
        .filter(|c| c.captive)
        .and_then(|c| c.captor.clone());
    let captor_name = captor.as_ref().map_or(String::new(), |f| {
        data.factions
            .get(f)
            .map_or_else(|| f.to_string(), |d| d.name.display.clone())
    });
    let ransom = if captor.is_some() {
        sim_campaign::ransom::ransom_amount(state, data, &view.id)
    } else {
        0
    };
    let terms = state
        .characters
        .get(&view.id)
        .and_then(|c| c.ransom_terms.clone())
        .unwrap_or_default()
        .key();
    let player = &state.player_faction;
    let ransom_action = match &captor {
        Some(captor) if captor == player => "release",
        Some(_)
            if state
                .characters
                .get(&view.id)
                .is_some_and(|c| &c.faction == player) =>
        {
            "pay"
        }
        _ => "",
    };
    let title = view
        .title
        .clone()
        .or_else(|| view.role.map(|r| role_label_fr(r).to_owned()))
        .unwrap_or_else(|| "Membre de la maison".to_owned());
    let learned = view.skills_learned.len();
    let opt = |id: &Option<CharacterId>| id.as_ref().map_or(String::new(), |i| i.to_string());
    vdict! {
        "id" => view.id.as_str(),
        "name" => view.name.as_str(),
        "epithet" => view.epithet.as_deref().unwrap_or(""),
        "sex" => match view.sex { data_model::Sex::Male => "male", data_model::Sex::Female => "female" },
        "age" => i64::from(view.age),
        "alive" => view.alive,
        "faction" => view.faction.as_str(),
        "house" => view.house.as_str(),
        "title" => title.as_str(),
        "role" => activity_label(state, data, view).as_str(),
        "skills" => &vdict! {
            "command" => i64::from(view.skills.command),
            "governance" => i64::from(view.skills.governance),
            "court" => i64::from(view.skills.court),
        },
        "experience" => i64::from(view.experience),
        "skill_points" => i64::from(view.skill_points),
        "xp_to_next" => i64::from(sim_campaign::skills::xp_for_next_point(learned)),
        "skills_learned" => &ids(view.skills_learned.iter()),
        "traits" => &traits,
        "spouse" => opt(&view.spouse).as_str(),
        "spouse_name" => view.spouse_name.as_deref().unwrap_or(""),
        "children" => &children,
        "father" => opt(&view.father).as_str(),
        "mother" => opt(&view.mother).as_str(),
        "location" => view.location.as_ref().map_or("", |p| p.as_str()),
        "army" => view.army.as_ref().map_or(String::new(), |a| a.to_string()).as_str(),
        "governor_of" => view.governor_of.as_ref().map_or("", |p| p.as_str()),
        "captive" => view.captive,
        // G1: captor, ransom and the ransom action the player may take.
        "captor" => captor.as_ref().map_or("", |f| f.as_str()),
        "captor_name" => captor_name.as_str(),
        "ransom" => ransom,
        "ransom_terms" => terms,
        "ransom_action" => ransom_action,
        "piety" => i64::from(view.piety),
        "prestige" => i64::from(view.prestige),
        // C7: year of death (0 while alive or unknown), retinue and its cap.
        "birth_year" => i64::from(state.characters.get(&view.id).map_or(0, |c| c.birth_year)),
        "death_year" => i64::from(state.characters.get(&view.id).and_then(|c| c.death_year).unwrap_or(0)),
        "retinue" => &crate::campaign_sim_retinue::retinue_array(state, data, &view.id),
        "retinue_max" => sim_campaign::retinue::max_per_character(data) as i64,
    }
}
