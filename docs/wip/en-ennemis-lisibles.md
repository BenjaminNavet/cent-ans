# EN — ennemis lisibles sur la carte (2026-10-02)

Demande du joueur : « on ne reconnaît pas assez bien les unités, territoires, villes ennemies par
rapport aux neutres ; je joue la France et un voisin a un liseré rouge sur la frontière mais ce
n'est pas un ennemi ».

Cause : hors mode Diplomatie, frontières (ADR 0074), plaques d'armée et villes ne portent que la
couleur héraldique ; un voisin aux armes rouges se lit « ennemi ». La relation n'apparaissait
qu'en mode Diplomatie (DZ).

Décision (ADR 0153) : le rouge est réservé aux ennemis du joueur, partout sur la carte.

Branche `feat/en`, worktree `../gp-en` (la session TB retouche `faction_borders.gd`,
`settlement_layer.gd`, `map_mode_controller.gd` : modifications gardées petites et localisées).

## Lots
- [x] EN0 squelette : `data/map/stance_cues.json` + schéma + test pytest, `StanceCues`
      (`game/scripts/map/stance_cues.gd`).
- [ ] EN1 frontières : `FactionBorders` lit les positions du joueur ; nous or, ennemis rouge,
      amis vert, autres héraldique assourdie. Le mode Diplomatie (DZ) garde sa palette.
- [ ] EN2 armées : bordure de plaque par catégorie, ⚔ sur les plaques ennemies, anneau au sol
      rouge.
- [ ] EN3 villes : nom des villes ennemies à l'encre rouge.
- [ ] EN4 test `game/tests/en_stance_cues_test.gd`, capture de contrôle, ADR 0153, fusion.

## Points ouverts
- Après fusion de TB2 (style « au repos », saturation × 0,45) : les frontières ennemies doivent
  rester à pleine intensité (sinon le rouge s'éteint). À régler à la fusion de la seconde branche.
