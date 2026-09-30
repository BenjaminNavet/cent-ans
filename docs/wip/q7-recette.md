# WIP Q7 — recette « comme un joueur » (2026-09-30)

Demande du joueur : tester une partie et corriger les bugs. Pilote `game/tests/q3_playtest.gd`
(France, 1920×1080 → vue 1280×720, 12 tours). Branche `fix/q7-recette`, worktree `../gp-q7`.

## Constats / corrections
- [x] PLANTAGE à l'ouverture de l'écran des factions en vue 1280×720 (« Message queue out of
      memory », SIGBUS) : `_fit` oscillait entre deux facteurs (texte replié → hauteur minimale
      dépendante de la largeur) et `CONNECT_DEFERRED` rejouait la boucle dans le même vidage de
      file. Ajustement une fois par image + réduction seule après 3 agrandissements
      (`faction_select.gd`, test `q7_faction_fit_test.gd`).

- [x] avis éphémères : la minuterie capturait l'avis ; fermé avant (clic, changement d'écran),
      « Lambda capture at index 0 was freed » (15 fois dans le smoke) → identifiant d'instance
      (`ui_layout.gd`).
- [x] sort de la place prise / décisions de chronique en vue 720 : l'enluminure remplissait le
      corps borné, les choix restaient sous le pli (« Plus tard » seul visible) → choix hors de la
      page défilante, corps borné à la place restante (`chronicle_window.gd`, contrôle dans
      `p2d_ui_test.gd`).
- [x] Entrée pendant la relecture du tour de l'IA (CT1) : ignorée en silence (une fin de tour sur
      deux perdue en jouant vite) → passe la relecture, comme Espace ; légende « Espace ou Entrée »
      (`campaign_map.gd`, `ai_turn_replay.gd`, `q7_end_turn_test.gd`).
- [x] `q6_diplomacy_test.gd` ne compilait plus (types `DiplomacyController`/`DiplomacyPanel`
      compilés avant les autoloads) ; étendu à une trêve refusée et à « Que faudrait-il ? ».
- Pilote : gère la fenêtre de sort des villes, attend la fin du tour, journalise les chemins.

## Écartés
- « OVERLAP » du pilote sur les boutons de diplomatie : survol lu avant le mouvement (test Q6 :
  boutons atteignables dans les 3 configurations).
- Arrêt au zoom minimal (run 3) : plus aucune image dessinée, fenêtre occultée ; non reproduit
  (50 i/s au zoom 0,3).
- Un worktree n'a pas `data/map/pyramid` (ignoré par git, 4,7 Go) : lien symbolique vers main
  avant toute recette, sinon relief absent.

## État
TERMINÉ 2026-09-30 : partie de contrôle (9 saisons d'affilée, sauvegarde/chargement OK), fusionné dans main (ff).
