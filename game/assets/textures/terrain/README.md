# Textures du terrain de campagne et de la mer

Textures PBR **CC0** (domaine public) de [Poly Haven](https://polyhaven.com), construites par
`uv run --project tools cent-ans geo textures` (`tools/cent_ans_tools/geo/textures.py`).
Identité des couches (ordre, asset Poly Haven, moyenne d'albédo) : `data/fx/
campaign_terrain_textures.json` (lot GA4, schéma `fx_campaign_terrain_textures.schema.json`) ;
l'ordre suit le contrat d'index fixe de `terrain.gdshader`.

| Couche (index) | Asset Poly Haven | Usage |
|---|---|---|
| 0 `grass` | [`aerial_grass_rock`](https://polyhaven.com/a/aerial_grass_rock) | prairie (splat R) |
| 1 `farmland` | [`aerial_mud_1`](https://polyhaven.com/a/aerial_mud_1) | cultures, labours (splat G) |
| 2 `forest` | [`forrest_ground_01`](https://polyhaven.com/a/forrest_ground_01) | forêt (splat B) |
| 3 `rock` | [`aerial_rocks_01`](https://polyhaven.com/a/aerial_rocks_01) | roche (splat A en altitude / pente) |
| 4 `heath` | [`sparse_grass`](https://polyhaven.com/a/sparse_grass) | lande (splat A en plaine) |
| 5 `snow` | [`snow_field_aerial`](https://polyhaven.com/a/snow_field_aerial) | neige des hauts sommets (altitude) |
| 6 `sand` | [`aerial_beach_01`](https://polyhaven.com/a/aerial_beach_01) | sable des côtes (distance à la côte) |

## Lot GA4 (chemin par défaut)

- `terrain_albedo_array.jpg` : cartes `Diffuse` 2k (sRGB) empilées verticalement (2048 × 7·2048),
  importées en `Texture2DArray` compressé en VRAM avec mipmaps (`.import` : `slices/vertical=7`) ;
- `terrain_normal_array.jpg` : même empilement en 2k ; R, G = normale OpenGL (`nor_gl`) X, Y ;
  B = rugosité (`Rough`). Le shader reconstruit Z ;
- `water_normal.png` : normale de mer tuilable 1024², **procédurale** (œuvre propre, CC0) —
  ni Poly Haven ni ambientCG ne proposent de normale d'eau CC0 ; somme de trains d'ondes à
  vecteurs d'onde entiers (raccord exact), direction de vent dominante
  (`textures.water_normal`). Importée en carte de normales (RGTC).

## Ancien chemin (`--no-ga4`, comparaison A/B)

- `<couche>_albedo.jpg` / `<couche>_normal_rough.jpg` : mêmes assets en 1k, assemblés à
  l'exécution en `Texture2DArray` RGBA8 non compressés par `TerrainBuilder`. À supprimer après
  le jugement de GA6 si le chemin GA4 est retenu.

Licence : CC0 1.0 Universal, https://polyhaven.com/license — aucune attribution requise.
