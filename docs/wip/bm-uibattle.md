# bm-uibattle : qualité de l'UI de bataille (client Bevy)

Retour joueur : « l'UI est désordonnée et ne ressemble pas à celle de Godot ». Référence : battle_hud.gd (bandeau du bas, coin minicarte, barre du haut, journal à gauche).

## État
- Fait (commit 1) : nouvelle mise en page de `client/src/battle/hud/`.
  - Bandeau du bas unique (140 px comme Godot) : sceau du chef dans le bandeau (108 px, armes de la faction à défaut de portrait), cartes groupées par « bataille » (illustration recadrée en portrait comme `_draw_art`), grille d'ordres (nouveau `commands.rs`) et « Retraite générale » (confirmation).
  - Coin bas droite : minicarte (200 x 124) + colonne de vitesse dans un seul panneau de même hauteur que le bandeau.
  - Bouton « Formation » = première case de la grille (icône + forme courte) ; le menu s'ouvre au-dessus, vers la gauche.
  - Barre des ordres du chef : page du kit à marges serrées (le kit en met 28 px).
  - Colonne en haut à gauche : journal puis alertes (plus de recouvrement par la bulle du chroniqueur, en haut à droite).
  - Barre du haut : titre complet (« Bataille personnalisée : France contre Angleterre »), météo sur la ligne du site, rapport de forces.
  - Emplacements (`geometry::HudSlot`) : les pièces construites par d'autres plugins du dossier s'y accrochent au lieu de deviner des décalages absolus.
  - `hud_layout` (dump) : rectangles mesurés, chevauchements, pièces qui débordent de leur conteneur (`escapes`), haut de l'écran (`left_stack`), cartes / boutons hors du bandeau (`strip_fits`).

## Écarts à Godot (voulus)
- Bouton « Quitter » gardé dans la barre du haut.
- Vitesses en texte (II, ×0,5…) plutôt qu'en pictogrammes dessinés.

- Fait (commit 2) : barre du haut centrée hors de la zone des avis (`TOASTS_RESERVE`, la bulle du chroniqueur est un avis du kit), ligne de siège idem ; marges de la barre des ordres ; tests minicarte à la nouvelle taille.
- Vérifié (dumps, sans recouvrement ni débordement) : bataille par défaut, siège de Paris, menu des formations ouvert, `Mode_run` / `Command_halt` acceptés par le cœur, « Retraite générale » confirmée (`withdrawn_all`).

## Lot clos
Restes : vérification à 1024 px seulement par le test de géométrie (pas d'option de taille de fenêtre dans run_bg.sh) ; journal déplié par défaut (Godot : replié) ; sceau sans bord de cire festonné.

## Restes (hors dossier)
- Bulle du chroniqueur (`audio/advisor.rs`) en haut à droite : Godot la place sous le journal, à gauche ; elle peut toucher la barre du haut quand le titre est long.
- Panneau « Formations de groupe » (`battle/orders/formations.rs`) déplié par défaut sans déploiement (`CENT_ANS_NO_DEPLOY`) ; Godot le montre replié.
