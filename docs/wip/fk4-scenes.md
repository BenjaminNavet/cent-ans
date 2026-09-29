# FK4 — scènes de province (rendu GDScript)

Branche `feat/fk4-scenes` (base `integration/fk`, 299233c8c). Spec
`docs/design/2026-09-29-carte-vivante-folk.md` §§ 2.2, 3.3, 5-7 ; suivi `docs/wip/fk.md`.

## État
- [x] Réglages alignés : `data/rules/map_scenes.json` = source unique (schéma, `MapSceneRules`,
  pont `get_map_scene_rules` complet) ; clés du rendu ajoutées (`figure_height`,
  `road_folk_per_unit`, `*_probability`, `guard_value`, `scene_figures_min/max`) ;
  `carts_per_value` → `carts_per_trade_value` (valeur du rendu, 0,02) ; `peasants_per_thousand`
  supprimé (inutilisé : la routine se règle par la population relative). pytest : toute clé lue
  par `life_folk/*.gd` est dans le schéma et le fichier.
- [x] `folk_scenes.gd` (`FolkScenes`, 10 types), branchement `CampaignLife` (`--scene=`,
  `--folk-off=scenes`), enregistré avant les marchands.
- [x] Peste : cheminées coupées (`LifeEffects.quiet_settlements`) ; famine : champs vides
  (`FolkRoutine.idle_provinces`) ; fumées de bûcher et d'émeute (`LifeEffects.scene_fires`).
- [x] `FolkModels` : accessoires de scène (dead_cart, market_stall, pyre, scaffold, stone_cart,
  procession_cross/banner, flood_water en maquette), rôles `recruit`, `monk`, activités
  `procession` (0,6 m/s), `drill`.
- [x] Tests : `fk_folk_test.gd` OK (chaque type forcé, intensité, loin vide, repli chef-lieu,
  province sans colonie ignorée, disette 96 → 2 travailleurs aux champs) ; smoke OK (89 scènes
  résolues en partie réelle) ; pytest OK (1275).

## Prochaine étape
Lot terminé (cargo test : 0 échec). Fusion dans `integration/fk` par l'orchestrateur. Vérification visuelle (FK6) : orientation des étals et de
l'échafaudage, taille des nappes de crue, fumées de scène (taille des incendies CV1).

## FK6 — correctifs (branche `integration/fk`)
Diagnostic chiffré (sonde `FK_DEBUG=1` : `FolkPool._debug_dump`, Paris d=12, peste forcée) :
- **Coût** : `place_ms` 211-485 ms dont `create_ms` 482 ms ; la création du premier groupe
  d'accessoire (`dead_cart`) coûte 57-500 ms, dont 51+ ms dans le premier `surface_get_arrays`
  (lecture des sommets d'un glb : attente du fil de rendu). Placement pur : ≈ 2-3 ms.
  → préchauffage étalé (`FolkPool._warm_queue`, ≥ 1 élément/image, budget 4 ms/image) avant tout
  placement ; échec de modèle mémorisé (`_missing`) ; points de fleuve (crue) calculés au
  `refresh` (25 ms au premier placement sinon).
- **Invisibilité** : échelle. À d=12, `MapPropScale.exaggeration` ≈ 3,3 → figurine 0,013 unité
  = 1,0-1,5 px à l'écran (instances bien posées, hauteur = `surface_height_at`, AABB correcte).
  → `figure_min_view_fraction` (0,018, `map_scenes.json` + schéma + `MapSceneRules`) : hauteur
  ≥ 0,018 × min(d, 28) ; raccord continu avec `figure_height` (0,5) à d ≥ 28. À d=12 : 0,21
  unité, 14-19 px mesurés.
- **Scène dans Paris** : ancre au rayon de maquette L1 (`core_radius` 6) + 6 m, soit à 6,04 du
  centre, dans l'emprise L1 (`zone_radius` 6,8) et la ville 1:1. → `FolkPool.exclusions`
  (cercles `landmark_zones` + `landmark_cities.zone_of`) : aucune instance dedans ;
  `FolkScenes._footprint` = emprise de ville emblématique couvrant la colonie.

État : code fait, `fk_folk_test` vert (placement ≤ 8 ms par scène, aucune instance dans Paris,
lisibilité). Prochaine étape : dylib reconstruite, `fk5_incidents_test`, `smoke`, mesures.
