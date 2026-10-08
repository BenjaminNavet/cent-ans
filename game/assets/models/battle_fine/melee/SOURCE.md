# Clips de mêlée par défaut (lot NT14, ADR 0129 complément)

Clips cuits (`human.bones.bin`, `CAB1`, os du rig fin `human`) par
`tools/blender_scripts/nt13_video_trial.py -- bake-melee` : pour chaque geste de mêlée, la
meilleure source parmi keyframé, CMU (NT12), vidéos du joueur (NT13, NT14), grille dans
`docs/archive/chantiers.md`. Lus par défaut sur les figurines fines ; `--keyframed-melee`
après `--` rétablit les clips keyframés. Les gestes dont le keyframé l'emporte ne sont pas
cuits ici (clips d'origine du rig).

Clips substitués : `guard`, `overhead`, `parry`, `thrust` (second tournage vidéo du joueur,
NT14). Mesures de chaque clip dans `manifest.json` (`clip_sources`).

Source : vidéos filmées par le joueur au téléphone ; propriété du joueur, cuisson versionnée
avec son accord ; vidéos et données intermédiaires hors dépôt (`~/dev/cent-ans-mocap-src/`).
Outils : MediaPipe Pose Landmarker « heavy » (Apache 2.0), OpenCV (Apache 2.0), NumPy
(BSD-3), Blender (GPL, outil seulement).
