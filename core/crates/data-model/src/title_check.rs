//! Invariants of the feudal title registry (lot FE, spec § 3.1).
//!
//! Checked at load time, once `data/titles/` exists:
//! - every reference (liege, provinces, 1337 holder, primary title) resolves;
//! - a title's liege is of strictly higher rank (hence no cycle and at most
//!   three levels);
//! - every province belongs to exactly one title's own domain;
//! - every faction owning a province in 1337 has a primary title, which it
//!   holds in 1337.

use std::collections::BTreeMap;

use crate::ids::{ProvinceId, TitleId};
use crate::load::{DataError, GameData};

/// Validates the title registry; no-op when no title is loaded.
pub fn validate_titles(data: &GameData) -> Result<(), DataError> {
    let errors = title_errors(data);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(DataError::InvalidTitles(errors))
    }
}

/// Every broken invariant of the registry, as readable messages.
pub fn title_errors(data: &GameData) -> Vec<String> {
    let mut errors = Vec::new();
    if data.titles.is_empty() {
        return errors;
    }
    let mut domain_of: BTreeMap<&ProvinceId, Vec<&TitleId>> = BTreeMap::new();
    for (id, title) in &data.titles {
        if let Some(liege) = &title.de_jure_liege {
            match data.titles.get(liege) {
                None => errors.push(format!("{id}: unknown de_jure_liege {liege}")),
                Some(liege_title) if liege_title.rank <= title.rank => errors.push(format!(
                    "{id}: liege {liege} ({}) is not of higher rank than {}",
                    liege_title.rank.key(),
                    title.rank.key()
                )),
                Some(_) => {}
            }
        }
        if !data.factions.contains_key(&title.holder_1337.faction) {
            errors.push(format!(
                "{id}: unknown holder_1337 {}",
                title.holder_1337.faction
            ));
        }
        for province in &title.de_jure_provinces {
            if !data.provinces.contains_key(province) {
                errors.push(format!("{id}: unknown province {province}"));
            }
            domain_of.entry(province).or_default().push(id);
        }
    }
    for province in data.provinces.keys() {
        match domain_of.get(province).map(Vec::as_slice) {
            None | Some([]) => errors.push(format!("{province}: in no title")),
            Some([_]) => {}
            Some(many) => errors.push(format!(
                "{province}: in several titles ({})",
                many.iter()
                    .map(|t| t.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    }
    let owners: std::collections::BTreeSet<_> = data.provinces.values().map(|p| &p.owner).collect();
    for (id, faction) in &data.factions {
        match &faction.primary_title {
            None if owners.contains(id) => {
                errors.push(format!("{id}: owns provinces but has no primary_title"));
            }
            None => {}
            Some(primary) => match data.titles.get(primary) {
                None => errors.push(format!("{id}: unknown primary_title {primary}")),
                Some(title) if &title.holder_1337.faction != id => errors.push(format!(
                    "{id}: primary_title {primary} is held by {} in 1337",
                    title.holder_1337.faction
                )),
                Some(_) => {}
            },
        }
    }
    errors
}
