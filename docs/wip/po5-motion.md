# PO5 — Mouvement et transitions (wip)

Branche `feat/po5-motion` (depuis `feat/po-polish`). Plan : `docs/superpowers/plans/2026-09-27-po-polish.md` § PO5.

## État
- [x] Squelette : autoload `SceneFader` (`game/scripts/ui/scene_fader.gd`), `CameraFeel`
  (`game/scripts/ui/camera_feel.gd`), `data/ui/camera_feel.json` + schéma + pytest.
- [x] 1. `change_scene_to_file` → `SceneFader.go` (hors `scripts/dev/release_journey.gd`).
- [x] 2. Caméras (carte : inertie, zoom lissé, glissement 0,4 s ; bataille : zoom + glissement).
- [ ] 3. Fin de tour : fondu du cartouche, bandeau de saison 1,2 s, lumière lissée.
- [ ] 4. Onde d'encre au clic d'ordre (carte).
- [ ] 5. `game/tests/po5_motion_test.gd`, smoke.

## Prochaine étape
Fin de tour (`turn_light.gd`, `turn_wait_indicator.gd`, bandeau de saison).

## Notes
- `battle_camera.gd` inchangé sur `feat/cb-m2-path-hover` (vérifié) : la bataille est traitée.
- Bataille : `glide_to` pour le rappel de groupe et le double clic sur une carte ; la minicarte
  reste une coupe franche (elle émet à chaque mouvement du glisser : un glissement y traînerait).
  `look_at_point` reste instantané (plans scriptés, captures).
- Nouvelle `class_name` (`CameraFeel`) : relancer `godot --headless --path game --import`.
