# bm-bdecor2 : décor de bataille Bevy, vague 4

Branche `bm/bdecor2`. Suite de `bm-bdecor.md`.

## État
- `decor/look.rs` + `data/fx/battle_decor_look.json` (schéma `fx_battle_decor_look`) : brouillard, ambiance, ciel, horizon, anneau de bois, vignes, vergers, herbe, vent, sol.
- `decor/horizon.rs` : tuile de relief réel `HZR1`, anneau d'horizon, silhouette lointaine, dôme de ciel en dégradé avec lueur solaire.
- `decor/wind.rs` + `wind.wesl` : vent sur herbe et houppiers.
- `decor/ground.rs` : mipmaps CPU + tableau 2D répété (sans couture), normales du paquet.
- `trees.rs` : anneau de bois, vergers, LOD des troncs.
- `site.rs` : vignes.

## Reste
- Panorama peint (webp non activé), mer lointaine, clochers/fumées (battle_horizon.gd).
- Imposteurs d'arbres, squelette ramifié (battle_trees.gd), herbe couchée, atlas FA7.
- Province de la bataille : absente de `PendingBattle`, la tuile est choisie par terrain + graine.
- Caméra : `far` = 12 000 m ; l'anneau va à 11 400 m du centre du champ.
