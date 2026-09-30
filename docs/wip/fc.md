# FC — FPS carte et décor campagne (orchestration)

Spec : `docs/superpowers/specs/2026-09-30-fc-fps-et-decor-campagne-design.md`. Branche `feat/fc`,
worktree `../game_project-fc`. Mandat : autonomie. Mesures seulement sur machine calme.

## Lots
- [ ] FC0 base (attend la fin de SR3b/SR5 et de toute compilation)
- [x] FC1 ombres maquettes/moulins par préréglage
- [ ] FC2 arbres imposteurs
- [x] FC3 herbe et broussailles proches
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
- Tests : smoke, cv1, settlements_render, c5, pb1_veg_job OK ; sz4b_colonies_forests échoue
  (maquettes non réduites) : worktree sans `data/map/pyramid` (ignoré par git), sans lien avec FC1/FC3.
- Points ouverts : réglage visuel (taille 0,14/0,26 u, couleurs) à juger en capture (FC4) ;
  +~10-17 appels de dessin sous d = 40 (objectif FC4 « aucune hausse » mesuré au zoom panoramique).

## FC2 — arbres imposteurs (agent FC2)
État : atlas cuits et commités (`game/assets/textures/vegetation/campaign_impostors_{albedo,normal}.png`,
2048×768 = 8 vues × chêne/hêtre/sapin à 256², VRAM BPTC + mipmaps), générateur
`tools/blender_scripts/campaign_tree_impostors.py` (Cycles CPU ~1 min, arbres sources de 5 400 /
4 700 / 780 triangles en cartes de feuilles, cuisson à 35° de tangage, émission pure : albédo,
normale en repère de vue, occlusion en alpha de la normale). Shader
`game/shaders/campaign_tree_impostor.gdshader` (vue par azimut local + tramage de Bayer entre deux
vues, découpage alpha, hiver ajouré dans le même shader) ; uniformes et fonctions du feuillage
extraits dans `foliage_common.gdshaderinc` (inclus par `foliage.gdshaderinc`, comportement
inchangé). `vegetation.gd` : tuiles au-delà de `detail_distance` → `VegetationMeshes.impostor()`
(2 triangles) + matériau imposteur pour chêne/hêtre/sapin, haies en maillage bas ;
`--no-fc2` = maillages bas ; `lod_census()` pour tests/bancs. Contact : `docs/img/fc/fc2_impostors.jpg`.
Non fait : ForestDetail (garde le maillage bas : changer de maillage y coûte ~1 ms/MultiMesh) ;
portée 700 → 1000 (pas de clé de préréglage FC1 ; `max_camera_distance` reste 700).
Prochaine étape : import Godot, `fc2_impostors_test.gd`, tests végétation, smoke.
