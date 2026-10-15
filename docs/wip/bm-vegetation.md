# bm/vegetation

Etat : porte et verifie par dump (`CENT_ANS_CAMERA="2,48,40"` : 18 k arbres, 323 en modeles ; a 420 : 139 k semes, 50 k dessines).
Fait : chargement (data.rs), masques (mask.rs), semis par tuile via la crate `vegetation` (tiles.rs), rendu instancie modeles/imposteurs (render.rs), zones humides (masque), dump `vegetation`.
Reste : shader d'imposteurs de Godot (azimut interpole, normales, teinte saisonniere, vent, lisieres : `campaign_tree_impostor.gdshader*`, `foliage*.gdshader*`), `TreeClearance` (lacs, routes), foret dense par zoom (`forest_detail*.gd`, eteinte en style generalise), ombres des arbres (le pipeline instancie ne projette pas), bruit FastNoiseLite exact, clairieres des hameaux et capitales, decor des zones humides (roseaux, etangs : `freshwater_layer.gd` est au lot water).
