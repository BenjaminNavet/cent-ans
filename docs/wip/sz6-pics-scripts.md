# SZ6 — pics d'images côté scripts sur la carte de campagne (suite S6 de ZG7c)

Branche `feat/sz6-script-spikes` (worktree d'agent, depuis `main` 7e1ac032). Liens symboliques non
versionnés `data/map/pyramid`, `tools/geo/raw` → dépôt principal. Dylib :
`CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/core/target cargo build -p godot-bridge` puis
copie dans `game/bin/libcent_ans.debug.dylib`.

Objectif : p99 des images < 50 ms sur le parcours `--bench-map`, sans changement visuel.

## Outils
- `godot --path game res://scenes/campaign_map.tscn -- --stage=map --hide-armies --bench-map
  --bench-listeners --bench-probe` ; `--bench-probe` active `PerfProbe` (minuteries par section)
  et rapporte `probe` : par section, temps cumulé dans les pics > 50 ms, nombre de pics où elle
  domine, pire durée.

## Mesures
Base (main 7e1ac032, charge ≈ 19, 16 Godot) : parcours complet p50 16,5 ms, p99 76,8 ms,
189 images > 50 ms ; descente p50 18,1, p99 90,3, 140 pics dont 137 dominés par les scripts
(médiane 57,6 ms de scripts) ; `qt_update_ms_max` 14 ; écouteurs de `chunk_surface_changed` :
colonies 231 ms et ponts 148 ms cumulés sur tout le parcours.

## État
- [x] Squelette : `PerfProbe`, doc wip.
- [ ] Instrumentation (`campaign_map._process`, `update_lod`, `_process` des nœuds de la carte).
- [ ] Attribution des pics, correctifs.
- [ ] Mesures entrelacées avant / après, docs.

## Prochaine étape
Instrumenter et attribuer les pics.
