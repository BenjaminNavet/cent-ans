# HC2 — eaux lisibles sur la carte de campagne (lacs, étangs, mares)

Worktree `../gp-hc2`, branche `feat/hc2` (issue de `feat/hc`). ADR 0161 §3. Rendu seul, aucune
règle ni cuisson autre que `geo lakes`.

## État
- [x] Squelette : note, test `hc_water_test.gd` (désactivé), planche `hc_water_shots.gd` (vide).
- [ ] Lacs éclaircis (nappe `river_water.gdshader` en mode `sheet` + eau peinte du terrain).
- [ ] Plus de lacs (`min_area_px` 30 → 8 ou 4), comptage, coût de `LakesRenderer`.
- [ ] Étangs et mares généralisés par paliers d'empreinte (`relief_landcover.gdshaderinc`).
- [ ] Test headless et planche 2×2.

## Prochaine étape
Régénérer `lakes.json` aux seuils 8 et 4, mesurer.
