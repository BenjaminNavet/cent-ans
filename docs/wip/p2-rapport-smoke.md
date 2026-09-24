# WIP — P2 : rapport de saison + batailles ; smoke Godot

Tâche (lot P2) : voir `docs/wip/finalisation.md` § « Défauts relevés »
(« Rapport de saison : n'inclut pas les batailles résolues par le dialogue
après la fin du tour ») + vérification/robustesse du smoke Godot.

## État

- **Cause trouvée** : `campaign_map.gd::_on_end_turn` appelle
  `flow.after_end_turn(events)` (qui construit et affiche le rapport de
  saison) **avant** `_offer_pending_battles()`. Les batailles interactives
  (`interactive_battles` on) ne sont encore que « en vue » à ce moment
  (événement `EventKind::Battle` côté `battle_request.rs::defer_player_battle`,
  déjà inclus dans `events`) ; leur résultat réel
  (`movement.rs::auto_fight`, texte « Bataille … Vainqueur : … ») n'arrive
  que plus tard, via `_on_battle_auto` (résolution auto) ou
  `_on_battle_returned` (combat joué), et n'était renvoyé qu'à
  `ui.add_events` (chronique) — jamais au rapport de saison déjà affiché.
  Bug purement côté Godot (`game/`), rien à changer côté `core/`.

- **Correctif** :
  - `game/scripts/ui/season_report.gd` : extraction du rendu dans `_render()`,
    ajout de `add_events(new_events, is_relevant)` (fusionne dans `groups` via
    le nouveau `merge_groups` statique, ré-affiche, rouvre le rapport si le
    joueur l'avait fermé) et `merge_groups(existing, additional)`.
  - `game/scripts/map/flow_controller.gd` : `report_late_events(events)`
    — fusionne dans `last_events` et appelle `season_report.add_events` si le
    réglage `interface/season_report` est actif.
  - `game/scripts/map/campaign_map.gd` : `_on_battle_auto` et
    `_on_battle_returned` appellent désormais `flow.report_late_events(events)`
    en plus de `ui.add_events(...)`.

- **Smoke Godot** : `core/build.sh` puis `godot --headless --path game --import`
  puis `godot --headless --path game --script res://tests/smoke.gd` étaient
  **déjà verts avant le correctif** (exit 0, aucun `FAIL`). L'assertion
  `projected_income should increase after construction` (finalisation.md /
  consigne) est déjà robuste dans le fichier actuel : `_run_city_economy`
  (smoke.gd ~l.298-381) ne fait qu'un `print` d'information sur la fenêtre de
  convergence du revenu après construction (dérive de fond IA/guerre déjà
  documentée en commentaire), le seul `_check` strict sur `projected_income`
  porte sur l'effet immédiat de l'impôt (même tour, sans dérive). Rien à
  changer ici.
  - Étendu `_run_flow` (smoke.gd, section F3) pour couvrir le défaut 1 :
    bataille mise en scène (`debug_stage_battle`, déjà utilisé par
    `_run_battle`) résolue automatiquement après un tour où le rapport de
    saison est déjà affiché ; vérifie que le rapport contient bien un
    événement de bataille avec un texte de résultat (« Vainqueur »), pas
    seulement l'annonce « en vue ».

## Prochaine étape

- `cd core && cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test` (aucun
  changement Rust prévu, mais à lancer si `core/` est touché entre-temps par
  d'autres sessions avant de committer ici — pas nécessaire pour ce lot qui ne
  touche que `game/`).
- Relancer le smoke complet après l'extension de `_run_flow` pour confirmer
  qu'il reste vert.
