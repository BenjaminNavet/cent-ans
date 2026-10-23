# solari-soldats (23/10) : ombres des soldats sous Solari

État : code terminé, vérifié SANS client (clippy défaut + `--features dev`, tests unitaires). Aucune vérification GPU
faite (règle du lot) : voir « Lancements à faire par la session principale ».

## Conception
Solari remplace les cascades du soleil par des rayons ; les soldats (instanciés, skinnés par texture d'os, sans entité)
n'étaient pas dans la TLAS donc sans ombre. Ici : PROXIES de rayons.
- Code : `client/src/battle/scene/solari_soldiers.rs` (système `sync_soldier_proxies`, après `update_regiments`).
- Un maillage boîte de 12 triangles (24 sommets) par type : fantassin (0,55 x 1,8 x 0,4 m) et monté (0,9 x 2,4 x 2,2 m),
  donc 2 BLAS partagés ; un seul `StandardMaterial` sombre. Boîte posée sur y = 0 (le pied du soldat).
- Une entité proxy par soldat : `RaytracingMesh3d` + `MeshMaterial3d<StandardMaterial>` + `Transform` (racine, sans
  `Mesh3d` : invisible au raster). Transform = position + cap (`Quat::from_rotation_y(yaw)`, même convention que
  `rotate_y` de skinned.wesl) lus dans `SkinnedBatch.records` (octets 0..16 de chaque enregistrement de 48 octets).
- Sources : les `SkinnedBatch` portant le nouveau marqueur `ProxyCaster { mounted }` (`units/crowd.rs`, posé dans
  `regiments.rs` sur les batches principal et imposteur ; les cadavres n'en ont pas : pas de proxy debout sur un mort).
  On prend `batch.count` enregistrements (= ce que le raster dessine : tous pour le batch principal, les visibles pour
  l'imposteur ; le batch principal à 0 en LOD imposteur est ignoré).
- Ordre/budget : visibles (préfixe `SkinnedColourCount`) de tous les régiments d'abord, puis « ombre seule » ; coupe à
  `soldier_proxy_budget`.
- Pool : un par type, croît paresseusement jusqu'au budget, ne rétrécit jamais ; proxies inutilisés garés à y = -80 m
  (sous le sol de substitution abaissé de 2 m : n'ombrent rien) ; une pose inchangée n'est pas réécrite (le TLAS ne copie
  que les `GlobalTransform` changés). Aucune allocation par image (tampons réutilisés).
- Détection de changement : signature (batch, `version`, count, visible) de ~2 batches par régiment ; si identique, le
  système sort sans rien lire. Aucune requête sur les proxies ni les 13 000 soldats (la requête `Transform` n'est
  interrogée que pour les proxies réellement modifiés).

## Réglages (`data/fx/solari.json`, schéma mis à jour)
`soldiers_in_tlas` (true), `soldier_proxy_budget` (4000), `soldier_foot_size_m`, `soldier_mounted_size_m`.
`enabled` reste false. `soldiers_in_tlas:false` ou budget 0 : aucun proxy (pool garé, aucune entité créée).
Dump d'état : section `solari_soldiers` (`proxies_foot|mounted`, `active_*`, `updates`, `skipped_unchanged`,
`last_written`, `last_wanted`, `dropped_by_budget`).

## Coût attendu (à confirmer au banc)
- Entités : au plus `soldier_proxy_budget` (4000) + 0 en dehors de Solari. TLAS : +4000 instances (le décor en compte ~25 000),
  2 BLAS de 12 triangles.
- CPU par image : en cas de changement (quasi toujours en mêlée) lecture des enregistrements (~4000 à 13 000 lectures
  de 16 octets, selon visibles + ombres seules avant coupe du budget) + comparaison/écriture de <= 4000 `Transform` ;
  propagation de transforms de <= 4000 racines (chemin rapide) ; copie des transforms changés vers le GPU par Solari
  (<= 4000 x 64 octets). À l'arrêt : O(batches), négligeable.
- Cavaliers : inclus (boîte distincte via `ProxyCaster.mounted`).

## Écarts, limites, risques
- Les ombres sont celles d'une boîte, pas de la silhouette animée (pas de bras, lance, jambes) ; bannières et fanions : non.
- Axe longitudinal du cheval supposé +/-Z local (profondeur 2,2 m sur z) : à vérifier sur une capture ; sinon permuter
  les composantes de `soldier_mounted_size_m`.
- Soldats au-delà du budget : les derniers régiments (ordre d'entité) n'ont pas d'ombre en mêlée dense (le nombre est dans
  `dropped_by_budget`) ; une répartition par distance à la caméra n'est pas faite.
- Une boîte opaque ombre aussi par GI/rebonds : le sol sous la troupe sera plus sombre qu'avec une silhouette fine.
- Pas de valeur de rendu côté pixel (soldats toujours rasterisés, éclairés par le raster sans cascades) : l'ombre RT
  tombe sur le sol/décor déférés mais pas SUR les soldats (ils ne sont pas lus par le G-buffer Solari).
- Pas de proxy pour les cadavres, armes jetées, machines de siège, `siege/crew.rs`.

## Lancements de vérification à faire par la session principale (aucun n'a été fait ici)
Préparation : `ln -s /Users/jean_hubert/dev/cent-ans-raw/compose/pyramid data/map/pyramid` et `rsync -a --ignore-existing
/Users/jean_hubert/dev/game_project/game/assets/ game/assets/`. Mettre `"enabled": true` dans `data/fx/solari.json`
pour les essais, REMETTRE `false` avant commit.
1. Démarrage + dump (texte d'abord), debug :
   `CARGO_INCREMENTAL=0 CENT_ANS_MUTE=1 CENT_ANS_FEATURES=dev,solari CENT_ANS_PROFILE=debug client/run_bg.sh --state Battle --frames 400 --offscreen --dump-state out/solari-soldats.json`
   puis lire la section `solari_soldiers` du dump (proxies_foot > 0, last_wanted ~ soldiers_drawn+shadow_only plafonné à 4000,
   `skipped_unchanged` > 0 en mise en place, pas de panique, ligne `solari: ray-query GPU found`).
2. Capture de la mise en place avec soldats (1 capture, 640 px, ombres au sol sous les troupes ; comparer avec
   `soldiers_in_tlas:false`) :
   `CARGO_INCREMENTAL=0 CENT_ANS_MUTE=1 CENT_ANS_FEATURES=dev,solari CENT_ANS_PROFILE=debug client/run_bg.sh --state Battle --frames 500 --offscreen --shot out/shots/solari-soldats-on.png`
   puis même commande avec `"soldiers_in_tlas": false` et `--shot out/shots/solari-soldats-off.png`. Juger : ombres de troupes
   présentes/cohérentes avec le soleil, boîtes qui flottent ou traversent le sol, ombres de cavaliers allongées dans le bon sens.
3. Banc (release, un seul à la fois, machine au calme) :
   `tools/bevy_bench/bench.sh solari-soldats-on --bench-battle --offscreen` avec `enabled:true, soldiers_in_tlas:true` ;
   `tools/bevy_bench/bench.sh solari-soldats-off --bench-battle --offscreen` avec `enabled:true, soldiers_in_tlas:false` ;
   `tools/bevy_bench/bench.sh raster --bench-battle --offscreen` avec `enabled:false`. Référence connue (polish) : mêlée 52 fps
   Solari sans soldats contre 41 raster. Comparer les fps, p95/p99 et le coût CPU (temps de `sync_soldier_proxies` si le banc
   le sort) ; si la chute est forte, essayer `soldier_proxy_budget` 2000 / 1000.
