//! Loading of `data/settlements/` and `data/map/settlement_graph.json` (lot C1,
//! `docs/design/2026-09-24-echelle-colonies.md` § 3.1 and § 4.1).
//!
//! Every inconsistency is a [`Warning`], never an error, like province
//! references: the settlement files are written region by region, so the data
//! must stay playable while they are partial. Only malformed JSON fails.
//!
//! After loading, every known province has at least one settlement and exactly
//! one `city` at the head of its `settlements_by_province` list: provinces
//! without a file (or whose file has no `city`) receive a city generated from
//! `Province::capital_city`.

use std::collections::BTreeMap;
use std::path::Path;

use crate::entities::settlement::{city_id_for, Settlement, SettlementGraph, SettlementKind};
use crate::ids::{ProvinceId, SettlementId};
use crate::load::{folders, list_json_files, read_json, DataError, GameData, Warning};

/// Settlements a province file may hold (spec § 3.1: 3 to 6; 1 is the
/// generated fallback, tolerated in files too while research is under way).
pub const MIN_SETTLEMENTS_PER_PROVINCE: usize = 1;
pub const MAX_SETTLEMENTS_PER_PROVINCE: usize = 16;

impl GameData {
    /// The province's `city` (always present after [`GameData::load`] for a
    /// known province).
    pub fn province_city(&self, province: &ProvinceId) -> Option<&Settlement> {
        self.settlements_by_province
            .get(province)?
            .iter()
            .filter_map(|id| self.settlements.get(id))
            .find(|settlement| settlement.kind == SettlementKind::City)
    }

