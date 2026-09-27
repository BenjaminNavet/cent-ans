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
4. Pluie en aiguilles blanches — à faire
5. Relief peu lisible Paris-Orléans — à faire
6. Chemin de déplacement peu contrasté — à faire
7. Étiquettes qui se chevauchent — à faire
8. « Aucune recherche » deux fois — à faire
9. Panneau de faction plus haut que l'écran — à faire
10. Lettrine qui ne réserve pas sa place — à faire

## Prochaine étape
Traiter les points dans l'ordre, un commit `wip:` par point (ou groupe de points proches),
captures avant/après dans `docs/img/cv3/` pour les points 1-7.
