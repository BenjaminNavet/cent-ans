# bm-showfix : pourquoi les captures de référence bougent peu

## État (16/10)
- Constat : le « 2/255 » de la REPRISE comparait les captures de 13 h 30 à celles de 15 h 10 ; la plupart des
  lots visuels étaient déjà fusionnés avant 13 h 30. De 08 h 24 (réf.) à 15 h 10, l'écart Bevy est de 40 à 74/255
  par vue : les lots ont bien atteint les scènes. Restent des défauts réels, traités ici un par un.
- Chargements : bataille 300 vs 900 images = 1,1/255 ; campagne 600 vs 1500 = 6/255 (animation) ; dumps identiques
  (végétation prête, colormap, paquets) → l'attente de 600/300 images suffit.
- Rivières invisibles à d=60 : PROUVÉ. Les rubans sont cuits au premier `RebuildWater` (terrain prêt, pages fines
  pas encore résidentes) sur la carte de hauteurs mondiale ; le terrain proche affiche ensuite les pages fines.
  Sonde `CENT_ANS_SEAT_PROBE=1651,2733` (Paris, 81×81 pts) : page − carte = −51 … +54 m. Avec
  `CENT_ANS_RIVER_LIFT_DEBUG=1.0` la Seine réapparaît → elle est enterrée sous le relief.
- Toits des tours de siège plats : cône de hauteur fixe 5 m, rayon ×1,25, tuile ; Godot : hauteur r×1,55, ardoise
  sombre, une tour sur deux (autres crénelées) — défaut de portage dans `battle/siege/works.rs`.

- Suite (16/10) : le re-calage sur les pages fines (`water/reseat.rs`, max des niveaux résidents) ne déplace les
  sommets que de 0,3 m : les rubans étaient déjà sur les bonnes hauteurs CPU. Densifier les rivières (0,5 px) et
  creuser le lit dans le terrain proche (`river_carve`, port de `river_banks.gdshaderinc`) n'en font réapparaître
  qu'un peu (3 000 px bleus contre 19 000 avec +0,15 de levée). Hypothèse en cours : un autre maillage (zones
  humides, surcouches) couvre la rivière ; crochet `CENT_ANS_HIDE=<sous-chaîne>` (agent) pour le vérifier.
- Tours de siège : cône d'ardoise r×1,55 une tour sur deux, merlons sinon, corbeau (`works.rs`), dump `tower_roofs`.

## Causes prouvées (reprise 16/10 après-midi)
- **Rivières/routes enterrées** : le uniform `ReliefParams` n'atteignait jamais les matériaux drapés
  (dump : `relief_scale` = [0,0,0,0] sur les deux matériaux rivière, alors que `relief_gpu.scale` =
  [0.006,0,1,1]) ; `relief_display_height` inactif renvoie `h × scale.x` = 0 → rubans à y = 0 + lift,
  sous le sol (Paris ≈ 30 m = 0,18 u, le relèvement de debug requis était 0,15). `sync_drape` ne
  copiait que si `ReliefGpu` changeait ou si le NOMBRE de matériaux changeait (matériaux remplacés à
  nombre égal). Correctif : vérification à chaque image (`water/drape.rs`), commit 666d2a240. Mesure
  d=60 : pixels bleus 85 030 → 104 360 (= relèvement de debug 0,15 : 104 218).
- Complément conservé : reseat sur le treillis des patchs affichés (+ niveau parent), creusement du lit.
  Retirés : pente de berge dans `normal.y` (sans effet).
- **Arbres pâles** : le terrain est multiplié par `ambient.a` = `TERRAIN_EXPOSURE` 0,78 puis étalonné
  (`campaign_grade`), la végétation ni l'un ni l'autre (Godot étalonnait toute l'image). Correctif :
  exposition + `foliage_grade` dans `billboard.wesl` (grade/lift/highlights copiés via `VegLight`).

## État à la pause (16/10)
- Commits : 96a1fce94, 5c46a51f0, puis ce wip (crochet `CENT_ANS_HIDE` rendu collant, réappliqué à chaque image :
  le LOD remettait `Visibility::Visible` à chaque reconstruction, le test « masquer TerrainNode » était donc
  invalide ; journal `AGENT_UNNAMED_MESHES` avec `CENT_ANS_HIDE=?`). Ce dernier changement n'est PAS compilé
  (build arrêté sur demande de pause).
- Cause des rivières enterrées (10-25 m sous le sol affiché à d=60) toujours non trouvée. Écartés : chargement,
  page fine vs heightmap côté CPU (reseat ≤ 0,39 m), densification, voiles/provinces/nuages. Formule d'exagération
  identique (`displayed_y` terrain = `relief_display_height_at_fields` rivière) ; restent à comparer : les
  `ReliefParams` passés au matériau rivière vs terrain (scale.x/y/z/w), les champs lus en texture
  (`relief_fields`, centres de texels) vs `fields_at` CPU (bilerp), `fb.y`/lift D, le géomorphe, la jupe `slope.z`.
- Toits des tours de siège corrigés dans `works.rs`, non vérifiés par capture/dump.
- Pas encore de recaptures `tools/compare_views.sh --side bevy` ni d'écarts par vue.

## Prochaine étape
1. `cargo build --release` (≈ 10 min, arrière-plan) puis `sf/run.sh` avec `CENT_ANS_HIDE=TerrainNode` à d=60 :
   confirmer que le terrain recouvre la rivière.
2. Dumper/comparer `ReliefParams` rivière vs terrain et la hauteur affichée au même point (CPU `display_height_fields`
   vs shader) ; corriger la source divergente. À défaut : relever `lift_base` proche (~0,1-0,15) et documenter.
3. Vérifier `battle_siege.tower_roofs` ; arbres pâles (pas de grading sur `billboard.wesl`) ; cadrage d=60.
4. Recaptures `--side bevy`, écarts par vue ; fmt, `clippy --features dev -D warnings`, `cargo test` ; commit final.
5. Envisager de retirer reseat/dense (sans effet mesuré) si la vraie cause est trouvée.
