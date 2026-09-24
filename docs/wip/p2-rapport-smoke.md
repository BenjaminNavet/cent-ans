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

## État final

- `core/build.sh` : OK (aucun changement Rust, ce lot ne touche que `game/`).
- `godot --headless --path game --import` : OK.
- `godot --headless --path game --script res://tests/smoke.gd` : **vert** (exit 0,
  aucun `FAIL`), y compris la nouvelle vérification P2 dans `_run_flow`
  (smoke.gd ~l.1293-1309) : bataille mise en scène (`debug_stage_battle`),
  résolue automatiquement après affichage du rapport de saison
  (`map._on_battle_auto`), rapport vérifié contenant le texte de résultat
  (« Vainqueur »), pas seulement l'annonce « en vue ».
- Régression vérifiée manuellement : en retirant temporairement l'appel
  `flow.report_late_events(events)` dans `_on_battle_auto`, ce nouveau check
  échoue bien (`smoke FAIL: flow: season report should include the resolved
  battle...`) — confirme qu'il couvre effectivement le défaut. Remis en place
  ensuite (`git checkout --`, aucun résidu).

## Points ouverts

- Le même correctif ne couvre que `_on_battle_auto` et `_on_battle_returned`
  (résolution auto et bataille jouée) ; c'est la totalité des chemins de
  résolution différée identifiés dans `campaign_map.gd`. Rien côté siège :
  `_on_battle_auto`/`_on_battle_returned` sont partagés bataille/siège (mêmes
  signaux `PreBattleDialog`), donc couvert aussi.
- Pas de test unitaire Rust ajouté : le bug était uniquement côté Godot
  (les événements `core/` existent déjà et sont corrects) — voir
  `movement.rs:702` (texte de résultat) vs `battle_request.rs:91` (« en
  vue »).
