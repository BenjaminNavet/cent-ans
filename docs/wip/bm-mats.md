# bm-mats : matériaux des bâtiments (parité visuelle Bevy)

Branche `bm/mats`. Constat joueur : bâtiments, tentes, convois et ville assiégée rendus en volumes blancs/gris.

## Fait
- `client/src/campaign/towns/atlas/` : matériau atlas PBR du kit (`building_atlas.gdshader`, `town_building.gdshader`,
  `building_aging.gdshaderinc`) en `ExtendedMaterial` + WESL : couche de matière dans l'alpha de la couleur de sommet,
  tableau 16 couches albédo + normales (`building_{albedo,normal}_array.jpg`), teintes/tuiles/rugosités de
  `data/art/building_materials.json`, usure SR5, neige des toits, suie TB3, normales par repère cotangent (le kit n'a pas
  de tangentes), UV boîte pour les blocs/murs/tours.
- Remplacement automatique : tout maillage glTF dont le matériau s'appelle `Building` (kit `models/buildings`,
  `town_kit`, `outbuildings`, maquettes) prend l'atlas, dans tous les états (villes 1:1, site de bataille, ville de siège)
  — aucune modification des dossiers des autres lots.
- Villes 1:1 : blocs (enduit + tuiles par l'alpha de `house_block_mesh`), maçonnerie, planches et monuments
  (`landmark_monuments::color` code la couche) en atlas.

- Matières nommées (`Plaster`, `Thatch`, `Timber`, `Canvas`…) des maquettes `settlements/` et des anciens modèles
  `castle/town/village/cathedral` : couche atlas du même nom (`ModelLibrary.is_building_model`).
- Bataille : neige sur les toits du kit en hiver (`site.rs`, `BuildingAtlas::kit_style`).
- Preuves : dump bataille 116 maillages remappés, campagne (Paris, recul 25) 1005 maillages, 12 variantes, 2 tableaux
  convertis ; captures battle_low / siege_paris : maisons, église, tentes texturées (plus de volumes blancs).

## Restes
- Murailles, tours et maisons en volumes de la ville de siège (`battle/siege/`, hors lot) : brancher
  `BuildingAtlas::material` (Masonry/Ashlar, `box_uv`) ; rues pavées de `landmark_siege_town.gd` non portées.
- Volumes du site de bataille (haies, palissades, ponts, accessoires sans modèle) : couleurs de sommet seules ; les ponts
  du kit (`bridge_*`) ne sont pas encore utilisés.
- Suie par ville sur les maisons du kit 1:1 (TB3 `town_soot` d'instance) : seule la variante globale existe.
- Matières régionales TX T4 (`tx_building_pack.json`), micro-grain de maçonnerie, roofscape lointain, neige saisonnière
  de campagne (`campaign_roof_snow`).
- Convois (`campaign/life`) : modèles DN déjà texturés (Material_0), rien à changer.

## Écarts
- Normales du kit aussi en campagne (Godot « far » sans normales) : repère cotangent par dérivées, fondu 40-120 m.