    /// Settlements of `province`, the city first then file order.
    pub fn province_settlements(&self, province: &ProvinceId) -> Vec<&Settlement> {
        self.settlements_by_province
            .get(province)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.settlements.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Reads settlements, rules and graph, then generates the fallback cities.
    pub(crate) fn load_settlements(
        &mut self,
        root: &Path,
        warnings: &mut Vec<Warning>,
    ) -> Result<(), DataError> {
        let dir = root.join(folders::SETTLEMENTS);
        if dir.is_dir() {
            for path in list_json_files(&dir)? {
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if file_name == folders::SETTLEMENT_RULES {
                    self.settlement_rules = Some(read_json(&path)?);
                    continue;
                }
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_owned();
                let entries: Vec<Settlement> = read_json(&path)?;
                self.add_settlement_file(&stem, entries, warnings);
            }
        }
        self.add_fallback_cities(warnings);
        self.check_settlement_counts(warnings);
        self.check_settlement_rules(warnings);

        let graph_path = root.join(folders::MAP).join(folders::SETTLEMENT_GRAPH);
        if graph_path.is_file() {
            let graph: SettlementGraph = read_json(&graph_path)?;
            for edge in graph.edges {
                let known = [&edge.from, &edge.to]
                    .into_iter()
                    .all(|id| self.settlements.contains_key(id));
                if known {
                    self.settlement_graph.push(edge);
                } else {
                    warnings.push(Warning {
                        entity: folders::SETTLEMENT_GRAPH.to_owned(),
                        field: "edges".to_owned(),
                        message: format!(
                            "edge {} -> {} references an unknown settlement, ignored",
                            edge.from, edge.to
                        ),
                    });
                }
            }
        }
        Ok(())
    }

    /// Adds the settlements of one `<province>.json` file, skipping entries
    /// that cannot be attached (unknown province, duplicate id).
    fn add_settlement_file(
        &mut self,
        stem: &str,
        entries: Vec<Settlement>,
        warnings: &mut Vec<Warning>,
    ) {
        let entity = format!("{}/{stem}.json", folders::SETTLEMENTS);
        for mut settlement in entries {
            let label = settlement_entity(&entity, &settlement.id);
            let warn = |warnings: &mut Vec<Warning>, field: &str, message: String| {
                warnings.push(Warning {
                    entity: label.clone(),
                    field: field.to_owned(),
                    message,
                });
            };
            if settlement.province.as_str() != stem {
                warn(
                    warnings,
                    "province",
                    format!("province {} differs from file name", settlement.province),
                );
            }
            if !self.provinces.contains_key(&settlement.province) {
                warn(
                    warnings,
                    "province",
                    format!("unknown province {}, ignored", settlement.province),
                );
                continue;
            }
            if self.settlements.contains_key(&settlement.id) {
                warn(
                    warnings,
                    "id",
                    "duplicate settlement id, ignored".to_owned(),
                );
                continue;
            }
            if let Some(owner) = &settlement.owner {
                if !self.factions.contains_key(owner) {
                    warn(
                        warnings,
                        "owner",
                        format!("unknown faction {owner}, province owner used"),
                    );
                    settlement.owner = None;
                }
            }
            let unknown: Vec<_> = settlement
                .buildings
                .iter()
                .filter(|b| !self.buildings.contains_key(*b))
                .cloned()
                .collect();
            for building in &unknown {
                warn(
                    warnings,
                    "buildings",
                    format!("unknown building {building}, ignored"),
                );
            }
            settlement.buildings.retain(|b| !unknown.contains(b));
            if !(1..=100).contains(&settlement.weight) {
                warn(
                    warnings,
                    "weight",
                    format!("weight {} outside 1-100", settlement.weight),
                );
            }
            if settlement.fortification_level > 4 {
                warn(
                    warnings,
                    "fortification_level",
                    format!("level {} above 4", settlement.fortification_level),
                );
            }
            let list = self
                .settlements_by_province
                .entry(settlement.province.clone())
                .or_default();
            if settlement.kind == SettlementKind::City && !list.is_empty() {
                // Keep the first city at the head of the list.
                let has_city = list.iter().any(|id| {
                    self.settlements
                        .get(id)
                        .is_some_and(|s| s.kind == SettlementKind::City)
                });
                if has_city {
                    list.push(settlement.id.clone());
                } else {
                    list.insert(0, settlement.id.clone());
                }
            } else {
                list.push(settlement.id.clone());
            }
            self.settlements.insert(settlement.id.clone(), settlement);
        }
    }

    /// Generates a city from `capital_city` for every province lacking one.
    fn add_fallback_cities(&mut self, warnings: &mut Vec<Warning>) {
        let mut generated = Vec::new();
        for (id, province) in &self.provinces {
            if self.province_city(id).is_some() {
                continue;
            }
            if self.settlements_by_province.contains_key(id) {
                warnings.push(Warning {
                    entity: id.to_string(),
                    field: "settlements".to_owned(),
                    message: "no settlement of kind city, one generated from capital_city"
                        .to_owned(),
                });
            }
            let settlement_id = self.fallback_city_id(id, &province.capital_city.name.display);
            generated.push(Settlement::fallback_city(province, settlement_id));
        }
        for city in generated {
            self.settlements_by_province
                .entry(city.province.clone())
                .or_default()
                .insert(0, city.id.clone());
            self.settlements.insert(city.id.clone(), city);
        }
    }

    /// `set_<slug of display name>`, or `set_<slug>_<province>` when taken
    /// (or `set_<province>` when the name has no usable character).
    fn fallback_city_id(&self, province: &ProvinceId, display_name: &str) -> SettlementId {
        let suffix = province
            .as_str()
            .strip_prefix(ProvinceId::PREFIX)
            .unwrap_or(province.as_str());
        let by_province = || {
            SettlementId::new(format!("{}{suffix}", SettlementId::PREFIX))
                .expect("province suffix is a valid id body")
        };
        match city_id_for(display_name) {
            Some(id) if !self.settlements.contains_key(&id) => id,
            Some(id) => {
                SettlementId::new(format!("{id}_{suffix}")).unwrap_or_else(|_| by_province())
            }
            None => by_province(),
        }
    }

    /// 1-16 settlements and exactly one city per province.
    fn check_settlement_counts(&self, warnings: &mut Vec<Warning>) {
        for (province, ids) in &self.settlements_by_province {
            let count = ids.len();
            if !(MIN_SETTLEMENTS_PER_PROVINCE..=MAX_SETTLEMENTS_PER_PROVINCE).contains(&count) {
                warnings.push(Warning {
                    entity: province.to_string(),
                    field: "settlements".to_owned(),
                    message: format!(
                        "{count} settlements, expected {MIN_SETTLEMENTS_PER_PROVINCE}-{MAX_SETTLEMENTS_PER_PROVINCE}"
                    ),
                });
            }
            let cities = ids
                .iter()
                .filter(|id| {
                    self.settlements
                        .get(*id)
                        .is_some_and(|s| s.kind == SettlementKind::City)
                })
                .count();
            if cities != 1 {
                warnings.push(Warning {
                    entity: province.to_string(),
                    field: "settlements".to_owned(),
                    message: format!("{cities} settlements of kind city, expected exactly 1"),
                });
            }
        }
    }

    /// Unit types of the starting garrisons must exist.
    fn check_settlement_rules(&self, warnings: &mut Vec<Warning>) {
        let Some(rules) = &self.settlement_rules else {
            return;
        };
        for (kind, units) in &rules.starting_garrison {
            for unit in units {
                if !self.unit_types.contains_key(unit) {
                    warnings.push(Warning {
                        entity: format!("{}/{}", folders::SETTLEMENTS, folders::SETTLEMENT_RULES),
                        field: format!("starting_garrison.{}", kind.key()),
                        message: format!("unknown unit type {unit}"),
                    });
                }
            }
        }
    }
}

fn settlement_entity(file: &str, id: &SettlementId) -> String {
    format!("{file}#{id}")
}

/// Settlement count per kind, handy for tests and logs.
pub fn count_by_kind<'a>(
    settlements: impl IntoIterator<Item = &'a Settlement>,
) -> BTreeMap<SettlementKind, usize> {
    let mut counts = BTreeMap::new();
    for settlement in settlements {
        *counts.entry(settlement.kind).or_insert(0) += 1;
    }
    counts
}
