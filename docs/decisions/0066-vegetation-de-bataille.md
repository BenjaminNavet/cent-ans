# 0066 — Végétation de bataille : lisières douces, feuillus ramifiés, imposteurs (DA6)

Date : 2026-09-26. Statut : accepté. Lot DA6 (bible DA § 3.3, § 6 « Végétation », § 10 ligne 6).

## Contexte
En bataille, caméra rapprochée (`docs/img/da/etat_2509_battle_close.jpg`, `docs/img/da6/avant_*`) :
herbe en cartes plates (pied rectiligne, plans lisibles), parcelles à lisière droite (une jachère
crème vif s'arrêtait net), arbres « sucette » (houppier ellipsoïdal de cartes sur un fût nu) de
près comme à l'horizon, sol flou sous la caméra, saturation moyenne du décor 38 % (plein jour,
gros plan) à 53 % (vue haute, automne) pour une cible de 35 %.

## Décision
Rendu seulement, tout derrière `BattleTerrain.da6` (`--no-da6` rend l'ancienne végétation).
1. **Lisières** (`battle_common.gdshaderinc`) : `bt_edge_warp` déforme le point de lecture des
   parcelles (±5 m sur ~28 m, frange ±1,2 m sur ~5 m) ; rampe de 9 m (procédurales) ou 8 m
   (décor EP6, `_stamp_decor`) à bord bruité ; bord des massifs sans parcelles fondu. Même formule
   au sol et dans l'herbe ; sillons non déformés. Dans l'herbe, chaque touffe bascule d'une
   parcelle à l'autre à son propre seuil : les deux côtés se mêlent sur quelques mètres.
2. **Touffes** (`battle_vegetation.gd`, `battle_grass.gdshader`) : 4 cartes cintrées et évasées
   (pied 0,24 m, sommet 0,64 m), normales arrondies portées par le maillage, pied assombri,
   cartes vues par la tranche amincies (seuil d'alpha, normale de carte dans `COLOR`), hauteur
   variée par touffe et par plaques ; carte `grass_blades.png` (éventail, pied resserré, épis),
   luminance seule (la couleur reste celle du sol). Jachère vert passé au lieu de crème.
3. **Arbres** (`battle_trees.gd`, `BattleTrees`) : squelette procédural par essence (tronc,
   charpentières, branches, rameaux ; enveloppe du houppier), bouquets de deux cartes
   (`leaf_spray.png`) au bout des rameaux, ombre propre cuite en couleur de sommet. Essences :
   chêne, hêtre, frêne (bocage), peuplier noir (et non d'Italie, introduit au XVIIIe siècle),
   saule têtard au bord de l'eau, fruitier (vergers, courtils), buisson (haies). Hiver :
   ramilles nues (`twig_spray.png`), chêne marcescent (`dead_leaves.png`). Sources procédurales
   (CC0, `build_da6_textures.py`), aucun asset tiers ajouté.
4. **Niveaux de détail par instance** (`battle_tree_lod.gdshaderinc`) : maillage complet
   (ombre) jusqu'à 120 m × qualité, allégé (même squelette, un bouquet sur deux agrandi, rameaux
   sans écorce, sans ombre) jusqu'à 300 m, imposteur au-delà. La distance est prise à la caméra
   principale (`MAIN_CAM_INV_VIEW_MATRIX`, vraie aussi dans la passe d'ombre) depuis l'origine de
   chaque arbre, avec fondu tramé complémentaire de 16 m ; les tuiles de MultiMesh (160 m) ne
   font que le tri grossier. Aucun arbre « sucette » en deçà de 300 m ; l'imposteur est l'image
   du vrai arbre.
5. **Imposteurs** (`battle_tree_impostor.gdshader`) : atlas cuit au lancement (une ligne par
   essence, 4 angles, cellules de 128 px) dans un `SubViewport` en lumière ambiante seule, comme
   les figurines (ADR 0024) ; un quadrilatère par arbre tourné vers la caméra (incliné vers elle
   quand elle est haute), normale arrondie reconstituée, teinte d'instance. L'anneau lointain
   (ancien « far ») n'a plus que des imposteurs, ramenés à la taille réelle.
6. **Sol de près** (`battle_ground.gdshader`) : la couche dominante relue à ~1/4 d'échelle
   module la luminance et la normale, fondue de 45 à 117 m.
7. **Décor sourd** : saturation ramenée vers la luminance (`decor_saturation` 0,6 ; 0,5 à
   l'automne) dans le sol, l'herbe, le feuillage et les imposteurs ; les armoiries n'y passent pas.

## Mesures
Banc dans un seul processus (`--benchmark --bench-ab=da6,no-da6` : les deux végétations sont
construites, bascule toutes les 30 images ; moyenne `ab_mean`, la médiane collant aux paliers de
la cadence sous Metal). Écart DA6 / ancien rendu : +1,5 à +3,5 % (standard, gros plan, bocage,
forêt, hiver), −1 % au palier épique (imposteurs moins chers que l'ancien LOD lointain) ; budget
de 5 % tenu. Saturation moyenne (zone 3D sous l'horizon) : gros plan 38 → 29 %, vue haute
53 → 33 %. Détail : `docs/wip/da6-vegetation-bataille.md`, captures `docs/img/da6/`.
`decor_saturation` final : 0,6 (automne 0,5).

## Conséquences
- Plus de géométrie par arbre proche (chêne complet ≈ 800 triangles contre ≈ 150), compensée par
  des imposteurs à 2 triangles au-delà de 300 m (l'ancien LOD restait dessiné sans limite).
- Les haies (buisson tous les 1,8 m) sont le pire cas : buisson allégé en feuillage seul dès 60 m.
- Autre copie de formules : lisières identiques dans le sol et l'herbe (`bt_field_info_soft`).
- `BattleMeshes.tree` reste pour `--no-da6` et le décor 3D du menu.
- Couleur d'automne encore > 35 % : elle vient surtout de la lumière et du brouillard dorés
  (`BattleAtmosphere`), hors de ce lot.
