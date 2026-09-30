# DV1 — câblage des deux vues

Branche `feat/dv1` (base `feat/dv`). Voir `docs/wip/dv.md`, ADR 0124.

## État
- [x] `StrategicView.weight_at` → `ZoomTiers.strategic_weight` ; `start/end_distance` retirés ;
  `overlay.draw_towns = true`.
- [x] `CityMarkers` supprimé (script, nœud, appels) ; routes : traits = 1 − stratégique − proche,
  rubans = proche × site.
- [x] Armées, vie (feux, bateaux) indexées sur `strategic_weight`.
- [x] Couleur politique du terrain neutralisée (`political_amount = 0`).
- [x] Tests OK : smoke, cm2_parchment_weather, zg4_camera, sz6_spikes, fr1_borders, ux1, zg2_quadtree.

## Prochaine étape
Terminé ; fusion dans feat/dv par l'orchestrateur (DV3 : retrait des enveloppes).

## Points ouverts
- ux1_test : erreur de compilation préexistante `SimFacade` dans map_mode_controller.gd:388 (test OK malgré tout, hors DV1).
- Traits de routes éteints sous 150 (rubans de toutes les routes à la place), comme l'ancien palier moyen.
- Ombres de nuages (`cloud_shadow`) désormais visibles jusqu'au parchemin (elles s'effaçaient avec la couche politique).
