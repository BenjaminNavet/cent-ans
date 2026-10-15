# bm-bdecor : décor de bataille Bevy

Branche `bm/bdecor`. Dossiers : `client/src/battle/decor/`, `client/src/battle/scene/terrain.rs`.

## État
- `decor/ground.rs` + `ground.wesl` : sol texturé en splat (paquet régional `data/art/tx_battle_b<biome>_pack.json`,
  13 couches, poids par sommet COLOR + UV_1), `ExtendedMaterial<StandardMaterial, GroundExtension>`.
- `decor/atmosphere.rs` : soleil selon l'heure du jour (`data/fx/atmosphere.json`), météo, ambiance, brouillard de distance.
- `decor/trees.rs` : arbres et buissons placés comme `battle_terrain_scatter.gd` (disques de bois), houppiers en cartes de feuilles.
- `decor/grass.rs` : touffes réparties une fois, fusionnées par 48 m, fondu par `VisibilityRange`.
- `decor/site.rs` : maisons, moulins, église, accessoires, camps, haies/clôtures/palissades, ponts (volumes colorés).
- Section de dump `battle_decor`.

## Reste (fichiers Godot)
- `battle_horizon.gd`, `landmark_backdrop.gd`, `landmark_siege_town.gd` : horizon réel, panorama peint, décor lointain.
- `battle_terrain_scatter.gd` : anneaux de bois proches/lointains, LOD et imposteurs ; `battle_trees.gd` : squelette ramifié.
- `battle_vegetation.gd` : vent, herbe couchée (flatten), atlas FA7.
- `battle_decor.gd` : kit de bâtiments, vignes, vergers, douves, cimetières, fumées/feux.
- `battle_atmosphere.gd` : ciel procédural/HDRI, LUT, SSAO, brouillard volumétrique, pluie/neige.
- Normales du paquet de sol, grain micro, parcelles EP6 (blé/chaume).
