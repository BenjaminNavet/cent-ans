# bm-bground : sol et arbres de bataille (V14, V15)

Branche `bm/bground` (base 42068a22b). Fichiers : `client/src/battle/decor/ground.rs`, `ground.wesl`,
`trees.rs`, `tree_kit.rs` (nouveau : squelette des arbres et textures d'écorce / de feuilles).

## État
- En cours : squelette posé.

## Plan
1. Sol (`battle_ground.gdshader`, `battle_common.gdshaderinc`, `battle_ground_textures.gd`) :
   pile de couches par priorité (13 rôles), grain fin `micro_ground`, parcelles EP6 (`decor_fields`),
   parcelles procédurales hors du centre, carte de relief R2 (pente, creux, crêtes), bosses de détail,
   détail de près DA6, aplanissement au loin, neige / boue piétinée (carte d'herbe couchée).
2. Arbres (`battle_trees.gd`, `battle_tree_*.gdshader`) : squelette récursif, bouquets de 2 cartes,
   2 maillages (complet, allégé) + imposteurs existants (`far_woods.rs`) = 3 niveaux, écorces TX T3
   (`tx_bark_pack`), planches de feuilles (`tx_leaves_pack`), hiver : ramilles nues / chêne marcescent.

## Prochaine étape
- Implémenter le sol.
