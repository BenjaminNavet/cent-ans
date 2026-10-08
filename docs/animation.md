# Produire les animations — mode d'emploi

Comment les animations du jeu ont été produites, et comment en refaire. Décisions : ADR 0187
(une source par famille), 0188 (bêtes en shader), 0189 (vidéos libres). Historique détaillé :
`docs/wip/nt12-mocap.md`, `nt13-video-mocap.md`, `nt14-video-set2.md`, `fa3-anim.md`, `as*.md`,
`as8*.md`.

## Principes
- **Tout est local et gratuit.** Aucune animation ne vient d'un service payant (fal.ai, Meshy…).
- **Une seule chaîne pour les personnages** : rig Quaternius/MakeHuman → clips Blender → texture
  d'os → `MultiMesh`. Pas d'`AnimationPlayer` ni de `Skeleton3D` dans le jeu.
- **Le reste est procédural** (shaders de sommets, GDScript), avec des réglages dans `data/fx/*.json`
  validés par `data/schemas/`. Les valeurs y portent un bloc `source` quand elles sont mesurées.
- **Vidéos jamais dans le dépôt** : `~/dev/cent-ans-mocap-src/video/` (tournages du joueur) et
  `video/free/<famille>/` (Commons, un `LICENSE.txt` par fichier). Seuls les clips cuits, les
  courbes mesurées et les scripts sont versionnés.
- **Un clip nouveau ne devient le défaut que s'il bat l'actuel sur les mesures** (glissement des
  pieds, tremblement, appuis) ; sinon il reste derrière une option. Jugement à l'œil ensuite (AS7).

## Sources autorisées (ADR 0187, 0189)
| Ordre | Source |
|---|---|
| 1 | Tournages du joueur (téléphone, caméra fixe, corps entier dans le cadre) |
| 2 | Domaine public (Muybridge…) et CC0 (Quaternius, Mesh2Motion, vidéos Commons CC0) |
| 3 | CC BY puis CC BY-SA (Commons), avec attribution |
| — | CMU mocap (permis par sa FAQ, NT12) |

**Interdits** : SMPL/AMASS/HumanML3D/Human3.6M et tout modèle qui en dérive (WHAM, GVHMR, TRAM,
4DHumans, MotionBERT, RTMW3D, générateurs texte → mouvement) ; DeepLabCut SuperAnimal ; RTMW 2D
par défaut ; fichiers Mixamo/Rokoko/MoCap Online ; extraits de films sous droits.

**Licences** : chaque fichier dérivé d'une vidéo CC BY / CC BY-SA a un `SOURCE.md` et une ligne
dans `CREDITS.md`. Une courbe dérivée de CC BY-SA vit **dans son propre fichier** (CC BY-SA), jamais
mêlée à un fichier de réglages commun (ex. `data/fx/trebuchet_swing_curve.json`,
`data/fx/camp_horse_motion.json`).

## Recettes par famille

### Gestes humains de combat (NT13/NT14) — défaut actuel
1. Filmer : caméra posée, une personne en pied, fond calme ; bouclier remplacé par un disque rouge
   saturé tenu à gauche.
2. Poses 3D : `uv run tools/video_mocap/extract_pose.py VIDEO.MOV MODEL.task OUT.npz`
   (MediaPipe 0.10.21 « heavy », modèle `~/dev/cent-ans-mocap-src/work/pose_landmarker_heavy.task`).
3. Bouclier : `uv run tools/video_mocap/track_disc.py VIDEO.MOV OUT.npz`.
4. Caméra à la main seulement : `uv run tools/video_mocap/camera_shift.py VIDEO POSES.npz OUT.npz`.
5. Nettoyage (One-Euro, sol, appuis, lacet) : `tools/blender_scripts/video_mocap_clean.py`
   (appelé par le script suivant).
6. Clips : déclarer les segments dans `CLIPS_NT14` de `tools/blender_scripts/nt13_video_trial.py`,
   puis `blender -b --factory-startup --python tools/blender_scripts/nt13_video_trial.py -- <cmd>`
   avec `measure FICHIER` (compare tournage / keyframé / CMU), `bake-melee` (cuit le défaut
   `melee/`), `render DIR [clips]` puis `contact_sheet.py` pour les planches.
7. Option de repli : `--keyframed-melee`.

