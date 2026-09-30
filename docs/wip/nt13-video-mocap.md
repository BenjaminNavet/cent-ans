# NT13 — vidéo vers animation (mocap maison)

Branche `feat/nt13-video-mocap`. Orchestration : `docs/wip/nt.md`. Prédécesseur : NT12
(`docs/wip/nt12-mocap.md`, reciblage CMU, `--mocap-trial`).

## État : clips cuits, option `--video-trial` en place ; reste tests Godot et captures

Vidéos du joueur (hors dépôt) : `~/dev/cent-ans-mocap-src/video/IMG_6455..6458.MOV` (4K
portrait, 30 i/s) ; intermédiaires hors dépôt dans `~/dev/cent-ans-mocap-src/work/` : env
`env/.venv`, modèle `pose_landmarker_heavy.task`, poses `poses/*.npz`, rendus `render/`,
planches `planches/nt13_<clip>.png`, scripts jetables (`extract_all.sh`, `look_poses.py`,
`debug_blade.py`).

## Gestes identifiés (bâton tenu à deux mains, mains écartées de 25 à 60 cm)
- 6455 (4,5 s) : garde immobile bâton sur l'épaule droite (0-1,6 s) → `guard` (bouclé) ; puis
  montée et coupe diagonale descendante vers la gauche → `overhead` (i. 54-135, accéléré ×1,8).
- 6456 (1,2 s) : garde épaule droite qui bouge à peine, pas de pied gauche → **non utilisée**.
- 6457 (4,6 s) : garde basse à la hanche droite, fente et estoc à deux mains vers la droite de
  l'image puis retour → `thrust` (i. 57-126, ×1,4, lacet 70°).
- 6458 (2,6 s) : coupe horizontale de droite à gauche et retour → `slash` (i. 12-72, ×1,4).
- `parry`, `hit`, `death` : aucun geste filmé, clips keyframés conservés.

## Outils et licences
| Outil | Rôle | Licence |
|---|---|---|
| MediaPipe 0.10.21 (Pose Landmarker, tâche vidéo, CPU) | pose 3D (33 points monde) | Apache 2.0 |
| Modèle `pose_landmarker_heavy` (float16) | réseau de pose | Apache 2.0 (carte de modèle Google) |
| OpenCV (headless) | décodage vidéo, rotation | Apache 2.0 |
| NumPy | nettoyage | BSD-3 |
| Pillow, ffmpeg (planches seulement) | composition, extraction d'images | HPND / LGPL (outils, rien de redistribué) |
| Blender 4 (existant) | reciblage, cuisson | GPL (outil) |

Aucune dépendance SMPL / SMPL-X / AMASS / HumanML3D. MediaPipe 1.0.1 (dernière) plante sur macOS
(« Service is unavailable » dans `TensorsToDetectionsCalculator`, même en délégué CPU) : version
épinglée à 0.10.21 (métadonnées PEP 723 du script). Env dédié `uv` hors `tools/`.

## Pipeline
1. `tools/video_mocap/extract_pose.py` : applique la rotation du téléphone (métadonnée 90°,
   sinon le corps est couché et la pose se dégrade), réduit à 1920 px, Pose Landmarker heavy en
   mode vidéo → `.npz` (points monde, image, visibilité).
2. `tools/blender_scripts/video_mocap_clean.py` (numpy pur, `tools/tests/test_video_mocap_clean.py`) :
   axes Z haut ; One-Euro aller-retour (sans retard ; coupure 1,5 Hz, bêta 0,3) ; longueurs de
   segments médianes imposées par la profondeur seule ; appuis = pied lent dans l'image et pas
   plus de 7 cm au-dessus de l'autre en 3D (le critère image seul se trompait : en garde décalée
   le pied arrière est plus haut dans l'image) ; racine pilotée par les pieds posés, hanche
   posée pour que le pied le plus bas touche le sol ; rééchantillonnage 24 i/s avec accélération.
