# Textures du terrain de campagne

Textures PBR **CC0** (domaine public) de [Poly Haven](https://polyhaven.com), résolution 1k, téléchargées
par `uv run --project tools cent-ans geo textures` (`tools/cent_ans_tools/geo/textures.py`).

| Couche (index) | Asset Poly Haven | Usage |
|---|---|---|
| 0 `grass` | [`aerial_grass_rock`](https://polyhaven.com/a/aerial_grass_rock) | prairie (splat R) |
| 1 `farmland` | [`aerial_mud_1`](https://polyhaven.com/a/aerial_mud_1) | cultures, labours (splat G) |
| 2 `forest` | [`forrest_ground_01`](https://polyhaven.com/a/forrest_ground_01) | forêt (splat B) |
| 3 `rock` | [`aerial_rocks_01`](https://polyhaven.com/a/aerial_rocks_01) | roche (splat A en altitude / pente) |
| 4 `heath` | [`sparse_grass`](https://polyhaven.com/a/sparse_grass) | lande (splat A en plaine) |
| 5 `snow` | [`snow_field_aerial`](https://polyhaven.com/a/snow_field_aerial) | neige des hauts sommets (altitude) |
| 6 `sand` | [`aerial_beach_01`](https://polyhaven.com/a/aerial_beach_01) | sable des côtes (distance à la côte) |

Fichiers par couche :

- `<couche>_albedo.jpg` : carte `Diffuse` (sRGB) ;
- `<couche>_normal_rough.jpg` : R, G = normale OpenGL (`nor_gl`) X, Y ; B = rugosité (`Rough`).
  Le shader reconstruit Z.

`TerrainBuilder` assemble ces images en deux `Texture2DArray` (albédo ; normale + rugosité).
Licence : CC0 1.0 Universal, https://polyhaven.com/license — aucune attribution requise.
