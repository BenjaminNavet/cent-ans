# GA3-L2 — végétation réaliste de la carte de campagne

Worktree `game_project-ga3`, branche `feat/ga3`. Spec : `docs/wip/ga3.md` (S5, « Lots de production »).

## État
- [x] Squelette : `game/scripts/map/ga3_vegetation.gd` (option `--no-ga3-veg`, chemins).
- [ ] Génération (fal) : 3 essences × 8 azimuts, herbe dense, 2 rochers de plus.
- [ ] Atlas : grille d'imposteurs GA3 (albédo + normale), cartes de feuilles, touffe.
- [ ] Branchement : `vegetation_meshes.gd` / `vegetation.gd` (imposteurs aussi proches), `ground_clutter.gd` (herbe, rochers).
- [ ] Tests, perf (`--fps-probe`), planche, ADR, budget.

## Prochaine étape
Génération fal (script `tools/blender_scripts/ga3_vegetation.py`, étape `l2fal`).
