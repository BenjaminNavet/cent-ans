# bm-council (lot Bevy "council")

Branche `bm/council`, dossier `client/src/campaign/council/`. Port de `ui/council_room.gd` + scène de `ui/council_chamber.gd` + entrée de `map/council_controller.gd`.

État : salle 3D rendue dans une image hors écran (couche de rendu 7), insérée en haut de l'onglet « Charges » du panneau du conseil (`panels/court`). Survol = figure éclairée + carte, clic = sélection de la charge (le souverain ouvre sa fiche).
Reste : voir rapport final.

## Fait
`pick.rs` (sièges, rayon, projection, tests), `room.rs` (décor), `figures.rs` (figures peintes + styles mesh, tabouret, trône), `stage.rs` (widget, survol, clic, étiquettes « Charge vacante », carte), `mod.rs` (plugin, caméra, vacillement, sonde pixels, dump `council`). Édits hors dossier : `campaign/mod.rs` (+1 ligne), `panels/mod.rs` (+1 ligne), `panels/court/view.rs` (scène insérée dans l'onglet Charges, colonne gauche 460 px).
Preuve : `council.pixel_probe` moyenne 59/255, aucun pixel noir ni brûlé, sièges 64-94.

## council2 (16/10)
Fait : survol/clic prouvés en direct sans curseur (`CENT_ANS_COUNCIL_AUTOHOVER=<charge|ruler>` + `CENT_ANS_COUNCIL_AUTOCLICK=1`, script `council/proof_script.txt`, dump `council.{hovered,clicks,last_click,card}`) ; carte de survol avec portrait (`PortraitBank`, `card.portrait`) ; surbrillance additive des glb (`figures::apply_overlay`, enfants `HoverOverlay`, dump `highlight_overlays`) ; halo des flammes (sphère additive, `flame_halos`) ; figures de repli : style demandé, puis image/modèle du joueur s'il existe, capsule seulement sinon (`figure_kind`, 3 tests) ; en-tête de `panels/court/mod.rs` corrigé.
Preuve : style painted, survol chancelier -> clic, carte « Pierre Roger / Chancelier », portrait `portraits/chr_pierre_roger.png`, 9 halos ; style mesh : 1 surbrillance sur le connétable.
Restes : saturation 0,78 et tonemapping filmique exact ; capsule non prouvée en direct (tests unitaires seulement) ; teinte de l'état (mort/malade) absente du portrait de la carte ; portrait étiré 64x80 (pas de cover).
