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
- [x] P2/P3 : bandeau masqué par un panneau ouvert après lui (11) ; chronique ouverte après
      la fermeture du rapport de saison (12) ; « Faits notables » sous le bilan (13) ; écran
      des couronnes réduit d'un bloc sous 1280×860 (bouton « Commencer » visible en 720p) (02a66753)
- [x] 14 : « Taille des unités » et « Sang » présents dans main (BV1/BV2 fusionnés), rien à faire
- [x] main fusionné (C4, C5) ; section des édits en tête de l'onglet Ville (6a807c00)
- [x] captures C2, C8, 720p, édits : `docs/audit/captures/q2/`
- [ ] 10 (solde anglais négatif) : noté, lot G1
- [ ] nouveau : la bulle du chroniqueur (VO1, main) couvre le titre du rapport de saison
      (capture q2/04) — lot VO1

## Prochaine étape
Q2 terminé ; restent les deux points ouverts ci-dessus (autres lots).
