# bm-uileft : restes de l'interface (uicamp, uibattle, hudfix)

Branche `bm/uileft` (fusionne `bm/hudfix`).

## Fait
- Cartes fixes sur `FitZone` : tutoriel (Left), traditions et montre des hommes d'armes (Side).
- Bouton « Menu » : message propre `menu::pause::TogglePause` (plus l'arête Échap : un panneau ouvert reste ouvert).
- Journal d'événements en haut à gauche (`campaign/hud/journal.rs`), replié au départ, premier enfant de la colonne du conseil ; lignes de `ChronicleLog` (le filtre de genre et l'onglet « Monde » restent dans la fenêtre Chronique).
- Bataille : panneau des formations de groupe replié dès qu'aucun déploiement ne tourne (même déploiement sauté) ; journal de bataille replié au départ ; sous-titre du chroniqueur en bulle sous le journal (colonne gauche) au lieu du toast (`audio/advisor.rs`, campagne inchangée).
- Cloche désactivée tant qu'une fenêtre modale ou le menu pause est ouvert (`refresh_modal`) ; filet d'or entre les pastilles et la cloche ; minicarte à l'échelle de sa zone (`fit_ratio`, 0,18 × 0,28 de l'écran, 0,2 à 1).

## Restes
- Journal de campagne : pas de filtre de genre ni d'onglet « Monde » dans le widget (dans la fenêtre Chronique).
- Bulle du chroniqueur sans bouton « fermer ».

## uileft2
- Journal du HUD : filtre par genre (`hud:journal_genre:<id>`) et bloc « Monde » (`hud:journal_world`), état partagé avec la fenêtre Chronique (`ChroniclePanel::{genre,set_genre,toggle_world}`), comme le `remembered_genre` statique de Godot. Dump `journal.{genre,world_open,world_lines,shown}`. Preuve : `client/src/campaign/hud/proof_journal.txt`.
- Commande agent `type <texte>` (un caractère par image, vers le champ focalisé) ; test `script_type_command`.
- Audit `keyboard_free` : raccourcis de campagne sans garde corrigés : économie (M / I) et menu des filtres de la minicarte (F). Les touches de développement d'`armies/orders.rs` et la bataille (sans champ de texte) restent telles quelles.
- Surcouches fixes (tutoriel, traditions, montre) déjà en `FitZone` depuis uileft.
- Écart : le bouton « Menu » reste sur le message `TogglePause` (déjà en place, un panneau ouvert reste ouvert) au lieu de réutiliser l'arête Échap de `KeyEdges`.
- Reste : bulle du chroniqueur sans bouton « fermer ».
