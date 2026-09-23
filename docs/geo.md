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
| Terre, côte, rivières, lacs | [Natural Earth](https://www.naturalearthdata.com/) 10 m physical, servi par `https://naciscdn.org/naturalearth/10m/physical/<couche>.zip` | Domaine public | `ne_10m_land`, `ne_10m_coastline`, `ne_10m_rivers_lake_centerlines`, `ne_10m_rivers_europe`, `ne_10m_lakes`, `ne_10m_lakes_europe` |

Pourquoi ETOPO 2022 plutôt que GMTED2010 : téléchargement HTTPS direct sans compte, tuiles
légères, bathymétrie incluse (utile pour ombrer la mer) et résolution (≈ 300–460 m) plus fine
que le pixel de la carte (≈ 719 m), donc rééchantillonnage par moyenne sans artefact.

Les fichiers bruts sont mis en cache dans `tools/geo/raw/` (ignoré par git). Un fichier déjà
présent n'est jamais retéléchargé sauf avec `--force`.

## Commandes

```sh
uv run --project tools cent-ans geo build            # télécharge (cache) puis génère data/map/ (terrain + provinces) et docs/img/*-preview.png
uv run --project tools cent-ans geo build --force    # retélécharge les données brutes
uv run --project tools cent-ans geo provinces        # provinces seules (≈ 10 s), après modification de seeds ou de poids
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
