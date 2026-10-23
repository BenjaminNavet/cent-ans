# Boucle juge → corrections

Mandat 23/10 : boucle illimitée (arrêt par le joueur, ou deux passes de suite sans constat majeur).
Juge = sous-agent Sonnet indépendant, rapport dans `out/juge/iter-NN.md`. Corrections par lots, commits ciblés, pas de push.
Axes : jouabilité, gameplay, visuels, assets, couleur, textures, fps (cible 60 en Ultra), cohérence, véracité historique et géographique,
UI/UX, prise en main, équilibrage, accessibilité, stabilité, chargement/mémoire, aberrations visuelles, audio (logs/fichiers), textes/localisation,
échelle, cohérence artistique, données, authenticité matérielle, plausibilité militaire et politique.
Budget : Meshy 155 crédits (23/10, à choisir avec soin, confirmer le coût avant), 2D via Z-Image. Aucune autre dépense cloud.

## Itérations
| # | Juge | Majeurs | Corrigés | fps | Commit |
|---|------|---------|----------|-----|--------|
| 1 | out/juge/iter-01.md | 1 (C1 fps bataille) → infirmé par banc neuf `j1` | 0 code ; rejetés : C1 (bataille 75 fps calme / 59 mêlée, p95 18,5 ms), C2 (ADR 0183), C3 (météo brouillard ADR 0596), C5 (champ `playable` = sélecteur de faction, 139 voulues), C6, C8 (déjà daté 1360), C10 (artefact `--offscreen`) ; restent C4 (toits de siège sombres, mineur), C7 (conseiller Jean le Bel, mineur) | carte pan 69 moy., bataille 75/59 | 9518d5d0b |

## Prochaine étape
Itération 2 : juge ciblé sur gameplay/IA (parties scriptées, seed bots), tutoriel, mémoire, plausibilité ; constats à prouver (dump/mesure) avant d'être retenus. Faux positifs connus à éviter : voir ligne 1.

## Intégration 23/10 (branche integ/juge)
Fusion des lots MEM, HIST, DAUPHINE, ECO, CAUSES, UX, LUM, EAU, TERRAIN (partiel), ARMEES (partiel). ADR renumérotés : 0754 causes, 0755 palette.
Dérogation décidée par le joueur pour CE tour : ni smoke ni seed bots. Contrôle = clippy, tests cœur et client, pytest. Smoke et seed bots à passer au prochain tour d'intégration.
Restes : hameaux + ADR terrain, densité/imposteurs des armées, vérification visuelle campagne (LUM, EAU), 2 échecs pytest HIST (maisons sans blason, références d'archétypes de portraits), refontes de design (IA à doctrines, arcs de partie).

## Reprise (arrêt du joueur, 23/10)
État : branche `integ/juge` (worktree `../gp-integ`, HEAD 95fa23b29) = main + 10 lots fusionnés ; NON fusionnée dans main, rien de poussé.
Vert : pytest (2920), clippy + tests cœur (avant dernières retouches), clippy client, tests client sauf `real_table_and_glbs` (modèles 3D gitignorés absents du worktree : à valider sur main).
Contrôle final interrompu : relancer `/private/tmp/claude-501/full_check2.sh` (core clippy+test, client clippy+test) ou les commandes de CLAUDE.md. Pas de smoke ni seed bots (dérogation de ce tour : à passer au prochain).
Ensuite : `git merge --ff-only integ/juge` dans main (checkout principal), supprimer worktrees des lots (`.claude/worktrees/agent-*`, vérifier merge-base --is-ancestor) et `../gp-integ`, puis itération 3 du juge (4 angles : DA, design, histoire/UX, gameplay/IA ; captures campagne en `CENT_ANS_WEATHER=clear`, fps sur machine calme).
Restes : hameaux + ADR terrain (0758), densité/imposteurs/premier plan des armées (ARMEES partiel), vérification visuelle campagne (LUM, EAU), refontes de design (IA à doctrines, arcs de partie, scénarios), pistes mémoire (copies CPU, KTX2 sol), Dauphiné : données de faction (barks etc.) à compléter.
Outil : `tools/heavy.sh` (sémaphore 2 emplacements), règle en mémoire `feedback-build-semaphore-merge-checks`.
