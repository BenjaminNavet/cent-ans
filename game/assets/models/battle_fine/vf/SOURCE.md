# Clips tirés d'une vidéo libre (lot AS8a, ADR 0189)

Clips cuits (`human.bones.bin`, `CAB1`, os du rig fin `human`) par
`tools/blender_scripts/nt13_video_trial.py -- bake-vf` : `vf_thrust` (estoc en fente),
`vf_guard` (garde au bouclier, bouclée), `vf_strike` (épée levée, pas et coupe). Non branchés
par défaut : ils sont moins bons que les clips actuels de même rôle (glissement des pieds
0,36-0,47 m, contre 0), voir `docs/wip/as8.md`.

Source : « RoscheiderhofSpaetmittelter2018.webm », Helge Klaus Rieder (User:HelgeRieder), 2018,
https://commons.wikimedia.org/wiki/File:RoscheiderhofSpaetmittelter2018.webm, licence
**CC BY-SA 3.0** (https://creativecommons.org/licenses/by-sa/3.0). Adaptation (extraction de
poses, reciblage, cuisson) : ces fichiers dérivés sont distribués sous la même licence
CC BY-SA 3.0 (fichiers de données seuls ; le reste du jeu n'est pas concerné). Mesures dans
`manifest.json` (`clip_sources`). Vidéo et intermédiaires hors dépôt
(`~/dev/cent-ans-mocap-src/`).

Outils : MediaPipe Pose Landmarker « heavy » (Apache 2.0), OpenCV (Apache 2.0), NumPy (BSD-3),
Blender (GPL, outil seulement).
