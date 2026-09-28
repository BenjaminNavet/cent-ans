# M1 — Carte de campagne : spécification

Date : 2026-09-23. Objectif : une carte 3D de l'Europe de l'Ouest élargie, réelle, découpée
en provinces historiques de 1337, navigable à la caméra, avec villes placées et sélection de province.

## 1. Emprise et projection

- Emprise (ADR 0115, depuis OM2) : rectangle projeté fixe x 2 169 486 → 7 323 110 m,
  y 775 684 → 5 193 076 m, soit 7168 × 6144 unités de 718,9765625 m (28 × 24 tuiles racines de
  256) : Maroc atlantique → Oural, mer Blanche → delta du Nil, Caspienne occidentale.
  Avant OM2 : lon −11 → 16°, lat 35 → 60° en 4096² ; un ancien pixel (x, y) vaut (x, y + 1280).
- Projection : Lambert azimutale équivalente Europe (EPSG:3035). Toutes les coordonnées de jeu
  sont en **coordonnées carte** : origine au coin nord-ouest de l'emprise projetée, axe X vers l'est,
  axe Y vers le sud, unité = pixel de la heightmap.
- Heightmap : 7168 × 6144 pixels, PNG 16 bits en niveaux de gris, valeur 0 = -200 m, 65535 = 4800 m
  (linéaire, sommets du Caucase écrêtés à 4800 m). Résolution ≈ 719 m/pixel. Mer = altitude ≤ 0 → masque terre/mer séparé.
- Dans Godot : 1 pixel = 1 unité monde (X = x carte, Z = y carte, Y = altitude × exagération 0,02).

## 2. Contrat de données (`data/map/`)

| Fichier | Contenu |
|---|---|
| `map.json` | `{ "crs": "EPSG:3035", "bounds_projected": [minx, miny, maxx, maxy], "size_px": [7168, 6144], "meters_per_px": ..., "height_min_m": -200, "height_max_m": 4800 }` |
| `heightmap.png` | PNG 16 bits |
| `land_mask.png` | PNG 8 bits, 255 = terre |
| `rivers.geojson` | LineStrings en coordonnées carte, propriété `name`, `strahler` ou `scalerank` |
| `coastline.geojson` | LineStrings côte (pour rendu) |
| `provinces.geojson` | Polygones (MultiPolygon possible) en coordonnées carte ; propriétés : `id` (= id de `data/provinces/`), `centroid` [x, y], `neighbors` [ids], `capital_px` [x, y] |
| `province_ids.png` | PNG RGB 7168 × 6144, couleur = index de province (R = idx & 255, G = idx >> 8), 0 = mer/aucune (terres à plus de ~400 km d'une graine : hors provinces, infranchissables) ; sert au picking |

Les données de gameplay des provinces restent dans `data/provinces/*.json` (schéma existant).
Chaque province y reçoit des champs `geo: { capital_lonlat: [lon, lat], seed_lonlat: [lon, lat] }`.

## 3. Génération des provinces

Approche v1 : liste curée de ~120 provinces historiques (nom d'époque, capitale, propriétaire 1337,
terrain, ressources, seed lon/lat). Polygones = diagramme de Voronoï pondéré sur les seeds,
découpé par le masque terre, avec les grands fleuves comme barrières douces (coût de traversée) ;
puis lissage et calcul des voisins (arêtes partagées, plus liaisons maritimes explicites entre ports).
Les frontières ne seront pas parfaites ; elles seront retouchées par édition des seeds/poids.

## 4. Rendu Godot (`game/`)

- Terrain : maillage par tuiles (ex. 16 × 16 tuiles de 256 px) généré depuis la heightmap, LOD simple
  par distance, shader de terrain par altitude/pente (mer, plaine, colline, montagne, neige) avec
  teinte parchemin légère.
- Mer : plan à altitude 0, couleur unie, côte tracée.
- Provinces : overlay de frontières (texture d'ID → arêtes en shader) + surbrillance de la province
  survolée et sélectionnée ; couleur de faction par province (palette dans `data/factions`).
- Villes : marqueur 3D placeholder (cylindre + étiquette nom) à `capital_px`.
- Rivières : polylignes en léger relief.
- Caméra RTS : pan (WASD/bords/clic molette), zoom (molette, borné), rotation (Q/E), inclinaison liée au zoom.
- Picking : lecture de `province_ids.png` au point sous le curseur (raycast sur le terrain → x, z).
- Panneau latéral (placeholder) : nom de province, propriétaire, capitale, terrain.

## 5. Chargement des données (`core/`)

`data-model` charge tous les `data/**/*.json` en structs typées conformes aux schémas,
vérifie les références (ids inconnus → avertissement, pas d'échec, tant que la carte est partielle),
et expose à Godot : liste des provinces (id, nom, owner, capital_px, centroid, neighbors),
liste des factions (id, nom, couleur). Godot ne lit jamais `data/` directement sauf les images de `data/map/`.

## 6. Critères de fin de M1
- `tools/cent_ans_tools/geo` reproduit `data/map/` depuis zéro en une commande (téléchargements en cache).
- ~120 provinces validées par schéma, chacune avec polygone, voisins et capitale.
- `godot --path game` affiche la carte, permet de naviguer, survoler/sélectionner une province et voir son propriétaire.
- Smoke test headless étendu : chargement des données + carte sans erreur.
