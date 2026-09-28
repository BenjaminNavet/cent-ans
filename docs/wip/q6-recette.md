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

## Partie 1920×1080 (vue 1280×720, config du joueur) — en cours de correction (3 agents)
- A1 panneaux de droite : « Changer d'édit » sous la minicarte, « Recruter » sous la fin de tour
  (`game/tests/q6_side_panel_test.gd`).
- A2 diplomatie : « Proposer le traité » hors écran ; dialogue « Continuer » de fin de tour sous
  le panneau de diplomatie (`game/tests/q6_diplomacy_test.gd`).
- A3 avis/journal (zone TOASTS) au-dessus du registre des agents (clic sur un agent perdu) ;
  texte des avis tronqué en vue étroite (`game/tests/q6_toasts_test.gd`).

## Écartés (artefacts du pilote, jeu vérifié en headless)
- sélection d'agent depuis le registre : fonctionne au vrai clic quand rien ne le couvre (→ A3) ;
- menu pause / réglages : ESC ouvre bien le menu (enchaînement de touches du pilote) ;
- bouton « Commerce » masqué exprès (menu des filtres, touche V) ;
- clic sur la capitale : sélectionne l'armée qui s'y trouve (comportement voulu).

## Prochaine étape
Intégrer les 3 lots, smoke + tests, fusion ff dans main.
