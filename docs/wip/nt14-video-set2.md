# NT14 — vidéo vers animation, second tournage

Branche `feat/nt14-video-set2`. Orchestration : `docs/wip/nt.md`. Prédécesseurs : NT13
(`docs/wip/nt13-video-mocap.md`), NT12 (`docs/wip/nt12-mocap.md`).

## État : EN COURS (30/09) — pipeline, cuissons, défaut `melee/` et tests Godot écrits ;
reste : exécution des tests Godot, captures A/B NT12/NT13, banc i/s, complément ADR 0129.

Vidéos (hors dépôt) : `~/dev/cent-ans-mocap-src/video/IMG_6459..6461.MOV` (1080×1920 après
rotation, 59,94 i/s, 4,4 à 5,7 s). Intermédiaires hors dépôt dans `~/dev/cent-ans-mocap-src/work/` :
poses `poses/IMG_645x.npz` (23 s d'extraction par vidéo), disque `poses/IMG_645x_disc.npz`,
rendus `render_nt14/`, planches `planches/nt14_<clip>.png`, mesures `nt14_measures.json`,
scripts jetables `nt14_*.py`.

## Gestes identifiés (bâton court main droite, disque rouge tenu main gauche = bouclier)
- 6459 (5,7 s) : garde, levée au-dessus de la tête et coupe descendante en avant avec
  accompagnement (1,6-2,8 s) → `overhead` (i. 96-170, ×1,2) ; puis seconde levée et retour
  (non utilisés), le joueur regarde ensuite le sol (3,9-4,7 s).
- 6460 (4,8 s) : garde, disque jeté haut en avant, fente et flexion (1,0-2,9 s) → `parry`
  (i. 62-175, ×1,2).
- 6461 (4,4 s) : garde immobile (0,2-1,2 s) → `guard` (i. 12-72, bouclé) ; fente et estoc à une
  main vers la droite de l'image (1,2-2,75 s) → `thrust` (i. 72-165, ×1,2).
- Pas de `slash` horizontal, ni `hit`, ni `death` filmés.

## Pipeline NT14 (ajouts à NT13, mêmes commandes)
1. `tools/video_mocap/extract_pose.py` inchangé (MediaPipe 0.10.21) ; nouveau
   `tools/video_mocap/track_disc.py` : seuil HSV sur le rouge saturé, plus grande tache,
   ellipse (`cv2.fitEllipse`) → centre, axes, angle par image (vérifié sur 6 images : collé au
   disque). Le bras gauche MediaPipe est caché derrière le disque (visibilité coude/poignet
   0,11-0,22) : inutilisable.
2. `video_mocap_clean.py` :
   - **60 i/s** : durées de contact en secondes (0,1 s / 0,067 s) au lieu d'images ; lissage
     One-Euro inchangé en Hz (coupure 1,5 Hz, bêta 0,3), il s'applique au débit réel.
   - **Mise à niveau du sol** (`ground_tilt`) : MediaPipe suit la caméra, un téléphone penché
     incline le sol ; le pied arrière d'une garde décalée paraissait levé de 8 cm → pas d'appui
     → glissement. Droite du sol ajustée sur talons et orteils (tangage seul : un ajustement en
     roulis penchait tout le corps sur les planches).
   - **Disque** : `image_world_fit` (échelle et décalage image → monde sur les points fiables),
     `disc_points` (profondeur par la taille apparente, correction de perspective),
     `reach_anchor` (profondeur absolue : le disque le plus éloigné de la vidéo est à une
     longueur de bras de l'épaule), `disc_normals` (inclinaison par le rapport des axes, signe
     par la face tournée hors du buste et la continuité).
   - **Poignet** : `despike_quats` (pic isolé d'une image > 12° remplacé) + `smooth_quats`.
   - **Pieds** : `step_targets` : entre deux appuis à moins de 0,5 s, le pied est porté d'un
     point d'appui à l'autre sur un arc levé de 4-8 cm (IK à deux os déjà en place) au lieu de
     glisser.
   - **Orientation vers l'adversaire** (`facing_yaw`) : moyenne de la face du buste et de la
     direction de la main qui frappe (ou du disque pour la parade) sur les 20 % d'images où elle
     est la plus loin des hanches ; `yaw="auto"` dans la table (valeurs : garde 75°, estoc 76°,
     coupe 72°, parade 23°).
3. `nt13_video_trial.py` : table de clips en dictionnaires (`CLIPS_NT13`, `CLIPS_NT14`,
   options `grip`, `shield`, `reach`, `yaw_range`) ; prise à une main (`hand_sides` : roulis par
   l'axe de la main, jamais le long de la lame ; plafond de flexion 80°) ; **bras du bouclier
   depuis le disque** (`shield_arm_rotations` : avant-bras couché dans le plan du disque, centre
   de l'écu, 77 % de l'avant-bras, sur le centre du disque, face de l'écu mesurée sur le maillage
   `heater_shield` → normale du disque ; lissage de la chaîne `filter_chain`) ; nouvelle mesure
   **`tremor_deg`** (écart moyen de chaque os à sa moyenne binomiale sur 5 images : le
   tremblement sans le « claquement » d'un coup rapide, que `jitter_deg_per_frame2` compte
   aussi). Commandes : `-- measure FICHIER` (toutes les sources de chaque geste),
   `-- bake-melee` (défaut `melee/`), `-- render DIR [clips]` (lignes k / m / v / w / c).
4. `contact_sheet.py` : lignes keyframé / CMU / NT13 / NT14 / vue caméra / source, préfixe.

## Mesures (24 i/s, `-- measure`) et comparaison NT13
Glissement = pied à moins de 3 cm du sol qui bouge (m) ; tremblement = `tremor_deg` ;
accél. = `jitter_deg_per_frame2` (mesure NT13, compte aussi les coups secs).

| geste | source | glissement | tremblement | accél. | poignet max °/i |
|---|---|---|---|---|---|
| guard | keyframé | 0 | 0,01 | 0,01 | 0,3 |
| guard | CMU | 0,114 | 0,37 | 9,3 | 6,0 |
| guard | NT13 | 0 | 0,04 | 1,9 (2,86 cuit) | 0,6 |
| guard | **NT14** | 0 | 0,03 | 0,06 | 0,3 |
| slash | **keyframé** | 0,003 | 0,94 | 17,4 | 44,1 |
| slash | CMU | 0,054 | 0,42 | 9,8 | 13,6 |
| slash | NT13 | 0 | 0,93 | 12,6 (12,73 cuit) | 25,9 (31,2 cuit) |
| overhead | keyframé | 0 | 2,58 | 17,7 | 103,2 |
| overhead | CMU | 0,184 | 0,42 | 10,5 | 11,2 |
| overhead | NT13 | 0 | 0,47 | 11,2 (5,37 cuit) | 34,0 |
| overhead | **NT14** | 0 | 1,41 | 15,6 | 25,0 |
| thrust | keyframé | 0,04 | 2,02 | 28,3 | 37,8 |
| thrust | NT13 | 0,185 | 0,55 | 3,4 | 14,4 |
| thrust | **NT14** | 0 | 1,63 | 18,5 | 28,4 |
| parry | keyframé | 0 | 1,02 | 8,8 | 5,8 |
| parry | CMU | 0,023 | 0,27 | 6,0 | 6,9 |
| parry | **NT14** | 0,013 | 0,73 | 11,4 | 6,8 |
| hit | **keyframé** | 0 | 0,47 | 6,7 | 7,2 |
| hit | CMU | 0,017 | 0,41 | 13,8 | 14,4 |
| death | **keyframé** | 0,263 | 1,81 | 19,8 | 47,6 |
| death | CMU | 0,192 | 1,54 | 11,9 | 38,7 |

NT13 → NT14 : glissement de l'estoc 18,5 → 0 cm (mise à niveau du sol + pas portés) ; coupe
NT14 d'abord 11,6 cm (pas traînant) → 0 ; poignet sur plafond (114°, 122°) → 55-75° (prise à une
main) ; bras du bouclier vidéo au lieu du keyframé. Le tremblement NT14 (1,4-1,6 sur coupe et
estoc) est plus haut que NT13 (0,5) : bras droit rapide (×1,2 seulement, contre ×1,4-1,8) et
points de main MediaPipe grossiers ; il reste sous celui du keyframé (2,0-2,6, coups secs).

## Grille de choix du défaut (mandat orchestrateur) — `melee/`, `--keyframed-melee`
| geste | source retenue | raison |
|---|---|---|
| guard | NT14 | immobile (0,03), pieds tenus, épée à une main et écu du disque ; NT13 = bâton sur l'épaule à deux mains (poignet au plafond) ; CMU glisse 11 cm |
| overhead | NT14 | pieds tenus, geste complet levée-coupe-accompagnement, écu vivant ; CMU glisse 18 cm ; NT13 à deux mains |
| thrust | NT14 | fente sans glissement (NT13 : 18,5 cm), une main, écu en garde |
| parry | NT14 | seule parade au bouclier filmée, lisible sur planche (écu levé haut, flexion) ; CMU = épée levée sans écu |
| slash | keyframé | pas de coupe horizontale NT14 ; NT13 à deux mains, le plus bruité ; CMU glisse 5 cm |
| hit | keyframé | CMU sans vrai impact (NT12), pas filmé |
| death | keyframé | CMU pas nettement mieux (NT12) ; la dernière image sert aux cadavres |

## Défauts par clip (NT14)
- `guard` : très calme (garde « détendue », épée basse) ; lacet déduit de l'estoc de la même
  vidéo (75°).
- `overhead` : prise de l'épée approximative (orientation de lame tirée des 3 points de main
  MediaPipe) ; accéléré ×1,2 seulement.
- `thrust` : l'épée monte parfois de 20-40° au-dessus du bâton pendant l'estoc (points de main).
- `parry` : écu levé à hauteur de tête (le disque montait plus haut) ; bras gauche reconstruit
  (MediaPipe ne le voit pas), coude choisi par le solveur.
- Tous : profondeur du disque estimée (taille apparente + longueur de bras) ; face de l'écu = face
  convexe du disque tournée vers l'adversaire.

## Tests
- pytest `tools/tests/test_video_mocap_clean.py` : 25 (dont 10 NT14).
- Godot : `nt14_melee_test.gd` (défaut, `--keyframed-melee`, forçage 0/1, essai prioritaire),
  `nt13_video_test.gd` (+ parry, défaut écarté), `nt12_mocap_test.gd` (défaut écarté), smoke,
  nt7_anim_test, an1b_clips_test, nt10_test, bv3_check : à exécuter.
