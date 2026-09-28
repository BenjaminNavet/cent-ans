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

## État
- Squelette : sections de sonde ajoutées. Mesure « avant » à faire.

## Prochaine étape
Mesure avant (3 passes), diagnostic, correctifs, mesure après.
