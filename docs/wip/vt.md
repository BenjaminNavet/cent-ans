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

## Lot I — TERMINÉ (30/09)
- [x] Banc : `--bench-pan-only`, `startup_total_ms` et `town_far` dans le rapport (`map_bench.gd`).
- [x] Bancs base `main` 94cfa8103 / VT à d = 1100, 150, 30, 3 passes alternées (charge 6-25) :
  appels p50 475 → 201 / 3 284 → 751 / 1 088 → 517 ; primitives 1,40 → 1,30 / 5,24 → 3,55 /
  9,19 → 8,60 M ; i/s égaux ; chargement 6,4-7,4 → 5,3-6,0 s ; fil principal ≤ 2,2 ms (un pic
  isolé de 17 ms, une tuile, machine chargée). Tous les critères du plan tenus ; aucun réglage changé.
  Worktree de base supprimé.
- [x] Captures `game/tests/vt_shots.gd` → `docs/img/vt/` (locales, `docs/img/` est ignoré par git).
- [x] Docs : `godot-map.md` (section « Villes 1:1 à toutes les hauteurs », notes sur les anciennes
  sections maquettes/SZ4b, option de banc), ADR 0138 acceptée avec les mesures.
- [x] Tests finaux OK : smoke, tf_far_mesh, tf_far_shader, zg6_towns, vh4_landmarks,
  settlements_render, c5_settlements_ui, sz4_prop_scale. `tf_far_layer_test` instable sous charge
  (15-20) : seuil « image principale ≤ 8 ms » dépassé 3 fois sur 4 (10-22 ms, parfois avec une tuile
  ≤ 1,4 ms : fil principal préempté), passe à 2,05 ms quand la machine le laisse. À repasser sur
  machine calme ; si l'écart persiste, soupçonner la contention du `_mutex` avec les fils.

## Points à juger par le joueur (captures 640 px, `docs/img/vt/`)
- De haut (d ≥ 150), les villes sont quasi invisibles : Paris ≈ quelques pixels, Amiens caché sous
  son écu. Voulu par l'ADR (repérage par nom et écu), mais à confirmer en partie réelle.
- d = 15 au-dessus de Paris (`vt_paris_d15_detail.jpg`) : ville lisible sur le relief, Seine qui la
  traverse (coupure retirée), finage sans arbres. Toits plats bruns à bords clairs (jupe éclairée ?) :
  aspect « découpé » à juger ; pas de toit flottant vu.
- Figurants FK (bêtes, charrettes, gens) gardés lisibles (`figure_min_view_fraction`) : à d = 15 ils
  sont plus gros que les îlots de Paris. Incohérence d'échelle la plus visible ; relève de FK
  (ADR 0122), non modifiée.
- Moulins et panaches de cheminée suivent encore l'exagération commune (jusqu'à ×125) : à côté
  d'une ville 1:1, un moulin fait ~2 km. L'ADR disait « cheminées réelles » : à trancher.
- Arbres grossis : le finage les écarte des villes ; au-delà, un houppier vaut un îlot à d = 15.
- Non jugeables à 640 px : fondu F1/F2 à d ≈ 300 (villes trop petites), teinte F1/F2 à côté des
  blocs, bande 0,86-1,1 × `block_range` en détail. À regarder en partie réelle.

## Prochaine étape
Chantier VT terminé sur `feat/vt` : fusion dans main par la session principale, partie pilote.

## Notes d'intégration
- Rivières : plus de coupure sous les villes (5484cea26), les villes 1:1 enjambent la vraie rivière.
- `smoke` OK après A-D, F-H. `at1_attack_order` échoue (l'armée assiège Bordeaux au lieu d'attaquer) : règle de simulation ; VT ne touche ni `core/` ni l'UI. Probablement antérieur, à confirmer sur main.
- Code mort laissé : `life_effects._update_overlays` + `life_overlay.gdshader` (neige/suie sur maquettes) ; `ModelLibrary.HAMLET_SCALE` encore lu par settlement_layer.

## Correctif 30/09 — Paris en dalles brunes (retour joueur)
- Symptôme : vue rapprochée de Paris, dalles brunes plates qui flottent ou s'enfoncent, points
  rouges (îlots 1:1 dans la bande de fondu) en avant.
- Cause : le lointain des villes v2 (`build_v2_far` : Paris, Londres, Orléans…) était posé sur un
  sol uniforme (altitude du centre) ; le relief affiché est exagéré, la jupe de 30 m ne suffit pas.
  Les quartiers étaient en plus triangulés sur leur seul contour (triangles de 2 km).
- Correctif (`fix/vt-v2-ground`) : instantané du relief sur l'emprise (`TownFarLayer._heights_at`,
  même `TownPlan.Heights` que la ville 1:1), sol échantillonné par sommet ; nappe des quartiers
  découpée en cellules de 200 m (`DISTRICT_CELL_M`), jupe densifiée. Test ajouté dans
  `tf_far_mesh_test` (écart sommet/relief < 0,5 m).

## Ville détaillée plus haut (30/09, ADR 0144, demande du joueur)
- Portées : `max_rig_distance` 16 → 45, `block_range` 14 → 42, `stream_max` 18 → 48.
- Toits des blocs : fondu vers la teinte « masse de toits » du lointain (1,2 → 3,5 u) ; essai
  inverse (lointain en tuile des blocs) écarté, taches rouge sang de loin.
- Banc `--bench-pan-only`, 1280 × 720, qualité Haute, 2 passes alternées, machine chargée
  (charge 30-70) : d = 30 appels p50 411-414 → 436, primitives 2,77 → 2,80 M ; d = 40 appels
  466-468 → 463-481, primitives 2,81-2,85 → 2,79-2,83 M ; coût CPU du rendu 0,6-0,7 ms dans les
  deux ; i/s et pics dans le bruit (pics > 50 ms : 17 en base, 28 en nouveau, passes différentes).
  En panoramique seules 2-4 villes 1:1 ont le temps de se charger (plans 2 fils) ; à l'arrêt,
  Paris d = 35 : 7 villes construites, d = 22 : 9.
- Captures d = 35 / 22 (locales) : Paris 1:1 active, même teinte que le lointain, plus de dalles.
- À juger en partie réelle : chargement progressif en déplacement rapide ; banc sur machine calme.
