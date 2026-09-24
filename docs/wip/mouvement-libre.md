# Orchestration : mouvement libre des armées

Spec : `docs/design/2026-09-24-mouvement-libre.md`. ADR : `docs/decisions/0010-free-army-movement.md`. Conception validée par le joueur le 2026-09-24.

| Lot | État | Notes |
|---|---|---|
| M1 grille de navigation | **fusionné** | 79 ponts, 17 gués/bacs, 12 cols ; 97,9 % de la terre franchissable ; notes `docs/wip/m1-navgrid.md` |
| M2 cœur | en cours (worktree `agent-acbe5c6448250fa88`, branche `m2-core-movement`, relancé après plantage le 24/09 à 23 h 55) | grille uniforme de repli tant que M1 n'est pas fusionné |
| M3 tour séquentiel + IA | à faire | |
| M4 pont + UI | à faire | |
| M5 vision, équilibrage, docs | à faire | |

## Décisions prises en cours de route

- 2026-09-24 : le joueur valide la spec telle quelle (« fais le »).
- 2026-09-24 : la session 6 (TW) est prévenue que son lot C2 « zone de contrôle » (vague 6) recoupe M2 ; proposition de le suspendre jusqu'à la fusion de M2. C2 est suspendu.
- 2026-09-24 : l'ADR est renumérotée en 0010 (0006 à 0009 déjà pris).
- M1 : Saône non tracée en aval de Chalon (ponts sans effet) ; 397 croisements voies romaines/fleuves ouvrent beaucoup de passages, à surveiller en M5.
- À la fusion de M2, vérifier : `agents.rs` (C6, Dijkstra sur le graphe des colonies, s'appuie sur `movement.rs` ; les agents ne sont pas arrêtés par la zone de contrôle) et la ligne C7 dans `apply_outcome`.

## Prochaine étape

M1 et M2 tournent en parallèle dans des worktrees. Ensuite : fusion de M1, puis de M2 (dans un worktree séparé, ff-only dans main). Puis lancement de M3 et M4 en parallèle, et enfin de M5.
