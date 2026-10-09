# Textures des batailles (murailles, toits, écorce, feuillage, herbe)

Toutes les textures photographiques viennent de **Poly Haven** (https://polyhaven.com), licence
**CC0** (domaine public, aucune attribution requise), récupérées via
`https://api.polyhaven.com/files/<id>` puis assemblées par `build_textures.py`.

## Sol de bataille

Le sol ne vient plus de Poly Haven (ADR 0244) : les paquets TX régionaux de `tx/` (un par biome,
`data/art/tx_battle_bNN_pack.json`, ADR 0240) sont la seule voie, avec le grain fin
`micro_ground` (paquet partagé avec la campagne, `data/art/tx_micro_ground_pack.json`). L'identité et l'ordre des couches vivent dans `data/fx/battle_ground_layers.json`.

## Bâtiments et écorce (lot V4 ; 3 matières en 2k depuis GA5)

| Fichier | Identifiant(s) Poly Haven | Usage | Résolution |
|---|---|---|---|
| `castle_wall_varriation_*` | `castle_wall_varriation` | Courtines, tours, porte (`BuildingMaterials` « Masonry ») | albédo 2k, normale 1k (GA5) |
| `roof_slates_02_*` | `roof_slates_02` | Toits d'ardoise (tours, église) (« RoofSlate ») | albédo 2k, normale 1k (GA5) |
| `thatch_roof_angled_*` | `thatch_roof_angled` | Toits de chaume (« Thatch ») | albédo 2k, normale 1k (GA5) |
| `clay_roof_tiles_02_*` | `clay_roof_tiles_02` | Toits de tuiles (décor divers, hors `BuildingMaterials`) | albédo 1k, normale 512² |
| `plastered_wall_02_*` | `plastered_wall_02` | Murs enduits (décor divers, hors `BuildingMaterials`) | albédo 1k, normale 512² |
| `cobblestone_floor_01_*` | `cobblestone_floor_01` | Place pavée | albédo 1k, normale 512² |
| `wood_planks_*` | `wood_planks` | Vantaux, machines de siège | albédo 1k, normale 512² |
| `bark_brown_02_*` | `bark_brown_02` | Troncs d'arbres | albédo 1k, normale 512² |

Les trois premières lignes sont aussi des matières du kit de bâtiments
(`data/art/building_materials.json`, lu par `BuildingMaterials`) : lot GA5, bump 2k/1k (même
convention albédo 2k / normale 1k que GA2). Les autres `SINGLE` de `build_textures.py` (écorce,
décor divers) ne sont pas des matières de bâtiments et restent en 1k/512², hors périmètre GA5.

Textures **procédurales** (générées par `build_textures.py`, CC0 elles aussi) :
- `foliage_leaves.png` : carte alpha d'un amas de feuilles (houppiers, buissons) ;
- `grass_clump.png` : carte alpha d'une touffe d'herbe.

Textures **procédurales du lot DA6** (générées par `build_da6_textures.py`, CC0, aucune source
tierce ; `uv run --with pillow --with numpy python build_da6_textures.py`) :
- `grass_blades.png` : touffe en éventail (luminance et alpha seuls) ;
- `leaf_spray.png` : rameau feuillu des arbres (`BattleTrees`) ;
- `twig_spray.png` : ramilles nues (feuillus en hiver) ;
- `dead_leaves.png` : feuilles sèches (chêne marcescent en hiver).

Rameaux de **vraies feuilles** du lot FA1 (`build_fa_leaf_sprays.py`, catalogue
`data/art/battle_tree_leaves.json` ; atlas ambientCG LeafSet, CC0, téléchargés par le script hors
dépôt ; `uv run --with pillow --with numpy --with scipy python build_fa_leaf_sprays.py`) :
- `leaf_spray_<essence>.png` (1024², mipmaps, compression haute qualité) : rameau feuillu par
  essence de `BattleTrees` ; `leaf_spray.png` ne sert plus que de repli (`--no-fa` après `--`) ;
- `dead_leaves_oak.png` : feuilles de chêne sèches sur `twig_spray.png` (remplace
  `dead_leaves.png`, qui n'est plus chargé).

Herbe en **vrais brins** du lot FA7 (`build_fa_grass.py`, catalogue `data/art/battle_grass.json` ;
atlas ambientCG Foliage001 à Foliage008 et LeafSet020, CC0, téléchargés par le script hors dépôt ;
`uv run --with pillow --with numpy --with scipy python build_fa_grass.py`) :
- `grass_tufts.png` (2048 × 768, 4 × 3 cases de 512 × 256, mipmaps, compression haute qualité,
  ~2 Mo en mémoire) : douze touffes (herbe rase, touffes hautes, graminées à épis, herbes folles
  avec pissenlit et pâquerettes, blé) faites de brins détourés, penchés et courbés. Carte neutre en
  moyenne : la couleur vient du sol sous la touffe (`battle_grass.gdshader`, chemin `fa_on`).
  `grass_blades.png` ne sert plus que de repli (`--no-fa-grass` après `--`, ou `--no-da6`).

Régénérer : télécharger, pour les textures uniques ci-dessus, `<id>_diff_1k.jpg` et
`<id>_nor_gl_1k.jpg` (`_diff_2k.jpg` pour les trois matières GA5), tous dans un même dossier, puis
`uv run --with pillow --with numpy python build_textures.py <dossier>`.
