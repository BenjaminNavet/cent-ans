# PB3g — quadtree de relief natif, `request_reground` sans copie

Branche `perf/pb3g-native-quadtree` (worktree d'agent
`.claude/worktrees/agent-a390841202fac1ad2`, depuis `main` a7877ac6). ADR réservé : 0092.
Liens symboliques non versionnés : `data/map/pyramid`, `tools/geo/raw` → dépôt principal.
Cible Cargo PRIVÉE : `<worktree>/core/target` (à supprimer à la fin, pas avant).

## État (PAUSE demandée par le joueur, 26/09)
- Fait, compile :
  - crate pure `core/crates/relief-lod` (workspace + opt-level 3 en dev) : `Pyramid`
    (`has_tile`, `max_level_under`, `finest_ancestor`, tuiles cassées), `Selector` = portage
    de `_prepare_camera`/`_select`/`_depth_cap`/`_y_bounds`/`_page_of`/hystérésis `px_scale`,
    résidence des pages (`add_page`, `remove_page`, `oldest_page` = LRU de `_alloc_layer`,
    ordre d'insertion sur égalité), `apply` = diff avec l'image précédente (ajouts, retraits,
    ombres, paramètres d'instance avec masque des changés, fondu, cache des paramètres de page
    par version de résidence, voisines marquées utilisées comme en GDScript). Numérique calquée
    sur GDScript (scalaires f64, vecteurs f32).
  - Tests `cargo test -p relief-lod` : 5/6 OK ; `budget_widens_the_threshold` corrigé
    (max_items 2) **mais non relancé** (pause).
- Écrit, **pas encore compilé** :
  - `core/crates/godot-bridge/src/relief_lod_bridge.rs` : classe `ReliefLod` (`set_pyramid`,
    `set_bounds`, `mark_broken`, `add_page` (garde les octets en `Arc`), `remove_page`,
    `clear_pages`, `oldest_page`, `clear_slots`, `update(camera, planes, view, config)` →
    dictionnaire de tableaux Packed, `items()` pour les tests) ; enregistrée dans `lib.rs`.
  - `vegetation` : `Ground::Pages` en `Arc<Vec<u8>>` (`PageBytes`) ;
    `vegetation_scatter.rs::ground_of` prend les octets dans `qt_store` (un `ReliefLod`) si
    l'instantané en porte un, sinon copie comme avant.
- Pas commencé : côté GDScript.

## Prochaine étape
1. `cd core && cargo fmt --all && CARGO_TARGET_DIR=<worktree>/core/target cargo clippy
   --all-targets -- -D warnings && cargo test -p relief-lod -p vegetation` ; corriger.
2. `relief_quadtree.gd` : `use_native_select` (+ `--no-native-quadtree`) ; `setup` crée
   `ReliefLod` (`set_pyramid` depuis `ReliefPyramid._tiles` → ajouter `tile_indices(level)`,
   `set_bounds`) ; `_upload` → `add_page(key, layer, t, _frame, bytes)` ; `_alloc_layer` →
   `oldest_page(_frame - 1)` + `remove_page` ; `mark_broken` relayé ; `update_view` natif :
   appliquer `removed`/`added`/`cast`/`params`, `_wanted_order` (PackedInt64Array) pour
   `_start_jobs`/`wait_jobs`, `_missing_wanted`, `_item_count` ; `surface_snapshot` ajoute
   `"qt_store": _native`.
3. Test `game/tests/pb3g_quadtree_test.gd` : sélection GDScript vs native (clés, fine, coarse)
   sur plusieurs caméras (même `px_scale`).
4. Mesures A/B (`pb1_bench.gd`, `--bench-probe`), captures `docs/img/pb3g/`, ADR 0092, tests
   zg2/zg4/zg5b/zg6/zg7a/zg8/sz + smoke, point 3 (hameaux, `TownLayer`) si le temps le permet.
