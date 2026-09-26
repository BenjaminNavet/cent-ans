# SZ6 — pics d'images côté scripts sur la carte de campagne (suite S6 de ZG7c)

Branche `feat/sz6-script-spikes` (worktree d'agent, depuis `main` 7e1ac032). Liens symboliques non
versionnés `data/map/pyramid`, `tools/geo/raw` → dépôt principal. Dylib :
`CARGO_TARGET_DIR=/Users/jean_hubert/dev/game_project/core/target cargo build -p godot-bridge` puis
copie dans `game/bin/libcent_ans.debug.dylib`.

Objectif : p99 des images < 50 ms sur le parcours `--bench-map`, sans changement visuel.
Coordination : sélection du quadtree en Rust = lot PB3g (après fusion de SZ6) ; ADR éventuel :
0082 (0079-0081 réservés par PB3) ; ne pas toucher aux profils Cargo (PB3a).

## Outils
- `godot --path game res://scenes/campaign_map.tscn -- --stage=map --hide-armies --bench-map
  --bench-probe` : `PerfProbe` (minuteries par section, `game/scripts/dev/perf_probe.gd`) ;
  rapport `probe` : par section, temps cumulé dans les pics > 50 ms, nombre de pics où elle
  domine, pire durée ; les 12 pires images et leurs sections. Sections permanentes :
  `campaign_map._process` (`map.*`), `update_lod` (`lod/*`), étapes du quadtree (`qt/*`).
  Pendant l'analyse, les `_process` des autres nœuds ont été enveloppés temporairement (non commité).
- A/B entrelacé : copies du dossier `game/` par clonage APFS (`cp -cR`, sans place disque) dans le
  scratchpad, scripts de `main` d'un côté et de la branche de l'autre, passes alternées.

## Diagnostic (sonde, machine à charge 80-150)
1. `RoadRenderer.update_view` : un ruban de route drapé par image au minimum, jusqu'à 115 ms
   (densification tous les 0,5 u + `surface_height_at` point par point) : 195 pics sur 212.
2. `Vegetation._apply_lod` : changement de maillage détaillé / simple par
   `MultiMesh.mesh = …` après `buffer` → le serveur de rendu relit le tampon au GPU
   (`buffer_get_data`, synchrone) pour recalculer la boîte : jusqu'à 74-86 ms.
3. `LandmarkModel` : recuisson des hauteurs par tranches de 1,5 ms par maquette et par image,
   jusqu'à 16-31 ms par image avec plusieurs maquettes.
4. Ensuite : `update_lod` (quadtree : `select` / `apply` / `collect` 10-20 ms sous charge),
   colonies `settlement_layer.update_view` (≤ 20 ms), `life.update_view` (≤ 29), émissions de
   changement de niveau (≤ 35), `next_hint_controller` (≤ 15), recalage végétation (≤ 24).

## Correctifs
- [x] Rubans de route construits dans des fils (`RoadRenderer.RibbonJob`, instantané
  `surface_snapshot` de l'emprise des tronçons, même échantillonnage que
  `surface_heights_at`) ; repli synchrone sans quadtree et dans `flush`.
- [x] Végétation : nouveau `MultiMesh` recréé depuis la copie processeur du tampon au changement
  de maillage (`_with_mesh`), plus de relecture GPU.
- [x] Maquettes : recuisson complète dans un fil (`_bake_rows` sur instantané), installation de la
  texture sur le fil principal.
- [ ] Suivants : voir diagnostic 4.

## Mesures
Base (main 7e1ac032, charge ≈ 19) : parcours p50 16,5 ms, p99 76,8, 189 images > 50 ms ;
descente p99 90,3, 140 pics dont 137 dominés par les scripts.

A/B r1 (2 passes chacun, charge 80-210 : relatif seulement), routes + végétation :
| | p50 | p99 | max | > 50 ms | descente p99 |
|---|---|---|---|---|---|
| main | 24,5 | 122,8 | 298 | 281 | 138,0 |
| SZ6 | 31,4 | 63,1 | 122 | 112 | 62,7 |
p50 plus haut côté SZ6 : charge plus forte pendant ces passes (212) ; à revérifier.

## Prochaine étape
A/B avec les maquettes ; puis colonies / vie / émissions de niveau / recalage végétation.
