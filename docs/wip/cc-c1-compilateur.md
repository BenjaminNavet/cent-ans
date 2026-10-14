# CC C1 — compilateur de la carte composée

Branche `cc/c1` (worktree `game_project-cc1`). Contrat : `docs/wip/cc-carte-composee.md` § Contrat commun.

## Plan
1. `compose/contract.py` : écriture/lecture de `warp.json` + `warp_forward.f32` + `warp_inverse.f32`, classe `ComposedWarp` (interpolation bilinéaire, NaN hors emprise).
2. `geo/project.py` : `ComposedGrid` (même API que `MapGrid`), `RealGrid` = ancienne grille, `grid_from_metadata` gère `"crs": "composed"`.
3. `compose/compile.py` + `cent-ans geo compose` : warp, land_mask, coastline, rivers, heightmap provisoire, ridges.json, map.json.
4. Schémas : `compose_warp`, `compose_pins`, `compose_generalise`, `compose_ridges`, `map`.
5. Recensement des copies de projection Python.
6. Tests de contrat `tools/tests/test_compose_contract.py`.

## État
- Squelette posé.

## Prochaine étape
- contract.py + project.py.

## État (14/10) : C1 TERMINÉ
- `cent-ans geo compose` (≈ 50 s warp relu, `--rebuild-warp` ≈ 2 min) ; tests `tools/tests/test_compose_contract.py` (15) verts ; planche `~/dev/cent-ans-raw/compose/c1/planche_c1.png`.
- Sources réelles : `data/map/real/` (gitignoré, reconstruit par `git show 8e251a874:`).
- Projection : `ComposedGrid` / `RealGrid` / `real_grid_from_metadata` (geo/project.py). Copies recensées : `decor_bake` et `battle_ground.province_biomes` passent par `MapGrid` ; `pyramid.map_bounds`, `horizon` lisent `real_grid` ; `detail_dem`, `battle_site`, `anachronisms`, `alpage`, `detail_sources`, `fine_anchors`, `hydro_fine` restent en EPSG:3035 explicite (données réelles, commentaire CC).

## Reste pour C2 / C4
- C2 : `ridges.json` -> hauteur procédurale ; `height/` tuiles : canevas 7080 × 6104 non multiple de 256/512 (compléter) ; `heightmap.png` actuel provisoire.
- C4 : régénérer provinces, lieux, navgrid, biomes, coast_types, sea_basins, sea_lanes, graphe : 59 tests Python échouent d'ici là (`test_biomes`, `test_coast_types`, `test_sea_basins`, `test_navgrid`, `test_settlement_graph`, `test_agri_regions`, `test_historical_water`, `test_sea_lanes`) car ils lisent `data/map/` désormais composé avec des couches dérivées réelles.
- C4 : les autres usages de `MapGrid.transform`/`.bounds` sur la carte de campagne (colormap, biomes, landcover, lakes, provinces, relief…) doivent passer par `grid.real` quand ils lisent des rasters réels ; `ComposedGrid.transform` lève une exception pour les repérer.
- Lacs : le masque composé ne garde que les plans d'eau ≥ 1800 km² (Fucino, Léman… absents) ; lacs fins à reprendre via `historical_lakes.json` en C4.
