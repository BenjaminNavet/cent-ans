# HC2 — eaux lisibles sur la carte de campagne (lacs, étangs, mares)

Worktree `../gp-hc2`, branche `feat/hc2` (issue de `feat/hc`). ADR 0161 §3. Rendu seul, aucune
règle ni cuisson autre que `geo lakes`.

## État
- [x] Squelette : note, test `hc_water_test.gd` (désactivé), planche `hc_water_shots.gd`.
- [x] Plus de lacs : `min_area_px` 30 → 8, `lakes.json` 762 → 1231 lacs (13 473 → 17 046 sommets).
      Le seuil 4 donne exactement le même fichier (un bassin exige un cœur plein, `flat_basins`).
      Noms : couche Natural Earth absente du worktree, lue dans le checkout principal
      (`--natural-earth ../game_project/tools/geo/raw/natural_earth/ne_10m_lakes/ne_10m_lakes.shp`),
      67 lacs nommés comme avant. `tools/tests/test_lakes.py` : 10 réussis.
- [x] Lacs éclaircis : `lake_color` (0.07, 0.23, 0.38), `lake_shallow_color`, `lake_bank_color`,
      `lake_sky_color`, `lake_sky_reflect` 0,35 (terrain.gdshader, bloc des lacs) ; couleurs de
      nappe propres aux lacs (`LakesRenderer.deep_color`, `shallow_color`, `sky_reflect`).
      Mesure (cœur des nappes, sRGB) : eau ≈ (100, 134, 142) contre sol ≈ (87, 99, 59).
- [x] Étangs et mares généralisés par paliers d'empreinte (`relief_landcover.gdshaderinc`,
      uniformes `rl_pond_*`, `rl_pool_*`, `rl_water_ref_height`). Empreinte = moyenne géométrique
      des deux axes écran, en pixels d'un écran de 1080 de haut. Dombes : 25 nappes à rig 150,
      6 à rig 300 (image 640×400, nappes ≥ 6 px).
- [x] Test `hc_water_test.gd` : headless OK ; avec fenêtre, contrôle chiffré de la Dombes OK.
- [x] Outils : `game/tests/hc_water_view.gd` (carte, cadrage, masque d'eau par passage magenta,
      statistiques), `hc_water_shots.gd` (planche 2×2 + ligne `HC water` chiffrée par vue).
- [ ] Hiver (`--season=winter`), smoke, planche finale, relecture.

## Prochaine étape
Planche v3 (2e lecture sur 3), contrôle d'hiver chiffré, smoke, rapport. Lectures de planche : 1 / 3.
