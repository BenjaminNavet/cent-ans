//! Helpers partagés par les tests des autres crates (feature `test-support`).

use std::path::PathBuf;
use std::sync::OnceLock;

use crate::{FactionId, GameData, ProvinceId};

/// Répertoire `data/` du dépôt.
pub fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../data")
}

/// Données de jeu réelles, chargées une seule fois par binaire de test.
/// Un test qui les modifie doit cloner (`game_data().clone()`).
pub fn game_data() -> &'static GameData {
    static DATA: OnceLock<GameData> = OnceLock::new();
    DATA.get_or_init(|| GameData::load(&data_dir()).expect("game data loads").0)
}

/// Identifiant de faction (panique si invalide).
pub fn fac(id: &str) -> FactionId {
    FactionId::new(id).unwrap()
}

/// Identifiant de province (panique si invalide).
pub fn prov(id: &str) -> ProvinceId {
    ProvinceId::new(id).unwrap()
}
