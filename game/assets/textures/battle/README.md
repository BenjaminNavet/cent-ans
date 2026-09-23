# Textures des batailles (lot V4)

Toutes les textures photographiques viennent de **Poly Haven** (https://polyhaven.com), licence
**CC0** (domaine public, aucune attribution requise). Résolution source 1k (JPG), récupérées via
`https://api.polyhaven.com/files/<id>` puis assemblées par `build_textures.py`.

| Fichier | Identifiant(s) Poly Haven | Usage |
|---|---|---|
| `ground_albedo_array.jpg` / `ground_normal_array.jpg` | `sparse_grass`, `rocky_terrain_02`, `forest_leaves_04`, `coast_sand_01`, `muddy_tracks`, `river_small_rocks`, `aerial_rocks_02`, `farm_soil`, `snow_02` | Sol (Texture2DArray de 9 couches, dans cet ordre) |
| `castle_wall_varriation_*` | `castle_wall_varriation` | Courtines, tours, porte |
| `roof_slates_02_*` | `roof_slates_02` | Toits d'ardoise (tours, église) |
| `clay_roof_tiles_02_*` | `clay_roof_tiles_02` | Toits de tuiles |
| `thatch_roof_angled_*` | `thatch_roof_angled` | Toits de chaume |
| `plastered_wall_02_*` | `plastered_wall_02` | Murs enduits des maisons |
| `cobblestone_floor_01_*` | `cobblestone_floor_01` | Place pavée |
| `wood_planks_*` | `wood_planks` | Vantaux, machines de siège |
| `bark_brown_02_*` | `bark_brown_02` | Troncs d'arbres |

`*_diff.jpg` = albédo 1024², `*_nor.jpg` = normale OpenGL 512² (importée en carte normale).

Textures **procédurales** (générées par `build_textures.py`, CC0 elles aussi) :
- `foliage_leaves.png` : carte alpha d'un amas de feuilles (houppiers, buissons) ;
- `grass_clump.png` : carte alpha d'une touffe d'herbe.

Régénérer : télécharger les fichiers `<id>_diff_1k.jpg` et `<id>_nor_gl_1k.jpg` dans un dossier,
puis `uv run --with pillow --with numpy python build_textures.py <dossier>`.
