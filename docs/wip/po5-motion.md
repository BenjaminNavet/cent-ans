# PO5 — Mouvement et transitions (wip)

Branche `feat/po5-motion` (depuis `feat/po-polish`). Plan : `docs/superpowers/plans/2026-09-27-po-polish.md` § PO5.

## État
- [x] Squelette : autoload `SceneFader` (`game/scripts/ui/scene_fader.gd`), `CameraFeel`
  (`game/scripts/ui/camera_feel.gd`), `data/ui/camera_feel.json` + schéma + pytest.
- [ ] 1. `change_scene_to_file` → `SceneFader.go` (hors `scripts/dev/release_journey.gd`).
- [ ] 2. Caméras (carte : inertie, zoom lissé, glissement 0,4 s ; bataille : zoom + glissement).
- [ ] 3. Fin de tour : fondu du cartouche, bandeau de saison 1,2 s, lumière lissée.
- [ ] 4. Onde d'encre au clic d'ordre (carte).
- [ ] 5. `game/tests/po5_motion_test.gd`, smoke.

## Prochaine étape
Remplacer les `change_scene_to_file`.

## Notes
- `battle_camera.gd` inchangé sur `feat/cb-m2-path-hover` (vérifié) : la bataille est traitée.
