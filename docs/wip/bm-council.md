# bm-council (lot Bevy "council")

Branche `bm/council`, dossier `client/src/campaign/council/`. Port de `ui/council_room.gd` + scène de `ui/council_chamber.gd` + entrée de `map/council_controller.gd`.

État : salle 3D rendue dans une image hors écran (couche de rendu 7), insérée en haut de l'onglet « Charges » du panneau du conseil (`panels/court`). Survol = figure éclairée + carte, clic = sélection de la charge (le souverain ouvre sa fiche).
Reste : voir rapport final.

## Fait
`pick.rs` (sièges, rayon, projection, tests), `room.rs` (décor), `figures.rs` (figures peintes + styles mesh, tabouret, trône), `stage.rs` (widget, survol, clic, étiquettes « Charge vacante », carte), `mod.rs` (plugin, caméra, vacillement, sonde pixels, dump `council`). Édits hors dossier : `campaign/mod.rs` (+1 ligne), `panels/mod.rs` (+1 ligne), `panels/court/view.rs` (scène insérée dans l'onglet Charges, colonne gauche 460 px).
Preuve : `council.pixel_probe` moyenne 59/255, aucun pixel noir ni brûlé, sièges 64-94.

## Restes
- Survol/clic non testés en direct (le script de l'agent ne place pas le curseur) ; géométrie testée unitairement.
- Figures de secours (capsule + sphère) si un PNG manque : non portées. Surbrillance des figures glb (style mesh) : non portée (`material_overlay`).
- Pas de saturation 0,78 ni de tonemapping filmique exact (TonyMcMapface) ; carte de survol sans portrait ; flammes sans halo.
- Doc de tête de `panels/court/mod.rs` dit encore « salle 3D non portée » (à corriger).
