# AN1b — nouveaux clips cuits (victoire, attentes, parade, coup par-dessus, cheval, impacts)

Branche `feat/an1b-clips` (worktree agent). Orchestration : `docs/wip/an1-animation-vivante.md`.
ADR : `docs/decisions/0096-animation-vivante.md` § B (décision, coût, limites).

## Fait
- Poses calculées dans `tools/blender_scripts/battle_skinned_poses.py` (section AN1b) ;
  13 clips humains et 3 clips cheval ajoutés en fin de listes (`battle_skinned.py`,
  `battle_skinned_cavalry.py`) ; surcharge cheval `pose.horse` (aussi dans
  `battle_fine_proto.pose_cavalry`).
- Textures d'os fines recuites (`blender -b --factory-startup --python
  tools/blender_scripts/battle_fine.py -- rigs`, 9 s) ; anciens clips identiques.
  Note : l'étape `bake` (45 min) ne recuit que maillages et cartes, pas les textures d'os :
  inutile ici.
- Shader : `clips[64]`, jeux de 8 (`pick_clip`) ; `battle_standard_flag.gdshader` : `clips[64]`.
- GDScript : `STYLES` enrichi, `_present` (clips absents écartés), `victor_side`,
  `melee_pikes`, cycle de charge avec trébuchement, écran de fin après 3 s d'acclamation.
- Planche : `docs/img/an1/an1b_clips.png` (`an1b_render.py` + `an1b_planche.py`).
- Test : `game/tests/an1b_clips_test.gd` (+ `-- --coarse-figures`).

## Tests
Après fusion de `main` (6a2f9dc7) : an1b_clips_test (fin et `--coarse-figures`), bv3_check,
fg3_maps_test, ep13_replay_test, smoke OK.

## Prochaine étape
Lot terminé, prêt à fusionner par l'orchestrateur. Suites possibles : voir ADR 0096 § B
« Limites » (kit grossier non recuit, pas de fondu entre clips d'un cycle de mêlée).
