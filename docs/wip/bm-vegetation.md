# bm/vegetation

Etat : porte et verifie par dump (`CENT_ANS_CAMERA="2,48,40"` : 18 k arbres, 323 en modeles ; a 420 : 139 k semes, 50 k dessines).
Fait : chargement (data.rs), masques (mask.rs), semis par tuile via la crate `vegetation` (tiles.rs), rendu instancie modeles/imposteurs (render.rs), zones humides (masque), dump `vegetation`.
Reste : shader d'imposteurs de Godot (azimut interpole, normales, teinte saisonniere, vent, lisieres : `campaign_tree_impostor.gdshader*`, `foliage*.gdshader*`), `TreeClearance` (lacs, routes), foret dense par zoom (`forest_detail*.gd`, eteinte en style generalise), ombres des arbres (le pipeline instancie ne projette pas), bruit FastNoiseLite exact, clairieres des hameaux et capitales, decor des zones humides (roseaux, etangs : `freshwater_layer.gd` est au lot water).

## vegetation2 (vague 4)
Fait : `clearance.rs` (lacs, mer, routes `main` ; aucun `main` dans roads.geojson actuel, comme Godot), clairieres hameaux/capitales ; pipeline billboard (`billboard.rs/.wesl` : azimut + Bayer, teinte saisonniere, vent, normales) ; `reeds.rs` (roseaux + etangs, palier patch) ; portee 900, cache 96 (mesure : 14 ms/tuile, 52 tuiles a distance 500).
Reste : palier proche des roseaux (`near_*`, pas de 0,07 u), fondu `fade_start/end` des etangs, lisieres par couverture forestiere (`foliage_keep`/`edge_*`), BACKLIGHT/ombres des nuages sur les arbres, ombres portees des arbres, rayon de houppier exact (`crown_widen`), mipmaps de l'atlas, retrait des arbres sur rivieres (`river_sd_at`), foret dense par zoom.
