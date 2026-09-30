# WIP Q8 — recette « comme un joueur » (2026-09-30)

Demande du joueur : tester le jeu comme un joueur et corriger les bugs (budget 100 captures).
Pilote `game/tests/q3_playtest.gd` : partie 1 Angleterre (toutes phases, 10 saisons), partie 2
Bourgogne (naval, commerce, diplomatie, tours, sauvegarde, réglages).
Branche `fix/q8-recette`, worktree `../gp-q8` (lien `data/map/pyramid` vers main).

## Constats / corrections
- [x] Infobulles brutes « ib:plain:roster_card_detail [img=…][b]…[/b] » sur les cartes de
      régiment d'avant-bataille (et mini-carte de bataille, carte d'indice, lettres) : classes
      scriptées sans `_make_custom_tooltip` → ajouté ; `attach_plain` signale désormais le cas.
- [x] Liste de recrutement de la fiche de ville : motif d'indisponibilité en colonne étroite,
      replié mot à mot → sous la ligne, pleine largeur (`panel_widgets.gd`).
- [x] Avis identiques empilés (« Carte politique. » ×3, « Recrutement lancé » ×3) → le nouveau
      remplace l'ancien (`ui_layout.gd`).
- [x] Diplomatie : « Que faudrait-il ? » sur un traité déjà acceptable (trêve à 90 %) répondait
      « Rien de ce que vous pouvez offrir ne suffirait » → « Rien à ajouter : ils accepteraient
      déjà en l'état » ; refus : « il y avait N % de chances d'accord ».
- [x] Anneau de sélection de ville (sans test de profondeur) : disque jaune plein puis arc en
      travers du ciel aux zooms les plus proches → masqué sous 4 rayons.
- [x] Routes maritimes : ruban de 1,6 unité monde → large bande rayée rouge/bleu (routes
      superposées) sur la Tamise en vue rapprochée → largeur écran plafonnée à 5 px.
- [x] Menu pause invisible et inerte (écran assombri seul) : ses panneaux, rangés dans la zone
      modale, héritaient de la pause du jeu (fondu figé à alpha 0) → `PROCESS_MODE_ALWAYS`
      (`pause_menu.gd`, test `q8_pause_menu_test.gd`). C'était le « menu Réglages introuvable ».
- [x] Infobulles brutes aussi sur les lettres (classe interne `Letter`), les jetons d'agent,
      les lignes de la cour ; `RichTooltip.set_tooltip` pose désormais l'hôte générique sur tout
      contrôle natif et signale une classe scriptée sans `_make_custom_tooltip`.
- [x] Discours du général (déploiement) : bandeau translucide sur le panneau « Formations de
      groupe » → remonté à 48 % de la hauteur.
- Pilote : bascule du commerce par V (plus X), contrôle de la faction réellement jouée.

## Laissé à une autre session
- Fenêtre de chronique qui sort de l'écran à droite (620 px dans la zone latérale de ≈ 380 px
  en 1280×720) : corrigée ici en zone modale puis annulée, la session « VN » refond la fenêtre
  pour la zone latérale au même moment.

## À signaler au joueur (non corrigé)
- Bataille pilote : « Défaite presque certaine, 0 % » en résolution auto, victoire écrasante
  en bataille menée (5 % de pertes contre 36 %) : IA tactique faible ou prévision pessimiste
  (chantier IA en cours sur feat/ia).
- Qualité « ultra » : 15 i/s contre 60 en « haute » sur cette machine.

## Écartés (pas des bugs du jeu)
- « OVERLAP » du pilote : `gui_get_hovered_control` suit le vrai curseur, pas les événements
  injectés ; sans effet sur les clics.
- Partie lancée en Transylvanie, « Combattre » sans effet, naval « dialog visible true » : vrai
  curseur / fenêtre occultée pendant la recette ; non reproduits (test `q8_start_faction_test`).
  Fenêtre occultée = plus d'image → le pilote attend : ramener la fenêtre au premier plan.
- « Lever des troupes 0/3 » après 3 recrutements : mission proposée après.
- Fin de tour « ignorée » avec le rapport de saison : test Q7 vert ; lié à la fenêtre occultée.

## Captures lues : 25 / 100 (planches de 4 réduites)

## État
Parties 1 (Angleterre), 2 (Bourgogne) et de contrôle jouées ; main fusionnée dans la branche
(conflit `sea_lane.gdshader` : `abs()` de HB + plafond Q8) ; tests ciblés verts ; smoke final
puis fusion ff dans main.
