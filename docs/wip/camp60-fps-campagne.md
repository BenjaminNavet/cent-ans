# camp60 : 60 fps sur la carte de campagne

Mandat du 22/10 (23 h) : monter la carte de campagne à 60 fps (16,7 ms par image), autonomie complète pour la nuit.
Branche `perf/camp60`, worktree `../gp-camp60` (données ignorées liées vers le dépôt principal).

## Référence
- `out/bench/map_mem-map.json` (22/10 10 h 40, machine chargée) : panoramique d = 30 p50 41 ms / p95 79 ms (~24 fps),
  descente p50 35 ms ; 573 meshes visibles (p50), 34 316 entités.
- Seuil `perf_guard` actuel : panoramique p50 20 ms.

## État
- 22/10 23 h 15 : en attente, à la demande du joueur, de la fin d'une autre session (game-project-79) qui fait de gros changements.

- Partage avec game-project-79 (chantier Bevy/rendu) : elle cède la perf campagne. Ne pas toucher `campaign/terrain/`
  avant la fusion de son correctif des samplers : sans lui, la campagne plante au premier rendu sur Metal (17 samplers > 16).
  Prévenir avant chaque banc release de campagne.
- Plan : audit `git show worktree-agent-a37447ae40299d827:docs/wip/bevy-audit-campagne.md`. L1 et L2 sont déjà dans main ;
  les lots L3 à L7 reviennent à camp60 (L5 relief = `campaign/terrain/`, après la fusion de game-project-79).
- Piège : lier `data/map/pyramid` vers `~/dev/cent-ans-raw/compose/pyramid` et faire
  `rsync -a --ignore-existing <principal>/game/assets/ game/assets/` ; tuer le client par son pid ; `CARGO_INCREMENTAL=0`.

## Prochaine étape
1. Rebaser sur main une fois l'autre session finie.
2. Banc de référence + banc profilé (`BENCH_FEATURES=profile`) : CPU ou GPU ?
3. A/B `CENT_ANS_PERF_OFF=...` par famille pour chiffrer le GPU.
