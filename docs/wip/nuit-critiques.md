# Nuit de critiques par rôle (2026-10-22)

Mandat : ~12 agents Sonnet critiquent le jeu, chacun depuis un rôle de développeur ; Claude synthétise, priorise et modifie le jeu (nouveaux systèmes autorisés, avec ADR). Commits sur main, sans push.

## État
- 12 critiques rendues : `docs/wip/nuit-critiques/<rôle>.md` ; synthèse `SYNTHESE.md`
- Vague A (A1-A6) : fusionnée dans main
- Vague B : B1 (IA), B2 (mêlée), B6 (docs/audio) fusionnés ; B3 (carte) EN ATTENTE, voir ci-dessous
- ARRÊT demandé par le joueur le 22/10 ; aucun agent en cours

## Prochaine étape (reprise)
1. B3 : rebaser les données non binaires puis régénérer la chaîne géo (voir section B3).
2. Lots restants : jauge de tension + armée rebelle (ADR, garde-fou part de guerre FR-EN), ombres d'impostors/perf, remparts de siège (pierre triplanaire), impacts VFX complémentaires.
3. À décider par le joueur : remplaçants musicaux d'époque (england/burgundy/iberia/islamic), plafond budget (136 $ vs 50 $), push, capture de contrôle UI (tutoriel) et mêlée.

## B3 (carte, Guyenne) — NON FUSIONNÉ (22/10 nuit)
Branche `worktree-agent-a53dfb50c3248f38b` (commit 13a834f88, worktree `.claude/worktrees/agent-a53dfb50c3248f38b`). Rebase sur main en conflit sur 8 binaires générés (`data/map/colormap_bc1_*.bin`, `colormap_preview.jpg`, `navgrid.png`, `province_border_dist.png`, `splat.png`) : main a reçu entre-temps des correctifs de côte/lacs (f4d8106aa, f30fa5d86) d'une autre session. Prendre l'une ou l'autre version écraserait un travail ; il faut régénérer la chaîne `geo provinces` → `cross_fixture` sur main après rebase des données (hors binaires). À faire avec le joueur ou en séance dédiée. Reste de B3 (Gwynedd/Lothian, 3 cartes de bataille, `neighbors`, ADR 0730) fusionnable à part en cherry-pickant les fichiers non binaires.
