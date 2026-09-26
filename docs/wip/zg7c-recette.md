# ZG7c — recette visuelle aux 3 paliers et clôture du chantier ZG (ADR 0036)

Worktree d'agent `worktree-agent-a980562fd6a6cb636` (depuis `main` 553a345d). Liens symboliques non
versionnés : `data/map/pyramid`, `tools/geo/raw` → dépôt principal. Dylib :
`CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/core/target cargo build -p godot-bridge` puis copie
dans `game/bin/libcent_ans.debug.dylib`. Visuel et données seulement, rien dans le jeu de `core/`.

## Plan
1. `ReliefPyramid` robuste à une pyramide trouée (repli par tuile sur l'ancêtre le plus fin présent),
   test avec une pyramide factice trouée.
2. Recuisson complète `geo detail-dem` (plancher v4 sur les 34 zones), `detail-check`,
   `relief-all --check` ; écriture atomique des tuiles ; fonds de vallée E1-E4 (plancher 0,5 m) : à évaluer.
3. Tamise fine à −7,8 m : corriger `hydro_fine` (Londres ~0-2 m).
4. Banc : attribution de `process_ms` dans `map_bench`.
5. Recette visuelle aux 3 paliers (12 lieux), captures `docs/img/zg7c/`, défauts.
6. Clôture : `docs/godot-map.md` (vue d'ensemble ZG), addendum final ADR 0036.

## État
- [x] 1 (repli par tuile, `tests/zg7c_partial_cache_test.gd` OK) · [ ] 2 (recuisson detail-dem en cours) · [ ] 3 · [ ] 4 · [ ] 5 · [ ] 6

## Prochaine étape
Suivre la recuisson (journal scratchpad `zg7c_detail_dem.log`), puis Tamise (hydro_fine).
