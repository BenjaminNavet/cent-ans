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
- Pilote : bascule du commerce par V (plus X), contrôle de la faction réellement jouée.

## Écartés (pas des bugs du jeu)
- « OVERLAP » du pilote : `gui_get_hovered_control` suit le vrai curseur, pas les événements
  injectés ; sans effet sur les clics.
- Partie lancée en Transylvanie, « Combattre » sans effet, naval « dialog visible true » : vrai
  curseur / fenêtre occultée pendant la recette ; non reproduits (test `q8_start_faction_test`).
  Fenêtre occultée = plus d'image → le pilote attend : ramener la fenêtre au premier plan.
- « Lever des troupes 0/3 » après 3 recrutements : mission proposée après.
- Fin de tour « ignorée » avec le rapport de saison : test Q7 vert ; lié à la fenêtre occultée.

## Captures lues : ~17 / 100

## État
Partie 2 (Bourgogne) en cours ; ensuite smoke, tests, fusion dans main.
