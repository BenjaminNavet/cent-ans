# Lot devtools (Bevy 0.20 dev tools)

## État
- Features Cargo : `bevy-devtools` (comprise dans `dev`) = `bevy/bevy_dev_tools` + `bevy/render_dev_tools` + `bevy/bevy_debug_stepping` ; `hotpatch` = `bevy/hotpatching`, séparée de `dev`.
- Code : `client/src/agent/bevy_tools.rs` (dev uniquement), branché dans `DevToolsPlugin`. Activation : `CENT_ANS_DEVTOOLS=diagnostics,render,stepping|all` ; commandes console `overlay`, `step on|off|frame|continue|status`. Rien d'actif par défaut.
- `free_camera` / `bevy_camera_controller` : non retenu (la caméra du jeu a ses contrôleurs ; un second se battrait avec eux). Le crate n'est d'ailleurs pas dans le registre local.
- Doc : section « Outils Bevy 0.20 » de `client/README.md`.

## Résultats
- `cargo clippy --features dev --all-targets -- -D warnings` : OK.
- `cargo check --features dev,hotpatch` : COMPILE avec `dynamic_linking` (typage seulement). Le fonctionnement réel du hotpatch (`dx serve --hotpatch`, sous-seconde + `bevy_dylib`) n'est PAS vérifié : demande l'outil `dx` (non installé) et un lancement fenêtré. À essayer par le joueur ; le Cargo.lock gagne les dépendances optionnelles `dioxus-devtools`/`subsecond` (aucune n'est liée sans `hotpatch`).
- Build joueur : `cargo tree -e features -i bevy` sans `dev` ne contient aucune de ces features (hotpatching, bevy_dev_tools, render_dev_tools, debug_stepping, free_camera).

## Reste
- Aucune capture ni lancement fenêtré fait (consigne). Le test unitaire `parse_requested` couvre l'analyse de la variable d'environnement.
