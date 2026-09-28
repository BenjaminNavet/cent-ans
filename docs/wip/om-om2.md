# OM2 — chaîne géo sur l'emprise Oural–Méditerranée

Branche `feat/om-om2`, worktree `../gp-om-om2`. ADR 0115 : EPSG:3035, 718,9765625 m/unité,
7168 × 6144, x 2 169 486 → 7 323 110, y 775 684 → 5 193 076 ; pixels existants : x identique,
y + 1280.

## Environnement du worktree
- `tools/geo/raw/*` : liens symboliques vers les bruts du dépôt principal, sauf `kk10/`,
  `copernicus_cache/`, `pyramid_work/` (dossiers propres : caches dépendant de la grille, à ne pas
  écraser pour `main`).
- Pas de lien `data/map/pyramid` (rien n'est recuit).

## État
- [x] project.py : grille rectangulaire (`MapGrid(bounds, width_px, height_px)`, `shape`,
  `grid_from_metadata`), bornes explicites `BOUNDS_PROJECTED`, `LEGACY_Y_OFFSET_PX = 1280`.
- [x] téléchargements : 14 tuiles ETOPO (26 au total, `download.etopo_tiles_for_grid` ne garde que
  les tuiles qui touchent le rectangle), KK10 nouvelle bbox (16 s, `kk10_1330_1349_om.npz`),
  Natural Earth `geography_regions_polys` (déserts).
- [x] générateurs adaptés (terrain, build, relief, relief_shade en bandes, splat, landcover avec
  aridité de repli, navgrid W/2 × H/2 + terres hors provinces infranchissables, roads,
  settlements, hamlets, river_render, horizon).
- [x] provinces : règle de distance (400 km par la terre, îlots sans graine ≤ 64 000 px à 400 km).
- [x] relief_pyramid.json `root_origin_tiles: [0, 5]` (+ schéma) ; code de cuisson dans le cadre
  de la pyramide (`pyramid.map_bounds` = cadre, `FineRelief` décale E0).
- [x] migration +1280 y : `geo/migrate_om2.py` (landmarks anchor.px, fine_anchors.json,
  towns_1340.json) ; littéraux des tests/scripts Godot.
- [x] génération complète sur feat/om fusionné (406 provinces : D1-D5), artefacts commités.
- [x] fichiers ≤ 50 Mo, pytest, docs/geo.md (section « Emprise Oural–Méditerranée »),
  m1-campaign-map.md.

## Régénération complète (ordre, durées mesurées, 406 provinces)
1. `uv run --project tools cent-ans geo build` — 5 min 08 (provinces 34 s)
2. `uv run --project tools cent-ans geo splat` — 2 min 20 (appelle landcover)
3. `uv run --project tools cent-ans geo relief-shade` — 1 min 04 (après build, qui écrase `height/`)
4. `uv run --project tools cent-ans geo navgrid` — 6 s
5. `uv run --project tools cent-ans geo rivers-render` — 14 s
6. `uv run --project tools cent-ans geo horizon` — 26 s
Total ≈ 9 min 20. Ne pas relancer `migrate_om2` ni `anchors-fine`/`towns`.
Worktree : lier `tools/geo/raw/*` au dépôt principal sauf `kk10/` et `copernicus_cache/`
(caches dépendant de la grille ; KK10 : `uv run --with h5py --with fsspec --with aiohttp
python -c "from cent_ans_tools.geo import kk10; kk10.extract()"`, 16 s).

## Prochaine étape
Aucune dans ce lot : régénération finale par l'orchestrateur après D6.

## Échecs pytest restants (données des lots D, pas OM2)
- `test_settlements_schema` : 9 provinces à 2 colonies (3 attendues : beysehir, elbistan, lublin,
  muntenia, muntenia_east, oltenia, polotsk, teke, vitebsk) ; `set_emba` (53,5° E, 48° N) hors
  carte.
- `test_heraldry::test_shields_are_distinct_and_masked` : 169 blasons distincts pour 171.
- Avertissements navgrid : colonies sur des masses isolées sans port (set_bereket, set_emba,
  set_itil, set_saraitchik : delta de la Volga / Oural ; set_bergen_rugen, set_poide, set_valaam).

## Points ouverts / coordination OM1
- `relief_shade.png` est remplacé par des bandes `relief_shade_<i>.png` (3072 lignes,
  `map.json.relief_shade.bands`) ; `relief_landcover.gd` les empile (fait ici).
- `map.json` : `size_px` [7168, 6144], `height_tiles.size_px` [14336, 12288] (liste, plus un
  entier), `navgrid.size_px` [3584, 3072].
- Pyramide : tuiles et points CAFV (fleuves/routes fins) restent dans l'ancien cadre ; le moteur
  ajoute `root_origin_tiles × 256` (= +1280 en y) aux positions des points CAFV comme aux
  adresses de tuiles. `fine_anchors.json` et `towns_1340.json` sont en unités monde (migrés).
- Tests Godot laissés à OM1 (adressage de tuiles) : `zg2_quadtree_test.gd` (tuile E1 (1,17,15),
  `h_8_7.png` → `h_8_12.png`, points), `zg5b_fine_geo_test.gd` (tuiles factices et ancres),
  bornes 4096 de `zg4_camera_test`, `po5_motion_test`, `rs_k_finest_levels_test`.
- HEIGHT_MAX reste 4800 m (voir plus bas) : le quadtree décode E0 (`height/`) avec
  l'encodage de la pyramide (−200 / 5000 m) ; changer la plage casserait ce partage.

## Migration +1280 y (liste)
- `data/landmarks/*.json` : `anchor.px` (7 fichiers).
- `data/map/fine_anchors.json`, `data/map/towns_1340.json` (générateurs dépendants de la
  pyramide, refusent de tourner tant que `root_origin_tiles` ≠ [0, 0]).
- `game/tests/*` (36 fichiers de captures, bancs et tests visant des lieux) et
  `game/scripts/dev/map_bench.gd`, `release_journey.gd`.
- Rien à migrer : `landmarks_v2` (origin_3035), `detail_zones.json` (lon/lat), provinces et
  colonies (lon/lat), `battle_maps`, `naval` (coordonnées locales), `tools/geo/paris_v2_author.py`
  (EPSG:3035).
