# PO5 — Mouvement et transitions (wip)

Branche `feat/po5-motion` (depuis `feat/po-polish`, qui y est fusionnée). Plan :
`docs/superpowers/plans/2026-09-27-po-polish.md` § PO5.

## État : terminé, en attente de fusion par l'orchestrateur
- [x] `SceneFader.go(path)` / `cover()` / `reveal()` (`game/scripts/ui/scene_fader.gd`, façade
  statique ; nœud `SceneFaderLayer` créé à la demande sous `/root`). Tous les
  `change_scene_to_file` du jeu passent par lui, sauf `scripts/dev/release_journey.gd`.
  Retour de bataille vers la carte sous le voile (`battle_scene.gd::_on_return`).
- [x] Caméras : `data/ui/camera_feel.json` + schéma + pytest, lu par `CameraFeel`.
  Carte : inertie (clavier, bords, glisser), zoom lissé, glissement de focus 0,4 s.
  Bataille : zoom lissé, `glide_to` (rappel de groupe, double clic sur une carte).
- [x] Fin de tour : sortie en fondu du cartouche d'attente, `SeasonBanner` (« Printemps 1338 »,
  1,2 s) posé par `TurnLight` quand la date change, teinte soir → aube lissée.
- [x] Onde d'encre (`OrderRipple`, 0,5 s) au point d'un ordre accepté ; `ui_order` existait.
- [x] `game/tests/po5_motion_test.gd` vert ; smoke, zg4_camera, trackpad_zoom, po_grade verts.

## Écarts
- `SceneFader` n'est pas un autoload : en mode `--script` (smoke.gd), un identifiant d'autoload
  n'est pas déclaré au compilateur (erreur « Identifier not found: SceneFader »). Façade statique
  `class_name SceneFader` à la place, même API.
- Entrée en bataille depuis la carte : pas de voile ajouté (l'écran de chargement AR1 fait déjà
  la transition ; `campaign_map.gd` et `battle_loading_card.gd` hors périmètre).
- Minicarte de bataille : coupe franche gardée (elle émet à chaque mouvement du glisser).
  `BattleCamera.look_at_point` reste instantané (plans scriptés, captures).
- Le bandeau U5 (`map_ui.gd`, PO1) affiche aussi la date en titre à la fin du tour ; le cartouche
  de saison vient après son retrait. PO1 peut retirer la date de ce titre si c'est redondant.

## Notes
- Nouvelles `class_name` (`CameraFeel`, `SceneFader`, `SceneFaderLayer`, `SeasonBanner`,
  `OrderRipple`) : relancer `godot --headless --path game --import` après fusion.
- `battle_camera.gd` inchangé sur `feat/cb-m2-path-hover` (vérifié au démarrage).
