# AS8c — bêtes et charrettes mesurées sur vidéos libres

Branche `feat/as8c` (partie de `main` d83187ee5, main fusionné pour `tools/godot_bg.sh`). ADR 0189, plan `docs/wip/as8.md`.

## État : terminé, en attente de fusion
- Vidéos hors dépôt `~/dev/cent-ans-mocap-src/video/free/animals/<slug>/` + `LICENSE.txt` chacune.
- `tools/video_mocap/measure_motion.py` (sheet, track LK+MIL, overlay, period, analyse) et
  `tools/video_mocap/as8c_measure.py` (liste des clips, points posés, dérivation) -> `data/fx/animal_motion_measured.json`.
- `data/fx/animal_motion.json` (+ schéma) : tables de pas `swing_lut`/`lift_lut`, `graze_ramp_s`, `roll_rad`,
  `jolt_lines` (3 raies), mâchonnement, tête et queue des chevaux de camp, bloc `source`.
- Shaders `animal_motion.gdshaderinc` (tables, cahot en raies, roulis, descente de tête mesurée) et `camp_horse.gdshader`.
- Tests : `as1_test`, `as2_test`, `as8c_test`, smoke, pytest `test_animal_motion_schema.py`; sonde de compilation
  `as8c_shader_probe.gd` (via `tools/godot_bg.sh`) sans erreur de shader.
- Attribution : `data/fx/animal_motion_SOURCE.md`, `CREDITS.md` (fichiers de données CC BY-SA 2.0 fr : Rama).

## Points ouverts
- Suivi du sabot instable : la courbe de pas vient du centre de la jambe arrière (cadence 0,71 Hz, sinusoïde nette).
- Rapport de marche 0,6, levée, rebond, souffle (borne haute < 0,5 px), poids sur un sabot : non mesurés.
- Moutons : foulée non mesurée (vidéo en broutage). Ploughing.ogv inutilisable (352x288, boue).
- Jugement visuel en jeu (AS7) : foulée des bovins passée de 1,4 à 0,8-0,86 m (jambes plus vives à vitesse égale).
