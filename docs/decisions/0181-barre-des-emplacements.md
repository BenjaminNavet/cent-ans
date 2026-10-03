# 0181 — Barre des emplacements de colonie

## Contexte
Audit joueur A6, constat U14 : le panneau de colonie est une liste ; le joueur veut lire d'un coup d'œil
ce qui est bâti, ce qui manque et ce qui est bloqué, comme dans Total War. Le cœur n'a pas de notion
d'emplacement ni de plafond par taille de colonie : une colonie porte un ensemble de bâtiments, filtré par
`settlement_kinds`, avec des chaînes d'amélioration (`upgrades_from`).

## Décision
- Un emplacement = une chaîne d'amélioration des bâtiments permis pour le type de colonie ; il contient au
  plus un bâtiment de sa chaîne (l'amélioration remplace le précédent, règle existante). Dérivation en
  lecture seule dans `core/crates/sim-campaign/src/building_slots.rs` ; aucune règle ni donnée nouvelle.
- Case verrouillée = emplacement vide dont le premier palier est impossible pour une raison structurelle
  (ressource, côte, rivière, technologie, prérequis). Manquer d'argent ou avoir un chantier en cours ne
  verrouille pas.
- Le pont expose `CampaignSim.settlement_slots(id)` (lecture seule) ; le composant
  `game/scripts/map/settlement_slot_bar.gd` ne fait que dessiner (icône, niveau, « + », infobulle IB).
- Un clic ouvre l'onglet « Bâtiments » du panneau de colonie et donne le focus à la ligne du premier palier ;
  l'ordre de construction reste donné dans le panneau.
- La bande se place dans la zone `BOTTOM_SELECTION`, entre le sceau et la cloche de fin de tour, au-dessus du
  bandeau d'ost s'il est visible ; elle défile si les cases débordent.

## Conséquences
- Pas de verrou « par taille de colonie » : il n'existe pas dans le cœur. Le type de colonie (cité, ville,
  château, abbaye, village) filtre déjà les chaînes ; un plafond d'emplacements par taille serait une règle
  de jeu à décider (nombre de cases, déblocage par croissance) et à placer dans `data/settlements/rules.json`.
- Une cité compte une vingtaine de cases ; au besoin la barre défile.
