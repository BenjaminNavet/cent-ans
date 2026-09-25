# Pipeline géographique (`tools/cent_ans_tools/geo`)

Construit le terrain de la carte de campagne (`data/map/`) depuis des données ouvertes,
conformément au contrat de `docs/design/m1-campaign-map.md`, puis les provinces
(`provinces.geojson`, `province_ids.png`, voir la section « Provinces » ci-dessous) à partir
de ces sorties (grille, masque terre, rivières) et des seeds de `data/provinces/`.

![Aperçu de la carte](img/map-preview.png)

## Sources de données

| Donnée | Source | Licence | Fichiers |
|---|---|---|---|
| Relief (terre + bathymétrie) | [ETOPO 2022 v1](https://www.ncei.noaa.gov/products/etopo-global-relief-model), NOAA NCEI, grille « ice surface », 15 secondes d'arc, tuiles GeoTIFF 15° × 15° nommées par leur coin nord-ouest | Domaine public (données du gouvernement des États-Unis) ; citation : *NOAA National Centers for Environmental Information. 2022: ETOPO 2022 15 Arc-Second Global Relief Model. doi:10.25921/fd45-gt74* | `https://www.ngdc.noaa.gov/mgg/global/relief/ETOPO2022/data/15s/15s_surface_elev_gtif/ETOPO_2022_v1_15s_<NlatWlon>_surface.tif` (12 tuiles, ~300 Mo) |
| Voies romaines (routes) | [Itiner-e](https://itiner-e.org), *A High-Resolution Dataset of Roads of the Roman Empire*, version statique 2024 v1.3 (de Soto, Pažout, Brughmans et al.), Zenodo [doi:10.5281/zenodo.17122148](https://doi.org/10.5281/zenodo.17122148) | CC BY 4.0 ; citation : *de Soto P., Pažout A., Brughmans T. et al. (2025). Itiner-e: A high-resolution dataset of roads of the Roman Empire. Scientific Data. doi:10.1038/s41597-025-06140-z* | `https://zenodo.org/api/records/17122148/files/itinere_roads.gpkg/content` (GeoPackage, 34 Mo, sans compte) |
| Hameaux | [GeoNames](https://www.geonames.org/) `cities500.zip` (lieux habités de 500 habitants et plus) | CC BY 4.0, © GeoNames (https://www.geonames.org) | `https://download.geonames.org/export/dump/cities500.zip` (14 Mo) |
| Terre, côte, rivières, lacs | [Natural Earth](https://www.naturalearthdata.com/) 10 m physical, servi par `https://naciscdn.org/naturalearth/10m/physical/<couche>.zip` | Domaine public | `ne_10m_land`, `ne_10m_coastline`, `ne_10m_rivers_lake_centerlines`, `ne_10m_rivers_europe`, `ne_10m_lakes`, `ne_10m_lakes_europe` |

Pourquoi ETOPO 2022 plutôt que GMTED2010 : téléchargement HTTPS direct sans compte, tuiles
légères, bathymétrie incluse (utile pour ombrer la mer) et résolution (≈ 300–460 m) plus fine
que le pixel de la carte (≈ 719 m), donc rééchantillonnage par moyenne sans artefact.

Les fichiers bruts sont mis en cache dans `tools/geo/raw/` (ignoré par git). Un fichier déjà
présent n'est jamais retéléchargé sauf avec `--force`.

## Commandes

```sh
uv run --project tools cent-ans geo build            # télécharge (cache) puis génère data/map/ (terrain, provinces, relief 8192², routes, colonies, hameaux) et docs/img/*-preview.png
uv run --project tools cent-ans geo build --force    # retélécharge les données brutes
uv run --project tools cent-ans geo provinces        # provinces seules (≈ 10 s), après modification de seeds ou de poids
uv run --project tools cent-ans geo relief           # relief 8192² en 16 × 16 tuiles (≈ 10 s)
uv run --project tools cent-ans geo roads            # routes (Itiner-e + complément calculé) puis graphe des colonies
uv run --project tools cent-ans geo roads --computed # routes entièrement calculées (repli)
uv run --project tools cent-ans geo settlements      # graphe des colonies seul et tracé routier des arêtes (≈ 7 s), après modification de data/settlements
uv run --project tools cent-ans geo hamlets          # hameaux GeoNames (≈ 5 s)
uv run --project tools cent-ans geo info             # métadonnées, plage d'altitudes, fraction de terre, tailles
uv run --project tools pytest tests/test_geo.py      # tests sans réseau (grille, encodage des altitudes)
```

La construction complète prend quelques minutes (téléchargement inclus la première fois).

## Projection et convention de pixels

- CRS : **EPSG:3035** (Lambert azimutale équivalente Europe, centre 10° E / 52° N).
- Emprise géographique demandée : longitude −11° → 16°, latitude 35° → 60°. Projetée, cette
  emprise fait ≈ 2 464 km de large pour 2 945 km de haut : elle n'est pas carrée. Comme la
  heightmap est carrée (4096²) avec un seul `meters_per_px`, l'emprise **est élargie
  symétriquement en X** jusqu'à obtenir un carré (`project.squared_bounds`). L'emprise finale
  est stockée dans `map.json` (`bounds_projected`, mètres entiers) et couvre un peu plus
  d'Atlantique et de Germanie que demandé. **C'est l'unique écart au contrat.**
- Grille : 4096 × 4096 pixels, `meters_per_px` ≈ 719 m (isotrope).
- Coordonnées carte : origine au coin **nord-ouest** de `bounds_projected`, X vers l'est,
  Y vers le sud, unité = pixel. Conversion : `px = (x − minx) / mpp`, `py = (maxy − y) / mpp`.
  Le pixel entier `(i, j)` couvre `[i, i+1[ × [j, j+1[`, son centre est `(i + 0,5 ; j + 0,5)`.
  Transformée affine rasterio équivalente : `Affine(mpp, 0, minx, 0, −mpp, maxy)`.
- Helpers : `MapGrid.projected_to_pixel`, `pixel_to_projected`, `lonlat_to_pixel`,
  `pixel_to_lonlat` (`cent_ans_tools.geo.project`).

## Sorties (`data/map/`)

| Fichier | Fabrication |
|---|---|
| `map.json` | Contrat (`crs`, `bounds_projected`, `size_px`, `meters_per_px`, `height_min_m`, `height_max_m`) + `extent_lonlat`, `sources`, `generated_at`. |
| `heightmap.png` | Mosaïque des tuiles ETOPO → reprojection EPSG:3035 par **moyenne** (`rasterio.warp.reproject`) → encodage linéaire 16 bits : `v = round((h + 200) × 65535 / 5000)`, borné à [0, 65535] (−200 m → 0, 4800 m → 65535, pas ≈ 7,6 cm). Les fonds marins sous −200 m sont donc écrêtés à 0. |
| `land_mask.png` | Rasterisation des polygones `ne_10m_land` (255) puis retrait des lacs `ne_10m_lakes` + `ne_10m_lakes_europe` (0). Un pixel est terre si son centre tombe dans un polygone. |
| `rivers.geojson` | `ne_10m_rivers_lake_centerlines` + `ne_10m_rivers_europe`, projetés, découpés à l'emprise, convertis en pixels (1 décimale). Propriétés : `name`, `scalerank`, `featurecla`, `source`. Sans CRS (coordonnées carte). |
| `coastline.geojson` | `ne_10m_coastline` idem, propriétés `scalerank`, `featurecla`. |

Relevé de la construction du 2026-09-23 : `heightmap.png` 14,4 Mo (altitudes présentes −200 → 4549 m,
Mont Blanc moyenné sur 719 m), `land_mask.png` 0,1 Mo (41 % de terre), `rivers.geojson` 0,6 Mo
(485 tronçons, dont Rhin, Loire, Seine, Rhône, Garonne, Èbre, Tamise), `coastline.geojson` 0,6 Mo
(289 tronçons). Construction : ≈ 10 s hors téléchargement.

`docs/img/map-preview.png` : aperçu 1024² (relief hypsométrique + ombrage, côte en noir,
rivières en bleu) pour vérification visuelle.

## Régénérer

1. `uv run --project tools cent-ans geo build` (ajouter `--force` pour ignorer le cache).
2. Vérifier `uv run --project tools cent-ans geo info` et l'aperçu.
3. Commiter `data/map/*` et `docs/img/map-preview.png` ensemble : les provinces en dépendent.

Pour changer l'emprise ou la résolution, modifier les constantes de
`cent_ans_tools/geo/project.py` (`LON_MIN`… `SIZE_PX`) et la plage d'altitudes dans
`terrain.py`, puis reconstruire et mettre à jour le contrat de design.

## Provinces (`cent_ans_tools/geo/provinces.py`)

![Aperçu des provinces](img/provinces-preview.png)

Entrées : `data/map/` (grille, `land_mask.png`, `rivers.geojson`) et, pour chaque
`data/provinces/*.json`, `geo.seed_lonlat`, `geo.capital_lonlat`, `geo.voronoi_weight`, `owner`,
`name.display`. Les provinces sont indexées de 1 à 132 dans l'ordre alphabétique des ids
(`index` dans les propriétés du GeoJSON).

### Méthode : Voronoï pondéré par distance de coût

1. **Grille de travail 1024²** (blocs de 4 px, terre si ≥ 8 des 16 pixels sont terre). Chaque
   province a deux sources : son seed et sa capitale (la capitale appartient par définition à
   sa province ; un seed en mer est ramené sur la terre la plus proche, cas de Gênes).
2. **Coût par cellule** : terre = 1, cellule traversée par un fleuve majeur (`scalerank ≤ 4`,
   rastérisé) = 4, mer = infranchissable. Pour chaque province, `skimage.graph.MCP_Geometric`
   (Dijkstra géodésique, 8-connexité) donne la distance de coût depuis ses sources ; la cellule
   va à la province minimisant `coût / voronoi_weight`. Les provinces ne sautent donc jamais
   un détroit (Manche, Pyrénées contournées par les cols…) et les fleuves font frontière douce.
   132 propagations sur 1024² ≈ 8 s.
3. **Retour à 4096²** : suréchantillonnage au plus proche, masquage par `land_mask.png`,
   puis remplissage des pixels terre sans étiquette (îles sans seed — Man, Wight, Anglesey,
   Baléares mineures — et pixels perdus par le sous-échantillonnage) par l'étiquette la plus
   proche (distance euclidienne), **sauf** les masses continentales sans seed de plus de
   64 000 px (≈ 33 000 km²) qui restent à 0 : c'est l'Afrique du Nord (≈ 600 000 px), qui n'est
   pas jouable. Lissage des frontières par filtre majoritaire 5 × 5 (la mer est d'abord
   remplie par le plus proche voisin pour ne pas éroder les côtes, puis remasquée).
4. **Vectorisation** : `rasterio.features.shapes` (coins de pixels, 4-connexité) → union →
   `simplify(1,5 px, topologie préservée)` → parties < 30 px² supprimées (au moins une partie
   conservée). Les polygones sont simplifiés indépendamment : de minuscules écarts entre
   voisins sont possibles ; `province_ids.png` reste la référence pour le picking.
5. **Propriétés** : `centroid` = centroïde de la plus grande partie si elle le contient, sinon
   `representative_point` ; `capital_px` = projection de `capital_lonlat`, ramenée au pixel le
   plus proche de sa province si elle tombe en mer ou ailleurs (le rapport de construction le
   signale) ; `neighbors` = provinces partageant ≥ 3 paires de pixels adjacents (4-connexité)
   ; `sea_neighbors` = provinces non voisines par terre dont des pixels côtiers (1 sur 4,
   KD-tree) sont à moins de 60 px (≈ 43 km) ; `area_px` = nombre de pixels.

### Sorties

| Fichier | Contenu |
|---|---|
| `data/map/province_ids.png` | PNG RGB 4096² : `R = index & 255`, `G = index >> 8`, `B = 0`, 0 = mer ou aucune. |
| `data/map/provinces.geojson` | `FeatureCollection` en coordonnées carte (1 décimale), propriétés `id`, `index`, `name`, `owner`, `centroid`, `capital_px`, `neighbors`, `sea_neighbors`, `area_px`. |
| `docs/img/provinces-preview.png` | 1024² : remplissage par `heraldry.primary_color` du propriétaire sur ombrage du relief, frontières noires, capitales en points blancs. |

Relevé de la construction du 2026-09-23 : 10 s, 0,35 Mo de GeoJSON (34 multipolygones), 302
arêtes terrestres (0 à 10 voisins, 4,6 en moyenne ; la Sardaigne n'a que des liaisons
maritimes), 15 liaisons maritimes (Kent–Boulonnais, Boulonnais–Flandre, Flandre–Hollande,
Corse–Sardaigne, Normandie–Ponthieu, Galloway–Highlands…). Seed déplacé : Gênes (en mer sur la
grille de travail). Capitales ramenées : Corse (Ajaccio) et Vénétie (Venise), toutes deux en mer
sur le masque.

Pour retoucher une frontière : modifier `seed_lonlat` / `voronoi_weight` dans
`data/provinces/<id>.json`, relancer `cent-ans geo provinces`, vérifier l'aperçu, commiter
`data/map/provinces.geojson`, `data/map/province_ids.png` et `docs/img/provinces-preview.png`
ensemble.

## Colonies (`cent_ans_tools/geo/settlements.py`, lot C3)

![Aperçu des colonies](img/settlements-preview.png)

Entrées : `data/settlements/*.json`, `data/provinces/` (terrain, `capital_city`, ports),
`province_ids.png`, `provinces.geojson` (`neighbors`, `sea_neighbors`) et `roads.geojson` s'il
existe. Une province sans fichier (ou sans `city`) reçoit **la même cité de repli que le chargeur
Rust** (`Settlement::fallback_city`) : id `set_<slug>` (slug identique à `slugify` Rust), ou
`set_<slug>_<province>` si l'id est déjà pris par un fichier, `port` si la province est côtière
avec ports. Le graphe couvre donc toujours les 132 provinces ; relancer la commande quand des
fichiers de colonies arrivent.

1. **Position de jeu** : `lonlat` projeté (`MapGrid.lonlat_to_pixel`), tronqué au dixième de pixel.
   Si le pixel n'appartient pas à la province de la colonie (`province_ids.png`), la position est
   ramenée au pixel de la province le plus proche situé à ≥ 3 px de sa frontière et à ≥ 4 px des
   autres colonies (pour ne pas empiler Rye, Winchelsea et Hastings) ; le rapport le signale. Le
   `lonlat` des données n'est jamais modifié.
2. **Graphe** (spec § 4.4), non orienté, une entrée par paire (`from < to`) :
   - dans une province : triangulation de Delaunay ; arêtes > 2,5 × la médiane de la province
     retirées sauf si elles appartiennent à l'arbre couvrant minimal (graphe interne connexe) ;
   - entre provinces voisines (`neighbors`) : 1 à 3 paires les plus proches, sans colonie
     commune, à moins de 1,5 × la distance de la plus proche. Si le segment passe à plus de 50 %
     sur l'eau (détroits de Messine, Øresund…) et relie deux ports, l'arête devient `sea` ; sinon
     elle reste terrestre et le rapport le signale ;
   - entre `sea_neighbors` : la paire de colonies `port` la plus proche (`sea: true`) ; si l'une
     des provinces n'a aucun port, le rapport le signale.
3. **Coût** (sans unité, le cœur le rééchelonne) : longueur en km × coût du terrain
   (`terrain_cost` de `movement.rs` : 2 pour `mountains` et `marsh`, 1 sinon ; moyenne des deux
   provinces pour une arête frontalière), ÷ 2 si `road`. Arête `sea` : 100 (embarquement et
   débarquement, ≈ une traversée de province) + km.
4. **Route** : une arête terrestre porte `road: true` si la distance moyenne de son segment au
   réseau de `roads.geojson` (transformée de distance sur 4096²) est < max(3 km, 4 % de sa
   longueur) — la tolérance relative évite de rater une voie qui serpente le long d'une arête
   de 200 km entre deux cités de repli.

| Fichier | Contenu |
|---|---|
| `data/map/settlement_graph.json` | `{"edges": [{"from", "to", "cost", "road", "sea"}]}`, format du lot C1 (chargé par `settlement_load.rs`, aucun autre champ). |
| `data/map/settlements_px.json` | `{"set_…": [x, y]}` : position de jeu en pixels carte 4096 (1 décimale), pour Godot. |
| `data/map/settlement_edge_paths.json` | Lot C7b, affichage seulement : `{"edges": [{"from", "to", "points"}]}`, tracé routier (pixels carte, 1 décimale, de `from` à `to`, extrémités sur les colonies) de chaque arête `road` qu'une route suit ; lu par `SettlementData.edge_path` pour l'aperçu de chemin d'armée. Voir ci-dessous. |
| `docs/img/settlements-preview.png` | 2048² : provinces, colonies par type (cité rouge, ville orange, château gris, abbaye violette, village vert ; contour blanc = position ramenée), arêtes grises, routes brunes, liaisons maritimes en tirets bleus. |

Relevé du 2026-09-24 (132 fichiers de colonies, aucune cité de repli) : 568 colonies,
1 345 arêtes terrestres dont 634 sur route, 28 maritimes, graphe connexe ; 71 colonies ramenées
dans leur province (côtes et frontières du Voronoï : Saint-Malo, La Rochelle, Plymouth, Venise,
Alicante… ; certaines sont peut-être rattachées à la mauvaise province dans les données, par ex.
Galway en Ulster, Lund en Sjælland, Auch en Rouergue, Mantoue et Modène à Ferrare).

### Tracé routier des arêtes (`cent_ans_tools/geo/edge_paths.py`, lot C7b)

Écrit par la même commande (`geo settlements`, `geo roads`, `geo build`). Le graphe garde des
arêtes droites (coûts, règles) ; ce fichier ne sert qu'à dessiner l'aperçu de chemin le long des
vraies routes.

1. **Réseau** : les routes de `roads.geojson` sont densifiées (un point par pixel au plus) ; les
   points consécutifs sont reliés (poids = longueur) ainsi que tous les points à moins de 3 px
   l'un de l'autre (croisements, jonctions et petits trous entre tronçons Itiner-e ; poids
   × 1,5).
2. **Recherche** par arête `road` : Dijkstra depuis les points de route à moins de
   `min(16, max(6, 15 % de la longueur))` px de la première colonie (accès hors route compté
   × 3) jusqu'aux points proches de la seconde.
3. **Rejet** : un tracé plus long que 1,6 × le segment droit + 3 px est écarté (la route fait un
   détour par une autre ville) ; le jeu dessine alors le segment droit. Tracé simplifié à 0,15 px.

Relevé du 2026-09-24 : 533 arêtes tracées sur 633 arêtes `road` (84 %) ; détour médian 1,11,
95ᵉ centile 1,37 ; 29 points par arête en moyenne ; 0,3 Mo ; ≈ 4 s. Les 100 autres : colonie à
plus de 16 px d'une route (64), détour trop long (35), réseau coupé (1).

## Routes (`cent_ans_tools/geo/roads.py`)

Source retenue : **Itiner-e** (voir « Sources de données »), téléchargeable sans compte via l'API
Zenodo. Les tronçons `Hypothetical` sont écartés ; `Certain` et `Conjectured` (voies principales
et secondaires) sont reprojetés en pixels carte, découpés à l'emprise, simplifiés (0,4 px) et
gardés s'ils passent pour moitié au moins dans une province jouable.

Itiner-e s'arrête au limes (Irlande, Écosse, Germanie à l'est du Rhin, Danemark, Suède,
Bohême…). Les provinces qu'il ne couvre pas (densité < 4 px de route pour 1 000 px de province)
reçoivent des **routes calculées** : plus court chemin de coût (`skimage.graph.MCP_Geometric`,
grille 2048², coût = 1 + 25 × pente de `heightmap.png` + 6 sur un fleuve de `scalerank` ≤ 6, mer
infranchissable) le long des arêtes terrestres du graphe entre colonies `city`/`town`. Le même
calcul sert de repli complet si Itiner-e est indisponible (3 tentatives) ou avec `--computed`.

`data/map/roads.geojson` : `FeatureCollection` sans CRS, `LineString` en pixels carte
(1 décimale), propriétés `name`, `type` (`main`, `secondary`, `computed`), `certainty`,
`source` (`itiner-e` ou `computed`). Relevé du 2026-09-24 : 2,3 Mo, 6 834 tronçons Itiner-e et
70 tronçons calculés, ≈ 153 000 km.

## Hameaux (`cent_ans_tools/geo/hamlets.py`)

Décor sans état de jeu (spec § 3.3). Source : GeoNames `cities500` (CC BY 4.0, © GeoNames).

1. Classe `P`, codes `PPL`, `PPLA`–`PPLA5`, `PPLC`, `PPLF`, `PPLG`, `PPLL`, `PPLR`, `PPLS`
   (sections, lieux historiques, abandonnés ou détruits exclus), dans une province jouable ;
2. à plus de 3 km de toute colonie ;
3. ≈ 3 000 au total, répartis entre provinces pour moitié selon la surface, pour moitié selon
   la population de 1337 (`data/provinces`) ;
4. dans une province, candidats dans un ordre pseudo-aléatoire fixe (graine 1337 ; la population
   moderne favoriserait les villes industrielles), acceptés avec un espacement
   max(6 km, 0,6 × √(surface / quota)), puis complétés à 6 km si le quota n'est pas atteint.
   L'espacement de 6 km vaut aussi d'une province à l'autre.

`data/map/hamlets.json` : `[{"name", "px": [x, y], "province"}]`, nom GeoNames `name` (forme
locale, pas `asciiname`), une entrée par ligne. Relevé du 2026-09-24 : 2 999 hameaux dans les 132
provinces (71 306 candidats), 0,2 Mo.

## Relief en tuiles (`cent_ans_tools/geo/relief.py`)

Relief **8192²** (≈ 360 m/px) sur la même emprise (`bounds_projected` inchangé : 1 px 8192 = ½ px
4096), ETOPO 2022 15″ rééchantillonné **bilinéairement** (la cible est aussi fine que la
source ; une moyenne laisserait des marches), même encodage 16 bits que `heightmap.png`.
Découpé en 16 × 16 tuiles PNG de 512² : `data/map/height/h_<col>_<row>.png` couvre les pixels
`[col × 512, (col + 1) × 512[` en X et `[row × 512, (row + 1) × 512[` en Y, sans recouvrement
(pour coudre deux maillages, lire la première ligne ou colonne de la tuile voisine).
`map.json` déclare :

```json
"height_tiles": {"size_px": 8192, "tile_px": 512, "dir": "height", "pattern": "h_{col}_{row}.png"}
```

`heightmap.png` (4096²) est conservé pour la compatibilité. Relevé du 2026-09-24 : 256 tuiles,
50,5 Mo au total (pas de LFS dans le dépôt ; sous le seuil de 150 Mo, PNG 16 bits conservé),
≈ 10 s. Écart moyen avec `heightmap.png` après moyenne 2 × 2 : 0,5 m.


## Relief fin Copernicus et occupation du sol vers 1340 (lot R1, ADR 0019)

Ordre de régénération : `geo relief-shade` (après `geo build`) puis `geo landcover` (après
`geo splat`, qui l'appelle d'ailleurs à la fin). `geo relief` (ETOPO seul) reste un repli : il
écrase les tuiles fines Copernicus.

- `cent-ans geo relief-shade` (`relief_shade.py`, `copernicus.py`) : télécharge les 312 tuiles
  Copernicus DEM GLO-90 de l'emprise jouable (lon −11 → 12, lat 41 → 60 ; ≈ 1 Go dans
  `tools/geo/raw/copernicus/`, HTTPS anonyme, gratuit), les **moyenne** sur la grille 8192²
  (la source à 90 m est 4 × plus fine : moyenne de zone, pas d'interpolation), ETOPO en mer et
  hors emprise. Écrit les tuiles `height/`, `heightmap_render.png` (moyenne 2 × 2, relief de
  rendu lu par `MapData`) et `relief_shade.png` (LA8 8192² : détail + occlusion). Rehaussement
  de rendu (masque flou, σ 5 km, gain 0,8, ±120 m, effacé au-dessus de 600-1 600 m), trait de
  côte identique à `heightmap.png`. `heightmap.png` et `navgrid.png` ne changent pas. ≈ 40 s
  avec le cache `tools/geo/raw/copernicus_cache/cop_8192.npy`.
- `cent-ans geo kk10` : extraction KK10 1330-1349 par requêtes HTTP partielles (le fichier
  complet fait 18,5 Go) ; dépendances ponctuelles :
  `uv run --project tools --with h5py --with fsspec --with aiohttp --with requests cent-ans geo kk10`
  (≈ 8 min, cache `tools/geo/raw/kk10/*.npz` de 190 ko).
- `cent-ans geo landcover` (`landcover.py`) : `splat.png` 4096² (forêts : défrichement KK10 ×
  potentiel forestier, massifs nommés de `historical_forests.json`, allocation binaire par score
  bruit + terrain, essarts autour des villes et hameaux), `wetlands.png` (RGB : marais, étangs,
  prés humides, depuis `wetlands.json` et les fonds de vallée), `forest_kind.png` (L8 2048², part
  de résineux, pour le rendu des forêts). ≈ 70 s.

## Relief palier 3 : zones de détail E5-E7 (lot ZG3, ADR 0036)

```sh
uv run --project tools cent-ans geo detail-dem                       # toutes les zones
uv run --project tools cent-ans geo detail-dem --zones crecy,calais  # quelques zones
uv run --project tools cent-ans geo detail-dem --zones crecy --force # recuit même si à jour
```

`detail_dem.py` (cuisson), `detail_sources.py` (récupérateurs), `anachronisms.py` (effacement).
Écrit les tuiles `data/map/pyramid/E{5,6,7}/{col}_{row}.png` (gitignorées, même encodage que
`data/map/height/` : PNG `I;16`, [-200, 4800] m) et **seulement** les lignes des étages 5-7 de
`data/map/relief_pyramid.json` (une ligne JSON compacte par étage ; les autres lignes, écrites
par `geo pyramid`, restent identiques octet pour octet).

- **Zones** : `data/map/detail_zones.json` (34 zones : les 7 villes emblématiques, batailles,
  sièges et forteresses, chacune justifiée dans `why`). Emprises carrées centrées : E5 (11,2 m)
  sur `half_size_km`, E6 (5,6 m) sur min(half, 6 km), E7 (2,8 m) sur min(half, 3 km), réglables
  par zone (`level_half_km`). Les zones dont les tuiles se chevauchent (Paris + Vincennes,
  Bruges + L'Écluse, Orléans + Patay en E5) sont cuites ensemble.
- **Sources** (vérifiées le 25/09/2026, gratuites, sans clé) : IGN RGE ALTI (WMS-R
  Géoplateforme, GeoTIFF float32, 5010 px max, 3 requêtes simultanées), Environment Agency
  LIDAR Composite DTM 1 m (WCS 2.0.1, `scalefactor`), AHN `dtm_05m` (WCS PDOK, `scalesize`,
  lent), DHM Vlaanderen (WCS 2.0.1, réponse multipart, pas de mise à l'échelle : blocs de
  2 km à 1 m moyennés localement ; `DHMVI_DTM_5m` pour E5). Wallonie : MNT servi en images
  rendues seulement → repli GLO-30 (Tournai), rendu grossièrement « sol nu » par ouverture
  morphologique (150 m) puis flou léger. Chaque source est demandée à la moitié du pixel de
  l'étage (moyenne de zone ensuite), dans sa projection native ; décalage d'altitude TAW → NAP
  (−2,33 m) pour la Flandre. Politesse : requêtes limitées par hôte (1 à 3 simultanées, écart
  minimal), 6 essais à attente exponentielle, User-Agent du projet.
- **Cache brut** : `tools/geo/raw/detail/<zone>/E<k>/<source>_NNN.tif` (float32 arrondi au
  1/64 m, deflate), réponses Overpass `osm_modern_v2.json`, marqueurs `done_E<k>.json`
  (empreinte des paramètres : une zone à jour n'est pas recuite ; reprise après interruption
  bloc par bloc).
- **Effacement des anachronismes** : Overpass (`overpass-api.de`, une requête par zone,
  en série) → autoroutes et voies rapides (bretelles comprises), voies ferrées (y compris
  désaffectées), carrières, décharges, réservoirs et bassins, canaux (hors zones urbaines, où
  ils sont souvent médiévaux), pistes d'aéroport, digues et jetées de port ; ponts et tunnels
  exclus. Tampon par classe (16 à 60 m), dilatation de 2 px, puis interpolation harmonique
  (Laplace, résolution directe par composante) depuis les bords. Talus, terrasses, mottes et
  fossés anciens restent. Petits trous sans donnée (rivières, étangs < 25 ha) comblés de même.
  Aperçus avant/après (ombrage, masque en rouge) : `docs/img/zg3/`.
- **Raccord** : rehaussement de rendu identique à `heightmap_render.png` (σ 5 km, gain 0,8,
  ±120 m, effacé au-dessus de 600-1 600 m), base σ 5 km calculée à 90 m sur GLO-90 autour de
  la zone avec la source fine à l'intérieur ; fondu vers l'ancêtre (tuile existante la plus
  fine, interpolée bilinéairement comme le moteur) sur 20 % du demi-côté au bord de l'emprise et
  sur 2 pixels là où la source n'a pas de donnée (mer : côte de la source). Après chaque étage,
  les tuiles parentes E5/E6 reprennent la moyenne 2 × 2 de leurs enfants (pondérée par le
  fondu) : la pyramide reste cohérente d'un étage à l'autre.

## Pyramide de relief, paliers 1-2 (lot ZG1, ADR 0036)

```sh
uv run --project tools cent-ans geo pyramid                 # E1-E4 (reprise : tuiles présentes sautées)
uv run --project tools cent-ans geo pyramid --levels 1,2    # palier 1 seul (≈ 1 min sur 14 cœurs)
uv run --project tools cent-ans geo pyramid --levels 3,4 --workers 8 --limit 5   # essai
uv run --project tools cent-ans geo pyramid --force         # réécrit tout
```

Modules : `pyramid.py` (géométrie, palier 1, manifeste), `surface.py` (correction de GLO-30,
palier 2), `glo30.py` (téléchargements GLO-30 et WorldCover, mosaïque avant reprojection).

- **Tuiles** : `data/map/pyramid/E{k}/{col}_{row}.png` (hors git ; dans un worktree, lien
  symbolique vers le dépôt principal). PNG 16 bits 512², même encodage qu'E0
  (`[-200, 4 800] m`), grille EPSG:3035 de `map.json` à `8192 · 2^k` pixels, pixel centré.
  Étage k : `359,49 / 2^k` m/px ; enfants de (k, c, r) : (k+1, 2c+dx, 2r+dy). Pas de tuile
  entièrement en mer (terre d'E0 ≥ 0,25 m dans l'empreinte).
- **E1-E2 (palier 1)** : Copernicus GLO-90 (cache de `geo relief-shade`), terres de l'emprise
  lon −11 → 12, lat 41 → 60. Par tuile E1 : E2 = moyenne de zone de la mosaïque GLO-90 (les
  tuiles 1° sont assemblées **avant** la reprojection : warper tuile par tuile laisse des
  coutures, présentes dans E0), E1 = moyenne 2 × 2 d'E2. Trait de côte d'E0 (surface bilinéaire
  d'E0 > 0).
- **E3-E4 (palier 2)** : Copernicus GLO-30 (172 tuiles, 5,2 Go, `tools/geo/raw/copernicus30/`)
  corrigé en modèle de terrain avec ESA WorldCover 2021 (23 tuiles, 1,5 Go,
  `tools/geo/raw/worldcover/`, lues au 1/2 ≈ 20 m) : canopée (10 m × fraction arborée, valeur
  mesurée aux lisières sur terrain plat), bâti (ouverture morphologique de 340 m sur la surface
  brute puis fermeture de 180 m contre les trous radar, fondu au bord du masque), retenues de
  `data/map/modern_reservoirs.json` (plan d'eau détecté : eau WorldCover au niveau plat de
  GLO-30 ; remplacé par une membrane interpolée depuis les berges moins un profil de vallée qui
  prolonge la pente des berges, plafonné à 0,8 × la hauteur du barrage ; rapport de détection
  dans `tools/geo/raw/pyramid_work/reservoirs/report.json`). Par tuile E2 du cœur : E4 calculé sur
  2 048² + marge de 128 px, E3 = moyenne 2 × 2. Côte : celle de la source le long du rivage d'E0
  (bande d'un pixel E0), celle d'E0 ailleurs, fondu de 2 pixels vers l'eau ; sans GLO-30 (bord du
  cœur), repli sur E2. Cœur : lon −6 → 9, lat 42 → 56 **moins** `pyramid.CORE_EXCLUDE` (Espagne,
  Italie, Suisse, Allemagne de la rive droite du Rhin, Écosse, Irlande) pour tenir le budget.
- **Rehaussement** : celui de `heightmap_render.png` (ADR 0019), avec la **même base floue**
  qu'E0 (`tools/geo/raw/pyramid_work/base.npy`, flou σ 5 km de la mosaïque Copernicus + ETOPO
  8192²) échantillonnée bilinéairement à chaque étage : E0 se reconstruit à 0,04 m près depuis
  cette base, et seul l'écrêtage à ±120 m n'est pas linéaire entre étages.
- **Manifeste** `data/map/relief_pyramid.json` : `update_manifest_levels` ne réécrit que les
  lignes des étages cuits (une entrée de `levels` par ligne) et la ligne `cache` (taille et nombre
  de tuiles de **tout** le cache, étages de ZG3 compris).
- **Cache de travail** : `tools/geo/raw/pyramid_work/` (`e0.npy`, `base.npy`, `coast.npy`,
  256 Mo chacun ; `reservoirs/*.npz`).

## Hydrographie fine, ancrages et routes drapées (lot ZG5a, ADR 0036)

```sh
uv run --project tools cent-ans geo hydro-fine                 # réseau fin (sources en cache, recalage parallèle, reprise)
uv run --project tools cent-ans geo hydro-fine --sources osor  # une seule source (essai)
uv run --project tools cent-ans geo anchors-fine               # colonies, hameaux, ponts, routes drapées (après hydro-fine)
```

Données de rendu seulement : `rivers_render.json`, `river_bed.png`, `crossings*.json`,
`navgrid.png` et les positions de règles ne changent pas (vue stratégique et règles).

### Sources (vérifiées le 25/09/2026, HTTPS anonyme, 0 $)

| Zone | Source | Licence | Accès |
|---|---|---|---|
| France | BD TOPAGE® 2025, `TronconHydrographique_FXX` (3,04 M tronçons ; `NumeroOrdreTH` vide → Strahler recalculé) | Licence Ouverte 2.0 | `services.sandre.eaufrance.fr/telechargement/geo/ETH/BDTopage/2025/TronconHydrographique/TronconHydrographique_FXX-gpkg.zip` (857 Mo, 3,0 Go décompressé) |
| Grande-Bretagne | OS Open Rivers (193 k tronçons, noms gallois/anglais) | OGL v3 | `api.os.uk/downloads/v1/products/OpenRivers/downloads?area=GB&format=GeoPackage&redirect` (52 Mo) |
| Bénélux, Rhénanie, Suisse romande, Piémont, versant sud des Pyrénées | EU-Hydro River Network Database v1.3 (ordres de Strahler 3 à 9) | Copernicus (libre, attribution) | service REST ArcGIS public de l'AEE `image.discomap.eea.europa.eu/arcgis/rest/services/EUHydro/EUHydro_RiverNetworkDatabase/MapServer/{5..13}/query`, par cellules de 1°, pages de 1 000 (`tools/geo/raw/hydro/euhydro/`) |
| Hors cœur | Natural Earth 10 m (lignes de `rivers.geojson`) | domaine public | déjà en cache |

EU-Hydro complet (téléchargement par bassin) exige un compte WEkEO/CLMS : le service de
consultation de l'AEE suffit. Aucune donnée OpenStreetMap. Les fichiers bruts sont dans
`tools/geo/raw/hydro/` (≈ 4 Go avec le gpkg TOPAGE décompressé), les tables de tronçons
projetées dans `tools/geo/raw/hydro/cache/links_<source>.npz`.

### Méthode (`geo/hydro_sources.py`, `geo/hydro_fine.py`, `geo/valley_snap.py`)

1. **Tronçons orientés** (EPSG:3035) : TOPAGE (nœuds `CdNoeudDebut`/`CdNoeudFin`, sans les
   conduites, buses et tronçons souterrains), OS (sens `flow_direction`, nom anglais préféré :
   `Afon Hafren` → `River Severn`), EU-Hydro (`FNODE`/`TNODE`, noms des masses d'eau nettoyés :
   `MEUSE 6` → `Meuse`), Natural Earth (orienté par l'altitude).
2. **Ordre de Strahler** calculé sur la topologie (tri topologique ; deux bras d'un même cours
   d'eau qui se rejoignent n'augmentent pas l'ordre).
3. **Sélection** : ordre ≥ 3 ; canaux de la source retirés sauf cours artificiels antérieurs à
   1340 (`kept_artificial` : Fossdyke, biefs de moulins) ; noms de canaux modernes retirés
   (`modern_canals` de `data/map/historical_hydro_notes.json`). Chaque source n'est gardée que
   là où elle fait autorité : TOPAGE et OS sur le cœur (tuile E3 présente), EU-Hydro sur le
   cœur hors des cellules de 2 km touchées par TOPAGE ou OS, Natural Earth hors du cœur.
4. **Traits** : les tronçons sont chaînés du haut vers le bas en suivant la branche principale à
   chaque confluence (ordre le plus haut, même cours d'eau, plus long amont) et, à chaque
   défluence, le bras principal (`BrasTH`, largeur, permanence, bras le plus direct).
5. **Recalage** sur l'étage le plus fin présent sous chaque point (E7 → E4 sur le cœur, E2 hors
   cœur ; hauteurs de rendu, rehaussement ADR 0019 compris) : rééchantillonnage à 20 m (60 m pour
   Natural Earth), puis recherche de Viterbi sur 33 à 61 décalages le long de la normale lissée
   (rayon 40 / 60 / 100 m selon l'ordre pour TOPAGE et OS, 60 / 90 / 150 m pour EU-Hydro,
   1 200 m pour Natural Earth) : coût = altitude du fond + attache au tracé source (2 à 4 m au
   bord du rayon) + changement latéral (0,03 à 0,08 m par mètre), saut latéral ≤ 0,7 × le pas,
   décalages bornés par le rayon de courbure (pas de raccourci à travers un méandre), lissage
   gaussien des décalages. Parallèle par lots de ~1 500 km, résultat de chaque lot en cache
   (`cache/snap/`, clé = version des paramètres + géométrie) : une commande interrompue reprend.
6. **Niveau d'eau** : régression isotone décroissante (PAVA) des altitudes du fond le long de
   chaque trait, en traitant chaque fleuve avant ses affluents ; la queue d'un affluent est
   relevée au niveau du fleuve récepteur au confluent (jamais d'eau qui remonte), et ses six
   derniers sommets sont fondus sur le point de confluence.
7. **Largeurs** (m) : ancrages nommés de `data/map/river_widths.json` (Seine, Loire, Garonne,
   Gironde, Dordogne, Somme, Rhône, Meuse, Escaut, Tamise, Severn, Trent, Humber, Rhin, Moselle,
   Charente, Vienne, Oise, Marne ; interpolation logarithmique le long du fleuve, décroissance
   en `(distance depuis la source)^0,7` en amont du premier ancrage), sinon largeur par ordre de
   Strahler (1,5 m à l'ordre 1 … 220 m à l'ordre 9) bornée par la classe `ClasseLargeurTH` de
   TOPAGE ; jamais plus étroit vers l'aval.
8. **Drapeaux** par ligne : zones de `historical_hydro_notes.json` (cours divagant non endigué,
   tronçon rectifié depuis 1340, marais non drainé, estuaire à marée), salinité TOPAGE / `tidalRiver`
   OS, écoulement intermittent, source grossière (Natural Earth).
9. **Tuiles** : simplification de Douglas-Peucker (0,35 × le pixel de l'étage le plus fin, 1,5 à
   30 m), passage en unités monde, découpe aux bords des tuiles E2.

### Format CAFV (tuiles `data/map/pyramid/hydro_fine/E2/{col}_{row}.bin`, hors git)

Une tuile E2 couvre 64 unités monde (≈ 46 km) : `col = floor(x / 64)`, `row = floor(y / 64)`.
Petit-boutiste :

| Champ | Type | Contenu |
|---|---|---|
| en-tête | `4s H H H H I I I I` (32 o) | `CAFV`, version 1, couche (1 fleuves, 2 routes), étage (2), 0, col, row, nombre de lignes, nombre de sommets |
| lignes | `u32 × 4` par ligne | entité, premier sommet, nombre de sommets, drapeaux |
| `x`, `y` | `f32 × n` chacun | unités monde (pixels carte 4096, origine nord-ouest) |
| `z` | `f32 × n` | niveau d'eau / surface de route en mètres (non exagéré) |
| `w` | `f32 × n` | largeur en mètres |

Drapeaux : 1 divagant, 2 marée, 4 intermittent, 8 rectifié depuis 1340, 16 source grossière,
32 marais (fleuve) ou chaussée en zone humide (route), 64 route principale, 128 route
calculée ; bits 24-27 : ordre de Strahler du trait ; bits 28-31 : source (`source_codes` du
manifeste : 1 TOPAGE, 2 OS, 3 EU-Hydro, 4 Natural Earth). Une ligne coupée au bord d'une tuile
partage son sommet de coupe avec la suivante. Lecture en GDScript :
`FileAccess.get_buffer(4 * n).to_float32_array()` pour chaque tableau.

- `data/map/rivers_fine.json` (versionné, schéma `rivers_fine.schema.json`) : format, drapeaux,
  sources, statistiques, index des tuiles (lignes, sommets, octets, empreinte SHA-1).
- `data/map/pyramid/hydro_fine/features.json` (cache) : table des entités (nom, source, ordre,
  récepteur, longueur).

### Ancrages et routes (`geo/fine_anchors.py`)

- `data/map/fine_anchors.json` (versionné, schéma `fine_anchors.schema.json`) :
  - `settlements` : `{id: {px, z, moved_m?, reason?}}` ; une colonie ne bouge (≤ 300 m, grille de
    25 m) que si sa position de règle tombe dans le lit d'un fleuve fin (`lit`) ou sur une pente
    > 20 % (`pente`) ; coût = pente, proximité du lit, distance parcourue.
  - `hamlets` : `items[i] = [x, y, z, moved_m]` dans l'ordre de `hamlets.json`, même règle.
  - `crossings` : dans l'ordre de `crossings_px.json` ; position **sur** le fleuve fin (le plus
    proche du même nom à ≤ 2 km, sinon le plus proche à ≤ 500 m ; les 114 passages historiques
    partent de leur lon/lat de `crossings.json`), `river_feature`, `dir` (sens du courant, unités
    monde), `width_m`, `z_water`, `z_deck` (plus haute des deux berges à ½ largeur + 8 m) ;
    `snapped: false` et `z_ground` sinon.
  - `roads` : index des tuiles CAFV couche 2 de `data/map/pyramid/roads_fine/E2/` (entité =
    index dans `roads.geojson`, largeur 6 / 4 / 3 m pour `main` / `secondary` / `computed`).
- **Routes** : densifiées à 25 m sur le cœur (90 m ailleurs) ; sur le cœur, décalage latéral de
  Viterbi (±50 m, 11 décalages) qui minimise dénivelé entre sommets, lit des fleuves fins,
  fonds humides (`wetlands.png` × position basse dans le profil en travers) et écart au tracé ;
  altitudes drapées (lissage gaussien 60 m, déblai/remblai ≤ 3 m), tablier horizontal entre les
  berges d'un fleuve franchi ; simplification 3D (2,5 m, z × 4).

### Contrat pour ZG5b (rendu)

- **Fleuves** : charger `rivers_fine.json`, puis les tuiles E2 sous les nœuds du quadtree à
  partir du palier 1 (zoom ≤ ~5 unités) ; hors cache (fichier absent), garder
  `rivers_render.json` + `river_bed.png`. Mailler chaque ligne en ruban : demi-largeur
  `max(w / 2, 1 pixel écran)` le long de la normale, altitude `z × échelle verticale`, berges en
  fondu ; creuser le lit dans le shader de terrain (le niveau `z` est ≤ au fond de vallée du
  relief à la précision de l'étage près). Les rubans de deux tuiles voisines se raccordent au
  sommet partagé. Drapeaux : `divagating` → bras et grèves (Rhin supérieur, Loire, Rhône),
  `tidal` → vasières et eau saumâtre, `wetland` → roselières, `intermittent` → lit caillouteux
  plus étroit. Ordre (bits 24-27) : n'afficher que les ordres ≥ 5 au palier 1, ≥ 3 au palier 2.
- **Routes** : même chargement ; ruban drapé à `z`, largeur `w` ; drapeau 32 → chaussée
  surélevée. Au zoom éloigné, garder `roads.geojson`.
- **Colonies, hameaux, ponts** : décaler les modèles aux `px` de `fine_anchors.json` et les poser
  à `z` (le sol le plus fin chargé reste la référence en cas d'écart) ; ponts orientés
  perpendiculairement à `dir`, portée `width_m`, tablier à `z_deck`.
