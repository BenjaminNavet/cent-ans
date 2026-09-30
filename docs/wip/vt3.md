# VT3 — arbres de la carte à l'échelle 1:1

Suite de VT/VT2 (ADR 0138). Demande validée par le joueur : les arbres de la carte de campagne à
l'échelle réelle à toute distance de la vue 3D ; au-delà de la portée où ils font ≈ 1 px, la forêt
est portée par le terrain (canopée). Les incendies restent exagérés. Worktree
`../game_project-vt3`, branche `feat/vt3`. Coût cloud : 0 $.

## État
- [x] Squelette : `MapPropScale` (`tree_scale()` constant, `tree_ratio` 0,035 → 0,018,
  `tree_max_distance`, `tree_view_range`, `clutter_scale` pour l'herbe FC3), test
  `vt3_trees_test.gd` vide.
- [ ] Végétation (tuiles) et forêt dense : portée, densité, budget.
- [ ] Herbe FC3 découplée de `campaign_prop_scale`.
- [ ] Canopée du terrain renforcée au loin.
- [ ] Tests adaptés, banc base/VT3, captures, docs.

## Prochaine étape
Vegetation.update_view : portée des arbres ; ForestDetail : part = poids de portée.
