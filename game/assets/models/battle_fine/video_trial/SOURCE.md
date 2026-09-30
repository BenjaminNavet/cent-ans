# Essai vidéo (lots NT13, NT14)

Clips cuits (`human.bones.bin`, `CAB1`, os du rig fin `human`) par
`tools/blender_scripts/nt13_video_trial.py`, lus seulement avec `--video-trial` après `--`.
Clips substitués : `guard`, `overhead`, `parry`, `thrust` (second tournage, NT14 : épée à une
main, bouclier suivi par le disque rouge) et `slash` (premier tournage, NT13 : bâton à deux
mains). Mesures de chaque clip dans `manifest.json` (`clip_sources`).

Source : vidéos de gestes de combat (bâton, disque) filmées par le joueur au téléphone ;
propriété du joueur, cuisson versionnée avec son accord. Les vidéos et toutes les données
intermédiaires (images, poses brutes, suivi du disque) restent hors dépôt
(`~/dev/cent-ans-mocap-src/`).

Outils : MediaPipe Pose Landmarker « heavy » (Apache 2.0, modèle Apache 2.0), OpenCV
(Apache 2.0), NumPy (BSD-3), Blender (GPL, outil seulement). Aucune dépendance SMPL, SMPL-X,
AMASS ni HumanML3D. Détails : `docs/wip/nt13-video-mocap.md`, `docs/wip/nt14-video-set2.md`.
