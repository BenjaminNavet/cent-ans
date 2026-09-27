# CB3 — Ralenti, caméra (lacet/tangage), vue tactique, `spotted`

Branche `feat/cb3-camera-tactical`, depuis `main` (cad1ca05 / 220a1e9f). Plan :
`docs/superpowers/plans/2026-09-27-controles-bataille-tw.md` (§ Écarts point 2, Conventions
communes, CB3). Spec : `docs/superpowers/specs/2026-09-27-controles-bataille-tw-design.md` (§ CB3).

## État (dernier commit)

- [x] `battle_camera.gd` : bouton du milieu seul = lacet + tangage manuel (`manual_pitch`,
      `manual_pitch_deg`, bornes 5°-85°, `MANUAL_PITCH_DRAG_SPEED`) ; Maj + bouton du milieu =
      panoramique (ancien comportement) ; suivi de régiment inchangé (orbite lacet seul) ;
      `look_at_point` et `follow_unit` remettent `manual_pitch` à zéro (recentrage).
- [ ] Vitesse ×0,5 (`SPEEDS`, `battle_hud.gd` boutons/tooltips/icônes).
- [ ] Cœur : champ dérivé `spotted` (`BattleSim::spotted_by`, réutilise `visible()` +
      `missile_arc.spotter_range_m`), test `cb3_spotted.rs`, exposition dans `get_units`.
- [ ] `battle_tactical_view.gd` (Tab) : caméra du dessus, overlay d'assombrissement (pas de
      shader touché), pastilles B7 forcées, ennemis non `spotted` masqués, restauration.
- [ ] Tests Godot : `zg4_camera_test.gd` étendu, `cb3_tactical_view_test.gd`.
- [ ] Capture `cb3_tactical_shot.gd --probe`.
- [ ] Vérifications finales (fmt/clippy/test, pytest, smoke, tests CB, release ep7/eq7 si besoin).

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

Ajouter `spotted_by` au cœur + test `cb3_spotted.rs`, puis exposer `spotted` dans `get_units`
(`godot-bridge/src/battle_sim.rs`). Puis `battle_tactical_view.gd` et son branchement (Tab dans
`battle_input.gd`, nœud créé dans `battle_scene.gd::_ready`). Puis vitesse ×0,5. Puis tests Godot
et capture.
