//! Loading `data/` from disk into [`GameData`] and validating cross-references.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;

use crate::entities::ai_alignment::AiAlignment;
use crate::entities::battle_order::BattleOrder;
use crate::entities::building::Building;
use crate::entities::character::Character;
use crate::entities::chivalric_order::ChivalricOrder;
use crate::entities::diet::Diet;
use crate::entities::event::Event;
use crate::entities::faction::Faction;
use crate::entities::names::NameList;
use crate::entities::province::Province;
use crate::entities::r#trait::Trait;
use crate::entities::religion::Religion;
use crate::entities::resource::Resource;
use crate::entities::settlement::{Settlement, SettlementEdge, SettlementRules};
use crate::entities::skill::Skill;
use crate::entities::technology::Technology;
use crate::entities::unit_type::UnitType;
use crate::ids::{
    BuildingId, CharacterId, ChivalricOrderId, DietId, EventId, FactionId, NamesId, ProvinceId,
    ReligionId, ResourceId, SettlementId, SkillId, TechnologyId, TraitId, UnitTypeId,
};
use crate::map::{MapMeta, ProvinceFeatureCollection, ProvinceGeometry};

/// Folder names under `data/`, one per entity type.
pub mod folders {
    pub const FACTIONS: &str = "factions";
    pub const PROVINCES: &str = "provinces";
    pub const UNIT_TYPES: &str = "unit_types";
    pub const BUILDINGS: &str = "buildings";
    pub const TECHNOLOGIES: &str = "technologies";
    pub const CHARACTERS: &str = "characters";
    pub const RESOURCES: &str = "resources";
    pub const RELIGIONS: &str = "religions";
    pub const TRAITS: &str = "traits";
    pub const SKILLS: &str = "skills";
    pub const NAMES: &str = "names";
    /// Chronicle events (M10); optional folder.
    pub const EVENTS: &str = "events";
    /// Leader's battle orders (F10b); optional folder.
    pub const BATTLE_ORDERS: &str = "battle_orders";
    /// Province diets (H3 « La Table »); optional folder.
    pub const DIETS: &str = "diets";
    /// Chivalric orders (H6); optional folder.
    pub const CHIVALRIC_ORDERS: &str = "chivalric_orders";
    /// Settlements, one file per province (lot C1); optional folder.
    pub const SETTLEMENTS: &str = "settlements";
    /// Tuning of the settlement rules, inside `settlements/`; optional.
    pub const SETTLEMENT_RULES: &str = "rules.json";
    /// Settlement movement graph, inside `map/`; optional.
    pub const SETTLEMENT_GRAPH: &str = "settlement_graph.json";
    /// AI tuning files (G4); optional folder.
    pub const AI: &str = "ai";
    /// Side-change tuning of the AI, inside `ai/`; optional.
    pub const AI_ALIGNMENT: &str = "alignment.json";
    pub const MAP: &str = "map";
    pub const MAP_META: &str = "map.json";
    pub const PROVINCE_GEOMETRY: &str = "provinces.geojson";
}

/// Non-fatal problem found while loading: the data is usable, but something
/// should be fixed (typically a reference to a province not yet on the map).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    /// Entity holding the reference, e.g. `fac_england`.
    pub entity: String,
    /// JSON path of the field, e.g. `capital` or `neighbors`.
    pub field: String,
    pub message: String,
}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}: {}", self.entity, self.field, self.message)
    }
}

/// A dangling reference to an entity type that must be fully defined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceError {
    pub entity: String,
    pub field: String,
    pub target: String,
}

impl fmt::Display for ReferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}.{} references unknown id {:?}",
            self.entity, self.field, self.target
        )
    }
}

/// Errors raised while loading data files.
#[derive(Debug, thiserror::Error)]
pub enum DataError {
    #[error("cannot read {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid JSON in {path}: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("{path}: file name {file_stem:?} does not match id {id:?}")]
    FileNameMismatch {
        path: PathBuf,
        file_stem: String,
        id: String,
    },
    #[error("duplicate id {id:?} in {path}")]
    DuplicateId { path: PathBuf, id: String },
    #[error("{path}: expected a GeoJSON {expected}, found {found:?}")]
    GeoJsonKind {
        path: PathBuf,
        expected: &'static str,
        found: String,
    },
    #[error("invalid event {id}: {message}")]
    InvalidEvent { id: String, message: String },
    #[error("{} dangling reference(s):\n{}", .0.len(), format_reference_errors(.0))]
    References(Vec<ReferenceError>),
}

