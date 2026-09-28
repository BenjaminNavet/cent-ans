# WIP Q6 — recette « comme un joueur » (2026-09-28)

Demande du joueur : tester le jeu et corriger les bugs. Pilote `game/tests/q3_playtest.gd` sur
main 24824cee (France, 1280×720, 12 tours). Branche `fix/q6-recette`, worktree `../gp-q6`.

## Constats / corrections
- [x] smoke : 56 × « _free_wrapper: Cannot convert argument 1 » (enveloppe libérée avant l'appel
      différé) → passage par identifiant d'instance (`ui_layout.gd`).
- [x] menu des factions en 1280×720 : boutons du bas coupés (onglets FE6 ajoutés au-dessus
      de FIT_SIZE) → réduction calculée sur la taille minimale réelle du contenu.

## Prochaine étape
Suite de la partie pilote, tri des constats.
