# bm/vegetation

Etat : porte et verifie par dump (`CENT_ANS_CAMERA="2,48,40"` : 18 k arbres, 323 en modeles ; a 420 : 139 k semes, 50 k dessines).
Fait : chargement (data.rs), masques (mask.rs), semis par tuile via la crate `vegetation` (tiles.rs), rendu instancie modeles/imposteurs (render.rs), zones humides (masque), dump `vegetation`.
Reste : shader d'imposteurs de Godot (azimut interpole, normales, teinte saisonniere, vent, lisieres : `campaign_tree_impostor.gdshader*`, `foliage*.gdshader*`), `TreeClearance` (lacs, routes), foret dense par zoom (`forest_detail*.gd`, eteinte en style generalise), ombres des arbres (le pipeline instancie ne projette pas), bruit FastNoiseLite exact, clairieres des hameaux et capitales, decor des zones humides (roseaux, etangs : `freshwater_layer.gd` est au lot water).

## vegetation2 (vague 4)
Fait : `clearance.rs` (lacs, mer, routes `main` ; aucun `main` dans roads.geojson actuel, comme Godot), clairieres hameaux/capitales ; pipeline billboard (`billboard.rs/.wesl` : azimut + Bayer, teinte saisonniere, vent, normales) ; `reeds.rs` (roseaux + etangs, palier patch) ; portee 900, cache 96 (mesure : 14 ms/tuile, 52 tuiles a distance 500).
Reste : palier proche des roseaux (`near_*`, pas de 0,07 u), fondu `fade_start/end` des etangs, lisieres par couverture forestiere (`foliage_keep`/`edge_*`), BACKLIGHT/ombres des nuages sur les arbres, ombres portees des arbres, rayon de houppier exact (`crown_widen`), mipmaps de l'atlas, retrait des arbres sur rivieres (`river_sd_at`), foret dense par zoom.

## vegetation3 (vague 5)
Fait : `edges.rs` (lisieres par couverture forestiere : hauteur `edge_height`, eclaircie `edge_thin`, par tuile cote CPU ; la largeur suit la hauteur car 0,85 = 0,85), rivieres (`river_bed.png`, `river_sd_at`) et rayon de houppier exact (`TreeInst::crown` x `crown_widen` 1,25 x echelle x 0,5) dans `clearance.rs`, houppiers elargis (`CROWN_WIDEN` sur impostures et modeles), `mips.rs` (chaine de mipmaps des deux atlas sur un thread, 7 niveaux, jamais sous 4 texels par cellule), roseaux : palier proche (`near_*`, pas 0,07) + fondus par tramage (`range.w`) des deux paliers (Level::Near/Patch, rayons Godot). Mesure : tuile 17,7 ms a distance 500 (14 avant), 54 tuiles, 816 arbres de lisiere retires, 5845 degages ; atlas 7 niveaux.
Note : la camera de campagne ne descend pas sous 7 u, le palier proche (fondu a 6,8) n'est donc exerce que par tests.
Reste : `fade_start/end` (surface d'eau, lot water), foret dense par zoom (`forest_detail*.gd`, eteinte en style generalise), BACKLIGHT / ombres de nuages, ombres portees des arbres, bruit FastNoiseLite exact.
