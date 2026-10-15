//! Declarative table of the settings screen: one [`Row`] per setting, grouped by [`Tab`].
//! Labels and tooltips are the French UI strings of `settings_menu.gd`.

use super::model::Settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Display,
    Audio,
    Interface,
    Game,
    Controls,
}

impl Tab {
    pub const ALL: [Tab; 5] = [
        Tab::Display,
        Tab::Audio,
        Tab::Interface,
        Tab::Game,
        Tab::Controls,
    ];

    /// Widget name of the tab button (also the `TabPane` name suffix).
    pub fn id(self) -> &'static str {
        match self {
            Tab::Display => "display",
            Tab::Audio => "audio",
            Tab::Interface => "interface",
            Tab::Game => "game",
            Tab::Controls => "controls",
        }
    }

    pub fn caption(self) -> &'static str {
        match self {
            Tab::Display => "Affichage",
            Tab::Audio => "Audio",
            Tab::Interface => "Interface",
            Tab::Game => "Jeu",
            Tab::Controls => "Commandes",
        }
    }
}

/// How a slider value is shown next to it.
#[derive(Debug, Clone, Copy)]
pub enum Format {
    Percent,
    Times,
}

impl Format {
    pub fn show(self, value: f32) -> String {
        match self {
            Format::Percent => format!("{} %", (value * 100.0).round() as i32),
            Format::Times => format!("×{value:.1}"),
        }
    }
}

pub enum Kind {
    Check(fn(&mut Settings) -> &mut bool),
    Slider {
        min: f32,
        max: f32,
        step: f32,
        format: Format,
        field: fn(&mut Settings) -> &mut f32,
    },
    /// One choice among `labels`; `get` is `None` when the stored value is not a listed choice.
    Choice {
        labels: &'static [&'static str],
        get: fn(&Settings) -> Option<usize>,
        set: fn(&mut Settings, usize),
    },
}

pub struct Row {
    pub tab: Tab,
    /// Unique key; widgets are named `settings:<key>` (choices: `settings:<key>:<index>`).
    pub key: &'static str,
    pub label: &'static str,
    pub tip: &'static str,
    pub kind: Kind,
}

macro_rules! check {
    ($tab:ident, $field:ident, $label:expr, $tip:expr) => {
        Row {
            tab: Tab::$tab,
            key: stringify!($field),
            label: $label,
            tip: $tip,
            kind: Kind::Check(|s| &mut s.$field),
        }
    };
}

macro_rules! slider {
    ($tab:ident, $field:ident, $label:expr, $tip:expr, $min:expr, $max:expr, $step:expr, $format:ident) => {
        Row {
            tab: Tab::$tab,
            key: stringify!($field),
            label: $label,
            tip: $tip,
            kind: Kind::Slider {
                min: $min,
                max: $max,
                step: $step,
                format: Format::$format,
                field: |s| &mut s.$field,
            },
        }
    };
}

macro_rules! choice {
    ($tab:ident, $field:ident, $label:expr, $tip:expr, [$($value:expr => $name:expr),+ $(,)?]) => {
        Row {
            tab: Tab::$tab,
            key: stringify!($field),
            label: $label,
            tip: $tip,
            kind: Kind::Choice {
                labels: &[$($name),+],
                get: |s| [$($value),+].iter().position(|v| s.$field == *v),
                set: |s, index| s.$field = [$($value),+][index],
            },
        }
    };
}

/// Same as `choice!` for `String` fields.
macro_rules! choice_s {
    ($tab:ident, $field:ident, $label:expr, $tip:expr, [$($value:expr => $name:expr),+ $(,)?]) => {
        Row {
            tab: Tab::$tab,
            key: stringify!($field),
            label: $label,
            tip: $tip,
            kind: Kind::Choice {
                labels: &[$($name),+],
                get: |s| [$($value),+].iter().position(|v| s.$field == *v),
                set: |s, index| s.$field = [$($value),+][index].to_owned(),
            },
        }
    };
}

