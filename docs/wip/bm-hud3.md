# bm-hud3 : suite du HUD de campagne (client Bevy)

Branche `bm/hud3`, dossier `client/src/campaign/hud/` + `core/crates/game-api/src/ransoms.rs`. Détails : `client/README.md` § « HUD, lot hud3 ».

Fait : vue `game_api::ransoms::get_ransoms` (la fenêtre de rançons la consomme), minicarte (relief ombré cuit depuis la pyramide de terrain, brouillard par case avec lisière douce, frontières de royaume, clic droit = ordre de marche de l'armée choisie), bouton « Objectifs » et bouton « Traditions (n) » de l'armée choisie (`OpenObjectives` / `OpenTraditions`), cartouche d'attente de fin de tour (`wait.rs`), touches via `KeyMap` (`campaign_end_turn`, `campaign_end_turn_fast`, `help_open`, fiche F1 lue dans la carte des touches). Le « tracker de victoire » est le suivi de missions de `missions.rs` (déjà porté).

Restes : bulles d'armées / colonies (`map_hover_text.gd`) dans le lot armées ; touches de saut du rejeu IA (Espace / Échap / Entrée) restent fixes ; minicarte : décoration d'icônes des boutons de mode, adaptation à la fenêtre (`fit_to`), cadre de vue de la caméra (`set_view_frame`) ; clic droit non exercé par un vrai clic (script seulement).