fn format_reference_errors(errors: &[ReferenceError]) -> String {
    errors
        .iter()
        .map(|error| format!("  - {error}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every entity of the game, keyed by id, plus optional map geometry.
#[derive(Debug, Clone, Default)]
pub struct GameData {
    pub factions: BTreeMap<FactionId, Faction>,
    pub provinces: BTreeMap<ProvinceId, Province>,
    pub unit_types: BTreeMap<UnitTypeId, UnitType>,
    pub buildings: BTreeMap<BuildingId, Building>,
    pub technologies: BTreeMap<TechnologyId, Technology>,
    pub characters: BTreeMap<CharacterId, Character>,
    pub resources: BTreeMap<ResourceId, Resource>,
    pub religions: BTreeMap<ReligionId, Religion>,
    pub traits: BTreeMap<TraitId, Trait>,
    pub skills: BTreeMap<SkillId, Skill>,
    pub names: BTreeMap<NamesId, NameList>,
    /// Chronicle events (M10), empty when `data/events/` is absent.
    pub events: BTreeMap<EventId, Event>,
    /// Leader's battle orders, empty when `data/battle_orders/` is absent.
    pub battle_orders: BTreeMap<String, BattleOrder>,
    /// Province diets (H3), empty when `data/diets/` is absent.
    pub diets: BTreeMap<DietId, Diet>,
    /// Chivalric orders (H6), empty when `data/chivalric_orders/` is absent.
    pub chivalric_orders: BTreeMap<ChivalricOrderId, ChivalricOrder>,
    /// `data/map/map.json`, absent until the geo pipeline has run.
    pub map: Option<MapMeta>,
    /// `data/map/provinces.geojson`, empty until the geo pipeline has run.
    pub province_geometry: BTreeMap<ProvinceId, ProvinceGeometry>,
    /// Every settlement (lot C1): read from `data/settlements/<province>.json`,
    /// plus one city generated from `capital_city` for each province without
    /// a file (or without a `city` entry).
    pub settlements: BTreeMap<SettlementId, Settlement>,
    /// Settlement ids of each province, the `city` first then file order.
    pub settlements_by_province: BTreeMap<ProvinceId, Vec<SettlementId>>,
    /// `data/settlements/rules.json`, absent until written.
    pub settlement_rules: Option<SettlementRules>,
    /// Edges of `data/map/settlement_graph.json`, empty until `tools/geo` writes it.
    pub settlement_graph: Vec<SettlementEdge>,
    /// `data/ai/alignment.json` (G4), absent until written: the AI then
    /// makes no historical side change.
    pub ai_alignment: Option<AiAlignment>,
}

impl GameData {
    /// Loads every entity folder under `root` (the `data/` directory), then
    /// validates cross-references.
    ///
    /// Unknown province ids only produce warnings, because the province list
    /// stays partial until the full map exists. Any other dangling reference,
    /// duplicate id, or file whose name differs from its `id` is an error.
    pub fn load(root: &Path) -> Result<(GameData, Vec<Warning>), DataError> {
        let mut warnings = Vec::new();
        let mut data = GameData {
            factions: load_entities(&root.join(folders::FACTIONS), |f: &Faction| &f.id)?,
            provinces: load_entities(&root.join(folders::PROVINCES), |p: &Province| &p.id)?,
            unit_types: load_entities(&root.join(folders::UNIT_TYPES), |u: &UnitType| &u.id)?,
            buildings: load_entities(&root.join(folders::BUILDINGS), |b: &Building| &b.id)?,
            technologies: load_entities(&root.join(folders::TECHNOLOGIES), |t: &Technology| &t.id)?,
            characters: load_entities(&root.join(folders::CHARACTERS), |c: &Character| &c.id)?,
            resources: load_entities(&root.join(folders::RESOURCES), |r: &Resource| &r.id)?,
            religions: load_entities(&root.join(folders::RELIGIONS), |r: &Religion| &r.id)?,
            traits: load_entities(&root.join(folders::TRAITS), |t: &Trait| &t.id)?,
            skills: load_entities(&root.join(folders::SKILLS), |s: &Skill| &s.id)?,
            names: load_entities(&root.join(folders::NAMES), |n: &NameList| &n.id)?,
            events: BTreeMap::new(),
            battle_orders: BTreeMap::new(),
            diets: BTreeMap::new(),
            chivalric_orders: BTreeMap::new(),
            map: None,
            province_geometry: BTreeMap::new(),
            settlements: BTreeMap::new(),
            settlements_by_province: BTreeMap::new(),
            settlement_rules: None,
            settlement_graph: Vec::new(),
            ai_alignment: None,
        };
        let events_dir = root.join(folders::EVENTS);
        if events_dir.is_dir() {
            data.events = load_entities(&events_dir, |e: &Event| &e.id)?;
        }
        let orders_dir = root.join(folders::BATTLE_ORDERS);
        if orders_dir.is_dir() {
            data.battle_orders = load_entities(&orders_dir, |o: &BattleOrder| &o.id)?;
        }
        let diets_dir = root.join(folders::DIETS);
        if diets_dir.is_dir() {
            data.diets = load_entities(&diets_dir, |d: &Diet| &d.id)?;
        }
        let chivalric_dir = root.join(folders::CHIVALRIC_ORDERS);
        if chivalric_dir.is_dir() {
            data.chivalric_orders = load_entities(&chivalric_dir, |o: &ChivalricOrder| &o.id)?;
        }
        let alignment_path = root.join(folders::AI).join(folders::AI_ALIGNMENT);
        if alignment_path.is_file() {
            data.ai_alignment = Some(read_json(&alignment_path)?);
        }
        data.load_map(&root.join(folders::MAP), &mut warnings)?;
        data.load_settlements(root, &mut warnings)?;
        data.validate_references(&mut warnings)?;
        crate::event_check::validate_events(&data, &mut warnings)?;
        Ok((data, warnings))
    }

    /// Reads `map/map.json` and `map/provinces.geojson` when they exist.
    fn load_map(&mut self, map_dir: &Path, warnings: &mut Vec<Warning>) -> Result<(), DataError> {
        let meta_path = map_dir.join(folders::MAP_META);
        if meta_path.is_file() {
            self.map = Some(read_json(&meta_path)?);
        }
        let geometry_path = map_dir.join(folders::PROVINCE_GEOMETRY);
        if !geometry_path.is_file() {
            return Ok(());
        }
        let collection: ProvinceFeatureCollection = read_json(&geometry_path)?;
        if collection.kind != "FeatureCollection" {
            return Err(DataError::GeoJsonKind {
                path: geometry_path,
                expected: "FeatureCollection",
                found: collection.kind,
            });
        }
        for feature in collection.features {
            if feature.kind != "Feature" {
                return Err(DataError::GeoJsonKind {
                    path: geometry_path,
                    expected: "Feature",
                    found: feature.kind,
                });
            }
            let geometry = ProvinceGeometry::from(feature);
            let id = geometry.id.clone();
            if self
                .province_geometry
                .insert(id.clone(), geometry)
                .is_some()
            {
                return Err(DataError::DuplicateId {
                    path: geometry_path,
                    id: id.to_string(),
                });
            }
        }
        for id in self.province_geometry.keys() {
            if !self.provinces.contains_key(id) {
                warnings.push(Warning {
                    entity: folders::PROVINCE_GEOMETRY.to_owned(),
                    field: "features.id".to_owned(),
                    message: format!("geometry for unknown province {id}"),
                });
            }
        }
        for id in self.provinces.keys() {
            if !self.province_geometry.contains_key(id) {
                warnings.push(Warning {
                    entity: id.to_string(),
                    field: "geometry".to_owned(),
                    message: "province has no polygon in provinces.geojson".to_owned(),
                });
            }
        }
        Ok(())
    }

    /// Checks that every reference points to a loaded entity.
    fn validate_references(&self, warnings: &mut Vec<Warning>) -> Result<(), DataError> {
        let mut checker = ReferenceChecker {
            data: self,
            errors: Vec::new(),
            warnings,
        };
        checker.check_factions();
        checker.check_provinces();
        checker.check_unit_types();
        checker.check_buildings();
        checker.check_technologies();
        checker.check_characters();
        checker.check_religions();
        checker.check_traits();
        checker.check_skills();
        checker.check_diets();
        checker.check_chivalric_orders();
        if checker.errors.is_empty() {
            Ok(())
        } else {
            Err(DataError::References(checker.errors))
        }
    }
}

/// Walks every reference field of every entity and records dangling ones.
struct ReferenceChecker<'a> {
    data: &'a GameData,
    errors: Vec<ReferenceError>,
    warnings: &'a mut Vec<Warning>,
}

impl ReferenceChecker<'_> {
    /// Records an error if `target` is not a known key of `known`.
    fn require<K: Ord + fmt::Display, V>(
        &mut self,
        entity: &impl fmt::Display,
        field: &str,
        target: &K,
        known: &BTreeMap<K, V>,
    ) {
        if !known.contains_key(target) {
            self.errors.push(ReferenceError {
                entity: entity.to_string(),
                field: field.to_owned(),
                target: target.to_string(),
            });
        }
    }

    fn require_all<'k, K: Ord + fmt::Display + 'k, V>(
        &mut self,
        entity: &impl fmt::Display,
        field: &str,
        targets: impl IntoIterator<Item = &'k K>,
        known: &BTreeMap<K, V>,
    ) {
        for target in targets {
            self.require(entity, field, target, known);
        }
    }

    /// Provinces may be missing while the map is partial: warn instead of failing.
    fn province(&mut self, entity: &impl fmt::Display, field: &str, target: &ProvinceId) {
        if !self.data.provinces.contains_key(target) {
            self.warnings.push(Warning {
                entity: entity.to_string(),
                field: field.to_owned(),
                message: format!("unknown province {target} (map still partial)"),
            });
        }
    }

    fn check_factions(&mut self) {
        let data = self.data;
        for (id, faction) in &data.factions {
            if let Some(ruler) = &faction.ruler {
                self.require(id, "ruler", ruler, &data.characters);
            }
            if let Some(heir) = &faction.heir {
                self.require(id, "heir", heir, &data.characters);
            }
            self.province(id, "capital", &faction.capital);
            self.require(id, "religion", &faction.religion, &data.religions);
            if let Some(suzerain) = &faction.suzerain {
                self.require(id, "suzerain", suzerain, &data.factions);
            }
            self.require_all(
                id,
                "starting_technologies",
                &faction.starting_technologies,
                &data.technologies,
            );
            for relation in &faction.relations {
                self.require(id, "relations.faction", &relation.faction, &data.factions);
            }
            if let Some(victory) = &faction.victory {
                for objective in &victory.objectives {
                    use crate::entities::faction::ObjectiveCondition as C;
                    let (target, provinces): (Option<&FactionId>, &[ProvinceId]) = match &objective
                        .condition
                    {
                        C::ControlAll { provinces } | C::ControlCount { provinces, .. } => {
                            (None, provinces)
                        }
                        C::NoForeignControl { faction, provinces } => (Some(faction), provinces),
                        C::Subjugate { faction } => (Some(faction), &[]),
                        C::Independent => (None, &[]),
                    };
                    if let Some(target) = target {
                        self.require(id, "victory.faction", target, &data.factions);
                    }
                    for province in provinces {
                        self.require(id, "victory.provinces", province, &data.provinces);
                    }
                }
            }
            for claim in &faction.claims {
                if let Some(target) = &claim.faction {
                    self.require(id, "claims.faction", target, &data.factions);
                }
                if let Some(province) = &claim.province {
                    self.require(id, "claims.province", province, &data.provinces);
                }
            }
        }
    }

    fn check_provinces(&mut self) {
        let data = self.data;
        for (id, province) in &data.provinces {
            for neighbor in &province.neighbors {
                self.province(id, "neighbors", neighbor);
            }
            self.require_all(id, "resources", &province.resources, &data.resources);
            self.require(id, "owner", &province.owner, &data.factions);
            if let Some(overlord) = &province.overlord {
                self.require(id, "overlord", overlord, &data.factions);
            }
            if let Some(holder) = &province.holder {
                self.require(id, "holder", holder, &data.characters);
            }
            self.require(id, "religion", &province.religion, &data.religions);
            self.require_all(id, "buildings", &province.buildings, &data.buildings);
        }
    }

    fn check_unit_types(&mut self) {
        let data = self.data;
        for (id, unit) in &data.unit_types {
            if let Some(tech) = &unit.required_technology {
                self.require(id, "required_technology", tech, &data.technologies);
            }
            if let Some(building) = &unit.required_building {
                self.require(id, "required_building", building, &data.buildings);
            }
            self.require_all(
                id,
                "required_faction",
                &unit.required_faction,
                &data.factions,
            );
            self.require_all(
                id,
                "cost.resources",
                unit.cost.resources.keys(),
                &data.resources,
            );
        }
    }

    fn check_buildings(&mut self) {
        let data = self.data;
        for (id, building) in &data.buildings {
            if let Some(base) = &building.upgrades_from {
                self.require(id, "upgrades_from", base, &data.buildings);
            }
            if let Some(tech) = &building.required_technology {
                self.require(id, "required_technology", tech, &data.technologies);
            }
            if let Some(required) = &building.required_building {
                self.require(id, "required_building", required, &data.buildings);
            }
            if let Some(resource) = &building.required_resource {
                self.require(id, "required_resource", resource, &data.resources);
            }
            self.require_all(
                id,
                "enables_units",
                &building.enables_units,
                &data.unit_types,
            );
            self.require_all(
                id,
                "cost.resources",
                building.cost.resources.keys(),
                &data.resources,
            );
        }
    }

    fn check_technologies(&mut self) {
        let data = self.data;
        for (id, tech) in &data.technologies {
            self.require_all(id, "prerequisites", &tech.prerequisites, &data.technologies);
            self.require_all(id, "unlocks.units", &tech.unlocks.units, &data.unit_types);
            self.require_all(
                id,
                "unlocks.buildings",
                &tech.unlocks.buildings,
                &data.buildings,
            );
        }
    }

    fn check_characters(&mut self) {
        let data = self.data;
        for (id, character) in &data.characters {
            self.require(id, "faction", &character.faction, &data.factions);
            self.require_all(id, "traits", &character.traits, &data.traits);
            if let Some(family) = &character.family {
                for (field, relative) in family.references() {
                    self.require(id, field, relative, &data.characters);
                }
            }
            if let Some(location) = &character.starting_location {
                self.province(id, "starting_location", location);
            }
        }
    }

    fn check_religions(&mut self) {
        let data = self.data;
        for (id, religion) in &data.religions {
            if let Some(parent) = &religion.parent {
                self.require(id, "parent", parent, &data.religions);
            }
            if let Some(head) = &religion.head_faction {
                self.require(id, "head_faction", head, &data.factions);
            }
            for faction in &religion.historical_adherents {
                self.require(id, "historical_adherents", faction, &data.factions);
            }
            for province in &religion.origin_provinces {
                self.require(id, "origin_provinces", province, &data.provinces);
            }
        }
    }

    fn check_traits(&mut self) {
        let data = self.data;
        for (id, character_trait) in &data.traits {
            self.require_all(id, "opposites", &character_trait.opposites, &data.traits);
        }
    }

    fn check_skills(&mut self) {
        let data = self.data;
        for (id, skill) in &data.skills {
            self.require_all(id, "prerequisites", &skill.prerequisites, &data.skills);
        }
    }

    fn check_chivalric_orders(&mut self) {
        let data = self.data;
        for (id, order) in &data.chivalric_orders {
            if let Some(faction) = &order.faction {
                self.require(id, "faction", faction, &data.factions);
            }
        }
    }

    fn check_diets(&mut self) {
        let data = self.data;
        for (id, diet) in &data.diets {
            let requirements = &diet.requirements;
            self.require_all(
                id,
                "requirements.resources",
                &requirements.resources,
                &data.resources,
            );
            if let Some(tech) = &requirements.technology {
                self.require(id, "requirements.technology", tech, &data.technologies);
            }
            self.require_all(
                id,
                "requirements.any_building",
                &requirements.any_building,
                &data.buildings,
            );
        }
    }
}

/// Reads and deserializes one JSON file.
pub(crate) fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, DataError> {
    let text = fs::read_to_string(path).map_err(|source| DataError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&text).map_err(|source| DataError::Json {
        path: path.to_path_buf(),
        source,
    })
}

/// Lists `*.json` files directly under `dir`, sorted by path for determinism.
pub(crate) fn list_json_files(dir: &Path) -> Result<Vec<PathBuf>, DataError> {
    let entries = fs::read_dir(dir).map_err(|source| DataError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    let mut files = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|source| DataError::Io {
                path: dir.to_path_buf(),
                source,
            })?
            .path();
        if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// Reads every `*.json` file in `dir` into a map keyed by the entity id.
///
/// The file stem must equal the entity's id; ids must be unique. Sub-directories
/// and non-JSON files are ignored. A missing directory is an error.
pub fn load_entities<T, K>(
    dir: &Path,
    id_of: impl Fn(&T) -> &K,
) -> Result<BTreeMap<K, T>, DataError>
where
    T: DeserializeOwned,
    K: Ord + Clone + fmt::Display,
{
    let mut loaded = BTreeMap::new();
    for path in list_json_files(dir)? {
        let value: T = read_json(&path)?;
        let id = id_of(&value).clone();
        let file_stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default()
            .to_owned();
        if file_stem != id.to_string() {
            return Err(DataError::FileNameMismatch {
                path,
                file_stem,
                id: id.to_string(),
            });
        }
        if loaded.insert(id.clone(), value).is_some() {
            return Err(DataError::DuplicateId {
                path,
                id: id.to_string(),
            });
        }
    }
    Ok(loaded)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A throw-away `data/` tree; removed on drop.
    struct Fixture {
        root: PathBuf,
    }

    impl Fixture {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir()
                .join(format!("cent-ans-data-model-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            for folder in [
                folders::FACTIONS,
                folders::PROVINCES,
                folders::UNIT_TYPES,
                folders::BUILDINGS,
                folders::TECHNOLOGIES,
                folders::CHARACTERS,
                folders::RESOURCES,
                folders::RELIGIONS,
                folders::TRAITS,
                folders::SKILLS,
                folders::NAMES,
            ] {
                fs::create_dir_all(root.join(folder)).unwrap();
            }
            let fixture = Fixture { root };
            fixture.write(folders::RELIGIONS, "rel_catholic", RELIGION);
            fixture.write(folders::RESOURCES, "res_wheat", RESOURCE);
            fixture.write(folders::TECHNOLOGIES, "tech_masonry", TECHNOLOGY);
            fixture.write(folders::BUILDINGS, "bld_market", BUILDING);
            fixture.write(folders::TRAITS, "trait_proud", TRAIT);
            fixture.write(folders::SKILLS, "skill_hardiesse", SKILL);
            fixture.write(folders::NAMES, "names_fr", NAMES_LIST);
            fixture.write(folders::CHARACTERS, "chr_philippe_vi", CHARACTER);
            fixture.write(folders::FACTIONS, "fac_france", FACTION);
            fixture.write(folders::PROVINCES, "prov_normandie", PROVINCE);
            fixture
        }

        fn write(&self, folder: &str, file_stem: &str, json: &str) {
            fs::write(
                self.root.join(folder).join(format!("{file_stem}.json")),
                json,
            )
            .unwrap();
        }

        fn load(&self) -> Result<(GameData, Vec<Warning>), DataError> {
            GameData::load(&self.root)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    const RELIGION: &str =
        r#"{"id":"rel_catholic","name":{"display":"Catholicisme"},"kind":"church"}"#;
    const RESOURCE: &str =
        r#"{"id":"res_wheat","name":{"display":"Blé"},"category":"food","base_price":4}"#;
    const TECHNOLOGY: &str = r#"{"id":"tech_masonry","name":{"display":"Maçonnerie"},"branch":"civil","tier":1,"cost":100,"prerequisites":[]}"#;
    const BUILDING: &str = r#"{"id":"bld_market","name":{"display":"Marché"},"category":"commerce","tier":1,"cost":{"money":500,"resources":{"res_wheat":1}},"build_time_turns":2,"effects":[{"effect":"trade_income","value":10,"mode":"percent"}]}"#;
    const TRAIT: &str = r#"{"id":"trait_proud","name":{"display":"Fier"},"category":"personality","effects":[{"effect":"prestige","value":3,"mode":"add"}],"description":"Orgueilleux."}"#;
    const SKILL: &str = r#"{"id":"skill_hardiesse","name":{"display":"Hardiesse"},"branch":"command","tier":1,"prerequisites":[],"cost":1,"effects":[{"effect":"battle_charge","value":10,"mode":"percent"}],"description":"Charge plus forte."}"#;
    const NAMES_LIST: &str = r#"{"id":"names_fr","language":"français médiéval","cultures":["cul_french"],"male_first_names":["Jehan"],"female_first_names":["Aliénor"]}"#;
    const CHARACTER: &str = r#"{"id":"chr_philippe_vi","name":{"display":"Philippe VI"},"sex":"male","house":"Valois","faction":"fac_france","role":"ruler","birth":{"value":"1293","uncertain":true},"skills":{"command":5,"governance":4,"court":6},"traits":["trait_proud"],"starting_location":"prov_normandie"}"#;
    const FACTION: &str = r##"{"id":"fac_france","name":{"display":"Royaume de France"},"government":"kingdom","playable":true,"ruler":"chr_philippe_vi","capital":"prov_normandie","religion":"rel_catholic","culture":"cul_french","succession_law":"salic","heraldry":{"blazon":"D'azur semé de fleurs de lis d'or.","primary_color":"#1F3A93"},"starting_technologies":["tech_masonry"]}"##;
    const PROVINCE: &str = r#"{"id":"prov_normandie","name":{"display":"Normandie","local":"Normendie","local_language":"ancien français"},"region":"france_nord","terrain":"bocage","neighbors":["prov_ile_de_france"],"coastal":true,"ports":["Rouen"],"resources":["res_wheat"],"capital_city":{"name":{"display":"Rouen"},"lat":49.44,"lon":1.1},"owner":"fac_france","holder":"chr_philippe_vi","culture":"cul_french","religion":"rel_catholic","population":{"classes":{"peasants":{"count":750000,"unrest":10,"health":55,"wealth":50,"goods_satisfaction":60},"burghers":{"count":130000,"unrest":10,"health":50,"wealth":65,"goods_satisfaction":70},"clergy":{"count":20000,"unrest":5,"health":60,"wealth":70,"goods_satisfaction":75},"nobility":{"count":6000,"unrest":10,"health":60,"wealth":75,"goods_satisfaction":80}},"uncertain":true},"buildings":["bld_market"]}"#;

    fn reference_errors(error: DataError) -> Vec<ReferenceError> {
        match error {
            DataError::References(errors) => errors,
            other => panic!("expected reference errors, got {other}"),
        }
    }

    #[test]
    fn minimal_fixture_loads_with_province_warning_only() {
        let fixture = Fixture::new("minimal");
        let (data, warnings) = fixture.load().unwrap();
        assert_eq!(data.factions.len(), 1);
        assert_eq!(
            data.provinces["prov_normandie"].population.classes.total(),
            906_000
        );
        assert!(data.provinces["prov_normandie"].population.uncertain);
        assert_eq!(
            warnings,
            vec![Warning {
                entity: "prov_normandie".into(),
                field: "neighbors".into(),
                message: "unknown province prov_ile_de_france (map still partial)".into(),
            }]
        );
    }

    #[test]
    fn province_without_settlement_file_gets_a_city() {
        let fixture = Fixture::new("fallback-city");
        let (data, _) = fixture.load().unwrap();
        let normandie = ProvinceId::new("prov_normandie").unwrap();
        let city = data.province_city(&normandie).expect("generated city");
        assert_eq!(city.id.as_str(), "set_rouen");
        assert_eq!(city.lonlat, [1.1, 49.44]);
        assert_eq!(city.weight, 100);
        assert!(city.port);
        assert_eq!(city.buildings, data.provinces[&normandie].buildings);
        assert_eq!(data.settlements.len(), 1);
    }

    #[test]
    fn settlement_file_problems_are_warnings() {
        let fixture = Fixture::new("settlement-warnings");
        fs::create_dir_all(fixture.root.join(folders::SETTLEMENTS)).unwrap();
        fixture.write(
            folders::SETTLEMENTS,
            "prov_normandie",
            r#"[{"id":"set_caen","province":"prov_normandie","kind":"town","name":{"display":"Caen"},"lonlat":[-0.37,49.18],"weight":30,"owner":"fac_atlantis","fortification_level":2,"buildings":["bld_market","bld_unknown"]},
               {"id":"set_caen","province":"prov_normandie","kind":"village","name":{"display":"Caen bis"},"lonlat":[-0.3,49.1],"weight":5,"fortification_level":0},
               {"id":"set_x","province":"prov_atlantis","kind":"village","name":{"display":"X"},"lonlat":[0.0,49.0],"weight":5,"fortification_level":0}]"#,
        );
        let (data, warnings) = fixture.load().unwrap();
        let normandie = ProvinceId::new("prov_normandie").unwrap();
        let ids: Vec<_> = data
            .province_settlements(&normandie)
            .iter()
            .map(|s| s.id.as_str())
            .collect();
        assert_eq!(ids, ["set_rouen", "set_caen"], "generated city first");
        let caen = &data.settlements["set_caen"];
        assert_eq!(caen.owner, None);
        assert_eq!(caen.buildings, [BuildingId::new("bld_market").unwrap()]);
        let fields: Vec<_> = warnings
            .iter()
            .filter(|w| w.entity.starts_with("settlements/") || w.entity == "prov_normandie")
            .map(|w| w.field.as_str())
            .collect();
        for expected in ["owner", "buildings", "id", "province", "settlements"] {
            assert!(
                fields.contains(&expected),
                "missing {expected} warning in {fields:?}"
            );
        }
    }

    #[test]
    fn unknown_faction_owner_is_an_error() {
        let fixture = Fixture::new("owner");
        fixture.write(
            folders::PROVINCES,
            "prov_normandie",
            &PROVINCE.replace("\"owner\":\"fac_france\"", "\"owner\":\"fac_atlantis\""),
        );
        let errors = reference_errors(fixture.load().unwrap_err());
        assert_eq!(
            errors,
            vec![ReferenceError {
                entity: "prov_normandie".into(),
                field: "owner".into(),
                target: "fac_atlantis".into(),
            }]
        );
    }

    #[test]
    fn unknown_religion_resource_technology_and_character_are_errors() {
        let fixture = Fixture::new("multi");
        fixture.write(
            folders::FACTIONS,
            "fac_france",
            &FACTION
                .replace("rel_catholic", "rel_cathar")
                .replace("tech_masonry", "tech_printing")
                .replace("\"ruler\":\"chr_philippe_vi\"", "\"ruler\":\"chr_nobody\""),
        );
        fixture.write(
            folders::BUILDINGS,
            "bld_market",
            &BUILDING.replace("res_wheat", "res_spice"),
        );
        let errors = reference_errors(fixture.load().unwrap_err());
        let mut fields: Vec<_> = errors
            .iter()
            .map(|error| format!("{}.{}={}", error.entity, error.field, error.target))
            .collect();
        fields.sort();
        assert_eq!(
            fields,
            vec![
                "bld_market.cost.resources=res_spice",
                "fac_france.religion=rel_cathar",
                "fac_france.ruler=chr_nobody",
                "fac_france.starting_technologies=tech_printing",
            ]
        );
    }

    #[test]
    fn unknown_tech_prerequisite_and_building_requirement_are_errors() {
        let fixture = Fixture::new("prereq");
        fixture.write(
            folders::TECHNOLOGIES,
            "tech_masonry",
            &TECHNOLOGY.replace("\"prerequisites\":[]", "\"prerequisites\":[\"tech_ghost\"]"),
        );
        fixture.write(
            folders::BUILDINGS,
            "bld_market",
            &BUILDING.replace(
                "\"tier\":1",
                "\"tier\":1,\"required_building\":\"bld_ghost\",\"upgrades_from\":\"bld_stall\"",
            ),
        );
        let errors = reference_errors(fixture.load().unwrap_err());
        let mut targets: Vec<_> = errors.iter().map(|error| error.target.as_str()).collect();
        targets.sort_unstable();
        assert_eq!(targets, vec!["bld_ghost", "bld_stall", "tech_ghost"]);
    }

    #[test]
    fn unknown_character_faction_and_family_are_errors() {
        let fixture = Fixture::new("character");
        fixture.write(
            folders::CHARACTERS,
            "chr_philippe_vi",
            &CHARACTER.replace(
                "\"faction\":\"fac_france\"",
                "\"faction\":\"fac_mars\",\"family\":{\"children\":[\"chr_jean\"]}",
            ),
        );
        // fac_france.ruler still resolves; only the character's own references break.
        let errors = reference_errors(fixture.load().unwrap_err());
        let fields: Vec<_> = errors.iter().map(|error| error.field.as_str()).collect();
        assert_eq!(fields, vec!["faction", "family.children"]);
    }

    #[test]
    fn character_unknown_location_is_a_warning() {
        let fixture = Fixture::new("location");
        fixture.write(
            folders::CHARACTERS,
            "chr_philippe_vi",
            &CHARACTER.replace("prov_normandie", "prov_avalon"),
        );
        let (_, warnings) = fixture.load().unwrap();
        assert!(warnings
            .iter()
            .any(|warning| warning.entity == "chr_philippe_vi"
                && warning.field == "starting_location"));
    }

    #[test]
    fn file_name_must_match_id() {
        let fixture = Fixture::new("filename");
        fixture.write(folders::RESOURCES, "res_ble", RESOURCE);
        let error = fixture.load().unwrap_err();
        assert!(
            matches!(&error, DataError::FileNameMismatch { file_stem, id, .. } if file_stem == "res_ble" && id == "res_wheat"),
            "{error}"
        );
    }

    #[test]
    fn unknown_field_is_rejected() {
        let fixture = Fixture::new("drift");
        fixture.write(
            folders::RESOURCES,
            "res_wheat",
            &RESOURCE.replace("\"base_price\":4", "\"base_price\":4,\"colour\":\"gold\""),
        );
        let error = fixture.load().unwrap_err();
        assert!(matches!(error, DataError::Json { .. }), "{error}");
        assert!(error.to_string().contains("colour"));
    }

    #[test]
    fn wrong_id_prefix_is_rejected() {
        let fixture = Fixture::new("prefix");
        fixture.write(
            folders::PROVINCES,
            "prov_normandie",
            &PROVINCE.replace("\"owner\":\"fac_france\"", "\"owner\":\"prov_normandie\""),
        );
        let error = fixture.load().unwrap_err();
        assert!(error.to_string().contains("invalid FactionId"), "{error}");
    }

    #[test]
    fn missing_map_folder_is_fine_and_geometry_is_loaded_when_present() {
        let fixture = Fixture::new("map");
        let (data, _) = fixture.load().unwrap();
        assert!(data.map.is_none());
        assert!(data.province_geometry.is_empty());

        let map_dir = fixture.root.join(folders::MAP);
        fs::create_dir_all(&map_dir).unwrap();
        fs::write(
            map_dir.join(folders::MAP_META),
            r#"{"crs":"EPSG:3035","bounds_projected":[0,0,4096000,4096000],"size_px":[4096,4096],"meters_per_px":1000,"height_min_m":-200,"height_max_m":4800,"generated_by":"tools/geo"}"#,
        )
        .unwrap();
        fs::write(
            map_dir.join(folders::PROVINCE_GEOMETRY),
            r#"{"type":"FeatureCollection","features":[
                {"type":"Feature","properties":{"id":"prov_normandie","centroid":[10.5,20],"neighbors":["prov_ile_de_france"],"capital_px":[11,21]},"geometry":{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[0,0]]]}},
                {"type":"Feature","properties":{"id":"prov_ile_de_france","centroid":[30,40],"neighbors":["prov_normandie"],"capital_px":[31,41],"area_px2":12.5},"geometry":{"type":"MultiPolygon","coordinates":[]}}
            ]}"#,
        )
        .unwrap();

        let (data, warnings) = fixture.load().unwrap();
        let map = data.map.unwrap();
        assert_eq!(map.size_px, [4096, 4096]);
        assert_eq!(map.extra["generated_by"], "tools/geo");
        let geometry = &data.province_geometry["prov_normandie"];
        assert_eq!(geometry.centroid, [10.5, 20.0]);
        assert_eq!(geometry.capital_px, [11.0, 21.0]);
        assert_eq!(geometry.geometry["type"], "Polygon");
        assert_eq!(
            data.province_geometry["prov_ile_de_france"].extra["area_px2"],
            12.5
        );
        assert!(warnings.iter().any(|warning| warning
            .message
            .contains("geometry for unknown province prov_ile_de_france")));
    }

    #[test]
    fn missing_entity_directory_is_an_io_error() {
        let fixture = Fixture::new("missing-dir");
        fs::remove_dir_all(fixture.root.join(folders::RELIGIONS)).unwrap();
        assert!(matches!(fixture.load(), Err(DataError::Io { .. })));
    }

    const EVENT: &str = r#"{"id":"evt_test","title":"Essai","text":"Un texte.","kind":"historical","trigger":{"date":{"year":1340,"season":"summer"},"conditions":[{"type":"at_war","a":"fac_france","b":"fac_atlantis"}]},"scope":{"type":"faction","faction":"fac_france"},"options":[{"text":"Oui","effects":[{"type":"treasury","amount":100},{"type":"prestige","character":"ruler","amount":5}],"ai_weight":2}]}"#;

    fn write_event(fixture: &Fixture, json: &str) {
        fs::create_dir_all(fixture.root.join(folders::EVENTS)).unwrap();
        fixture.write(folders::EVENTS, "evt_test", json);
    }

    #[test]
    fn event_with_unknown_faction_loads_with_a_warning() {
        let fixture = Fixture::new("event-warning");
        write_event(&fixture, EVENT);
        let (data, warnings) = fixture.load().unwrap();
        assert_eq!(data.events.len(), 1);
        assert!(warnings
            .iter()
            .any(|w| w.entity == "evt_test" && w.message.contains("fac_atlantis")));
    }

    #[test]
    fn event_structure_errors_are_fatal() {
        let fixture = Fixture::new("event-structure");
        let four = EVENT.replace(
            r#""options":[{"text":"Oui""#,
            r#""options":[{"text":"a"},{"text":"b"},{"text":"c"},{"text":"Oui""#,
        );
        write_event(&fixture, &four);
        assert!(matches!(
            fixture.load(),
            Err(DataError::InvalidEvent { .. })
        ));
        let undated = EVENT.replace(r#""date":{"year":1340,"season":"summer"},"#, "");
        write_event(&fixture, &undated);
        assert!(matches!(
            fixture.load(),
            Err(DataError::InvalidEvent { .. })
        ));
    }

    #[test]
    fn schedule_event_is_validated() {
        let fixture = Fixture::new("event-schedule");
        let scheduling = |delay: u32, target: &str| {
            EVENT.replace(
                r#"{"type":"treasury","amount":100}"#,
                &format!(r#"{{"type":"schedule_event","event":"{target}","delay":{delay}}}"#),
            )
        };
        write_event(&fixture, &scheduling(0, "evt_other"));
        assert!(matches!(
            fixture.load(),
            Err(DataError::InvalidEvent { .. })
        ));
        write_event(&fixture, &scheduling(2, "evt_test"));
        assert!(matches!(
            fixture.load(),
            Err(DataError::InvalidEvent { .. })
        ));
        write_event(&fixture, &scheduling(2, "evt_other"));
        let (_, warnings) = fixture.load().unwrap();
        assert!(warnings
            .iter()
            .any(|w| w.field == "effects.event" && w.message.contains("evt_other")));
    }

    #[test]
    fn event_unknown_effect_field_is_rejected() {
        let fixture = Fixture::new("event-field");
        write_event(
            &fixture,
            &EVENT.replace(r#""amount":100"#, r#""amount":100,"oops":1"#),
        );
        assert!(matches!(fixture.load(), Err(DataError::Json { .. })));
        write_event(
            &fixture,
            &EVENT.replace(r#""type":"treasury""#, r#""type":"teleport""#),
        );
        assert!(matches!(fixture.load(), Err(DataError::Json { .. })));
    }
}
