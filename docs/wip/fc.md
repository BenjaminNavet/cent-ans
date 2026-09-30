# FC — FPS carte et décor campagne (orchestration)

Spec : `docs/superpowers/specs/2026-09-30-fc-fps-et-decor-campagne-design.md`. Branche `feat/fc`,
worktree `../game_project-fc`. Mandat : autonomie. Mesures seulement sur machine calme.

## Lots
- [ ] FC0 base (attend la fin de SR3b/SR5 et de toute compilation)
- [x] FC1 ombres maquettes/moulins par préréglage
- [x] FC2 arbres imposteurs
- [x] FC3 herbe et broussailles proches
- [x] FC5 arbres proches en cartes, herbe visible (hors plan initial)
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
Tests : `fc2_impostors_test.gd` OK (headless et fenêtré, hiver compris : shaders compilés),
smoke, sz4b, fc1 OK. Triangles des arbres lointains (Orléans, d = 150) : 407 000 → 41 600
(≈ 19,6 → 2 par arbre), même nombre de MultiMesh. Terminé ; jugement visuel à FC4.

## FC5 — arbres proches semi-réalistes et herbe visible (agent FC2)
État : fait (commits `wip: FC5 …`). Mesures `game/tests/fc5_probe.gd` (Orléans 2122,3362, 720p) :
- d = 25 : 7,73 M → 8,65 M primitives (+0,92 M, plafond +2 M), 842 → 845 appels ;
- d = 15 : 9,03 M ; d = 150 : 5,20 M → 4,39 M (imposteurs entre 45 et 170 au lieu des 90 tri).
Tests : fc2 (avec repli `--no-fc5`), fc3 (tolérance de bord de cellule), fc1, smoke, sz4b OK.
Arbres : variante « mid » dans `campaign_trees.glb` (`<essence>_mid_crown/_trunk`, cartes UV :
chêne 248, hêtre 232, sapin 144 tri), `VegetationMeshes.essence_mid`, shader
`foliage_cards.gdshader` (FOLIAGE_CARDS dans `foliage.gdshaderinc` : atlas
`campaign_leaf_cards.png` de `build_leaf_cards.py`, découpe alpha, normale arrondie des deux côtés,
hiver ajouré). `vegetation.gd` : 3 niveaux (cartes < `near_distance` 45, imposteurs avec ombres
jusqu'à `detail_distance`, imposteurs au-delà) ; `ForestDetail` : cartes à < 0,4 × d du point
visé, imposteurs ailleurs (au lieu des boules de 20 tri). `--no-fc5` = comportement FC2.
Imposteurs/cartes : niveau de mipmap borné (sinon carrés pleins pour les arbres de 3 px).
Herbe (FC3) : touffes texturées (`campaign_grass_tuft.png`, copie mipmappée de grass_blades),
0,22 / 0,42 de haut, réduction en `prop_scale`^0,5 (visible jusqu'à d ≈ 10), teintes de la prairie
du terrain, 4 000 candidats/cellule. Pose sur le fil principal ≈ 16 ms/cellule (FC3 : 7-12 ms).
Captures : `~/dev/cent-ans-raw/fc5/` (avant/après `fc5_before_after_25.jpg`, `final_15/25`).
Ouvert : herbe encore discrète à d = 15 (lue comme petites touffes/buissons à d = 25) ; pose des
cellules d'herbe à sortir du fil principal ; bord net entre cartes et imposteurs à 45.

## FC6 — herbe sans à-coup, transition cartes/imposteurs, herbe à d 12-20 (agent FC2)
État : démarré. Plan : hauteurs des touffes calculées dans le `WorkerThreadPool` (instantané
`ReliefQuadtree.surface_snapshot`), fil principal = MultiMesh seul (≤ 1 cellule/image, ≤ 1 ms) ;
fondu tramé cartes ↔ imposteurs sur ~8 u autour de `near_distance` ; herbe un peu plus visible
à d 12-20, inchangée à d ≥ 30.
