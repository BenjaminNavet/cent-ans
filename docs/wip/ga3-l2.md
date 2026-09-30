# GA3-L2 — végétation réaliste de la carte de campagne

Worktree `game_project-ga3`, branche `feat/ga3`. Spec : `docs/wip/ga3.md` (S5, « Lots de production »).

## État
- [x] Squelette : `game/scripts/map/ga3_vegetation.gd` (option `--no-ga3-veg`, `--no-ga3-near`, chemins).
- [x] Génération fal (0,474 $) : `tools/blender_scripts/ga3_vegetation_l2.py fal` ; brutes `~/dev/cent-ans-raw/ga3/l2/`.
  Par essence 1 flux-2 (vue de référence) + 1 nano-banana-2/edit (planche 4 × 2 = 8 azimuts) + 1 bria.
  Herbe dense (flux-2 1024×512 + bria), rochers b/c (flux-2 + bria + trellis ; b refait graine 7).
- [x] Atlas (`atlas`) : `game/assets/textures/vegetation/ga3/` (grille imposteurs albédo + normale, feuilles, herbe 41 %).
- [x] Rochers (`rocks`) : `game/assets/models/vegetation/ga3/ga3_rock_{a,b,c}_lod{0,1,2}.glb` (120/60/18).
- [x] Branchement : `vegetation.gd` (imposteurs aussi proches, `ga3_near_impostors`), `ground_clutter.gd`
  (herbe GA3, rochers enfants par cellule), `shaders/ground_rocks.gdshader`.
- [x] Candidats S5 retirés de `game/assets/models/vegetation/ga3/` (régénérables par `ga3_vegetation.py`).
- [x] Tests : `ga3_l2_vegetation_test.gd` OK ; pytest `tools/tests/test_ga3_vegetation_l2.py`.
- [ ] Tests existants (smoke, fc2, fc3, sz4b, sz6), perf `--fps-probe`, planche, ADR, budget.

## Mesures
- Test d = 25 (forêt d'Orléans, headless) : triangles d'arbres FC 2 402 236 → GA3 50 308.

## Prochaine étape
Tests existants, `--fps-probe` avant/après, capture avant/après, doc.
