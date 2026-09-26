# PB3g — quadtree de relief natif, `request_reground` sans copie

Branche `perf/pb3g-native-quadtree` (worktree d'agent
`.claude/worktrees/agent-a390841202fac1ad2`, depuis `main` a7877ac6). ADR réservé : 0092.
Liens symboliques non versionnés : `data/map/pyramid`, `tools/geo/raw` → dépôt principal.
Cible Cargo PRIVÉE : `<worktree>/core/target` (à supprimer à la fin, pas avant).

## État (reprise du 26/09)
- `main` fusionné (35ddb216). `cargo fmt`/`clippy -D warnings`/`cargo test` (relief-lod,
  vegetation) OK.
- Fait : crate `relief-lod` ; classe `ReliefLod` ; `ReliefQuadtree` branché (natif par défaut,
  `--no-native-quadtree` / `use_native_select` pour le repli) ; pages partagées `qt_store` pour
  `request_reground` ; `ReliefPyramid.tile_indices` / `broken_keys`.
- Test `game/tests/pb3g_quadtree_test.gd` : 30 sélections comparées (5 caméras × 6 images) :
  clés, pages fines/grossières, distances, pages voulues identiques ; recalage avec pages
  partagées = pages copiées. OK.
- Tests : smoke, zg2, zg4, zg5b, zg6, zg7a, zg8, sz4, sz5, sz6 OK. `sz1_mountain_test` échoue
  (Paris ×0,50) **aussi en repli GDScript** : sans lien avec PB3g (données/relief de `main`).
- ADR 0092 écrit.
- En cours : mesures A/B (`--bench-map --bench-probe`, natif N / repli G alternés, script
  scratchpad `ab_bench.sh`).

## Prochaine étape
1. Mesures A/B (bench-map/probe, pb1_bench), médianes de 3 ; consigner ici et dans l'ADR.
2. Captures A/B `zg7c_recette_shots.gd --only=... --prefix=native_|gd_` → `docs/img/pb3g/`.
3. Point 3 si le temps le permet (hameaux en lot, `TownLayer`, `NextHintController`).
4. Fusion de `main`, tests, suppression de `core/target`.
