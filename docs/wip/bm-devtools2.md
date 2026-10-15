# bm-devtools2 : outils de productivité essayés

Branche `bm/devtools2`. ADR 0364. Tout est derrière la feature `dev` (`client/src/agent/devtools.rs`, `client/Cargo.toml`). Mesures du 15/10, toujours `--offscreen` + `CENT_ANS_MUTE=1`, via `client/run_bg.sh`.

## 1. bevy_brp_extras 0.23 : ADOPTÉ

- Méthodes ajoutées à BRP (43 au total avec le BRP natif) : `brp_extras/screenshot`, `send_keys`, `type_text`, souris, manette, `shutdown`, `set_window_title`, `get_diagnostics`.
- Essais réels sur un client `--state Campaign --offscreen` : capture en 1280x800 écrite (PNG) ; `send_keys F4` ouvre puis ferme le panneau des technologies (vérifié par la section de dump `tech.open`) ; `shutdown` termine proprement (10 images) et `run_bg.sh` rend la main ; `type_text` + `Enter` atteignent bien le clavier du jeu.
- Piège : sous `--offscreen` les caméras rendent dans une image, donc la capture « fenêtre principale » (sans paramètre) est **noire** (1 couleur). Avec `camera: <entité>` (trouver l'entité par `world.query` sur `bevy_camera::camera::Camera`) la capture est correcte (51 000 couleurs). `--shot` reste plus simple pour une capture en une ligne.
- Comparaison avec `agent/` : les scripts restent la voie reproductible (preuves, smoke, CI). BRP gagne sur le pilotage interactif d'un client déjà lancé : un agent ouvre un panneau, lit l'état, capture, ferme, sans relancer (10-30 s par lancement). Ajouts maison pour que ce soit utile :
  - méthode `cent_ans/dump` : le JSON de `--dump-state` en direct (89 sections en Campaign) ;
  - `CENT_ANS_BRP_PORT` (défaut 15702) : plusieurs agents en parallèle ne se disputent plus le port ;
  - `client/brp.sh <méthode> [json]` : curl JSON-RPC, zéro jeton de schéma.
- Coût : une dépendance de plus (dev seulement), pas de coût au runtime hors `dev`.

## 2. bevy_brp_mcp 0.23 : REJETÉ en réglage partagé (option locale documentée)

- Installé : `cargo install bevy_brp_mcp --version 0.23.0 --root <dossier>` (binaire `bevy_brp_mcp`, ~3 min de compilation, installé dans le bac à sable du lot, pas dans `~/.cargo/bin`).
- Vérifié sur un client lancé par `run_bg.sh` (port 15742) : `brp_status` (app_name=cent-ans, port), `world_query`, `brp_extras_screenshot`, `brp_extras_send_keys`, `brp_execute cent_ans/dump`, `brp_shutdown` : tout répond. Tout outil prend `port`.
- **52 outils**, 125 538 caractères de définitions (environ 31 000 à 36 000 jetons si tout est chargé d'avance ; les plus lourds : `brp_launch` 4,9 k car., `world_query` 4,8 k, `brp_extras_screenshot` 4,0 k, `world_trigger_event` 3,7 k). Les outils manette (7), watches (4), logs (3) et gestes ne servent pas à Cent Ans. Pas de réglage pour en retirer.
- `brp_launch` lance `cargo run` fenêtré : vole le focus et fait du bruit, contraire à la règle joueur. Ne jamais l'utiliser ; lancer par `run_bg.sh` puis attacher le MCP avec `port`.
- Raison du rejet : `client/brp.sh` + `cent_ans/dump` donnent la même chose pour zéro jeton de schéma. Il reste utile pour une session de debug fin d'entités (type guides, mutation de composants).
- Activer en local pour une session (jamais dans un `.mcp.json` partagé) :
  1. `cargo install bevy_brp_mcp --version 0.23.0` ;
  2. `claude mcp add --scope local brp -- bevy_brp_mcp` (écrit dans la config locale de l'utilisateur, hors dépôt) ;
  3. lancer le client : `CENT_ANS_MUTE=1 CENT_ANS_FEATURES=dev CENT_ANS_PROFILE=debug CENT_ANS_BRP_PORT=15742 client/run_bg.sh --offscreen --state Campaign --exit-after 100000 > out/run.log 2>&1 &` ;
  4. passer `port: 15742` à chaque outil ; `claude mcp remove brp` après.

## 3. Mélange des schedules : ADOPTÉ

- Natif 0.20 : `ScheduleBuildSettings::shuffle_seed` (feature `bevy/debug`, ajoutée à `dev`). Mise en œuvre : `CENT_ANS_SHUFFLE_SEED=<n>` (`devtools::shuffle_all`) met la graine sur tous les schedules et force l'exécuteur mono-fil (reproductible).
- Test unitaire (`client/tests/devtools.rs`, `cargo test --features dev`) : 20 graines donnent plusieurs ordres, une chaîne `.chain()` est respectée, une graine redonne le même ordre.
- Tâche de CI/agent : `client/tests/shuffle.sh [-s "1 2 3"] [scénario...]` rejoue les scénarios de `client/tests/smoke/` (offscreen, muet) et échoue sur panique, `AGENT_FATAL`, ligne ERROR ou script qui n'atteint pas `exit`. Journaux dans `out/shuffle/`.
- **Résultat : aucun bogue d'ordre trouvé.** 8 graines (1 à 8) sur main_menu, campaign, battle, codex, faction_select, campaign_turns, crecy, siege et campaign_battle : tout passe. Deux faux positifs écartés : `campaign_battle` exige les variables `CENT_ANS_STAGE_BATTLE` etc. (ajoutées à `shuffle.sh`), et un `campaign` tué parce qu'une autre instance tournait en parallèle (passe à la relance). Réserve : la détection reste limitée par `auto_insert_apply_deferred` (TODO connu de Bevy) et par la couverture des scénarios ; ne remplace pas la relecture des ordres.
- Bonus `CENT_ANS_AMBIGUITIES=1` : journalise les paires de systèmes en conflit sans ordre. Campaign, 200 images : `Update` 12 420 paires, `Startup` 84, `PreUpdate` 72, `PostUpdate` 47, `Last` 29, `StateTransition` 9. Ressources les plus en cause dans `Update` : `Sim` (3 955 conflits), `BattleSession` (2 645), `Transform` (1 253), `Visibility` (986), `Node` (773), `Messages<ToastRequest>` (767), `Text` (542), `Messages<OpenPanel>` (238). Bruyant (le mélange n'a rien trouvé), à utiliser pour cibler un bogue précis, pas en CI.

## 4. bevy_egui 0.43 + bevy_console 0.19 : ADOPTÉS (humain), affichage non vérifié

- Touches : `F8` (ou `CENT_ANS_CONSOLE_KEY=<KeyCode>`) et la touche physique sous Échap (`Backquote`, `²` en AZERTY). **F12 refusée** : c'est l'action `map_screenshot` du `KeyMap`. Test `console_keys_are_free_in_the_default_keymap` : F8 et ` ne sont liées à aucune action par défaut, F12 l'est.
- `ConsoleConfiguration { block_keyboard: true, block_mouse: true }` : tant qu'egui veut le clavier (champ de saisie), `ButtonInput<KeyCode>` est vidé en `PreUpdate` pour que les raccourcis du jeu (KeyMap) ne partent pas ; clics bloqués sur la fenêtre egui.
- Commandes : `dump-state [fichier]` (sans fichier : sections et tailles ; chemin absolu conseillé, le répertoire courant de l'app est `/`), `goto <État>`, `wireframe` (bascule `WireframeConfig.global`, wgpu `POLYGON_MODE_LINE` demandé par `tune_default_plugins`), `shadows` (bascule `shadow_maps_enabled` des lumières directionnelles).
- Preuve sans fenêtre : `CENT_ANS_CONSOLE_DEMO="dump-state /abs/f.json;wireframe;shadows;goto bogus;goto codex"` entre une ligne toutes les 30 images dès la 90e et journalise les réponses (`CONSOLE ...`). Résultat : dump de 91 Ko écrit, `wireframe on`, `shadows on (1 lights)`, `unknown state bogus [failed]`, état final `Codex`.
- **Non vérifié** : le dessin egui lui-même. Sous `--offscreen` aucune fenêtre n'est dessinée (aucun `EguiContext` n'existe ; bevy_egui s'attache à la première caméra, ici la `UiCamera`, qui rend dans une image). À essayer par le joueur fenêtré : F8. Risque connu : le contexte vit avec cette caméra.
- Aucune interférence vue avec `KeyEdges` / Échap (la console ne consomme pas Échap).

## Restes

- Essai fenêtré de la console (joueur).
- `bevy_mod_debugdump` / `bevy-inspector-egui` : attendre 0.20 (voir le relevé). `schedule.graph` / `schedule.list` existent déjà dans le BRP natif de 0.20 (non essayés).
- Collision d'ADR : les lots launcher et devtools1 ont tous deux pris 0362 ; dataassets a pris 0363 ; ce lot prend 0364.
