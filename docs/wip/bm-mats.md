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

## Prochaine étape
- Vérification visuelle (battle_low, siege_paris, camp_0025), réglages.
- Tentes / accessoires du site (`battle/decor/site.rs` volumes blancs), convois (`campaign/life`).

## Écarts
- Matières régionales (TX T4), micro-grain de maçonnerie, roofscape : non portés.
