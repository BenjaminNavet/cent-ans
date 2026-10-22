# Nuit de critiques par rôle (2026-10-22)

Mandat : ~12 agents Sonnet critiquent le jeu, chacun depuis un rôle de développeur ; Claude synthétise, priorise et modifie le jeu (nouveaux systèmes autorisés, avec ADR). Commits sur main, sans push.

## État
- Vague 1 (6 rôles) : lancée — rapports dans `docs/wip/nuit-critiques/<rôle>.md`
- Vague 2 (6 rôles) : à lancer après la vague 1
- Synthèse : `docs/wip/nuit-critiques/SYNTHESE.md` (à écrire)
- Lots de correction : à définir après synthèse

## Prochaine étape
Attendre les rapports vague 1, lancer vague 2, synthétiser, découper en lots.

## B3 (carte, Guyenne) — NON FUSIONNÉ (22/10 nuit)
Branche `worktree-agent-a53dfb50c3248f38b` (commit 13a834f88, worktree `.claude/worktrees/agent-a53dfb50c3248f38b`). Rebase sur main en conflit sur 8 binaires générés (`data/map/colormap_bc1_*.bin`, `colormap_preview.jpg`, `navgrid.png`, `province_border_dist.png`, `splat.png`) : main a reçu entre-temps des correctifs de côte/lacs (f4d8106aa, f30fa5d86) d'une autre session. Prendre l'une ou l'autre version écraserait un travail ; il faut régénérer la chaîne `geo provinces` → `cross_fixture` sur main après rebase des données (hors binaires). À faire avec le joueur ou en séance dédiée. Reste de B3 (Gwynedd/Lothian, 3 cartes de bataille, `neighbors`, ADR 0730) fusionnable à part en cherry-pickant les fichiers non binaires.