pub static ROWS: &[Row] = &[
    // ---- Affichage ----
    check!(Display, fullscreen, "Plein écran", ""),
    choice!(Display, resolution, "Résolution (fenêtré)", "", [
        (1280, 720) => "1280 × 720", (1440, 900) => "1440 × 900", (1600, 900) => "1600 × 900",
        (1920, 1080) => "1920 × 1080", (2560, 1440) => "2560 × 1440"
    ]),
    check!(
        Display,
        vsync,
        "Synchronisation verticale",
        "Limite l’affichage à la fréquence de l’écran."
    ),
    choice_s!(Display, quality, "Qualité graphique",
        "Automatique : choisie selon la carte graphique détectée. Basse : sans anticrénelage, ombres simples. Moyenne : occlusion ambiante. Haute : lumière rebondie, brume volumétrique. Ultra : illumination globale.",
        ["auto" => "Automatique", "low" => "Basse", "medium" => "Moyenne", "high" => "Haute", "ultra" => "Ultra"]),
    choice_s!(Display, msaa, "Anticrénelage (MSAA)",
        "Automatique : suit la qualité graphique. Adoucit les arêtes des modèles au prix de la fluidité.",
        ["auto" => "Automatique", "off" => "Désactivé", "2x" => "2×", "4x" => "4×"]),
    check!(
        Display,
        shadows,
        "Ombres",
        "Ombres portées du soleil (taille et distance selon la qualité graphique)."
    ),
    choice!(Display, view_distance, "Distance de vue",
        "Multiplie la distance d’affichage des caméras : plus courte, plus de fluidité.",
        [0.5 => "Courte", 0.75 => "Moyenne", 1.0 => "Normale", 1.5 => "Longue"]),
    choice_s!(Display, texture_quality, "Qualité des textures",
        "Haute : sols et parcelles en 2k quand les paquets haute définition sont installés. Moyenne : paquets 1k seuls. Prise en compte au prochain chargement.",
        ["high" => "Haute", "medium" => "Moyenne"]),
    choice_s!(Display, upscale, "Mise à l’échelle",
        "Calcule l’image 3D en plus petit puis l’agrandit : plus d’images par seconde, image un peu plus douce. L’interface reste nette.",
        ["auto" => "Automatique", "off" => "Désactivée", "quality" => "Qualité", "performance" => "Performance"]),
    choice!(Display, ui_size, "Taille de l’interface",
        "Échelle automatique selon la hauteur de la fenêtre, multipliée par ce réglage.",
        [0.8 => "Très petite", 0.9 => "Petite", 1.0 => "Normale", 1.1 => "Grande", 1.25 => "Très grande"]),
    choice!(Display, text_size, "Taille du texte", "Agrandit les textes seuls, sans changer la taille des panneaux.",
        [0.9 => "Petite", 1.0 => "Normale", 1.15 => "Grande", 1.3 => "Très grande"]),
    // ---- Audio ----
    slider!(Audio, audio_master, "Général", "", 0.0, 1.0, 0.05, Percent),
    slider!(Audio, audio_music, "Musique", "", 0.0, 1.0, 0.05, Percent),
    slider!(
        Audio,
        audio_battle_music,
        "Musique de bataille",
        "",
        0.0,
        1.0,
        0.05,
        Percent
    ),
    slider!(
        Audio,
        audio_ambience,
        "Ambiance",
        "",
        0.0,
        1.0,
        0.05,
        Percent
    ),
    slider!(
        Audio,
        audio_battle_sfx,
        "Bataille",
        "",
        0.0,
        1.0,
        0.05,
        Percent
    ),
    slider!(
        Audio,
        audio_sfx,
        "Interface",
        "Effets sonores de l’interface et de la carte.",
        0.0,
        1.0,
        0.05,
        Percent
    ),
    slider!(Audio, audio_voice, "Voix", "", 0.0, 1.0, 0.05, Percent),
    check!(
        Audio,
        voice_advisor,
        "Conseiller",
        "Le chroniqueur commente le premier tour, la première bataille, le premier siège et les alertes importantes."
    ),
    check!(
        Audio,
        voice_barks,
        "Répliques des unités",
        "Les régiments répondent à la sélection et aux ordres, crient à la charge et en déroute."
    ),
    // ---- Interface ----
    check!(
        Interface,
        colorblind,
        "Mode daltonien",
        "Ajoute motifs et symboles aux couleurs."
    ),
    check!(
        Interface,
        reduce_motion,
        "Réduire les animations",
        "Supprime les fondus et les travellings de caméra."
    ),
    check!(
        Interface,
        high_contrast,
        "Contraste renforcé",
        "Encre plus sombre, parchemin plus clair, bords plus épais."
    ),
    check!(
        Interface,
        skip_loading,
        "Écran de chargement passable",
        "Les écrans illustrés de bataille ne restent plus 3 s s’ils sont prêts."
    ),
    check!(
        Interface,
        campaign_briefing,
        "Briefing de début de campagne",
        "Fenêtre du premier tour : objectifs, voisins hostiles, premiers conseils."
    ),
    check!(
        Interface,
        next_hint,
        "Conseil : que faire maintenant",
        "Encart en haut à gauche de la carte qui propose l’action la plus utile du moment."
    ),
    choice_s!(Interface, season_report, "Rapport de saison",
        "Si important : fenêtre seulement quand vous perdez ou prenez une place, ou qu’une bataille a eu lieu ; sinon une ligne discrète.",
        ["always" => "Toujours", "auto" => "Si important", "off" => "Jamais"]),
    choice_s!(Interface, confirm_end_turn, "Confirmer la fin du tour",
        "Si oubli : demande confirmation quand une armée n’a pas d’ordre, qu’un emplacement de construction est libre ou qu’aucune recherche n’est en cours.",
        ["off" => "Jamais", "warnings" => "Si oubli", "always" => "Toujours"]),
    choice_s!(Interface, news_filter, "Nouvelles reçues", "Lettres scellées et bandeau du haut. Le journal garde toutes les nouvelles.",
        ["interest" => "Voisins et puissances", "all" => "Toute l’Europe", "own" => "Mon royaume"]),
    choice!(Interface, decor_hover_delay, "Bulle du décor naturel",
        "Arbres, troupeaux et rochers : une bulle du Codex apparaît quand la souris reste immobile dessus.",
        [0.75 => "0,75 s", 1.5 => "1,5 s", 3.0 => "3 s", 0.0 => "Désactivée"]),
    // ---- Jeu ----
    check!(
        Game,
        edge_pan,
        "Défilement par les bords de l’écran",
        "Aussi basculé par F2 sur la carte."
    ),
    slider!(
        Game,
        camera_speed,
        "Vitesse de la caméra",
        "",
        0.4,
        2.5,
        0.1,
        Times
    ),
    check!(
        Game,
        fog_of_war,
        "Brouillard de guerre",
        "Provinces hors de vue voilées, armées étrangères masquées."
    ),
    check!(
        Game,
        stance_fill,
        "Lavis des positions diplomatiques",
        "En vue 3D, chaque province est teintée selon son détenteur."
    ),
    check!(
        Game,
        show_zoc,
        "Zones de contrôle ennemies",
        "Avec une armée sélectionnée, un cercle rouge marque la zone de contrôle de chaque armée ennemie en vue."
    ),
    choice!(Game, ai_moves_speed, "Vitesse des mouvements de l’IA",
        "En fin de tour, les armées des autres factions que vous voyez marchent sur la carte. Espace passe l’animation.",
        [1.0 => "×1", 2.0 => "×2", 4.0 => "×4"]),
    choice!(Game, autosave_interval, "Sauvegarde automatique", "Cinq emplacements tournants, plus une sauvegarde avant chaque bataille.",
        [0 => "Désactivée", 1 => "Chaque tour", 2 => "Tous les 2 tours", 4 => "Tous les 4 tours", 8 => "Tous les 8 tours"]),
    check!(
        Game,
        interactive_battles,
        "Livrer ses batailles en 3D",
        "Décoché : toutes les batailles du joueur sont résolues automatiquement."
    ),
    check!(
        Game,
        tutorial_enabled,
        "Tutoriel des premiers tours",
        "Guide pas à pas au début d’une nouvelle partie."
    ),
    choice!(Game, blood, "Sang", "Modéré : plus discret. Complet : démembrements sur les coups critiques.",
        [0 => "Désactivé", 1 => "Modéré", 2 => "Complet"]),
    choice!(Game, unit_size, "Taille des unités",
        "Figurines dessinées par soldat simulé : les effectifs et l’équilibre ne changent pas.",
        [0.5 => "Petite", 1.0 => "Normale", 1.5 => "Grande", 2.5 => "Ultra", 4.0 => "Épique"]),
    choice!(Game, max_figures, "Figurines maximum", "Plafond de figurines dessinées sur tout le champ de bataille.",
        [2000 => "2 000", 4000 => "4 000", 8000 => "8 000", 12000 => "12 000", 15000 => "15 000", 20000 => "20 000", 30000 => "30 000"]),
    check!(
        Game,
        auto_pause_on_alert,
        "Pause automatique sur alerte",
        "La bataille se met en pause quand une troupe est mise en déroute ou que le général tombe."
    ),
    check!(
        Game,
        cinematic,
        "Plan cinématique au premier choc",
        "Quelques secondes de caméra rapprochée sur le premier choc."
    ),
    check!(
        Game,
        cinematic_slowmo,
        "Ralenti du plan cinématique",
        "Le premier choc est montré au ralenti."
    ),
    check!(
        Game,
        dev_mode,
        "Mode développeur",
        "Textes d’outil visibles (identifiants, commandes)."
    ),
    // ---- Commandes ----
    choice_s!(Controls, keyboard_layout, "Disposition du clavier",
        "Change les lettres affichées sur les boutons et dans l’aide.",
        ["auto" => "Automatique", "azerty" => "AZERTY", "qwerty" => "QWERTY"]),
];

pub fn row(key: &str) -> Option<&'static Row> {
    ROWS.iter().find(|row| row.key == key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_listed_choices_and_keys_unique() {
        let defaults = Settings::default();
        for row in ROWS {
            if let Kind::Choice { get, labels, .. } = &row.kind {
                assert!(
                    get(&defaults).is_some(),
                    "{} default not in choices",
                    row.key
                );
                assert!(!labels.is_empty());
            }
            assert_eq!(ROWS.iter().filter(|other| other.key == row.key).count(), 1);
        }
    }

    #[test]
    fn choice_set_round_trips() {
        let mut settings = Settings::default();
        let resolution = row("resolution").unwrap();
        let Kind::Choice { get, set, .. } = &resolution.kind else {
            panic!()
        };
        set(&mut settings, 3);
        assert_eq!(settings.resolution, (1920, 1080));
        assert_eq!(get(&settings), Some(3));
    }
}
