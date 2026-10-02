# EN — ennemis lisibles sur la carte (2026-10-02)

Demande du joueur : « on ne reconnaît pas assez bien les unités, territoires, villes ennemies par
rapport aux neutres ; je joue la France et un voisin a un liseré rouge sur la frontière mais ce
n'est pas un ennemi ».

Cause : hors mode Diplomatie, frontières (ADR 0074), plaques d'armée et villes ne portent que la
couleur héraldique ; un voisin aux armes rouges se lit « ennemi ». La relation n'apparaissait
qu'en mode Diplomatie (DZ).

Décision (ADR 0155) : le rouge est réservé aux ennemis du joueur, partout sur la carte.

Branche `feat/en`, worktree `../gp-en` (la session TB retouche `faction_borders.gd`,
`settlement_layer.gd`, `map_mode_controller.gd` : modifications gardées petites et localisées).

## TERMINÉ (2026-10-02)
- EN0 données + schéma + `StanceCues` ; EN1 frontières (nous or, ennemis rouge, amis vert, autres
  héraldique assourdie ; ennemis à pleine intensité malgré le repos TB2) ; EN2 plaques (bordure,
  ⚔) et anneaux d'armée ; EN3 noms des villes ennemies à l'encre rouge ; EN4 test, captures, ADR 0155.
- Tests verts : `en_stance_cues_test`, `dz_diplo_borders_test`, `fr1_borders_test`,
  `tb2_declutter_test`, smoke, pytest des schémas.
- Captures jugées (Guyenne, 2 vues) : frontière anglaise rouge vif, voisins en paix ternes,
  anneaux verts des vassaux.

## Reste
- Jugement du joueur en partie. Réglages dans `data/map/stance_cues.json` (encre des noms
  ennemis `town.label.enemy` assez discrète : l'éclaircir si elle ne suffit pas).
- Minicarte et teinte des provinces restent héraldiques (hors lot).
- Aucune armée ennemie visible au tour 1 (brouillard) : plaque ennemie vérifiée par test seulement.
