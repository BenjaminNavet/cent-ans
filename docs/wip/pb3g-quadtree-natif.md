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
- Point 3 (partiel) : hameaux — hauteurs en un appel groupé (`TerrainBuilder.surface_heights_at`
  → `ReliefQuadtree.surface_heights_at` → `ReliefLod.heights_m`, même bilinéaire) et tampon
  `MultiMesh.buffer` écrit en une fois (`SettlementLayer.hamlet_buffer`).
  `TownLayer` et `NextHintController` non traités.

## Mesures
`--bench-map --bench-probe` (fenêtré, dylib dev, natif N / repli `--no-native-quadtree` G
alternés, 3 passes chacun, charge 9-15 ; médianes ; passe N2 perturbée par une compilation) :

| | p50 | p99 | pire | > 50 ms | descente p99 | descente scripts p99 | `qt/select` max | `qt/apply` max | `lod/quadtree` max |
|---|---|---|---|---|---|---|---|---|---|
| G | 17,9 | 38,4 | 59,2 | 3 | 39,9 | 31,0 | 14,9 | 7,0 | 24,4 |
| N | 14,9 | 34,5 | 46,6 | 0 | 25,3 | 15,0 | 0,22 | 0,89 | 9,4 |

`request_reground` (tuile de Paris, 103 pages) : 4,7 ms (copie) → 0,04 ms (pages partagées),
test headless hors charge ; en jeu la copie montait à ~70 ms (SZ6).
Reste dans `lod/quadtree` : `qt/collect` (téléversements + écouteurs de `surface_changed`,
≤ 9 ms).

## Prochaine étape
1. Mesures A/B (bench-map/probe, pb1_bench), médianes de 3 ; consigner ici et dans l'ADR.
2. Captures A/B `zg7c_recette_shots.gd --only=... --prefix=native_|gd_` → `docs/img/pb3g/`.
3. Point 3 si le temps le permet (hameaux en lot, `TownLayer`, `NextHintController`).
4. Fusion de `main`, tests, suppression de `core/target`.
