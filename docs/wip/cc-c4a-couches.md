# CC C4a — couches dérivées indépendantes des provinces

Branche `cc/c4a` (worktree `game_project-cc4a`), issue de `cc-carte-composee`. C4b (provinces, lieux, navgrid,
ports, routes) viendra après C3 et ne doit pas être anticipé ici.

## Plan
1. Carpates murées : `ridges.json` (chaîne d'auteur ≈ 2 500 m) + 3-4 cols dans `crossings.json`, `compose-relief`, test massif.
2. Outils communs : ellipses en km réels -> polygones lon/lat (échelle locale du warp), Köppen/déserts rééchantillonnés par lon/lat,
   filtre majoritaire, mode « indépendant » (sans provinces ni lieux) de splat/landcover/colormap.
3. Régénération : coast_dist, biomes (+ blend), agri_regions, landcover (wetlands, forest_kind, splat provisoire),
   lacs historiques, lakes, relief_occlusion, colormap provisoire (sans lieux/routes), textures GPU, freshwater.
4. Masques d'auteur `compose/masks/*.png` appliqués en surcharge (forêt, roche, champs, marais).
5. `horizon.py` / relief de bataille : relief réel.
6. Tests adaptés ; planche hors dépôt `~/dev/cent-ans-raw/compose/c4a/`.

## État (14/10) : C4a TERMINÉ
- Carpates (7097374c5) : chaîne à 2 500 m, cols Dukla / Verecke / Prédéal / Turnu Roșu dans `crossings.json` (coordonnées
  à ±0,1° près pour Verecke) ; arc muré, seules trouées = les 4 cols (test).
- Lacs : `compose/lakes_fine.py` (appelé par `geo compose`, clé `lakes_fine` de `generalise.json`) remet dans `land_mask.png`
  88 lacs Natural Earth >= 250 km² réels + les lacs historiques `punch_mask` (bassin dessiné sur le relief RÉEL puis déformé) ;
  `compose-relief` aplatit chaque plan d'eau intérieur (`relief.flatten_lakes`, niveau juste sous la rive) ; `geo lakes` en
  extrait 202 (Léman, Constance, Garde, Balaton, Fucino...). Les niveaux réels de `historical_lakes.json` ne servent pas.
- Outils : ellipses en km réels tracées par polygone à travers la déformation (`landcover.ellipse_polygon`,
  `project.composed_geometry`), déserts, Köppen par lon/lat + filtre majoritaire 3 x 3, `freshwater._ellipse_px` (échelle locale),
  `compose/masks.py` (masques d'auteur appliqués en surcharge, vides pour l'instant), `compose/layers.py`, `geo compose-layers`.
- Horizon (`geo/horizon.py`) : relief réel `data/map/real` (719 m), site converti par la déformation inverse (`Sources.site_of`),
  forêt lue à la position composée ; `PyramidGrid.from_map` lit le `meters_per_px` réel (719 nominal ≠ 718,98). Pas relancé
  (provinces de C4b) ; essayé sur les anciennes provinces : pas d'erreur.
- Tests : `tools/tests/test_compose_c4a.py` (nouveaux), `test_biomes` (lieux hors canevas ou sous une chaîne composée retirés,
  fenêtre de 6 px), `test_compose_relief` / `test_compose_contract` (lacs plats > 0), `test_lakes`, `test_historical_water`,
  `test_river_names`, `test_pyramid` (E0 = `height/` composé : sauté), `test_settlement_graph::test_relief_tiles` (taille complétée).
- Planche hors dépôt : `~/dev/cent-ans-raw/compose/c4a/` (biomes, landcover, colormap, water).

## Reconstruction
```
uv run --project tools cent-ans geo compose          # ≈ 30 s, côtes, lacs fins, fleuves, crêtes
uv run --project tools cent-ans geo compose-relief   # ≈ 3 min
uv run --project tools cent-ans geo compose-layers --board ~/dev/cent-ans-raw/compose/c4a   # ≈ 5 min
```
`geo compose` réécrit `map.json` : toujours enchaîner les trois (le relief et les couches y réinscrivent leurs entrées).

## Provisoire (refait en C4b)
`splat.png` (sans provinces ni essarts autour des lieux, mélange de terrain par défaut), `colormap_bc1_*` (sans routes, villages,
halos de villes), `crossings_px.json` (sans ponts de routes), `rivers_render` (rendu seul). Restent absents ou périmés :
`province_border_dist.png`, `province_ids.png`, `provinces.geojson`, `navgrid.png`, `hamlets.json`, `settlements_px.json`,
`roads.geojson`, `coast_types.json`, `sea_basins.json`, `sea_lanes_px.json`, `wetland_sites_px.json`, `forest_cover.json`,
`towns_1340.json`, `fine_anchors.json`, `horizon` (game/assets/horizon/relief).

## Tests qui attendent C4b (échouent tant que provinces / lieux / ports ne sont pas refaits)
`test_navgrid` (fichier, ponts, cols, connexité), `test_settlement_graph` (graphe, lieux dans leur province, hameaux),
`test_coast_types`, `test_sea_basins`, `test_sea_lanes`.

## Restes / points ouverts
- Neige : `colormap_style.yaml` (2 650-2 900 m) blanchit toute la chaîne des Alpes et les chaînes d'auteur >= 2 700 m (Atlas,
  Taurus, Pyrénées) ; à juger à l'œil en C7.
- Forêt 53 % des terres (`landcover_params.json` `cleared_scale`) : réglage de l'ancienne carte, à revoir en C6.
- Les masques d'auteur ne sont testés que sur des cas synthétiques (aucun masque peint).
- Köppen : tuiles de 1 km étirées de 0,5 à 1,5 selon la déformation ; filtre 3 x 3 suffisant à l'œil sur la planche.
