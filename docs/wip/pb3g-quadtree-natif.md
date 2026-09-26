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
`pb1_bench.gd --views=150,40` (3 passes alternées, médianes, charge 11-18 ; très bruité, GPU
dominant) : zoom France ↔ Paris p50 31,8 → 25,1 ms, p95 46,9 → 34,7 ms, pire 67,6 → 74,5 ms
(une passe N à 314 ms, isolée) ; arrivée au zoom 150 : 1 665 → 1 679 ms (pire image 137 → 142),
zoom 40 : 839 → 877 ms (pire 67 → 51) : inchangées au bruit près (décodage et téléversement des
pages, végétation : hors du périmètre).

Captures A/B (`zg7c_recette_shots.gd`, Paris, Val de Loire, Alpes, Rouen × 3 paliers, météo
claire) : relief identique ; seuls écarts dans les maisons de Paris au palier site, plus faibles
que l'écart entre deux passes du repli seul (9 565 contre 15 189 pixels) — construction
progressive de la ville. `docs/img/pb3g/ab_*.jpg` (gauche natif, droite repli).

Reste dans `lod/quadtree` : `qt/collect` (téléversements + écouteurs de `surface_changed`,
≤ 9 ms).

## Suites
- `qt/collect` (≤ 9 ms : écouteurs de `surface_changed` à l'arrivée des pages).
- `TownLayer` (~10 ms), `NextHintController.refresh` (15 ms/s : `get_faction_summary`,
  `get_army` par armée) non traités.

## Prochaine étape
Fusion de `main`, tests, suppression de `core/target`, rendu à l'orchestrateur.
