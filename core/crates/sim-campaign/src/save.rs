//! JSON persistence (spec § 1.5) and the crate's error type.

use data_model::FactionId;

use crate::state::{CampaignState, STATE_VERSION};

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
        "sauvegarde d'une version antérieure à la refonte des colonies (version {found}, \
         version attendue : {expected}) : elle ne peut pas être chargée"
    )]
    PreSettlementSave { found: u32, expected: u32 },
    #[error(
        "sauvegarde d'une version antérieure au mouvement libre des armées (version {found}, \
         version attendue : {expected}) : elle ne peut pas être chargée"
    )]
    PreFreeMovementSave { found: u32, expected: u32 },
}

/// First state version with settlements (lot C4); older saves are refused.
pub const SETTLEMENTS_STATE_VERSION: u32 = 5;
/// First state version with free army movement (lot M2); older saves are
/// refused.
pub const FREE_MOVEMENT_STATE_VERSION: u32 = 6;

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
        if header.state_version < SETTLEMENTS_STATE_VERSION {
            return Err(CampaignError::PreSettlementSave {
                found: header.state_version,
                expected: STATE_VERSION,
            });
        }
        if header.state_version < FREE_MOVEMENT_STATE_VERSION {
            return Err(CampaignError::PreFreeMovementSave {
                found: header.state_version,
                expected: STATE_VERSION,
            });
        }
        if header.state_version != STATE_VERSION {
            return Err(CampaignError::VersionMismatch {
                found: header.state_version,
                expected: STATE_VERSION,
            });
        }
        serde_json::from_str(json).map_err(|e| CampaignError::Deserialize(e.to_string()))
    }
}