3. `tools/blender_scripts/nt13_video_trial.py` : repères bassin / buste (épine interpolée), tête
   par oreilles et nez ; paires de membres visées sur les points avec charnière coude/genou du
   plan de flexion (portée par le parent si le membre est tendu) ; poignet droit orienté pour que
   la lame suive le bâton (ligne des deux poignets, continuité d'image en image, sens par vote
   index/pouce de la main droite ; roulis pris de l'avant-bras, comblé depuis les images voisines
   quand le bâton longe l'avant-bras) ; écart au poing keyframé plafonné (flexion 65°, torsion
   110°) ; bras gauche = garde keyframée au bouclier sur le buste vidéo (comme NT12) ; IK à deux
   os des pieds posés (rampes de 2 images) ; bouclage de la garde (dérive ôtée, fondu 6 images).
   `-- render DIR` : rendus keyframé / CMU / vidéo / vidéo vue caméra ;
   `tools/video_mocap/contact_sheet.py` compose les planches avec l'image source.
4. `BattleSkinned` : `--video-trial` après `--` (prioritaire sur `--mocap-trial`), même fusion que
   NT12 (`_merge_mocap_trial(rigs, dir)`) ; `video_trial_forced` pour l'A/B dans un processus.

Commandes (depuis la racine du dépôt) :
```
~/dev/cent-ans-mocap-src/work/env/.venv/bin/python tools/video_mocap/extract_pose.py VIDEO.MOV MODEL.task OUT.npz
blender -b --factory-startup --python tools/blender_scripts/nt13_video_trial.py
blender -b --factory-startup --python tools/blender_scripts/nt13_video_trial.py -- render ~/dev/cent-ans-mocap-src/work/render
uv run --project tools python tools/video_mocap/contact_sheet.py ~/dev/cent-ans-mocap-src/work/render ~/dev/cent-ans-mocap-src/video ~/dev/cent-ans-mocap-src/work/planches
```

## Temps de traitement (M-series, CPU)
Extraction : 6456 (1,2 s) 3,6 s ; 6455 (4,5 s) 12,7 s ; 6457 (4,6 s) 12,3 s ; 6458 (2,6 s) 6,8 s
(≈ 2,7 × la durée). Nettoyage + reciblage : < 0,1 s par clip ; cuisson Blender 2 s en tout ;
rendus de planche 5 s.

## Défauts par clip (mesures du manifeste `video_trial/manifest.json`, planches)
- `guard` : propre (tremblement 2,9°/image², pieds posés tout du long, glissement 0) ; poignet
  au plafond de torsion (le bâton sur l'épaule est loin de la tenue d'épée keyframée).
- `overhead` : pieds tenus (0 cm) ; tremblement 5,4 ; épée fidèle au bâton (flexion ≤ 73°).
- `slash` : le plus bruité (tremblement 12,7, pointes de poignet 31°/image : geste rapide, mains
  rapprochées, bâton le long de l'avant-bras en début de clip).
- `thrust` : fente bien rendue (vue caméra conforme à la vidéo) ; pieds : 18,5 cm de glissement
  pendant le pas de fente (non posés, près du sol) ; poignet au plafond en garde basse.
- Tous : profondeur MediaPipe faible (corrigée par longueurs de segments, signe parfois
  douteux en face caméra) ; lacet choisi à la main (45°, 70° pour l'estoc) car le joueur frappe
  vers la droite de l'image ; geste à deux mains → bras gauche keyframé sur figurine
  épée-bouclier ; poignets MediaPipe grossiers (3 points de main) ; aucune vraie torsion de lame.

## Tests
- pytest : `tools/tests/test_video_mocap_clean.py` (15).
- Godot : `game/tests/nt13_video_test.gd` (défaut, forcé 0/1, vidéo prioritaire sur CMU).

## Prochaine étape
Tests Godot (smoke, nt7_anim_test, an1b_clips_test, nt12_mocap_test, nt13_video_test avec et sans
`--video-trial`), captures `nt13_video_shot.gd`, verdict.
