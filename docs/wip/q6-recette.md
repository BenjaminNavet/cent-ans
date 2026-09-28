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

## Partie 1920×1080 (vue 1280×720, config du joueur) — corrigé
- panneaux de droite (province, colonie) tenus dans leur zone : lignes repliées, actions en flux,
  onglets à la hauteur restante (`q6_side_panel_test.gd`, notes `q6-panneau-lateral.md`) ;
- diplomatie : « Proposer le traité » hors page défilante ; rapport de saison au-dessus du panneau
  modal (`q6_diplomacy_test.gd`, notes `q6-diplomatie.md`) ; ouverture automatique seulement
  pour une offre de poids (pas les accords commerciaux, ~90 factions), jamais sur une décision ;
- avis/journal (zone TOASTS) sous les fenêtres (registre des agents, Colonies), texte replié
  en vue étroite (`q6_toasts_test.gd`, notes `q6-toasts.md`).
- Partie de contrôle 1920×1080 : aucun chevauchement, agent / diplomatie / naval / décision OK.

## Points ouverts
- Panneau de province : l'en-tête (9 lignes) laisse peu de place aux onglets en vue 1280×720 ;
  l'édit s'atteint en faisant défiler la zone. Refonte de l'en-tête à juger visuellement.
- Pile d'avis : trois avis très longs dépassent la zone vers le bas (non observé en jeu).

## Écartés (artefacts du pilote, jeu vérifié en headless)
- sélection d'agent depuis le registre : fonctionne au vrai clic quand rien ne le couvre (→ A3) ;
- menu pause / réglages : ESC ouvre bien le menu (enchaînement de touches du pilote) ;
- bouton « Commerce » masqué exprès (menu des filtres, touche V) ;
- clic sur la capitale : sélectionne l'armée qui s'y trouve (comportement voulu).

## État
TERMINÉ 2026-09-29 : fusionné dans main (ff).
