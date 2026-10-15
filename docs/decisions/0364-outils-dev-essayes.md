# ADR 0364 : outils de développement essayés (lot devtools2)

Statut : accepté (15/10/2026). Suite du relevé `docs/research/bevy-outils-productivite.md` (ADR 0362).

## Contexte

Le relevé listait quatre outils à essayer. Le joueur a demandé de les essayer, de rendre un verdict et d'adopter ce qui sert. Contraintes : rien hors de la feature cargo `dev` (aucun octet dans le binaire joueur), ni focus ni son (`--offscreen`, `CENT_ANS_MUTE=1`, lancement par `client/run_bg.sh`). Détails des mesures : `docs/wip/bm-devtools2.md`.

## Décisions

1. **`bevy_brp_extras` 0.23 : adopté** (derrière `dev`, `client/src/agent/devtools.rs`). Capture, clavier, texte, souris et arrêt propre d'un client en marche, sans le relancer. Complète les scripts (`--script`, `--dump-state`, `--shot`) qui restent la voie reproductible et sans réseau pour les preuves. Ajouts maison : méthode BRP `cent_ans/dump` (le JSON de `--dump-state`, à tout moment), port choisi par `CENT_ANS_BRP_PORT` (agents parallèles), `client/brp.sh` (appel curl). Limite : sous `--offscreen`, la capture « fenêtre principale » est noire ; il faut passer l'entité de caméra (`camera`).
2. **`bevy_brp_mcp` 0.23 : rejeté comme réglage partagé, documenté en option.** Il fonctionne contre un client lancé par `run_bg.sh` (outils appelés avec `port`), mais ses 52 définitions pèsent 125 000 caractères (environ 31 000 jetons) et font doublon avec `client/brp.sh` + `cent_ans/dump` à zéro jeton. Son `brp_launch` lance `cargo run` fenêtré (vole le focus) : interdit. Jamais dans un `.mcp.json` partagé ; une session de pilotage fin d'entités peut l'ajouter en local (voir la note wip).
3. **Mélange des schedules : adopté** (`CENT_ANS_SHUFFLE_SEED`, `client/tests/shuffle.sh`, test `client/tests/devtools.rs`). Natif Bevy 0.20 (`bevy/debug`, `shuffle_seed`), exécuteur mono-fil pour qu'une graine redonne le même ordre. `CENT_ANS_AMBIGUITIES=1` journalise en plus les paires de systèmes en conflit sans ordre (information, bruyant : 12 420 paires dans `Update`).
4. **`bevy_egui` 0.43 + `bevy_console` 0.19 : adoptés pour l'humain, non vérifiés à l'écran.** Console F8 ou touche sous Échap (²), commandes `dump-state`, `goto`, `wireframe`, `shadows`. F12 est pris par `map_screenshot` (le test `console_keys_are_free_in_the_default_keymap` garde F8 et ` libres). Les commandes sont prouvées sans fenêtre (`CENT_ANS_CONSOLE_DEMO`) ; l'affichage egui ne peut pas l'être hors écran.

## Conséquences

- `dev` implique désormais `remote`, `bevy/debug` et quatre dépendances optionnelles (`bevy_brp_extras`, `bevy_egui`, `bevy_console`, `clap`) : première compilation `dev` rallongée (environ 10 min, bevy recompilé avec `debug`). Le build sans `dev` ne les contient pas.
- `main.rs` : un appel `agent::devtools::tune_default_plugins` (identité hors `dev`) demande à wgpu `POLYGON_MODE_LINE` pour le fil de fer.
- Si bevy_egui s'attache à la caméra d'interface (première caméra), la console vit tant qu'elle vit : à surveiller si cette caméra devient temporaire.
