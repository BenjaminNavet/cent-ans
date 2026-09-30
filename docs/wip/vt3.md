# VT3 — arbres de la carte à l'échelle 1:1

Suite de VT/VT2 (ADR 0138). Demande validée par le joueur : les arbres de la carte de campagne à
l'échelle réelle à toute distance de la vue 3D ; au-delà de la portée où ils font ≈ 1 px, la forêt
est portée par le terrain (canopée). Les incendies restent exagérés. Worktree
`../game_project-vt3`, branche `feat/vt3`. Coût cloud : 0 $.

## État
- [x] Squelette : `MapPropScale` (`tree_scale()` constant, `tree_ratio` 0,035 → 0,018,
  `tree_max_distance` 30, `tree_view_range` 30, `tree_shadow_distance` 12, `clutter_scale` pour
  l'herbe FC3), test `vt3_trees_test.gd`.
- [x] `Vegetation` : portée des arbres (fondu par graine), tuiles pré-semées jusqu'à 120 u,
  plus de courbe de densité au dézoom, ombres des arbres jusqu'à d = 12.
- [x] `ForestDetail` : part `fraction_at(d)` (1 jusqu'à d = 8, 0,35 à la portée, 0 au-delà),
  pas fin 0,022 (plancher natif 0,05 → 0,01 dans `vegetation_scatter.rs`), cellules 8 u × 2
  parties, budget 300 k.
- [x] Herbe FC3 découplée (`clutter_scale` des shaders `ground_clutter`/`ground_rocks`).
- [x] Canopée du terrain : `canopy_field` (4 octaves, relief + contraste), `terrain.gdshader`.
- [x] Tests adaptés : sz4_prop_scale, sz4b_colonies_forests, fc2_impostors, settlements_render ;
  fc1, fc3, ga3_l2, cv1, fk_folk, pb3g, sz6, smoke OK.
- [x] Captures (4, montage) : d = 300/40 lisibles ; d = 5 trop clairsemé → pas fin corrigé.
- [ ] Banc base/VT3 (script `scratchpad/bench.sh`, sorties `/tmp/claude-501/vt3bench`).
- [ ] Docs godot-map + addendum ADR 0138.

## Prochaine étape
Banc 3 passes (d = 1100, 150, 30, 5), puis docs.
