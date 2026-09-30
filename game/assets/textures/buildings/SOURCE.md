# Textures des bâtiments (lot BR1 ; 2k et torchis/colombage depuis GA5)

Identité, tuile, teinte et rugosité de chaque matière vivent dans
`data/art/building_materials.json` (schéma `data/schemas/art_building_materials.schema.json`),
**pas dans ce fichier** ni dans `build_textures.py` ni dans `building_materials.gd`, qui le
lisent tous deux. Ce fichier documente uniquement la **provenance** des textures.

Toutes les textures photographiques viennent de **Poly Haven** (https://polyhaven.com), licence
**CC0** (domaine public, aucune attribution requise), récupérées via
`https://api.polyhaven.com/files/<id>` puis dérivées par `build_textures.py`.

## Matières du kit (lot GA5, albédo 2k / normale 1k / rugosité 1k)

| Matière (`BuildingMaterials`) | Identifiant Poly Haven | Fichiers |
|---|---|---|
| `Plaster` (enduit) | `medieval_wall_01` (dérivé, voir ci-dessous) | `lime_plaster_diff.jpg` + `medieval_wall_01_nor/rough.jpg` |
| `Rubble` (moellons) | `stone_wall` | `stone_wall_diff/nor/rough.jpg` |
| `Ashlar` (pierre de taille) | `rustic_stone_wall` | `rustic_stone_wall_diff/nor/rough.jpg` |
| `Timber` (charpente) | `rough_wood` | `rough_wood_diff/nor/rough.jpg` |
| `Planks`/`Door` (planches, porte) | `weathered_brown_planks` | `weathered_brown_planks_diff/nor/rough.jpg` |
| `RoofTile` (tuiles) | `clay_roof_tiles_03` | `clay_roof_tiles_03_diff/nor/rough.jpg` |
| `RoofFlat` (tuiles plates) | `roof_tiles_14` | `roof_tiles_14_diff/nor/rough.jpg` |

`Masonry`, `RoofSlate`, `Thatch` partagent leurs textures avec `../battle/` (`castle_wall_varriation`,
`roof_slates_02`, `thatch_roof_angled` — voir `../battle/README.md`), aussi bumpées en 2k/1k par
GA5. `Window`, `Iron`, `Canvas` sont des matières unies (pas de texture).

`lime_plaster_diff.jpg` : dérivé de `medieval_wall_01_diff.jpg` (l'albédo brut est teinté orangé ;
un badigeon à la chaux est plus blanc, la couleur est donc tirée à 70 % vers sa propre luminance
et légèrement éclaircie — `build_textures.py::main()`) ; le relief et la carte normale restent
ceux de `medieval_wall_01`.

## Torchis et colombages (`TimberFrame`, lot GA5)

`timber_frame_{diff,nor,rough}.jpg` : mur à pans de bois (colombage : l'ossature de charpente ;
torchis : le remplissage d'enduit/terre entre les poteaux) — **composite procédural**, pas une
texture Poly Haven dédiée (aucune n'existe au catalogue à la date du lot pour ce motif précis) :
lattis de poteaux/sablières/entretoises dessiné algorithmiquement (`build_textures.py::_beam_mask`,
seed déterministe) et mélangé entre `lime_plaster` (remplissage, déjà CC0 Poly Haven) et
`rough_wood` assombri/désaturé (poutres, déjà CC0 Poly Haven) — aucun nouvel asset externe, 0 $.

`TimberFrame` est **câblée** depuis le lot TF (`"wired": true`) : panneaux des murs à pans de
bois du kit Blender (`building_kit.FRAME_PANEL`, sous les poutres réelles), dernière couche de
l'atlas `Building` (indice 14, `building_{albedo,normal}_array.jpg` à 15 tranches) ; le shader
traite les couches unies comme un bloc `first_plain`..`plain_last` (11..13), la couche 14 reste
texturée. Tuile 3 m, teinte éclaircie (1,12) : le lattis peint double les poutres réelles, plus
espacé et plus clair il alourdit moins la façade. Voir ADR 0105 §TF.

## Régénérer

Télécharger, pour chaque matière ci-dessus (buildings et battle partagées), les fichiers
`<id>_diff_2k.jpg`, `<id>_nor_gl_1k.jpg` et `<id>_rough_1k.jpg` (buildings) via
`https://api.polyhaven.com/files/<id>`, les placer dans ce dossier (ou `../battle/` pour Masonry/
RoofSlate/Thatch) sous les noms `<id>_diff.jpg`/`<id>_nor.jpg`/`<id>_rough.jpg`, puis
`uv run --with pillow --with numpy python build_textures.py` (régénère `lime_plaster_diff.jpg`,
`timber_frame_*.jpg` et les tranches `building_albedo_array.jpg`/`building_normal_array.jpg`).
