# NT — nuit niveau TWW3

Spec : `docs/superpowers/specs/2026-09-29-nt-nuit-tww3-design.md`. ADR : 0126 (types de places),
0127 (missions), 0128 (plafond et engins). Coût cloud : 0 $.

## État
- [x] NT1 sièges variés (château, bourg fortifié, cité), choisissables aussi en bataille personnalisée
- [x] NT2 bataille personnalisée (menu principal)
- [x] NT3 missions de campagne (panneau d'objectifs, touche O)
- [x] NT4 bataille-prologue (« Didacticiel de bataille », écran Batailles historiques ; invite au premier lancement)
- [x] NT5 plafond de 20 unités (N6), engins construits pendant le siège (N7)
- [x] NT6ab discours du général ennemi, indicateur « visé », surprime des mercenaires, en-tête de province compact
- [x] NT6cd IA au repos, réaffectation des touches (Réglages > Commandes)
- [x] Intégration : fmt, clippy, cargo test workspace, pytest 1285, build, smoke et tests Godot NT/DV/FK/UI verts

## Points ouverts (partie pilote)
- `fe_ui_test` échoue sur main aussi (sélecteur de carte cadré sur 7741 de large, seuil du test pensé pour 4096 ; hors NT).
- NT5 : batailles FR/EN ×1,9 par décennie (hypothèse : attente des engins → plus de batailles de secours, non mesuré) ;
  `cv3_ai_stances` : l'embuscade n'a plus lieu en graine 1 (trajectoire décalée par l'attente des échelles), test vérifié sur graines 1 et 5 ;
  le bélier n'a pas d'effet en résolution automatique.
- NT4 : l'ennemi passif peut fuir (moral) ; pas de commande « tenir » pour le camp non joué.
- NT3 : sorties de garnison non comptées ; équilibre des récompenses.
- NT1 : donjon = tour ronde agrandie (pas de maquette carrée) ; densité du bourg (~40 îlots) à juger à l'œil.
- NT2 : roster sans époque ni technologies ; engins par défaut échelles + bélier.
- Captures de contrôle NT1/NT2 : à faire (scripts `nt1_siege_shot.gd`, `nt2_shot.gd`, fenêtre requise).

## Pour le joueur
Mocap payante (corps à corps, combats appariés), clé fal.ai (GA3), chantier guerre civile / prétendants.
