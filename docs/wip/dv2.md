# DV2 — lieux sans pictogramme (écu + nom), HeraldryAtlas

Branche `feat/dv2` (depuis `feat/dv` 3fe0c34d6). Spec DV, ADR 0124 (Écarts priment).

## État
- [x] `HeraldryAtlas` (atlas des écus extrait de `SettlementMarkers`).
- [x] `SettlementMarkers` allégé (rangs, tailles, visibilité, dé-encombrement, écu `shield`).
- [x] `settlement_markers.json` + schéma allégés ; PNG + `.import` supprimés ;
  `tools/cent_ans_tools/map_markers.py`, son test et la commande `assets map-markers` supprimés
  (sources peintes `tools/assets/map_markers/*.jpg` gardées).
- [x] `settlement_icon.gdshader` : écu seul, ancré sur le nom, au-dessus du texte.
- [x] `settlement_layer.gd` : `normal = 1 − strategic_weight` ; maquettes et écus sur toute la
  vue normale ; nom + écu = un couple dé-encombré ; densité par rang seule (380 supprimé).
- [x] Légende : « maquette et écu », « vignette à l'encre ».
- [x] `da3_markers_test.gd` → `dv_markers_test.gd`, `ux1_test.gd` adapté.
- [ ] Tests Godot : smoke, settlements_render, dv_markers, ux1, da7d_overlap.

## Prochaine étape
Faire passer les tests (import Godot en cours dans le worktree).
