# FC — FPS carte et décor campagne (orchestration)

Spec : `docs/superpowers/specs/2026-09-30-fc-fps-et-decor-campagne-design.md`. Branche `feat/fc`,
worktree `../game_project-fc`. Mandat : autonomie. Mesures seulement sur machine calme.

## Lots
- [ ] FC0 base (attend la fin de SR3b/SR5 et de toute compilation)
- [x] FC1 ombres maquettes/moulins par préréglage
- [ ] FC2 arbres imposteurs
- [ ] FC3 herbe et broussailles proches
- [ ] FC4 banc A/B, captures, ADR 0137, fusion

## Journal
- 09-30 : spec, worktree.

## FC1
- État : clé `model_shadow_distance` (préréglages ; legacy 500 = avant FC1) lue par
  `SettlementLayer.apply_render_quality` ; moulins (`LifeEffects`) ombrés sous `veg_shadow_distance`.
- Test `game/tests/fc1_shadows_test.gd` OK (pf1/rl1 OK). Fait.

## FC3
- État : `game/scripts/map/ground_clutter.gd` (`GroundClutter`, créé dans `campaign_map.gd`
  `_setup_settlements`), `game/shaders/ground_clutter.gdshader`, test `game/tests/fc3_clutter_test.gd` OK.
- Cellules de 32 u (tuiles de terrain de 256 u trop grandes : écart assumé à « 1 appel par tuile ») ;
  semis dans le `WorkerThreadPool`, pose ≤ 0,3 ms/cellule ; d = 30, Haute, couverture pleine :
  15 k touffes en 9 cellules (9 appels, sans ombre) ; plafond 30 k.
- Masque procédural (brins / feuillage), pas de texture ; teinte saisonnière `campaign_season`.
- Points ouverts : réglage visuel (taille 0,14/0,26 u, couleurs) à juger en capture (FC4) ;
  +~10-17 appels de dessin sous d = 40 (objectif FC4 « aucune hausse » mesuré au zoom panoramique).

## FC2 — arbres imposteurs (agent FC2)
État : squelette. Générateur `tools/blender_scripts/campaign_tree_impostors.py` (Blender headless,
arbres sources riches → atlas 8 vues × 3 essences, albédo+alpha et normale),
shader `game/shaders/campaign_tree_impostor.gdshader`, câblage `vegetation.gd` (tuiles au-delà de
`detail_distance`), drapeau `--no-fc2`, test `game/tests/fc2_impostors_test.gd`.
Prochaine étape : script Blender et cuisson des atlas.
