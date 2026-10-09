//! JSON persistence (spec § 1.5) and the crate's error type.

use data_model::FactionId;

use crate::state::{CampaignState, STATE_VERSION};

/// Oldest save version that [`MIGRATIONS`] can still bring up to date.
/// Raise [`STATE_VERSION`] together with a new entry in [`MIGRATIONS`]; lower
/// this constant only when an old step is deliberately dropped (ADR 0256).
pub const OLDEST_MIGRATABLE_VERSION: u32 = 9;

/// One step `vN -> vN+1` on the raw JSON tree. `MIGRATIONS[i]` upgrades a
/// save of version `OLDEST_MIGRATABLE_VERSION + i`; it must not touch
/// `state_version` (the chain sets it). Empty while no format change needs one.
pub const MIGRATIONS: &[fn(&mut serde_json::Value)] = &[];

/// Applies the migration chain from `from` up to [`STATE_VERSION`].
fn migrate_to_current(value: &mut serde_json::Value, from: u32) -> Result<(), CampaignError> {
    migrate_with(
        value,
        from,
        STATE_VERSION,
        OLDEST_MIGRATABLE_VERSION,
        MIGRATIONS,
    )
}

/// Chain runner, parameterised so tests can exercise it with fake steps.
pub(crate) fn migrate_with(
    value: &mut serde_json::Value,
    from: u32,
    to: u32,
    oldest: u32,
    steps: &[fn(&mut serde_json::Value)],
) -> Result<(), CampaignError> {
    for version in from..to {
        let step = version
            .checked_sub(oldest)
            .and_then(|i| steps.get(i as usize))
            .ok_or(CampaignError::OlderSave {
                found: from,
                expected: to,
            })?;
        step(value);
        value["state_version"] = serde_json::Value::from(version + 1);
    }
    Ok(())
}

/// Errors of campaign construction and persistence.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CampaignError {
    #[error("unknown faction {0}")]
    UnknownFaction(FactionId),
    #[error("faction {0} is not playable")]
    NotPlayable(FactionId),
    #[error("game data is missing {0}")]
    MissingData(String),
    #[error("cannot serialise campaign state: {0}")]
    Serialize(String),
    #[error("cannot parse campaign save: {0}")]
    Deserialize(String),
    #[error(
        "version de sauvegarde {found} non prise en charge (version attendue : {expected}) ; \
         cette partie a été créée avec une version différente du jeu"
    )]
    VersionMismatch { found: u32, expected: u32 },
    #[error(
        "sauvegarde d'une version antérieure (version {found}, version attendue : {expected}) : \
         elle ne peut plus être chargée"
    )]
    OlderSave { found: u32, expected: u32 },
}

impl CampaignState {
    /// Serialises the whole state (RNG included) as JSON.
    pub fn save_json(&self) -> String {
        serde_json::to_string(self).expect("campaign state is always serialisable")
    }

    /// Restores a state produced by [`CampaignState::save_json`].
    pub fn load_json(json: &str) -> Result<Self, CampaignError> {
        #[derive(serde::Deserialize)]
        struct Header {
            #[serde(default)]
            state_version: u32,
        }
        let header: Header =
            serde_json::from_str(json).map_err(|e| CampaignError::Deserialize(e.to_string()))?;
        if header.state_version < OLDEST_MIGRATABLE_VERSION {
            return Err(CampaignError::OlderSave {
                found: header.state_version,
                expected: STATE_VERSION,
            });
        }
        if header.state_version > STATE_VERSION {
            return Err(CampaignError::VersionMismatch {
                found: header.state_version,
                expected: STATE_VERSION,
            });
        }
        if header.state_version < STATE_VERSION {
            let mut value: serde_json::Value = serde_json::from_str(json)
                .map_err(|e| CampaignError::Deserialize(e.to_string()))?;
            migrate_to_current(&mut value, header.state_version)?;
            return serde_json::from_value(value)
                .map_err(|e| CampaignError::Deserialize(e.to_string()));
        }
        serde_json::from_str(json).map_err(|e| CampaignError::Deserialize(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn add_field(v: &mut serde_json::Value) {
        v["added"] = json!(1);
    }
    fn rename_field(v: &mut serde_json::Value) {
        v["renamed"] = v["added"].take();
    }

    #[test]
    fn chain_applies_steps_in_order() {
        let mut v = json!({"state_version": 3});
        migrate_with(&mut v, 3, 5, 3, &[add_field, rename_field]).unwrap();
        assert_eq!(v["state_version"], 5);
        assert_eq!(v["renamed"], 1);
    }

    #[test]
    fn chain_without_step_is_an_older_save_error() {
        let mut v = json!({"state_version": 3});
        let err = migrate_with(&mut v, 3, 5, 3, &[add_field]).unwrap_err();
        assert!(matches!(err, CampaignError::OlderSave { found: 3, .. }));
    }

    #[test]
    fn current_migrations_cover_the_whole_range() {
        assert_eq!(
            MIGRATIONS.len() as u32,
            STATE_VERSION - OLDEST_MIGRATABLE_VERSION
        );
    }
}
