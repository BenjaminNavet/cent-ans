# Lot C6 : rendu de la carte par paliers de zoom

Spec : `docs/design/2026-09-24-echelle-colonies.md` § 6. Branche : `worktree-agent-ab78e060f95eaa4e6`.
Documentation : `docs/godot-map.md` § « Paliers de zoom, colonies, hameaux et routes (lot C6) ».

## État : terminé (à fusionner)

- [x] `ZoomTiers` (`game/scripts/map/zoom_tiers.gd`, `game/resources/zoom_tiers.tres`)
- [x] `SettlementData` : colonies, hameaux, routes (lecture `data/`)
- [x] Relief fin : `FineTerrainJob` + `TerrainBuilder.update_lod(..., view_center, fine_distance)`,
      `surface_height_at`, signal `chunk_surface_changed`, cache LRU, bords recollés + jupes
- [x] `SettlementLayer` : icônes, étiquettes dé-chevauchées, maquettes, hameaux (brûlés selon la
      dévastation), picking, sélection (`settlement_selected`)
- [x] `RoadRenderer` : routes principales (moyen), rubans drapés (près)
- [x] Générateur Blender `tools/blender_scripts/settlements.py` + 13 `.glb`
- [x] Intégration `campaign_map.gd`, caméra (`min_distance` 22, tangage 30° de près)
- [x] Test headless `game/tests/settlements_render_test.gd` (OK) ; smoke complet OK
- [x] Captures `docs/img/colonies/` (3 régions × 3 paliers), mesures `--fps-probe`

## Points ouverts (C5 / C7)

- C5 : brancher le panneau de colonie sur `SettlementLayer.settlement_selected(id)` (aujourd'hui toast
  + log) ; onglet Colonies du panneau de province.
- C4/C5 : poser les armées avec `TerrainBuilder.surface_height_at` (relief fin) et les placer sur la
  position de leur colonie (`SettlementLayer.world_position_of(id)`).
- Arbres non recalés sur le relief fin (heightmap 4096).
