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
- [x] Banc base/VT3 (3 passes, d = 1100/150/30/5) : appels de dessin et primitives ≤ base
  partout (d = 30 : 426 → 351, 2,82 → 2,22 M) ; i/s dans le bruit du rythme d'affichage.
- [x] Docs godot-map (« Arbres 1:1 (VT3) ») + addendum VT3 de l'ADR 0138.

## TERMINÉ (30/09), à fusionner par l'orchestrateur (branche feat/vt3)

## Compléments (demande de l'orchestrateur, 30/09)
- [x] Herbe, broussailles, rochers FC3 au 1:1 (portée 2,6, fondu), `clutter_scale` retiré,
  `MapPropScale` réduit à `fire_scale` pour l'exagération ; fc3, ga3_l2, sz4 adaptés.
- [x] d = 5 grand massif : `canopy_lift` (teinte de forêt vers les houppiers), `fade_from` 0,4 ;
  capture de contrôle regardée (6/6 du budget visuel) : couvert continu, relief de canopée,
  passage vers le lointain sans rupture ; cimes un peu jaunes (bords clairs des imposteurs).
- [x] Amorçage bloquant tardif (240-300 ms mesurés) : limité aux 60 images après `build`.

## Points ouverts
- Herbe 1:1 : quasi invisible (≤ 1 px au-delà de d ≈ 1-2,6) ; densité ≈ 2 500 touffes / u²
  (une pour ~200 m²), campagne rase de près. À réévaluer en partie réelle.
- Cimes des arbres un peu jaunes à d = 5 (liseré clair des imposteurs GA3 sur la canopée).
- Imposteurs lointains de la végétation (au-delà de `detail_distance` 170) morts en jeu ; laissés
  pour les A/B (`fc2_impostors_test`).
- Saut de caméra 300 → 10 : pire image ~60 ms due aux autres couches (terrain, villes), hors VT3.

## Prochaine étape
Fusion dans main par la session principale ; jugement du joueur en partie réelle.
