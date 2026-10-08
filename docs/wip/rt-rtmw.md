# RT — essai RTMW (poignets / mains)

Branche `feat/rt-rtmw`. Prédécesseurs : `docs/wip/nt14-video-set2.md`, `nt13-video-mocap.md`.

## État : squelette
- `tools/video_mocap/extract_pose_rtmw.py` : extraction RTMW 2D (rtmlib, 133 points) -> `.npz` hors dépôt `~/dev/cent-ans-mocap-src/work/rt/`.
- Wholebody3d (RTMW3D) : poids tiers HF `Soykaf/RTMW3D-x`, entraînés avec données dérivées de H36M (non commercial) -> NON utilisé.

## Prochaine étape
Extraire les 3 vidéos, comparer à MediaPipe (confiance poignet/main, tremblement, roulis, bras gauche).
