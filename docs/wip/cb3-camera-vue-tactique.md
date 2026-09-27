# CB3 — Ralenti, caméra (lacet/tangage), vue tactique, `spotted`

Branche `feat/cb3-camera-tactical`, depuis `main` (cad1ca05 / 220a1e9f). Plan :
`docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` (§ Écarts point 2, Conventions
communes, CB3). Spec : `docs/superpowers/specs/2026-09-27-controles-bataille-tw-design.md` (§ CB3).

## État (dernier commit)

- [x] `battle_camera.gd` : bouton du milieu seul = lacet + tangage manuel (`manual_pitch`,
      `manual_pitch_deg`, bornes 5°-85°, `MANUAL_PITCH_DRAG_SPEED`) ; Maj + bouton du milieu =
      panoramique (ancien comportement) ; suivi de régiment inchangé (orbite lacet seul) ;
      `look_at_point` et `follow_unit` remettent `manual_pitch` à zéro (recentrage).
- [x] Vitesse ×0,5 (`SPEEDS` dans `battle_scene.gd` ; boutons/tooltips/icônes dans `battle_hud.gd`
      — `SPEED_TOOLTIPS`, boucle `_build_corner`, `_draw_speed_icon` avec un demi-triangle pour
      ×0,5). `HELP_TEXT` **non touché** (CB2 le réécrit).
- [x] Cœur : champ dérivé `BattleSim::spotted_by(target, side)` (`sim.rs`, réutilise `visible()` +
      `missile_arc.spotter_range_m` = 350 m, pas de nouveau fichier de données), test
      `cb3_spotted.rs` (5 tests, dont un qui vérifie l'absence d'effet sur `state_digest`).
      Exposé dans `get_units` (`godot-bridge/src/battle_sim.rs`) sous `spotted` (vrai si pas de
      `player_side`, càd bataille spectée).
- [x] `battle_tactical_view.gd` (Tab) : caméra du dessus (`manual_pitch_deg = 85°`, cadrée sur
      `terrain.field_center()`/`FIELD_W`/`FIELD_D`), calque `ColorRect` semi-transparent inséré en
      premier enfant de `hud.root` (assombrit tout sauf le HUD, pas de shader touché), pastilles
      B7 forcées (`BattleUnitMarkers.update(force_clustered)`), ennemis non `spotted` retirés
      (`filter_spotted`, fonction pure). Tab bascule (`battle_input.gd`), Échap sort d'abord de la
      vue tactique si active (sinon comportement existant). Contraintes de déploiement : aucun
      changement nécessaire (elles vivent dans `DeploymentController`, indépendant de la caméra).
- [x] Tests Godot : `zg4_camera_test.gd` étendu (bornes 5°-85°, suspension/reprise via
      `look_at_point`), `cb3_tactical_view_test.gd` (fonction pure `filter_spotted` + intégration
      Tab/Tab, Échap, restauration caméra, masquage des repères si le champ le permet).
- [x] Capture `cb3_tactical_shot.gd` (support `--out` et `--probe`, seul `--probe` exécuté ici).
- [x] Vérifications finales : `cargo fmt --all`, `cargo clippy --workspace --all-targets -D
      warnings`, `cargo test --workspace` (tout vert), release `ep13_replay` (8/8, 2 ignorés),
      `b6` (12/12, 4 ignorés), `ep7_historical` (8/8) et `eq7_cavalry` (1/1) — marges inchangées ;
      `uv run --project tools pytest` (797 passés, 2 ignorés) ; `godot --headless --script
      tests/smoke.gd` (OK, avertissements de pyramide de relief manquante préexistants, sans
      rapport avec CB3) ; `zg4_camera_test.gd`, `cb3_tactical_view_test.gd`,
      `cb0_input_equivalence_test.gd`, `cb_m1_outline_test.gd`, `cb_m2_path_hover_test.gd`,
      `cb_m3_queue_test.gd`, `cb_m4_range_compare_test.gd`, `cb1_drag_formation_test.gd`,
      `ep8b_clock_test.gd` : tous OK, code 0, aucune erreur de script. Capture
      `cb3_tactical_shot.gd --probe` exécutée une fois (texte seulement) : inclinaison 85°,
      distance 900 (bornée à `max_distance`), pastilles regroupées, 0 ennemi masqué dans cette
      bataille de démo (aucun ennemi hors de la portée de repérage par défaut).
- [x] Correction en cours de route : le calque de la vue tactique doit être libéré (`queue_free`),
      pas seulement détaché (`remove_child`) — sinon fuite d'un `ColorRect` par entrée/sortie
      (repéré par les avertissements « RID leaked » de `cb3_tactical_view_test.gd`).

## Lot terminé — prêt pour fusion (en premier, vague 4)

## Décisions / écarts

- Portée de vue de `spotted` : réutilise `data/rules/missile_arc.json.spotter_range_m` (350 m,
  déjà la portée à laquelle un allié dirige un tir indirect) plutôt qu'un nouveau
  `battle_vision.json` — le champ existait déjà (§ Écarts du plan : « ou de la règle existante si
  elle existe déjà »).
- `spotted` est une fonction pure de l'état courant (pas un champ de `Unit`, pas de commande) :
  n'entre ni dans `state_digest`, ni dans le format de rejeu.
- Terrain assombri en vue tactique : overlay `ColorRect` semi-transparent dans le HUD (2D), pas
  de `CanvasModulate` (qui ne module que les `CanvasItem`, pas la scène 3D) ni de changement de
  shader de terrain (un autre chantier, PO, y touche).

## Touches nouvelles (pour CB2, qui réécrit `HELP_TEXT`)

- **Tab** : vue tactique (entrée / sortie). Échap sort aussi de la vue tactique si elle est active
  (sinon comportement existant : désélection).
- Bouton du milieu seul : rotation + inclinaison caméra (au lieu de panoramique).
- Maj + bouton du milieu : panoramique (déplacé depuis le bouton du milieu seul).

## Prochaine étape

Rien côté CB3 : lot terminé, à fusionner en premier (`docs/wip/cb.md`, vague 4 : CB3, puis CB5,
puis CB6, puis CB2). Après la fusion : réimporter Godot dans le worktree de fusion, puis mettre à
jour `docs/wip/cb.md`. Pour CB2 (aide F1 / `HELP_TEXT`) : voir la section « Touches nouvelles »
ci-dessus.
