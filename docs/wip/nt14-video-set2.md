# NT14 — vidéo vers animation, second tournage

Branche `feat/nt14-video-set2`. Orchestration : `docs/wip/nt.md`. Prédécesseurs : NT13
(`docs/wip/nt13-video-mocap.md`), NT12 (`docs/wip/nt12-mocap.md`).

## État : EN COURS (30/09)

Poses extraites (MediaPipe 0.10.21, heavy) : `~/dev/cent-ans-mocap-src/work/poses/IMG_6459..6461.npz`
(1080×1920 après rotation, 59,94 i/s, 23 s d'extraction par vidéo de 5 s).

## Gestes identifiés (bâton court main droite, disque rouge au bras gauche = bouclier)
- 6459 (5,7 s) : garde (0-1,5 s), montée et coupe descendante (2-4,5 s), retour → `guard`, `overhead`.
- 6460 (4,8 s) : garde, puis bouclier levé haut en avant, fente et flexion (1,3-2,4 s) → `parry`.
- 6461 (4,4 s) : garde, fente et estoc vers la droite de l'image (1,4-2,6 s) → `thrust`.
- Pas de `slash` horizontal, ni `hit`, ni `death` filmés.

## Prochaine étape
Découpe par immobilité, pipeline 60 i/s, bras gauche vidéo, pieds, poignet, lacet auto.

## Mandat ajouté (orchestrateur)
Choisir par geste de mêlée la meilleure source (keyframé / CMU NT12 / NT13 / NT14) et en faire le
défaut ; `--keyframed-melee` rétablit les anciens clips ; grille dans cette note + complément ADR 0129 ;
tests an1b, nt7, nt10, bv3_check, smoke ; banc A/B i/s bataille massive (≤ 5 %).
