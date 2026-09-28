# WIP Q6 — diplomatie (recette 2026-09-28)

## État
- [x] Test `game/tests/q6_diplomacy_test.gd` (1920×1080 @1,25 ; 1280×720 @1,0 ; 1280×640 @1,0).
- [x] « Proposer le traité » hors écran : la carte des relations gardait une taille minimale
      calculée avant que le reste du panneau grandisse (avis reçus, fiche) → panneau plus haut que
      la vue. `_fit_minimap` retranche l'excédent (différé sur `resized`) ; les boutons du traité
      sont sortis de la page défilante (pied de colonne, onglet Négociation seulement).
- [x] « Continuer » du rapport de saison sous la diplomatie : la fin de tour ouvre la diplomatie
      sur une offre nouvelle (Q5, voulu) dans l'image du rapport ; le rapport était placé à
      l'index de la fenêtre de chronique, qui vit désormais dans la zone latérale → sous le voile
      et la zone MODAL. Placé juste au-dessus de la zone MODAL (`flow_controller.gd`) ; pause /
      réglages / sauvegarde ouverts après lui le ferment.

## Prochaine étape
Tests d'UI voisins + smoke, puis rapport.
