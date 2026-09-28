# WIP Q6 — recette « comme un joueur » (2026-09-28)

Demande du joueur : tester le jeu et corriger les bugs. Pilote `game/tests/q3_playtest.gd` sur
main 24824cee (France, 1280×720, 12 tours). Branche `fix/q6-recette`, worktree `../gp-q6`.

## Constats / corrections
- [x] smoke : 56 × « _free_wrapper: Cannot convert argument 1 » (enveloppe libérée avant l'appel
      différé) → passage par identifiant d'instance (`ui_layout.gd`).
- [x] menu des factions en 1280×720 : boutons du bas coupés (onglets FE6 ajoutés au-dessus
      de FIT_SIZE) → réduction calculée sur la taille minimale réelle du contenu.

- [x] fenêtre de décision (chronique, sort des villes) vide à toute résolution : le corps
      défilant était borné par la taille minimale du ScrollContainer (0) au lieu du contenu.
- [x] barre du haut hors écran à gauche en vue étroite (1280×720 × taille d'interface 1,25 =
      vue 1137×640) → palier compact (libellés courts, faction masquée, recherche étroite).
- [x] pilote : trésor lu dans `get_faction_summary` ; naval en résolution automatique.
- Note : le pilote hérite des réglages du joueur (taille d'interface 1,25) ; en 1920×1080 la
  vue logique est 1280×720 (configuration réelle du joueur), en 1280×720 elle tombe à 1137×640.

## À trier (partie 720p)
- « Recruter » (panneau d'armée) sous le bouton de fin de tour ; « Changer d'édit » hors écran
  à droite ; journal et bulles à gauche tronqués à ~345 px ; dialogue « Continuer » sous le
  panneau de diplomatie ; espion recruté absent (la sim recrute bien : clic du pilote ?).

## Prochaine étape
Partie 1920×1080 (vue 1280×720) en cours ; trier ce qui reste à cette taille.