Essai AS8a : les vidéos de reconstitution Commons donnent des pieds qui glissent (0,36–0,47 m,
caméra à la main, jambes cachées par l'armure). Clips `vf_` non versionnés ; ils se recuisent hors
dépôt avec `-- bake-vf` (table `CLIPS_AS8A`). Conclusion : pour les humains, tourner soi-même.

Autres sources humaines déjà branchées : CMU (`nt12_mocap_trial.py`, `--mocap-trial`), CC0
retargeté (`fa3_anim_retarget.py`, `--fa-anim` / `--no-fa-anim`).

### Chevaux de bataille (AS3 → AS8b) — trot et galop mesurés sur Muybridge
1. Images : GIF/disques Muybridge (domaine public) dans `video/free/horses/`, image par image.
2. Points posés à la main (racines des membres, 4 sabots) sur ~12 images d'une foulée :
   `tools/video_mocap/data/horse_keypoints.json`. Aucun modèle de pose animale.
3. Ajustement d'une foulée (séries de Fourier) :
   `uv run tools/video_mocap/track_quadruped.py fit KEYPOINTS.json GAIT OUT.json`.
4. Table versionnée : `tools/video_mocap/export_horse_gaits.py` →
   `tools/blender_scripts/data/horse_gaits_free.json` (+ `SOURCE.md`).
5. Clips : `battle_skinned_gaits.py` (`horse_trot_free`, `gallop_free` ; réglages `FREE_*`) ;
   recuit fin `battle_fine.py -- rigs`, grossier `as8b_rebake_coarse.py`.
   `AS8B_LEGACY=1` rétablit les clips AS3.
6. Mesures : `blender -b --factory-startup --python tools/blender_scripts/as8b_measure.py -- c_trot c_gallop`.
7. Cadences de lecture : `data/fx/battle_gore.json` ; seuils d'allure : `data/fx/battle_animation.json`
   (`cavalry_gaits`).

Reste keyframé : pas (`c_walk`), cheval sans cavalier (`c_fall`).

### Bêtes et charrettes de campagne, chevaux du camp (AS1 → AS8c) — shader + mesures
- Rendu : `game/shaders/animal_motion.gdshaderinc`, `camp_horse.gdshader`, `folk_prop.gdshader`,
  piloté par `game/scripts/visual/animal_motion.gd`.
- Mesures : `uv run tools/video_mocap/measure_motion.py sheet|track|analyse …` (suivi optique de
  points posés), chaîne complète `uv run tools/video_mocap/as8c_measure.py` →
  `data/fx/animal_motion_measured.json` et `data/fx/camp_horse_motion.json` (valeurs Rama, CC BY-SA).
- Réglages : `data/fx/animal_motion.json` (tables de pas `swing_lut`/`lift_lut`, raies de cahot
  `jolt_lines`, bloc `source` qui distingue mesuré et supposé).

### Engins, feu, herbe, drapeaux (AS4/AS5 → AS8d)
- Mesures : `uv run tools/video_mocap/measure_motion_fx.py trebuchet|winch|swing-lut|cannon|grass|flag|flame …`
  (détail et limites : `docs/research/as8d-mesures.md`).
- Trébuchet : courbe du bras `data/fx/trebuchet_swing_curve.json` lue par `siege_engines_fx.gd`
  (repli sur la courbe procédurale si absent). Servants : `siege_crew_fx.gd`, `data/fx/siege_engines.json`.
- Feu : `life_flame.gdshader` (`fire.flicker_hz`), planche optionnelle
  `game/assets/textures/fx/flame_flipbook_video.png` (`fire.flipbook` dans `data/fx/map_fire_wind.json`).
- Bannières, arbres, vent : `maquette_banner.gdshader`, `map_banner.gdshader`,
  `battle_tree_impostor.gdshader`, `campaign_wind.gdshaderinc`.

### Armées sur la carte (AS2)
Cadence des figurines liée à la vitesse : `army_figures.gd`, `data/fx/campaign_army_walk.json`.

## Vérifier
Tests headless `game/tests/as1_test.gd` … `as8d_test.gd`, `an1b_clips_test.gd`, `smoke.gd` ;
schémas : `uv run --project tools pytest`. Godot fenêtré (planches, captures) uniquement via
`tools/godot_bg.sh`.

## Ce qui manque encore
Bombarde, mangonneau, porteurs de pierres, cheval attaché, pas du cheval : aucune vidéo libre
exploitable ; procédural ou tournage du joueur (AS6).
