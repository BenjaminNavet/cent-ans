# WIP orchestrateur — rapprochement Total War (session 6)

Mandat (24/09) : autonomie complète, plusieurs heures. Rapprocher le jeu de Total War, visuellement et en mécaniques ; garder les mécaniques propres à Cent Ans.
Hors périmètre : refonte colonies (C2c-C7, autre session, `docs/wip/colonies.md`), portraits.

Choix du joueur au lancement :
- Référence : **mélange moderne** (mécaniques de Medieval II + ergonomie et visuel des TW récents : Three Kingdoms, Pharaoh, Warhammer III).
- Priorité : **équilibrée**, alterner vagues bataille / campagne.
- Autorisé : dépenses cloud dans le plafond de 50 $, assets libres CC0/CC crédités, Blender local (MCP), push auto de main après chaque vague.

## Vagues

| Vague | Lots | État |
|---|---|---|
| 0 | P1 petits points cœur (no_quarter, étain, IA Normandie ouest) ; P2 rapport de saison + smoke ; R exploration TW | **fusionné** (P2 f48b557, P1 ae4ec92) |
| 1 | B1 maillages soldats/chevaux (Blender) ; B2 bannières d'unité + vignettes + écran de fin ; C1 minicarte + brouillard | **fusionné** : B2 c0b9635, B1 ff592ae, C1 ee4f2b6 |
| 2 | B3 musique dynamique + caméra de suivi ; B4 effets + animations (suites B1) + rythme d'engagement ; C2 zone de contrôle **après C4 colonies** (refonte du déplacement) | B3 fusionné (953a758) ; B4 terminé, non fusionné ; C2 en attente |
| 3 | C3 arbre familial + fiche de général (arbre de compétences) ; B5 champs de bataille tirés de la campagne (biome, village, saison) | B5 **fusionné** (be47ae9) ; C3 **terminé, NON fusionné** |

## État à l'arrêt de session (24/09 soir, arrêt demandé par le joueur)

main poussé (40d748c). Fusionnés : P1, P2, B1, B2, B3, B5, C1.
- **C3** (arbre familial dans la Cour + fiche TW avec arbre de compétences) : terminé, branche `worktree-agent-ab0ae4e048aaae5da` (5d75030), smoke vert dans sa branche (22 « smoke OK »). À fusionner.
- **B4** (animations, effets, rythme d'engagement) : **terminé sauf mesure de perf, NON fusionné**. Branche `worktree-agent-a3ef51586ad653fc2` (7b70c6a), worktree `.claude/worktrees/agent-a3ef51586ad653fc2`. Contact de la démo à 70 s au lieu de 301 s (IA `sim-battle/src/ai.rs` : duel d'archers perdu → marche au contact) ; archer/arbalétrier/cavalier/caparaçon/mêlée/chutes animés ; `battle_effects.gd` (poussière, flèches fichées, bombardes, choc de charge, pas de sang) ; captures `docs/img/b4/` ; cargo et smoke verts. Reste : banc de perf avec/sans `--no-effects` (commandes dans `docs/wip/b4-effets-animations.md` de la branche), éclaboussures de gué non vérifiées, fanions de lance restés verticaux quand la lance est couchée. Après fusion : `core/build.sh` puis `godot --headless --path game --import` (nouvelle classe `BattleEffects`).
- **C2** : attend la fusion de C4 colonies.

## Reprise
1. Fusionner C3 : dans `../gp-tw-merge` (`integration/tw`) : `git merge main`, `git merge --no-ff worktree-agent-ab0ae4e048aaae5da`, clippy + `cargo test`, `core/build.sh`, `godot --headless --path game --import`, smoke (compter 22 « smoke OK ») ; puis dans main `git merge --ff-only integration/tw` + push.
2. Fusionner B4 de même (conflits possibles avec C3/B5 dans `battle_scene.gd` et `smoke.gd`), puis mesurer la perf (banc avec/sans `--no-effects`).
3. Si C4 colonies est fusionné (`docs/wip/colonies.md`) : lancer C2 (zone de contrôle + aire atteignable colorée).
4. Suite du plan : C4 édits/chaînes de bâtiments (après colonies), C5 routes commerciales, C6 agents Medieval II.
5. Pistes relevées par les lots :
   - IA tactique qui cherche haies et village (archers anglais), `sim-battle` (B5) ;
   - dialogue d'avant-bataille et minicarte : afficher village/côte/sol (`ground_label`) (B5) ;
   - neige un peu plate (B5) ; repères d'unité regroupés en vue très lointaine (B2) ;
   - suite/compagnons du général et année de décès absents du cœur (C3) ; fiche qui recouvre la Cour sous 1500 px (C3) ;
   - pistes musicales dédiées (B3) ; mesures de perf B1/B5 à refaire machine au repos (écran plafonné à 60 Hz).
6. Supprimer les worktrees fusionnés encore verrouillés (C1 `agent-ad1dcecdc0ab3b8f5`, B5 `agent-ab6ae0addbf27dfdd`).

## Décisions
- B5 : éléments de site tirés d'un flux dérivé de la graine (batailles existantes inchangées) ; options `--terrain= --season= --village --coast --no-site`.
- C1 : règle de vue `vision.rs` + `data/rules/vision.json` ; brouillard désactivable (Réglages > Carte).
- B3 : musique de bataille = `war.ogg` transformé (filtre, couches sfx), pas de nouvelle piste.
- Piège : le smoke Godot sort en 0 même sur erreur d'analyse ; compter les lignes « smoke OK » (21 attendues) et `--import` après un nouveau `class_name`.
- B1 : figures modélisées en Blender scripté (`tools/blender/battle_figures.py`), ADR 0006 ; `--legacy-figures` pour comparer.
- B2 : illustrations d'unités générées par une autre session (non suivies) ; les cartes s'en servent dès qu'elles existent, sinon composition de repli.
- C2 reporté : C4 colonies refond le déplacement.
- P1 : pas de quartier = chef pris tué, −5 piété au vainqueur ; étain déjà utilisé (maison de fonte), schéma inchangé ; `frontier.rs` source unique de la classification frontière.
- Plan : `docs/design/2026-09-24-rapprochement-total-war.md` (lots B1-B4, C1-C6). Thème parchemin conservé, densité TW visée.

## Procédure de fusion
Worktree `../gp-tw-merge` (branche `integration/tw`) puis ff-only dans main + push ; commits avec chemins explicites (index partagé).
