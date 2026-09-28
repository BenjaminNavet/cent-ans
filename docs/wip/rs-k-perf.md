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

## Correctifs
- `TownLayer.take_changes()` : `SettlementLayer` ne revoit que les colonies des villes changées.
- `NextHintController` : la minuterie de secours ne relit le cœur que si l'empreinte (tour,
  tutoriel, alertes, croix) a changé ou si le dernier conseil n'a pas été calculé (panneau ouvert).

## Outils A/B
Copie APFS de `game/` (scratchpad `ab/base`, scripts instrumentés sans correctifs) + lien `data`.

## Prochaine étape
Mesure base instrumentée, attribution `settle/*`, `life/*`, `qt/jobs`, puis A/B 3 passes.
