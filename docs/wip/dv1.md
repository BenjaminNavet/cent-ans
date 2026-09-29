# DV1 — câblage des deux vues

Branche `feat/dv1` (base `feat/dv`). Voir `docs/wip/dv.md`, ADR 0124.

## État
- [x] `StrategicView.weight_at` → `ZoomTiers.strategic_weight` ; `start/end_distance` retirés ;
  `overlay.draw_towns = true`.
- [x] `CityMarkers` supprimé (script, nœud, appels) ; routes : traits = 1 − stratégique − proche,
  rubans = proche × site.
- [x] Armées, vie (feux, bateaux) indexées sur `strategic_weight`.
- [x] Couleur politique du terrain neutralisée (`political_amount = 0`).
- [ ] Tests : smoke, cm2_parchment_weather, zg4_camera.

## Prochaine étape
Lancer les tests une fois la dylib construite ; corriger ; rapport.
