# bm-bherald — armoiries des soldats en bataille (Bevy)

Branche bm/bherald, dossier `client/src/battle/units/`.

## État
- `heraldry.rs` : atlas de la bataille (Texture2DArray 128² RGBA8 sRGB, mipmaps) comme
  `_build_arms_atlas` de battle_soldiers.gd : armes des deux factions, maison du général
  (`banners::general_house`, réutilisé), maisons vassales (`vassal_of` de houses.json, triées par id).
  Choix par régiment (`_apply_arms`) : nobles = armes du seigneur + 3 bannerets tirés par régiment,
  commun = armes de faction sur l'écu et croix de livrée (`badges`) sur la poitrine.
- Boîte du buste (`BattleSkinned.chest_box`) calculée sur le LOD0 (`FigureSpec::chest`).
- `skinned.wesl` : poitrine/dos du surcot (C_LIVERY), faces peintes de l'écu (C_ARMS), croix et
  pièce d'étoffe, part des bannerets 0,3 ; bloc de paramètres passé à 126 vec4 ; liaisons 4-5.
- Dump `battle_heraldry` (atlas, couches, camps, régiments).

## Écarts à Godot
- Tirage des bannerets : même règle, autre générateur (le `hash()` de Godot n'est pas reproductible).
- Cadavres : gardent les armes de leur propre régiment (Godot : les 3 premiers bannerets du camp).

## Preuve
- Dump `battle_heraldry` (bataille par défaut France/Angleterre) : atlas 28 couches 128², 8 mips ;
  35 régiments avec couches, 15 à armoiries de poitrine (nobles), 18 à croix de livrée.
- Capture rapprochée (`CENT_ANS_UNITS_VIEW=2,9`) : semé de lis et bannerets sur les surcots.

## Restes
- Bataille par défaut sans maison de général (`lord: null`) : le seigneur porte les armes de faction,
  comme Godot ; non vérifié avec un général doté d'une maison (bataille de campagne).
- Impostors (> 300 m) sans armoiries (Godot : atlas cuit des impostors).
