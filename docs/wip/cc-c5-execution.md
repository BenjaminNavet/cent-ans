# CC C5 — exécution (Rust + GDScript), branche `cc/c5`

Spec : `docs/design/2026-10-14-carte-composee.md` ; contrat : `docs/wip/cc-carte-composee.md`.

## État
- Rust `data-model/src/map.rs` : `MapMeta` à deux CRS (`EPSG:3035`, `composed`) ; `ComposedWarp`
  (grilles float32 bilinéaires, forward/inverse, hors emprise -> `None`) ; `inverse_laea_3035`.
  Une seule projection : `lonlat_to_px`, `px_to_lonlat`, `projected_to_px`, `px_distance_km`,
  `MapMeta::load(map_dir)` / `load_warp`. `GameData::load` charge le warp (avertissement si absent).
- Règles : distances de jeu = px x `meters_per_px` en `composed` (`GameData::settlement_distance_km`,
  utilisé par `movement_graph` et `feudal`) ; météo/peste/sécheresse lisent déjà les `lonlat` réels
  des données (aucun changement) ; `terrain_mods` et `navgrid` passent par `MapMeta`.
- `vegetation/stands.rs` : copie LAEA supprimée, `StandTable.projection` (MapMeta).
- Pont : classe `MapProjectionNative` (`godot-bridge/src/map_projection.rs`).
- GDScript : `scripts/map/map_projection.gd` (`MapProjection`) ; copies supprimées :
  `fauna_layer.gd` (laea/lonlat_to_px), `landmark_v2_library.gd` (bornes). Tests migrés.
- Détail rapproché : `terrain_builder.gd` coupe la pyramide si `crs == composed` ; shader
  `terrain.gdshader` `proc_detail_apply` ; réglages `data/ui/campaign_map.json:composed_detail`.

## Convention warp.json lue (à respecter par C1)
`crop_box_px [x0,y0,x1,y1]`, `factor`, `size_px [w,h]` (canevas composé), `real_grid`
(`bounds_projected` ou `origin [minx,maxy]` + `meters_per_px`), optionnel `forward`/`inverse`
(nom de fichier, défaut `warp_forward.f32` / `warp_inverse.f32`, même dossier). Nœud (i,j) au centre
du bloc : coord = (px - origine)/factor - 0.5. forward : px réels absolus -> px composés (origine
= coin du recadrage) ; inverse : px composés -> px réels absolus. `map.json` : `crs: "composed"`,
`composed.warp` (défaut `compose/warp.json`), `size_px`/`meters_per_px` du canevas composé.

## Prochaine étape
Tests complets, build GDExtension, smoke.
