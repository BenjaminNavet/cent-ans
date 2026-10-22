# camp60 : 60 fps sur la carte de campagne

Mandat du 22/10 (23 h) : monter la carte de campagne à 60 fps (16,7 ms par image), autonomie complète pour la nuit.
Branche `perf/camp60`, worktree `../gp-camp60` (données ignorées liées vers le dépôt principal).

## Référence
- `out/bench/map_mem-map.json` (22/10 10 h 40, machine chargée) : panoramique d = 30 p50 41 ms / p95 79 ms (~24 fps),
  descente p50 35 ms ; 573 meshes visibles (p50), 34 316 entités.
- Seuil `perf_guard` actuel : panoramique p50 20 ms.

## État
- 22/10 23 h 15 : en attente, à la demande du joueur, de la fin d'une autre session (game-project-79) qui fait de gros changements.

## Prochaine étape
1. Rebaser sur main une fois l'autre session finie.
2. Banc de référence + banc profilé (`BENCH_FEATURES=profile`) : CPU ou GPU ?
3. A/B `CENT_ANS_PERF_OFF=...` par famille pour chiffrer le GPU.
