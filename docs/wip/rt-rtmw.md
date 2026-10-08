# RT — essai RTMW (poignets / mains)

Branche `feat/rt-rtmw`. Prédécesseurs : `docs/wip/nt14-video-set2.md`, `nt13-video-mocap.md`.

## État : essai TERMINÉ, rien promu, rien branché dans le pipeline NT14

- `tools/video_mocap/extract_pose_rtmw.py` : extraction RTMW 2D (rtmlib, `Wholebody(mode="balanced")` =
  YOLOX-m + `rtmw-dw-x-l_simcc-cocktail14_270e-256x192`, onnxruntime CPU, 133 points dont 21 par main).
  Sortie `.npz` hors dépôt `~/dev/cent-ans-mocap-src/work/rt/IMG_645x.npz` (pixels, score, bbox, fps, size, found).
  2D seulement : le pipeline NT14 est en 3D monde MediaPipe, un branchement serait hybride (2D RTMW + profondeur MediaPipe).
- Durée : 70-215 s par vidéo (CPU, 60 i/s, 1080x1920), contre 23 s pour MediaPipe.
- Mesures : scripts jetables hors dépôt (`scratchpad`), repris ici. Tremblement = écart à la moyenne binomiale sur 5 images
  (même esprit que `tremor_deg`), en pixels / largeur d'épaules pour le poignet, en degrés pour les angles de main.
  Direction de main = poignet -> milieu index/auriculaire (MP) ou poignet -> base du majeur (RTMW) ; paume = index -> auriculaire.

## Résultats (moyennes sur images, 3 vidéos 6459 / 6460 / 6461)

| mesure | MediaPipe | RTMW |
|---|---|---|
| poignet gauche (caché par le disque) : score moyen | 0,16 / 0,22 / 0,20 | 0,63 / 0,62 / 0,62 |
| poignet gauche : images > 0,5 | 6 % / 13 % / 2 % | 95 % / 100 % / 93 % |
| coude gauche : score moyen | 0,11 / 0,20 / 0,14 | 0,80 / 0,75 / 0,77 |
| distance poignet gauche - centre du disque (largeurs d'épaules, médiane) | 0,39 / 0,59 / 0,40 | 0,32 / 0,43 / 0,30 |
| poignet droit (arme) : score moyen | 0,96 / 0,99 / 0,98 | 0,95 / 0,99 / 0,95 |
| points de main droite > 0,5 | (3 points seulement) | 100 % |
| tremblement du poignet droit (px / largeur d'épaules) | 0,007 / 0,002 / 0,003 | 0,011 / 0,007 / 0,007 |
| tremblement direction de la main droite (deg) | 1,2 / 0,2 / 0,4 | 2,3 / 1,5 / 1,5 |
| tremblement axe de paume droite (deg) | 1,5 / 0,5 / 0,5 | 5,3 / 5,6 / 2,9 |
| écart max direction main droite entre 2 images (deg) | 109 / 5 / 7 | 108 / 12 / 18 |
| variation de longueur d'avant-bras droit (écart type / moyenne) | 0,30 / 0,16 / 0,13 | 0,33 / 0,14 / 0,17 |

Lecture :
- Bras gauche caché : RTMW le « voit » (score 0,6-0,8 contre 0,1-0,2), poignet un peu plus proche du disque (0,30-0,43 contre
  0,39-0,59 largeur d'épaules), mais seulement 17-50 % des points de la main gauche tombent dans le disque : c'est de
  l'inférence plausible, pas de la mesure. Gain réel mais modeste.
- Main d'arme : RTMW n'est PAS meilleur, il est 2 à 10 fois plus bruité (par image, aucun filtre temporel ; MediaPipe en mode VIDEO
  lisse). Le roulis de paume en 2D est inexploitable sans lissage lourd ; les 21 points de main n'apportent pas de roulis 3D
  (2D seulement, 256x192 sur une boîte de personne entière : main de ~15 px).
- Pas de `-- measure` ni de recuisson : rien n'a été branché, donc aucun clip modifié.

## Licences (état établi, non définitif)

- Code `rtmlib` : Apache 2.0. onnxruntime : MIT. OpenCV : Apache 2.0.
- Poids RTMW / YOLOX (OpenMMLab mmpose, `download.openmmlab.com`) : publiés sous Apache 2.0 (dépôt mmpose), MAIS entraînés sur
  le mélange « cocktail14 ». Licences des jeux d'entraînement NON vérifiées une à une ; de mémoire (à confirmer avant toute promotion) :
  COCO / COCO-WholeBody (annotations CC BY 4.0, images Flickr de licences variées), MPII (BSD simplifié), AI Challenger et CrowdPose
  (recherche), Halpe-FullBody (non commercial), UBody (recherche), Human-Art (conditions propres), InterHand2.6M (CC BY-NC 4.0),
  jeux de visages (300W, WFLW, COFW, LaPa, AFLW : recherche). Plusieurs sont donc probablement non commerciaux : doute sérieux sur
  la chaîne de licence des poids, que la licence Apache du fichier ne lève pas.
- RTMW3D / `Wholebody3d` / `RTMPose3d` : EXCLU (entraîné avec H3WB, dérivé de Human3.6M, non commercial ; poids hébergés par un tiers
  `Soykaf/RTMW3D-x` sur Hugging Face). Non téléchargé, non essayé.
- Aucune dépendance SMPL / AMASS / HumanML3D / H36M dans le script 2D.

## Décision

Pas de branchement dans `video_mocap_clean.py` ni `nt13_video_trial.py`, pas d'ADR (pipeline inchangé), pas de dépendance dans
`tools/pyproject.toml` (script PEP 723 autonome, `uv run`). Raisons : gain limité au bras gauche caché (déjà reconstruit depuis
le disque en NT14), pas de gain sur la main d'arme, 2D seulement, licence des poids incertaine.

## Points ouverts
- Si le doute de licence est levé et qu'on veut tout de même le bras gauche : fusionner le poignet gauche RTMW (2D) avec le
  disque pour contraindre `shield_arm_rotations` ; à mesurer avec `-- measure`.
- Améliorer la main d'arme : recadrer sur la main (second passage RTMPose/hand sur boîte serrée) + One-Euro avant tout calcul de roulis.
- Aucune vidéo, image ni sortie dans le dépôt.
