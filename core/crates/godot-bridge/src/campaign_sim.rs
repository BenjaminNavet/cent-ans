//! `CampaignSim`: Godot-facing handle on a [`CampaignState`] (spec M2 § 2).
//!
//! The game data is loaded once per data directory and shared between
//! instances so that `load_from_string` works on a fresh object.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use data_model::{
    BuildingId, CharacterId, Effect, FactionId, GameData, PopulationClass, ProvinceId,
    ResourceCategory, Role, Skill, SkillBranch,
};
use godot::classes::RefCounted;
use godot::prelude::*;
use sim_campaign::{
    Army, ArmyId, BuildOption, CampaignState, CharacterView, Construction, EffectTotals,
    EffectValue, FactionEconomy, GameEvent, Order, ProvinceCity, TaxRate, Unit,
};

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
    pub(crate) data: Option<Arc<GameData>>,
    pub(crate) state: Option<CampaignState>,
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
        let Some(economy) = FactionId::new(id.to_string())
            .ok()
            .and_then(|id| state.faction_economy(data, &id))
        else {
            return VarDictionary::new();
        };
        faction_economy_dict(&economy)
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
        if let Some(siege) = &province.siege {
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
        // M9: every AI faction plays with the strategic planner.
        events_array(&state.end_turn_with(data, ai::plan_turn))
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

fn army_dict(state: &CampaignState, data: &GameData, army: &Army) -> VarDictionary {
    let general_name = army
        .general
        .as_ref()
        .map_or_else(String::new, |id| state.character_name(data, id));
    // M10 : a pending sea crossing (next step not a land neighbour) shows the cog model.
    let embarked = army.path.first().is_some_and(|next| {
        !sim_campaign::movement::land_neighbors(data, &army.location).contains(next)
    });
    vdict! {
        "embarked" => embarked,
        "faction" => army.faction.as_str(),
        "general" => army.general.as_ref().map_or("", |id| id.as_str()),
        "general_name" => general_name.as_str(),
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

fn buildings_array(data: &GameData, buildings: &[BuildingId]) -> VarArray {
    buildings
        .iter()
        .map(|id| building_summary_dict(data, id).to_variant())
        .collect()
}

fn construction_dict(data: &GameData, construction: &Construction) -> VarDictionary {
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
    vdict! {
        "building" => option.building.as_str(),
        "name" => option.name.as_str(),
        "category" => category,
        "cost" => i64::from(option.cost),
        "turns" => i64::from(option.turns),
        "available" => option.available,
        "reason" => option.reason.as_deref().unwrap_or(""),
    }
}

fn buildable_array(data: &GameData, options: &[BuildOption]) -> VarArray {
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
        "army_upkeep" => economy.army_upkeep,
        "building_upkeep" => economy.building_upkeep,
        "administration_upkeep" => economy.administration_upkeep,
        "tax_rate" => tax_rate_key(economy.tax_rate),
        "goods" => &goods,
        "goods_categories" => &goods_categories,
    }
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
        return format!(
            "général de l'armée en {}",
            province_name(data, &army.location)
        );
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
        "piety" => i64::from(view.piety),
        "prestige" => i64::from(view.prestige),
    }
}
