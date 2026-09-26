# ADR 0086 — Rasters de la carte : pixel i centré en x = i + 0,5 (comme les outils)

Date : 2026-09-26. Statut : accepté. Lot SZ2b (`docs/wip/sz2b-nappe-fleuves.md`), chantier SZ
(`docs/wip/sz-suites-zoom.md`). Complète l'ADR 0036 (carte zoomable, pyramide de relief) et
l'ADR 0062 (végétation native). Numéro choisi au-dessus des numéros déjà pris sur les branches
(0079 laissé libre au cas où il serait réservé).

## Contexte

Les outils (`tools/cent_ans_tools/geo`) projettent tout en EPSG:3035 et passent en coordonnées
carte par x = (E − minx) / m, y = (maxy − N) / m (`MapGrid.projected_to_pixel`) : le pixel i d'un
raster (heightmap 4096, splat, `river_bed.png`, étages E1-E7 de la pyramide) couvre [i, i + 1] et
son centre est en i + 0,5. Les données vectorielles (fleuves fins CAFV, ancrages, colonies, villes
1:1 v2, `rivers_render.json`, `towns_1340.json`) suivent la même convention.

Godot, lui, lisait les rasters avec le pixel i centré en x = i : `uv = (p + 0,5) / taille` dans
`terrain.gdshader` et ses inclusions, `MapData.height_m_at` bilinéaire sur les pixels entiers, et
`ReliefPyramid.GRID_OFFSET = −0,5` (tuile (k, col, row) sur [col·T − 0,5, (col + 1)·T − 0,5]),
repris par la végétation native (Rust). Le relief affiché était donc décalé de 0,5 unité
(≈ 360 m) vers le nord-ouest par rapport à tout ce qui est posé dessus.

Invisible à l'échelle stratégique (719 m par unité), le décalage devient grossier au palier site
(1:1) : mesure du lot SZ2b, les fleuves fins collent au relief E4/E7 avec un décalage de
(−0,5 ; −0,5) exactement à Orléans, Rouen, Paris, Londres et Tours. Conséquences : le lit creusé
par `FineBedCarver` tombait sur la berge (Loire d'Orléans : fleuve fin sur la rive à 94 m, eau à
86,8 m, tranchée de 7 m) pendant que le vrai lit du relief restait sec à 360 m ; à Rouen, la
falaise de la côte Sainte-Catherine traversait la ville 1:1 et une grève séparait le mur de rive de
l'eau.

## Options

- **A. Décaler les données vectorielles de −0,5** à leur chargement : une dizaine de chargeurs
  (colonies, fleuves, ponts, routes, villes v1/v2, villes ZG6, ancrages), dans des fichiers tenus
  par d'autres lots ; convention à rappeler à chaque nouvelle donnée.
- **B. Aligner la lecture des rasters dans Godot sur la convention des outils** : quelques lignes
  (décalage de grille de la pyramide, lecture bilinéaire CPU, `uv` des shaders du terrain) ; rien
  à recuire ; les outils restent la référence.

## Décision

Option B. Le pixel carte i est centré en x = i + 0,5 partout côté rendu :
- `ReliefPyramid.GRID_OFFSET = 0` (quadtree, surface CPU, creusement du lit, caméra, villes ZG6
  passent par `tile_origin` / `tile_at`) ; `ReliefQuadtree.surface_height_at` n'a plus de −0,5
  codé en dur ;
- `MapData.height_m_at(x, y)` lit la heightmap en (x − 0,5, y − 0,5) ;
- shaders : `uv = p / map_size` (terrain, parchemin, mer, `river_bed`, parcellaire fin, repli
  heightmap du quadtree) ;
- végétation native (`core/crates/vegetation`) : même décalage de grille et même lecture
  bilinéaire (miroir de `MapData`).

## Conséquences

- Relief, textures de la carte, frontières (déjà lues par `floor(p)`), fleuves, colonies et villes
  1:1 coïncident au mètre près (aux erreurs des sources près).
- Restent dans l'ancienne convention, sans effet visible (≤ 0,5 pixel de 719 m, seulement en vue
  lointaine) : les maillages E0 des morceaux (`TerrainBuilder._build_chunk`, affichés seulement
  en vue parchemin ou sans pyramide), les tuiles fines d'avant la pyramide (`FineTerrainJob`,
  repli sans cache) et la grille du fond de vallée ZG8 (`ReliefFloor`, cellules de 8 pixels,
  lissée).
- Toute nouvelle lecture de raster côté Godot suit cette convention : pixel i ↔ [i, i + 1].
