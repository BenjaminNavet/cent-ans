# M5a — vision par rayon (état)

Spec : `docs/design/2026-09-24-mouvement-libre.md` § 5. Branche : `m5a-vision` (worktree `agent-aef7911654fc760d5`), partie de main `fa7efb3c`, main `b5a9c4d7` (M5b) fusionné.

## État : terminé (en attente de fusion par l'orchestrateur)

- [x] Cœur : `vision.rs` réécrit (`CampaignState::vision` → `Vision { mask: VisionMask, provinces, lenders }`, `visible_provinces`, `visible_armies`) ; `GameData::province_raster` ; `VisionRules.province_seen_percent` (+ schéma, `data/rules/vision.json`).
- [x] Tests Rust (`c1_vision.rs` réécrit : rayon armée, rayon colonie, allié, armée cachée, temps ≈ 1 ms par faction en release comme en debug).
- [x] Pont : `get_vision`, `get_visible_army_ids`, `is_point_visible` ; `get_visible_provinces` inchangé.
- [x] Godot : terrain + minicarte sur la texture 512² (bord doux effrangé, liseré sépia), marqueurs et points de minicarte filtrés par armée ; repli par province sans `get_vision`.
- [x] Tests Godot : `m5a_vision_ui_test.gd` (texture, armée cachée puis révélée), smoke adapté ; `m4_free_movement_ui_test`, `c5_settlements_ui_test` verts.
- [x] Captures `docs/img/m5a/` ; doc `docs/godot-map.md` § « Vision par rayon (lot M5a) ».

## Décisions

- Point (armée, colonie) : distance exacte aux sources ; texture : disques rastérisés 512² (8 px carte par texel), bord doux de 3 km centré sur le rayon.
- Province visible : ≥ 25 % de ses texels de terre vus, ou colonie vue, ou armée amie dedans, ou agent (C6, pas terrestres conservés).
- L'IA ne lit pas la vision (elle voit tout, comme avant M5a) : comportement gardé, documenté dans `vision.rs`.
- Anciennes portées en pas (`controlled_range`, `army_range`, `general_bonus`) ignorées, gardées en option pour compatibilité ; le bonus de chef disparaît.
- Agents : toujours masqués par province (`hidden_provinces`), inchangé.
- Capture « avant » : prise avec le nouveau cœur mais l'ancien rendu par province (le retour du cœur de main n'a pas été possible dans le worktree).

## Limites

- Avec 20 km autour des colonies, une grande partie du royaume du joueur est voilée entre ses villes (règle de la spec) ; à rééquilibrer si le rendu paraît trop « troué ».
- Les maquettes, forêts et hameaux ne sont pas voilés (seul le sol l'est), comme avant.
- La mer n'est jamais voilée (id de province 0), comme avant.

## Prochaine étape

Fusion par l'orchestrateur.
