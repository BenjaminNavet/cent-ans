//! Attribution des musiques tierces, affichée dans l'écran « Crédits » du menu principal.
//!
//! Données : `data/audio/credits.json` (validées par `data/schemas/audio_credits.schema.json`),
//! une entrée par piste de `data/audio/music.json`. Lues depuis `data/` comme le reste de l'audio,
//! donc présentes même si `CREDITS.md` (racine du dépôt) n'est pas livré avec le jeu.

use std::path::Path;

use bevy::prelude::warn;
use serde::Deserialize;

/// Une piste tierce et ce qu'il faut afficher pour la créditer.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MusicCredit {
    pub file: String,
    pub title: String,
    pub creators: String,
    pub licence: String,
    #[serde(default)]
    pub licence_url: Option<String>,
    pub source_url: String,
}

#[derive(Deserialize)]
struct CreditsFile {
    tracks: Vec<MusicCredit>,
}

/// Lit `audio/credits.json` sous `data_dir` ; liste vide (avec un avertissement) si absent ou invalide.
pub fn load(data_dir: &Path) -> Vec<MusicCredit> {
    let path = data_dir.join("audio/credits.json");
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => {
            warn!("credits: {}: {error}", path.display());
            return Vec::new();
        }
    };
    match serde_json::from_str::<CreditsFile>(&text) {
        Ok(file) => file.tracks,
        Err(error) => {
            warn!("credits: {} invalide: {error}", path.display());
            Vec::new()
        }
    }
}

/// Ligne affichée pour une piste : « œuvre — interprètes. Licence (URL de licence) ».
pub fn credit_line(credit: &MusicCredit) -> String {
    let licence = match &credit.licence_url {
        Some(url) => format!("{} ({url})", credit.licence),
        None => credit.licence.clone(),
    };
    format!("« {} » — {}. {licence}", credit.title, credit.creators)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data_dir() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../data")
    }

    #[test]
    fn shipped_credits_load_and_cover_kevin_macleod() {
        let credits = load(&data_dir());
        assert!(
            !credits.is_empty(),
            "data/audio/credits.json vide ou illisible"
        );
        let macleod: Vec<_> = credits
            .iter()
            .filter(|credit| credit.file.contains("kevin_macleod"))
            .collect();
        assert_eq!(macleod.len(), 17);
        for credit in macleod {
            assert_eq!(credit.creators, "Kevin MacLeod (incompetech.com)");
            assert_eq!(credit.licence, "CC BY 4.0");
            assert!(credit.licence_url.is_some());
        }
    }

    #[test]
    fn credit_line_names_title_creators_and_licence() {
        let credit = MusicCredit {
            file: "assets/third_party/music/kevin_macleod/galway.mp3".into(),
            title: "Galway".into(),
            creators: "Kevin MacLeod (incompetech.com)".into(),
            licence: "CC BY 4.0".into(),
            licence_url: Some("https://creativecommons.org/licenses/by/4.0/".into()),
            source_url: "https://incompetech.com/music/royalty-free/music.html".into(),
        };
        assert_eq!(
            credit_line(&credit),
            "« Galway » — Kevin MacLeod (incompetech.com). CC BY 4.0 (https://creativecommons.org/licenses/by/4.0/)"
        );
    }

    #[test]
    fn missing_file_gives_empty_list() {
        assert!(load(Path::new("/nonexistent-cent-ans-data")).is_empty());
    }
}
