# Textures des batailles (lot V4, sol en 2k depuis GA2)

Toutes les textures photographiques viennent de **Poly Haven** (https://polyhaven.com), licence
**CC0** (domaine public, aucune attribution requise), récupérées via
`https://api.polyhaven.com/files/<id>` puis assemblées par `build_textures.py`.

## Sol de bataille (lot GA2, 13 couches, 2k)

L'identité des couches (identifiant Poly Haven, ordre d'empilement, taille de répétition au sol)
vit dans `data/fx/battle_ground_layers.json` (schéma `data/schemas/fx_battle_ground_layers.schema.json`),
**pas dans ce README ni dans `build_textures.py`** qui la lisent tous deux. Résumé (voir le JSON
pour la source de vérité) :

| # | id | Identifiant Poly Haven | Rôle |
|---|---|---|---|
| 0 | `grass` | `sparse_grass` | herbe |
| 1 | `meadow` | `rocky_terrain_02` | prairie caillouteuse |
| 2 | `forest` | `forest_leaves_04` | sous-bois |
| 3 | `dirt` | `coast_sand_01` | terre battue (chemins) |
| 4 | `mud` | `muddy_tracks` | boue |
| 5 | `pebbles` | `river_small_rocks` | galets (lit, gués, berges) |
| 6 | `rock` | `aerial_rocks_02` | roche (pentes) |
| 7 | `plough` | `farm_soil` | labour ambiant/procédural (loin des parcelles du décor) |
| 8 | `snow` | `snow_02` | neige |
| 9 | `flowering_meadow` | `leafy_grass` | prairie fleurie (GA2 — voir note ci-dessous) |
| 10 | `trodden_grass` | `grassy_cobblestone` | herbe piétinée (abords des chemins) |
| 11 | `stubble` | `withered_grass` | chaume/éteules (parcelles moissonnées, EP6) |
| 12 | `fresh_plough` | `farm_furrows` | labour frais (parcelles de labour du décor, EP6) |

Note (« prairie fleurie ») : Poly Haven n'a pas de texture CC0 dédiée « prairie fleurie » /
wildflower meadow au moment du lot GA2 ; `leafy_grass` (herbe verte feuillue) est le substitut le
plus proche trouvé dans le catalogue. À revoir si une meilleure source CC0 apparaît.

Albédo : `ground_albedo_array.jpg`, 2048² par couche (`<id>_diff_2k.jpg`). Normale (OpenGL) :
`ground_normal_array.jpg`, 1024² par couche (`<id>_nor_gl_1k.jpg`) — normale en 1k pour tenir le
budget mémoire (mesure GA2 dans `docs/wip/ga.md` : 13 couches en 2k/2k ≈ 138,6 Mo > 120 Mo ;
2k/1k ≈ 86,6 Mo).

## Bâtiments et écorce (lot V4, inchangé par GA2)

| Fichier | Identifiant(s) Poly Haven | Usage |
|---|---|---|
| `castle_wall_varriation_*` | `castle_wall_varriation` | Courtines, tours, porte |
| `roof_slates_02_*` | `roof_slates_02` | Toits d'ardoise (tours, église) |
| `clay_roof_tiles_02_*` | `clay_roof_tiles_02` | Toits de tuiles |
| `thatch_roof_angled_*` | `thatch_roof_angled` | Toits de chaume |
| `plastered_wall_02_*` | `plastered_wall_02` | Murs enduits des maisons |
| `cobblestone_floor_01_*` | `cobblestone_floor_01` | Place pavée |
| `wood_planks_*` | `wood_planks` | Vantaux, machines de siège |
| `bark_brown_02_*` | `bark_brown_02` | Troncs d'arbres |

`*_diff.jpg` = albédo 1024², `*_nor.jpg` = normale OpenGL 512² (importée en carte normale).
Ces textures restent en 1k (lot GA5, hors périmètre GA2).

Textures **procédurales** (générées par `build_textures.py`, CC0 elles aussi) :
- `foliage_leaves.png` : carte alpha d'un amas de feuilles (houppiers, buissons) ;
- `grass_clump.png` : carte alpha d'une touffe d'herbe.

Textures **procédurales du lot DA6** (générées par `build_da6_textures.py`, CC0, aucune source
tierce ; `uv run --with pillow --with numpy python build_da6_textures.py`) :
- `grass_blades.png` : touffe en éventail (luminance et alpha seuls) ;
- `leaf_spray.png` : rameau feuillu des arbres (`BattleTrees`) ;
- `twig_spray.png` : ramilles nues (feuillus en hiver) ;
- `dead_leaves.png` : feuilles sèches (chêne marcescent en hiver).

Régénérer : télécharger, pour chaque couche du sol de `data/fx/battle_ground_layers.json`, les
fichiers `<id>_diff_2k.jpg` et `<id>_nor_gl_1k.jpg`, et pour les textures uniques ci-dessus
`<id>_diff_1k.jpg` et `<id>_nor_gl_1k.jpg`, tous dans un même dossier, puis
`uv run --with pillow --with numpy python build_textures.py <dossier>`.
