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
- [x] 1. `ReliefPyramid._drop_missing_tiles` : un listage de dossier par étage, seules les tuiles
  absentes sont retirées (avant : tout l'étage si sa 1re tuile manquait) ; repli par tuile sur
  l'ancêtre le plus fin présent (`finest_ancestor`) ; tuile corrompue écartée à l'exécution
  (`mark_broken`, déjà là). Test `tests/zg7c_partial_cache_test.gd` (trous E1/E2, E3 sous un trou
  E2, tuile corrompue, reliquat `.part.png`) : quadtree stable à d = 40, 10, 4.
- [x] 2. `geo detail-dem` complet : 30/31 grappes E5, 31/32 E6, 32/33 E7 recuites (Londres déjà à
  jour), 190 s ; 281 tuiles E4 et 300 E5 parentes recalées ; manifeste inchangé (mêmes tuiles).
  Écriture déjà atomique (`.part.png` puis `replace`, idem tuiles parentes et tuiles CAFV).
  `detail-check` : p95 quasi identiques à ZG3b (22 → 4 m) ; médianes relevées dans les vallées
  (Castillon 0 → 4,5 m, Château-Gaillard 1,1 → 4,5, Harfleur 1,1 → 4,4) : c'est l'écart entre le
  plancher v4 (≤ 5 m) et E4 plaqué à 0,5 m, attendu. `relief-all --check` : complet, 2,77 Go.
  Fonds de vallée E1-E4 à 0,5 m : **non corrigés** (recuisson E1-E4 = pyramide entière, plusieurs
  heures, et E0 `heightmap_render.png` porte le même rehaussement : continuité E0/E1 à reprendre
  ensemble) → suite.
- [x] 3. Tamise : `hydro_fine.water_level` borne le fond à 0 m (`MIN_WATER_LEVEL_M`) avant
  l'ajustement monotone ; `hydro-fine` (snaps en cache, 138 s) puis `anchors-fine` relancés.
  Londres −7,76 → 0,07 m ; Bordeaux (bac) −15,8 → 0 m ; Avignon/Arles −0,97 → 0,45 m ;
  28 passages modifiés. Sauvegardes (clones APFS) dans le scratchpad `zg7c/`.
- [x] 4. `map_bench` : `process_ms` = début de l'itération (nœud `FrameStart`, priorité minimale,
  physique comprise) → banc (priorité maximale). Essai descente : process p50 5,2 ms, p99 75 ms ;
  131 pics > 50 ms sur 131 dominés par les scripts (médiane 59 ms) → les pics sont côté scripts,
  pas GPU (hydro-fine tournait en parallèle : chiffres relatifs).
- [ ] 5 · [ ] 6

## Prochaine étape
Captures de recette (`tests/zg7c_recette_shots.gd`).
