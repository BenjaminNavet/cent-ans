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

## Pour le joueur — 3 décisions en attente (30/09)

1. **Capture de mouvement (mocap) payante** — plus gros écart restant avec TWW3 : corps à corps,
   combats appariés, réactions. Hors enveloppe v1 (ordre de grandeur 50-300 $). Pistes (licence
   commerciale, FBX, à reciblage sur le squelette MakeHuman par le pipeline Blender AN1b) :
   - Fab (ex-Marketplace Unreal) : « 260 Sword and Shield Mocap Animations » ; licence Standard Fab
     utilisable hors Unreal sauf mention contraire (vérifier la fiche).
   - MoCap Online : packs épée/bouclier, armes d'hast, réactions ; licence Standard libre de
     redevance jusqu'à 1 M$ de revenu, FBX/Blender inclus ; pack gratuit « T.C. Sword » pour tester
     le reciblage avant achat.
   - Game Dev Hero (itch.io) : 287-355 animations épée/bouclier, squelette UE5 + FBX.
   Mixamo exclu (licence). **Dépôt public** : ne jamais commiter les fichiers mocap bruts (FBX,
   BIP) ; les garder hors dépôt (dossier local ignoré) et ne commiter que les clips cuits, après
   lecture de la clause de redistribution de la licence choisie.
   **Essai NT12 (30/09, 0 $)** : base CMU (seule obtenue sans formulaire) reciblée sur 6 clips de mêlée, option `--mocap-trial` ; gain non net (pieds qui glissent 2-18 cm, pas de vrai impact ni mort, bras gauche repris du keyframé). Détail `docs/research/mocap-gratuite.md`, `docs/wip/nt12-mocap.md`. À télécharger à la main pour poursuivre l'essai (formulaire/e-mail) : MoCap Online T.C. Sword (redistribution publique interdite → hors dépôt), Rokoko 13 combats + 10 armes (redistribution non précisée → hors dépôt), Quaternius UAL2 (CC0) ; dossier `~/dev/cent-ans-mocap-src/`.
   **Essai NT13 (30/09, 0 $)** : vidéos du joueur → MediaPipe (Apache 2.0) → reciblage, option `--video-trial` (guard, overhead, slash, thrust). Verdict : égal au keyframé, mieux que CMU ; mouvement naturel mais bruit (slash), glissement des pieds (thrust 18 cm), profondeur faible. Second tournage conseillé : trépied, 60 i/s, 45° de la caméra, épée à une main + bouclier, pleine vitesse, 1 s immobile avant/après. Captures NT12 à refaire (bogue `reload` corrigé en NT13). Note `docs/wip/nt13-video-mocap.md`.
2. **Clé fal.ai (GA3, image → 3D)** — pour enrichir figurines (textures peintes) et roster
   (39 types d'unités). Voir `docs/wip/ga.md` l.196.
3. **Chantier guerre civile / prétendants** (Armagnacs et Bourguignons, États généraux, aide
   extraordinaire, coalitions N5/N8) — taille L, spec à écrire en brainstorming avec le joueur.

## Vague 3 (30/09 ~02:45)
- NT7 fondus entre clips de mêlée + clips porte-étendard / musicien / servants d'engins (pipeline ADR 0096, sans mocap)
- NT8 château : donjon carré procédural, échelle des tours du type château (capture : tours trop massives)
- NT9 équilibre NT5 : mesurer batailles FR/EN par décennie, corriger si besoin ; bélier en résolution automatique ; sorties de garnison comptées pour les missions

## Vague 3 — fusionnée (30/09)
- NT7 fondu 0,2 s entre clips de mêlée (shader, ADR 0129), 10 clips de rôle ; A/B dans le bruit. Ouvert : changements de clip secs pour rôles/servants ; kit grossier non recuit.
- NT8 château : tours rayon ≤ 6 m, donjon carré 32,5 m (toit en pavillon), basse-cour meublée. Ouvert : porte du donjon sans escalier ; une seule forme de toit.
- NT9 : une attaque IA par armée ennemie et par tour ; batailles FR/EN ×1,68 (au lieu de ×1,94) vs avant N6/N7, justifié ADR 0128 ; guerre FR–EN 64 % (10/12 graines dans 55-75) ; bélier +20 % en résolution auto ; sorties comptées pour les missions ; cv3_ai_stances rétabli graine 1.
- Intégration : schéma NT7 sans `$id` corrigé (465 pytest cassés sinon) ; pytest 1287, cargo test, clippy, smoke, tests Godot verts sauf `fe_ui_test` (préexistant, hors NT).

## Vague 4 (lancée après fusion de la vague 3, 063999ef0)
- NT10 fondu des rôles, imposteurs (casques, ombre, sang), herbe hors champ
- NT11 camp tenu (didacticiel), époque et engins en bataille personnalisée, donjon (escalier, toit en terrasse)

## Vague 4 — fusionnée (30/09)
- NT10 fondu 0,25 s des rôles (porte-étendards, musiciens, servants, étoffe) ; correctif indice de clip du drapeau ; imposteurs : 3 variantes casque/habit, ombre disque, sang ; herbe couchée +100 m hors champ. A/B : pas de perte mesurable.
- NT11 option cœur « camp tenu » (didacticiel) ; bataille perso : année 1337-1453 (roster filtré, 7 types datés), engins choisis ; donjon : porte haute + escalier, toit en terrasse (40 %).
- Vérifs : pytest 1301, cargo test, clippy, smoke, tests Godot verts sauf `fe_ui_test` (préexistant).
- Ouvert : fondus et foule à juger en jeu (captures nt10_* non lues, budget atteint) ; escalier hors emprise de la simulation ; ombre non orientée au soleil.
