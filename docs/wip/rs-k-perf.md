# RS-K — perf de la carte : TownLayer, `qt/collect`, NextHintController (+ effets de vie)

Branche `feat/rs-k-perf` (worktree d'agent, depuis `main` 68050e4f). **Aucun changement Rust**
(dylib copiée du dépôt principal), pas d'ADR (pas d'arbitrage d'architecture). Liens non
versionnés `data/map/pyramid`, `tools/geo/raw` → dépôt principal.

## Outils
- `godot --path game res://scenes/campaign_map.tscn -- --stage=map --hide-armies --bench-map
  --bench-probe` ; RS-K ajoute au rapport `probe` moyenne / médiane / p95 / p99 par section (toutes
  images mesurées, 0 si absente) et des sections permanentes : `town/*` (poll, lod_view, stream,
  step, reground, models), `settle/*` (labels, scale, towns, landmarks, hamlets, declutter),
  `life/*` (terroir, seasons, effects, ambient, smoke_rewrite, mill_rewrite, reground), `qt/*`
  (main_decode, jobs, alloc, add_page, wait, l_terrain, l_fine) et `hint.refresh`.
- A/B : copies APFS de `game/` (base = scripts instrumentés sans correctifs, new) + lien `data`,
  passes alternées (scripts `bench.sh`, `ab.sh`, `agg.py` du scratchpad de la session).

## Diagnostic
- `TownLayer.update_view` lui-même : < 5 ms au pire. Le pic « TownLayer » (~25 ms) venait de
  `SettlementLayer._update_towns` : toutes les maquettes (~570) revues et toutes les étiquettes
  recalculées à chaque changement de version, surtout à la bascule d'activité (palier vallée).
- `NextHintController` : la minuterie de secours de RS-E relisait tout l'état du cœur (armées,
  chantiers) toutes les 5 s : 12-15 ms.
- `qt/collect` : presque tout dans les écouteurs de `surface_changed` →
  `FineGeoLayer._on_surface_rect_changed` → `ReliefQuadtree.finest_levels` (parcours GDScript des
  ~256 pages, à chaque page arrivée ou évincée) ; l'éviction refaisait le même parcours.
- Hors périmètre initial mais le plus gros reste : `life/effects` p99 ~50 ms (réécriture des
  ~4 500 panaches et 962 moulins à chaque pas d'échelle : 3 `model_scale_at` par point).

## Correctifs (tous commités)
- `TownLayer.take_changes()` : `SettlementLayer` ne revoit que les colonies des villes construites
  ou retirées ; bascule d'activité : villes construites seulement ; plus de recalcul des étiquettes
  (leur hauteur ne dépend pas des villes 1:1 ; `_label_state` sans l'activité des villes).
- `NextHintController` : secours = empreinte bon marché (tour, tutoriel reporté, alertes, croix) ;
  relecture du cœur seulement si elle change ou si le dernier conseil n'a pas été calculé (panneau
  ouvert) ; sinon visibilité seule.
- `ReliefQuadtree.finest_levels` : pages indexées par étage (`_level_pages`), du plus fin au plus
  grossier avec arrêt, tuiles candidates ou pages de l'étage ; aussi pour `_chunk_top` à
  l'éviction. `finest_levels_scan` garde le parcours complet (référence des tests).
- `LifeEffects` : tampons MultiMesh écrits depuis des copies processeur (plus de lecture de
  `MultiMesh.buffer`), échelles de maquette mémorisées par passe, réécritures des nœuds masqués
  reportées jusqu'à leur réapparition.

## Mesures finales (A/B 3 passes alternées, charge 95-160, médianes des passes)
| | base | RS-K |
|---|---|---|
| image p50 / p99 / pire (ms) | 18,7 / 104,0 / 234 | 18,7 / 86,0 / 165 |
| images > 50 ms | 106 | 87 |
| descente p99 / scripts p99 | 118,3 / 113,6 | 97,6 / 89,5 |
| `town/models` pire | 25,3 | 0,0 |
| `settle/towns` p99 / pire | 1,17 / 25,3 | 0,78 / 4,7 |
| `hint.refresh` pire | 13,2 | 0,1 |
| `qt/collect` p95 / p99 / pire | 3,39 / 6,22 / 8,2 | 0,58 / 0,80 / 2,5 |
| `lod/quadtree` p99 | 6,56 | 1,40 |
| `life/effects` p99 / pire | 54,2 / 112,7 | 30,3 / 52,2 |

## Tests
smoke, rs_k_finest_levels_test (nouveau), rs_k_buffer_layout_test (nouveau, fenêtré : format
des tampons identique à `set_instance_*`), sz4b, zg6, pb3g, sz6, ux2, cv1, sz4 : OK.

## Restes
- `settle/scale`, `settle/labels`, `settle/hamlets` : p99 17-27 ms chacun aux pas d'échelle d'un
  zoom (570 maquettes replacées, absorption, 570 étiquettes, hameaux) — même motif que les effets
  de vie (`_effective_scale` recalculé plusieurs fois par colonie).
- `life/smoke_rewrite` p99 ~19 ms, `life/reground` pire ~50 ms (hauteurs point par point) :
  boucle par point en GDScript ; piste : passage en Rust ou étalement (ADR 0051).

## Prochaine étape
Lot terminé : fusion par l'orchestrateur (pas de dylib à reconstruire).
