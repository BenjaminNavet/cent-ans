# Lot C6 : rendu de la carte par paliers de zoom

Spec : `docs/design/2026-09-24-echelle-colonies.md` § 6. Branche : `worktree-agent-ab78e060f95eaa4e6`.

## État

- [x] `ZoomTiers` (`game/scripts/map/zoom_tiers.gd`, `game/resources/zoom_tiers.tres`)
- [x] `SettlementData` : colonies, hameaux, routes (lecture `data/`)
- [x] Relief fin : `FineTerrainJob` + `TerrainBuilder.update_lod(..., view_center, fine_distance)`,
      `surface_height_at`, signal `chunk_surface_changed`
- [ ] `SettlementLayer` : icônes, étiquettes, maquettes, hameaux, picking
- [ ] `RoadRenderer` : routes principales (moyen), rubans drapés (près)
- [ ] Générateur Blender `tools/blender_scripts/settlements.py` + `.glb`
- [ ] Intégration `campaign_map.gd`, caméra (`min_distance`)
- [ ] Test headless `tests/settlements_render_test.gd`
- [ ] Captures `docs/img/colonies/`, FPS, `docs/godot-map.md`

## Prochaine étape

Écrire `settlement_layer.gd` et `road_renderer.gd`, puis intégrer dans `campaign_map.gd`.
