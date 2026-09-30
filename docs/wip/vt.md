# VT — villes 1:1 à toutes les hauteurs (vrai territoire)

Demande du joueur (30/09) : la ville « en très rapproché » à toutes les hauteurs de la vue 3D
(sauf parchemin), pour toutes les villes, sans maquette agrandie. ADR 0138.
Worktree `../game_project-vt`, branche `feat/vt`. Coût cloud : 0 $.

## Lots
Vague 1 :
- A (Sonnet) [x] : grille `ground_m` dans `tools/.../geo/towns.py`, schéma, test, nouvelle exécution de `geo towns`.
- B : `game/scripts/map/town_far_builder.gd` (fonctions pures F1/F2/v2) + `tf_far_mesh_test.gd`.
- C : `game/shaders/town_far.gdshader`, `roofscape.gdshaderinc` extrait de `town_building.gdshader`, masque d'enfoncement.
- D : retraits dans `settlement_layer.gd` (maquettes, SZ4b, loupe), étiquettes au sol, clic et anneau sur l'emprise.
Vague 2 : E `town_far_layer.gd` (tuiles, masque) ; F (Sonnet) `max_rig_distance` TownLayer + nettoyage LandmarkCityLayer ; G (Sonnet) consommateurs `model_*`, zones caméra/armées, MapPropScale, hameaux 1:1 ; H (Sonnet) tests.
Vague 3 : I bancs (d = 1100, 150, 30), captures (≤ 6), docs `godot-map.md`.

## État
- [x] Plan, ADR 0138, cette note.
- [ ] Vague 1
- [x] G : consommateurs `model_*` recâblés (effets, rivières, foule, CV1), `MapPropScale` sans `settlement_*`, hameaux 1:1, `ModelLibrary.settlement_model` et `zoom_tiers.model_shadow_distance` retirés.
- [x] D : maquettes, SZ4b, DC4/DC6c, ombres retirés de `settlement_layer.gd` ; étiquettes, clic et anneau sur l'emprise réelle (`docs/wip/vt-d.md`).
  - [x] H : tests retirés (dc4, dc6c), adaptés (fc1, sz4b) ; résultats dans le rapport VT-H.
  - [x] C : `town_far.gdshader`, `roofscape.gdshaderinc`, `TownFarMask`, `tf_far_shader_test` (1e928affa).
- [x] B : `TownFarBuilder` (F1 294 tri/ville, max 706 ; F2 50,5 ; v2 max 3 613 ; 1,4 s un fil), `tf_far_mesh_test` (`docs/wip/vt-b-far-builder.md`).
- [x] E : `TownFarLayer` (`town_far_layer.gd`), créé dans `SettlementLayer._setup_towns`, mis à jour
  après les calques 1:1 dans `SettlementLayer.update_view` (`settle/townfar`, `townfar/build|mask|view`).
  2 141 villes ; F1 757 tuiles (128 u), 638 k tri ; F2 122 tuiles (512 u), 118 k tri ; 1,23 M sommets,
  ≈ 41 Mo estimés. Génération 0,9-1,3 s réelles sur 4 fils (1 fil : 2,1 s ; au-delà de 4, pas plus
  rapide), image principale max ≈ 1,3 ms (budget 2 ms, tuile ≤ 0,9 ms). F1 jusqu'à 300 (bas 150,
  moyen 240, ultra 360), F2 de là à `model_range`, fondus croisés 10 % ; ombres F1 sous rig 60
  (haut, ultra). Masque = union des `built_ids()` (sur changement de `version`), enfoncement sous
  `block_range` × qualité × 0,95. Test `tf_far_layer_test.gd`.
- [x] F : `max_rig_distance` (16, hystérésis 10 %) sur `TownLayer` et `LandmarkCityLayer`, maquettes/fondu retirés, `built_ids()` sur les deux calques.

## Prochaine étape
Lot I : bancs (d = 1100, 150, 30), captures (≤ 6), docs `godot-map.md`.
À juger sur capture (E) : fondu F1/F2 à d ≈ 300 ; bande 0,86-1,1 × `block_range` où blocs et
lointain partiellement enfoncé coexistent (toits qui percent ?) ; jupe et sol des villes v2
(sol uniforme échantillonné au centre au chargement, souvent sur la heightmap car pages pas encore
chargées) ; teinte des toits F1/F2 à côté des blocs.

## Notes d'intégration
- Rivières : plus de coupure sous les villes (5484cea26), les villes 1:1 enjambent la vraie rivière.
- `smoke` OK après A-D, F-H. `at1_attack_order` échoue (l'armée assiège Bordeaux au lieu d'attaquer) : règle de simulation ; VT ne touche ni `core/` ni l'UI. Probablement antérieur, à confirmer sur main.
- Code mort laissé : `life_effects._update_overlays` + `life_overlay.gdshader` (neige/suie sur maquettes) ; `ModelLibrary.HAMLET_SCALE` encore lu par settlement_layer.
