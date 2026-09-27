# CV3-0 — Défauts de la carte de campagne

Spec : annexe A de `docs/design/2026-09-27-campagne-vivante.md` (section « Défauts de la carte
de campagne relevés sur les captures »).

## État (fait / reste)
1. Caméra trop proche au max — fait : `campaign_camera.gd` max_distance 1500->2600,
   pitch_far_distance 2600 (recalculé dans `setup`), `strategic_view.gd` bande parchemin
   1180-1440 -> 2050-2500 (même ratio). Tests figés à 1500 mis à jour (zg4_camera_test,
   pb1_bench, v4_map_bench, cm2_parchment_weather_test). far_max (close_camera.tres, 6000)
   vérifié suffisant (déjà saturé avant ce lot). Capture à faire pour valider Bordeaux/Marseille
   visibles depuis Paris.
2. Brouillard trop sombre — fait : `terrain.gdshader` fog_veil éclairci (0.90/0.88/0.82),
   fog_desaturation 0.78, fog_veil_amount 0.36, fog_cloud_amount 0.32, mist*0.86 (était 0.68).
3. Nuages sur presque toutes les provinces — fait : nouvelle coupure d'intensité en vue large
   (`weather_wide_intensity_cut`, campaign_weather.gdshaderinc + campaign_weather_view.gd),
   cloud_max_alpha 0.6->0.45. L'orage reste toujours couvert (n'est pas coupé), pluie/neige
   légère disparaît en vue large.
4. Pluie en aiguilles blanches — fait : `precipitation.tres` gouttes plus courtes/fines
   (largeur 0.35->0.28 m, longueur 2.5->1.3 m), `campaign_weather_view.gd` couleur plus
   discrète et moins opaque (0.62/0.67/0.76, alpha 0.24, était 0.72/0.76/0.82/0.38),
   particles_far 320->220 (moins souvent visible de près).
5. Relief peu lisible Paris-Orléans — fait : `relief_exaggeration.tres` gain_far 0.3->0.42
   (l'écrasement des montagnes, indépendant, absorbe l'effet sur les Alpes),
   `terrain.gdshader` shading_relief 1.8->2.1.
6. Chemin de déplacement peu contrasté — fait : `terrain_line.gdshader` liseré sombre optionnel
   (`casing_width`/`casing_color`, 0 par défaut = fleuves/côte inchangés), activé dans
   `army_movement_path.gd` (0.32, presque noir) ; largeur mini 0.3->0.55.
7. Étiquettes qui se chevauchent — fait : `army_markers.gd` expose `screen_label_rects()`
   (plaques d'armée affichées), `settlement_layer.gd` réserve ces rectangles dans son
   `_placer` avant de placer marqueurs/noms de colonies (`label_obstacles`, câblé dans
   `campaign_map.gd`). Test croisé ajouté dans `da7d_overlap_test.gd` (armée/colonie, 0
   chevauchement).
8. « Aucune recherche » deux fois — fait : alerte `research_idle` retirée de
   `alerts.gd` (la barre du haut, `HudController.set_research_progress`, l'affiche déjà en
   permanence). `next_hint.gd` / `end_turn_cluster.gd` gardent le type au cas où (inoffensif,
   plus jamais émis par `collect`).
9. Panneau de faction plus haut que l'écran — fait : `faction_panel.gd` `_fit_height` soustrait
   les marges du stylebox (`get_theme_stylebox("panel")`) et une réserve HUD bas
   (`bottom_reserved_px`, posée par `map_ui.gd::layout_hud` via `update_bottom_reserve` =
   `end_turn_cluster.bell_height()` + marge, même calcul que `_dock_panel`) ; recalcule aussi
   au resize du viewport (`size_changed`), pas seulement à l'ouverture/au contenu.
10. Lettrine qui ne réserve pas sa place — à faire

## Prochaine étape
Traiter les points dans l'ordre, un commit `wip:` par point (ou groupe de points proches),
captures avant/après dans `docs/img/cv3/` pour les points 1-7.
