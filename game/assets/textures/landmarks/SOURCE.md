# Atlas des matériaux des villes emblématiques (lot L3)

Toutes les textures photographiques viennent de **Poly Haven** (https://polyhaven.com), licence
**CC0** (domaine public, aucune attribution requise). Fichiers 1k JPG (`<id>_diff_1k.jpg`,
`<id>_nor_gl_1k.jpg`) récupérés via `https://api.polyhaven.com/files/<id>`, réduits à 512² et
assemblés par `build_textures.py` en deux Texture2DArray de 16 couches (512 px, empilées
verticalement) :

| Couche | Nom | Source | Usage (palette du générateur) |
|---|---|---|---|
| 0 | calcaire appareillé | `medieval_blocks_03` | NDStone, Stone, Ochre |
| 1 | moellons | `castle_wall_varriation` | WallStone, DarkStone |
| 2 | brique | `medieval_red_brick` | Brick |
| 3 | ardoise | `roof_slates_02` | Slate |
| 4 | tuile | `clay_roof_tiles_02` | Tile |
| 5 | tuile ancienne | `roof_tiles_14` | TileOld |
| 6 | enduit | `clay_plaster` | Plaster, Whitewash, Canvas |
| 7 | pans de bois | procédural (enduit `clay_plaster` + bois `old_planks_02`) | Timber |
| 8 | planches | `old_planks_02` | Wood |
| 9 | chaume | `reed_roof_04` | Thatch |
| 10 | pavés | `cobblestone_floor_08` | Paving, Street |
| 11 | plomb | procédural (feuilles à joints debout, oxydation) | Lead |
| 12 | vitrail | procédural (losanges bleus, rouges, verts, plombs) | Glass |
| 13 | sol | procédural (bruit) | Grass, Garden, Vine, Dirt |
| 14 | uni | — | Water, Gold, Dark |
| 15 | vieillissement | procédural : R plaques, V coulures, B mousse | masques du shader |

Les couches photographiques sont des **cartes de détail** : chaque canal divisé par sa moyenne
(moyenne 0,5 en linéaire, peu de teinte propre), si bien que le shader multiplie la couleur de la
palette (calcaire blond de Paris, pierre de Caen, pierre d'Avignon, brique de Bruges) par le détail.
Le vitrail garde ses propres couleurs.

Régénérer : `uv run --no-project --with pillow --with numpy python build_textures.py <dossier>`.
