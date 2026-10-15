# bm-armies — armées de la carte de campagne (Bevy)

État (branche bm/armies) : figurines glTF (chef `army.glb` + escorte `army_foot/archer/army` selon la composition, camp de siège, flotte de cogues pendant une traversée) teintées à la livrée de la faction ; hauteur par `terrain::surface::GroundHeight` (trait `Ground`) ; embarquement depuis un port par clic droit sur la mer (`sea.rs`, ADR 0167) avec glissement le long des routes `sea_lanes_px.json` ; batailles navales auto-résolues (`sea::auto_resolve_pending`) ; bulle de mouvement (maillage de cases, `bubble.rs`) ; ondulation d'ordre en rayon écran ; curseur contextuel (attaque, embarquement, interdit) et bulle de survol (`feedback.rs`).
Test : `CENT_ANS_ARMY_AUTOTEST=1` (marche) ou `=sea` (clic sur une route maritime depuis un port ; `--faction fac_england`).

Reste :
- Figurines squelettées animées et instanciées (`army_figures.gd`, `battle_skinned`), marche/repos, balancement de l'étendard, fondu aux paliers de zoom.
- Anti-chevauchement des plaques (`army_plate_layout.gd`), écart à la ville (SA, ADR 0160), surbrillance survol/sélection des figurines.
- Bulle en masque shader (`reachable_bubble.gdshader`, contour lisse) ; ici un maillage de cases (plafond 9000).
- Clic droit sur une colonie atteinte par la mer (`prefers_sea`) : revient au lot de sélection des lieux (la commande `ArmyCommand::Embark` existe).
- Eau = hauteur <= 0 (repli de `MapData.is_land_px`), pas de masque terre/mer.
- Le curseur et les ordres ne testent pas si le pointeur est sur un panneau d'interface.
