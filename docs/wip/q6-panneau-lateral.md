# WIP Q6 — boutons du panneau latéral inaccessibles (2026-09-28)

Branche `fix/q6-recette`, worktree `../gp-q6`. Test : `game/tests/q6_side_panel_test.gd`
(1920×1080 taille 1,25 = vue 1280×720 ; 1280×720 et 1280×640 taille 1,0).

## Cause
- Largeur : la zone `SIDE_PANEL` mesure 384 px en vue 1280×720, mais le panneau de province
  réclamait jusqu'à 710 px (lignes de construction et de classes non coupées, 380 px de minimum
  fixe, fil d'Ariane dans l'en-tête, textes enrichis à 300 px minimum, cadre enluminé de
  44 px par côté). L'enveloppe défilante n'a pas de défilement horizontal : la droite du
  panneau et sa barre de défilement verticale passaient hors de la zone (coupées).
- Hauteur : onglets fixes de 300-360 px sous un en-tête de 260-390 px, dans une zone de 440 px :
  « Recruter » et « Changer d'édit » tombaient sous le bord de la zone (coupés), là où se
  trouvent la minicarte et la cloche de fin de tour ; le clic y arrivait.

## Correctifs (faits)
- `panel_widgets.gd` : boutons de ligne à points de suspension (`narrow_button`), notes à la
  ligne (`side_note`), puces à la ligne (`wrap_chip`), hauteur des onglets ajustée à la zone
  (`fit_tabs_to_side_zone`, plancher 200 px).
- `province_panel.gd` / `.tscn` : lignes de classes en `HFlowContainer`, actions en
  `HFlowContainer`, plus de minimum 380 px, fil d'Ariane sur sa propre ligne, titre ajusté
  (`Lettrine.attach(…, fit = true)`), onglets ajustés à la zone.
- `settlement_panel.gd` : idem (actions en flux, en-têtes à la ligne, onglets ajustés).
- `edict_section.gd`, `table_section.gd` : minimum des textes enrichis 300 → 160 px.
- `lettrine.gd` : option `fit` (titre rapetissé pour tenir dans la largeur donnée).

## Points ouverts
- Le panneau de colonie est ajouté comme frère du panneau de province, donc dans la même
  enveloppe défilante (`ProvincePanelScroll`, deux enfants) ; pas de bug constaté, à revoir.
- L'en-tête du panneau de province (grille de 9 lignes) reste haut : « Changer d'édit »
  demande encore un défilement de la zone en vue 1280×720.

## Prochaine étape
Tests liés + smoke, puis rapport.
