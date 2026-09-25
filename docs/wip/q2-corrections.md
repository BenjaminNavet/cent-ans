# WIP Q2 — corrections de la recette Q1

Branche `worktree-agent-aad9c6074d1c28dd3` (worktree agent-a72563bd61b138ea6), Q1 fusionné.
Rapport source : `docs/audit/q1-recette.md`. Pilote : `game/tests/q1_playtest.gd`, phases Q2
`q2zoom`, `q2click`, `q2tutorial` (+ `battle`). Sauvegarder
`~/Library/Application Support/Godot/app_userdata/Cent Ans/settings.cfg` avant, restaurer après.
Lancer en 1920×1080 (en 1280×720 le bouton « Commencer » du menu sort de l'écran du Mac).

## État
- [x] P1 figurines CV2 : échelle ∝ distance, plancher 0,27 (au lieu de 0,8) ; armée stationnée
      dans une ville L1-L3 posée hors de la zone de la maquette (531a2962)
- [x] P1 clic ville / armée : ce qui est sous le curseur (score), second clic au même endroit
      alterne (613c32fb)
- [x] P1 fin de bataille après retraite générale : `Unit::fate` (cœur) exposé `fate` par le
      pont, `SideResult.withdrew` ; « s'est retiré », fait notable « a sonné la retraite » (78b7e454)
- [x] P2 tutoriel : bloqué à « Vos provinces » car le clic sur la ville ouvre le panneau de
      colonie (C5) ; panneau de colonie accepté, onglet Bâtiments, chantiers de colonie comptés,
      flèche sur la ville ; guide masqué sous pause/réglages/chronique ; parchemin rangé à gauche
      d'un panneau ouvert (il couvrait la liste des bâtiments)
- [x] P2 son : une seule fenêtre (Menu → Son… ouvre Réglages → Son), curseurs thémés (e35d479c)
- [ ] P2/P3 restants : toast 3,5 s sur le titre du panneau (11), chronique sur rapport de
      saison (12), fin de bataille 720p (13), réglage taille des unités (14, à vérifier)
- [ ] captures C2, C3, C8
- [ ] édits (C4, dans main f9c2bd98) : section visible sans défilement dans l'onglet Ville —
      fusionner main d'abord
- [ ] diagnostic solde anglais (10) : noter seulement

## Prochaine étape
Première case non cochée.
