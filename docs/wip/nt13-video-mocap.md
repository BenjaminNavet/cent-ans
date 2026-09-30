# NT13 — vidéo vers animation (mocap maison)

Branche `feat/nt13-video-mocap`. Orchestration : `docs/wip/nt.md`. Prédécesseur : NT12
(`docs/wip/nt12-mocap.md`, reciblage CMU, `--mocap-trial`).

## État : squelette

Vidéos du joueur (hors dépôt) : `~/dev/cent-ans-mocap-src/video/IMG_6455..6458.MOV` ;
intermédiaires (images, poses brutes, planches) : `~/dev/cent-ans-mocap-src/work/`.

## Gestes identifiés (bâton tenu à deux mains)
- 6455 (4,5 s) : garde bâton sur l'épaule droite, puis montée et coup vertical vers l'avant
  → `guard` (début, bouclé) et `overhead`.
- 6456 (1,2 s) : coup diagonal de l'épaule droite vers le bas gauche → `slash`.
- 6457 (4,6 s) : estoc à deux mains vers l'avant puis retour à la hanche → `thrust`.
- 6458 (2,6 s) : bâton levé de la hanche vers une garde haute / coup horizontal → `parry`.

## Pipeline
1. `tools/video_mocap/extract_pose.py` (MediaPipe Pose Landmarker heavy, env `uv` dédié dans
   `work/env/`) → `work/poses/<vidéo>.npz` (33 points monde + image + visibilité).
2. `tools/blender_scripts/video_mocap_clean.py` (numpy pur, testé) : lissage One-Euro, longueurs
   de segments constantes (profondeur), appuis de pied, racine par les pieds, bouclage.
3. `tools/blender_scripts/nt13_video_trial.py` (Blender) : rotations d'os, reciblage sur le rig
   fin `human` (réutilise NT12), IK de jambe sur les appuis, épée par la main droite, cuisson
   `game/assets/models/battle_fine/video_trial/`, `-- render DIR` planche.
4. `BattleSkinned` : `--video-trial` après `--`.

## Prochaine étape
Extraction des poses.
