# FC — FPS carte et décor campagne (orchestration)

Spec : `docs/superpowers/specs/2026-09-30-fc-fps-et-decor-campagne-design.md`. Branche `feat/fc`,
worktree `../game_project-fc`. Mandat : autonomie. Mesures seulement sur machine calme.

## Lots
- [ ] FC0 base (attend la fin de SR3b/SR5 et de toute compilation)
- [ ] FC1 ombres maquettes/moulins par préréglage
- [ ] FC2 arbres imposteurs
- [ ] FC3 herbe et broussailles proches
- [ ] FC4 banc A/B, captures, ADR 0137, fusion

## Journal
- 09-30 : spec, worktree.

## FC1
- État : clé `model_shadow_distance` (préréglages ; legacy 500 = avant FC1) lue par
  `SettlementLayer.apply_render_quality` ; moulins (`LifeEffects`) ombrés sous `veg_shadow_distance`.
- Prochaine étape : test `game/tests/fc1_shadows_test.gd`, puis FC3.

## FC3
- À faire : `ground_clutter.gd` + `ground_clutter.gdshader`, clé `clutter_density`
  (ajoutée aux préréglages : 0 / 0,5 / 1 / 1,5, legacy 0).

## FC2 — arbres imposteurs (agent FC2)
État : squelette. Générateur `tools/blender_scripts/campaign_tree_impostors.py` (Blender headless,
arbres sources riches → atlas 8 vues × 3 essences, albédo+alpha et normale),
shader `game/shaders/campaign_tree_impostor.gdshader`, câblage `vegetation.gd` (tuiles au-delà de
`detail_distance`), drapeau `--no-fc2`, test `game/tests/fc2_impostors_test.gd`.
Prochaine étape : script Blender et cuisson des atlas.
