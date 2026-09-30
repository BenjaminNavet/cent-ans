# Essai vidéo (lot NT13)

Clips cuits (`human.bones.bin`, `CAB1`, os du rig fin `human`) par
`tools/blender_scripts/nt13_video_trial.py`, lus seulement avec `--video-trial` après `--`.
Clips substitués : `guard`, `overhead`, `slash`, `thrust`.

Source : vidéos de gestes de combat (bâton) filmées par le joueur au téléphone ; propriété du
joueur, cuisson versionnée avec son accord. Les vidéos et toutes les données intermédiaires
(images, poses brutes) restent hors dépôt (`~/dev/cent-ans-mocap-src/`).

Outils : MediaPipe Pose Landmarker « heavy » (Apache 2.0, modèle Apache 2.0), OpenCV
(Apache 2.0), NumPy (BSD-3), Blender (GPL, outil seulement). Aucune dépendance SMPL, SMPL-X,
AMASS ni HumanML3D. Détails : `docs/wip/nt13-video-mocap.md`.
