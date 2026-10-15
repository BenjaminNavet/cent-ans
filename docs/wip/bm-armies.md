# bm-armies — armées de la carte de campagne (Bevy)

État (branche bm/armies) : figurines glTF (chef `army.glb` + escorte `army_foot/archer/army` selon la composition, camp de siège, flotte de cogues pendant une traversée) teintées à la livrée de la faction ; hauteur par `terrain::surface::GroundHeight` (trait `Ground`) ; embarquement depuis un port par clic droit sur la mer (`sea.rs`, ADR 0167) avec glissement le long des routes `sea_lanes_px.json` ; batailles navales auto-résolues (`sea::auto_resolve_pending`) ; bulle de mouvement (maillage de cases, `bubble.rs`) ; ondulation d'ordre en rayon écran ; curseur contextuel (attaque, embarquement, interdit) et bulle de survol (`feedback.rs`).
Test : `CENT_ANS_ARMY_AUTOTEST=1` (marche) ou `=sea` (clic sur une route maritime depuis un port ; `--faction fac_england`).

armies3 (vague 4) : anti-chevauchement des plaques (`plate_layout.rs`, `LabelPlacer` de provinces, obstacles `RegionLabelObstacles`, priorité sélection > joueur > effectif), écart à la ville (ADR 0160 : rayon `Places` + marge + demi-emprise, côté sud-est, lissé à 0 pendant une traversée), masque terre/mer réel `land_mask.png` (`landmask.rs`, repli hauteur <= 0), clic droit / aperçu sur une colonie atteinte par la mer (`sea::prefers_sea`, `settlement_at`), brouillard (`FogOfWar::is_army_visible`, armées du joueur toujours visibles), `orders_open` sur les entrées. Dump : `plate_overlaps`, `plates_*`, `garrison_standoff_markers`, `land_mask_*`, `fog_active`, `armies_hidden_by_fog`.

Reste :
- Figurines squelettées animées : les glb `army*.glb` n'ont aucun clip ; il faut le pipeline os-cuits-en-texture de `army_figures.gd` (marche/repos, étendard, fondu de zoom).
- Surbrillance survol/sélection des figurines ; masque de bulle en shader (`reachable_bubble.gdshader`).
- Liste « Mes unités » (`unit_roster_controller.gd`, touche U, armées + agents) : non portée (aucun équivalent Bevy trouvé).
- Plaques : pas de recalcul périodique (0,25 s) ni de plaque « dessous » (`plate_below`) ; le curseur et les ordres ne testent pas le pointeur sur un panneau d'interface.
