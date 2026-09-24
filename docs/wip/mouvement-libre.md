# Orchestration : mouvement libre des armées

Spec : `docs/design/2026-09-24-mouvement-libre.md`. ADR : `docs/decisions/0006-free-army-movement.md`. Conception validée par le joueur le 2026-09-24.

| Lot | État | Notes |
|---|---|---|
| M1 grille de navigation | en cours (agent worktree) | |
| M2 cœur | en cours (agent worktree) | grille uniforme de repli tant que M1 n'est pas fusionné |
| M3 tour séquentiel + IA | à faire | |
| M4 pont + UI | à faire | |
| M5 vision, équilibrage, docs | à faire | |

## Décisions prises en cours de route

- 2026-09-24 : le joueur valide la spec telle quelle (« fais le »).
- 2026-09-24 : la session 6 (TW) est prévenue que son lot C2 « zone de contrôle » (vague 6) recoupe M2 ; proposition de le suspendre jusqu'à la fusion de M2.

## Prochaine étape

M1 et M2 tournent en parallèle dans des worktrees. Ensuite : fusion de M1, puis de M2 (dans un worktree séparé, ff-only dans main). Puis lancement de M3 et M4 en parallèle, et enfin de M5.
