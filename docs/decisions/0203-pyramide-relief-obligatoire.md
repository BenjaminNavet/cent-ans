# ADR 0203 — La pyramide de relief devient obligatoire

Date : 2026-10-09. Statut : accepté. Lots SC : PF-02, MB1, MB2, AD8 (MC10).

## Contexte

Le relief de campagne se dessinait par deux voies. La voie nominale (ADR 0036, 0092) : le
`ReliefQuadtree` affiche des patchs déplacés au GPU depuis les pages de la pyramide de relief ;
la sélection des nœuds, la résidence des pages (LRU) et les paramètres d'instance sont calculés
par la classe native `ReliefLod` (crate `relief-lod`). La voie de repli, conservée « au cas où » :
sélection GDScript équivalente dans `relief_quadtree.gd` (comparée à la voie native par
`pb3g_quadtree_test`), puis, sans pyramide, maillages « proches » et « fins » de `TerrainBuilder`
(LOD proche à `near_step`, relief fin 8192² via `FineTerrainJob`, cache LRU). Le repli n'était plus
exercé que par des tests : le jeu livré embarque toujours la pyramide (ADR 0036, ZG7b) et
l'extension native.

## Décision

- La pyramide de relief et la classe native `ReliefLod` sont obligatoires.
- Supprimés : la sélection GDScript du quadtree (`_select*`, `_page_of`, `_apply_items`,
  `_page_params`, `_neighbors`, `compare_selection`…), `fine_terrain_job.gd` et le repli tuilé de
  `terrain_builder.gd` (maillages proches et fins, tâches, cache, exports `near_step`, `fine_step*`,
  `max_fine_jobs`, `max_cached_fine`, `pyramid_enabled`, option `--fine-step`).
- Conservés dans `TerrainBuilder` : les morceaux E0 au pas `far_step` (vue parchemin CM2 et bornes
  du quadtree), les niveaux de morceau 0/1/2 et `chunk_surface_changed` (signal des calques).
- Si la pyramide ou l'extension manque : `push_error` explicite, `TerrainBuilder.relief_error`
  affiché par un toast d'erreur au lancement de la carte. Pas de repli : le terrain se réduit aux
  morceaux E0 lointains.
- `has_page` (relief-lod) : vérifié mort côté jeu, voir la section Conséquences.

## Conséquences

- Environ 1 000 lignes GDScript en moins ; plus de double implémentation à garder bit à bit avec le
  Rust.
- Un poste sans la pyramide (`data/map/pyramid`, `CENT_ANS_RELIEF_DIR`) ne démarre plus avec un
  relief dégradé : il affiche un message d'erreur. Les tests qui exigent le relief fin sautent
  déjà quand la pyramide manque.
- Les maillages de repli n'existent plus pour les captures sans pyramide ; les bancs de perf
  comparent désormais uniquement les réglages du quadtree.
