# M5a — vision par rayon (état)

Spec : `docs/design/2026-09-24-mouvement-libre.md` § 5. Branche : `m5a-vision` (worktree `agent-aef7911654fc760d5`), partie de main `fa7efb3c`.

## État

- [x] Cœur : `vision.rs` réécrit (`CampaignState::vision` → `Vision { mask: VisionMask, provinces, lenders }`, `visible_provinces`, `visible_armies`) ; `GameData::province_raster` ; `VisionRules.province_seen_percent` (+ schéma, `data/rules/vision.json`).
- [x] Tests Rust (`c1_vision.rs` réécrit : rayon armée, colonie, allié, armée cachée, temps ≈ 1 ms par faction en release).
- [ ] Pont : `get_vision`, `get_vision_mask`, `get_visible_army_ids`, `is_point_visible` ; `get_visible_provinces` inchangé.
- [ ] Godot : terrain + minicarte sur la texture 512², marqueurs d'armée filtrés par armée.
- [ ] Tests Godot, captures `docs/img/m5a/`.

## Décisions

- Point (armée, colonie) : distance exacte aux sources ; texture : disques rastérisés 512² (8 px carte par texel), bord doux de 3 km centré sur le rayon.
- Province visible : ≥ 25 % de ses texels de terre vus, ou colonie vue, ou armée amie dedans, ou agent (C6, pas terrestres conservés).
- L'IA ne lit pas la vision (elle voit tout, comme avant) : comportement gardé, documenté dans `vision.rs`.
- Anciennes portées en pas (`controlled_range`, `army_range`, `general_bonus`) ignorées, gardées en option pour compatibilité.

## Prochaine étape

Tests Rust, puis pont.
