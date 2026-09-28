# RS-K — perf de la carte : TownLayer, `qt/collect`, NextHintController

Branche `feat/rs-k-perf` (worktree d'agent, depuis `main` 68050e4f). Pas de Rust à ce stade :
dylib copiée du dépôt principal. Liens non versionnés `data/map/pyramid`, `tools/geo/raw`.

Restes de PB3g / SZ6 : `TownLayer` ~10 ms par image, `qt/collect` ≤ 9 ms (écouteurs de
`surface_changed`), `NextHintController.refresh` (déjà sur événement, RS-E : vérifier).

## Outils
- `godot --path game res://scenes/campaign_map.tscn -- --stage=map --hide-armies --bench-map
  --bench-probe [--bench-towns]` ; RS-K ajoute au rapport `probe` médiane / p95 / p99 par section
  (toutes images mesurées, 0 si absente) et les sections `town/*` (poll, lod_view, stream, step,
  reground, models) et `hint.refresh`.

## Constats (passe 1, charge 90-180)
- `town/*` de `TownLayer.update_view` : tous < 3 ms au pire ; le pic « TownLayer » vient de
  `SettlementLayer._update_model_visibility` (toutes les maquettes à chaque ville construite ou
  retirée) : 25 ms au pire (`town/models`).
- `hint.refresh` apparaît encore : 15,6 ms toutes les 5 s (minuterie de secours de RS-E).
- `map.settlements` p95 25 ms / p99 46 ms et `map.life` p99 57 ms : sous-sections `settle/*`,
  `life/*` ajoutées pour attribuer.

## A/B 1 (3 passes alternées, charge 12-60, médianes) : premiers correctifs
- `town/models` pire 22-23 ms des deux côtés : le pic venait de la bascule d'activité (vallée),
  pas des villes construites → 2e correctif (bascule : seules les villes construites, étiquettes
  non recalculées : elles ne dépendent pas des villes 1:1).
- `hint.refresh` pire 11,8 → 0,1 ms.
- `qt/collect` p99 ~5 ms : `qt/alloc` (éviction) p99 3,4 et `qt/emit` p99 1,6-1,8, presque tout
  dans `qt/l_fine` (`FineGeoLayer._on_surface_rect_changed` → `ReliefQuadtree.finest_levels` :
  parcours des ~256 pages en GDScript) ; l'éviction refaisait le même parcours pour `_chunk_top`.

## Correctifs
- `TownLayer.take_changes()` : `SettlementLayer` ne revoit que les colonies des villes changées ;
  bascule d'activité : villes construites seulement ; plus de recalcul des étiquettes.
- `NextHintController` : la minuterie de secours ne relit le cœur que si l'empreinte (tour,
  tutoriel, alertes, croix) a changé ou si le dernier conseil n'a pas été calculé (panneau ouvert).
- `ReliefQuadtree.finest_levels` : pages indexées par étage (`_level_pages`), du plus fin au plus
  grossier, tuiles candidates ou pages de l'étage ; utilisé aussi par l'éviction (`_chunk_top`).
  Test `rs_k_finest_levels_test.gd` : identique au parcours complet, ~7× plus rapide (3,5 contre
  24,5 ms pour 202 rectangles).

## Outils A/B
Copies APFS de `game/` (scratchpad `ab/base` : scripts instrumentés sans correctifs, `ab/new`) +
lien `data` ; `bench.sh`, `ab.sh`, `agg.py` (médianes des passes) dans le scratchpad.

## Prochaine étape
Tests (smoke, sz4b, zg6, pb3g, sz6, ux2, rs_k) puis A/B 2 (3 passes), chiffres ici.
