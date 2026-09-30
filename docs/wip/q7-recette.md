# WIP Q7 — recette « comme un joueur » (2026-09-30)

Demande du joueur : tester une partie et corriger les bugs. Pilote `game/tests/q3_playtest.gd`
(France, 1920×1080 → vue 1280×720, 12 tours). Branche `fix/q7-recette`, worktree `../gp-q7`.

## Constats / corrections
- [x] PLANTAGE à l'ouverture de l'écran des factions en vue 1280×720 (« Message queue out of
      memory », SIGBUS) : `_fit` oscillait entre deux facteurs (texte replié → hauteur minimale
      dépendante de la largeur) et `CONNECT_DEFERRED` rejouait la boucle dans le même vidage de
      file. Ajustement une fois par image + réduction seule après 3 agrandissements
      (`faction_select.gd`, test `q7_faction_fit_test.gd`).

## Prochaine étape
Relancer le pilote complet, relever les erreurs suivantes.
