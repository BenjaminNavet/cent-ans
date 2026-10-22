# Produire les animations — mode d'emploi (chaîne Bevy)

Comment les animations du jeu sont produites, et comment en refaire. Le client est en Bevy
(ADR 0357) ; le client Godot est retiré (ADR 0366) : les anciens chemins `game/scripts/`,
`game/shaders/`, `AnimationPlayer` et les tests `.gd` n'existent plus. Les recettes Blender et
Python ci-dessous restent valides. Décisions : ADR 0187 (une source par famille), 0188 (bêtes en
shader de sommets), 0189 (vidéos libres), 0014 (figurines skinnées, animations cuites en texture
d'os), 0239 (manifeste skinné cuit hors ligne). Historique détaillé : `docs/wip/nt12-mocap.md`,
`nt13-video-mocap.md`, `nt14-video-set2.md`, `fa3-anim.md`, `as*.md`, `as8*.md`.

## Principes
- **Tout est local et gratuit.** Aucune animation ne vient d'un service payant (fal.ai, Meshy…).
- **Les personnages sont cuits hors du jeu** : rig Quaternius / MakeHuman → clips Blender → manifeste
  `manifest_merged.json` et texture d'os. Le client ne lit pas de graphe d'animation : la mêlée, la
  marche et les réactions sont évaluées sur le GPU, dans le vertex shader, à partir de la texture d'os
  et de l'heure de chaque soldat.
- **Le reste est procédural** (shaders de sommets `.wesl`, systèmes Rust), avec des réglages dans
  `data/fx/*.json`, validés par `data/schemas/`. Les valeurs y portent un bloc `source` quand elles
  sont mesurées.
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

## Où ça vit dans le client Bevy

Chemins sous `client/src/` (code) et `game/assets/` (données cuites, racine d'assets du client).

| Besoin | Code Rust | Shader `.wesl` | Réglages `data/fx/` |
|---|---|---|---|
| Figurines de bataille : famille × état → clips | `battle/units/anim.rs` (table, modes `Loop` / `Cycle` / `Volley` / `Pick`) | `battle/units/skinned.wesl` (lecture de la texture d'os, réaction aux coups) | `battle_animation.json` |
| Batches skinnés, chargement du manifeste | `battle/units/skinned.rs`, `battle/units/figures.rs` (`models/battle_skinned/manifest_merged.json`) | idem | — |
| Allures de cavalerie (pas, trot, galop, virages) | `battle/units/gaits.rs` | idem | `battle_animation.json` (`cavalry_gaits`) |
| Cadences de lecture, sang, membres | `battle/units/corpses.rs`, `battle/decor/gore.rs`, `battle/decor/blood.rs` | — | `battle_gore.json` |
| Engins de siège (trébuchet, bélier, bombarde) | `battle/siege/engines.rs`, `battle/siege/animation.rs`, `battle/siege/fx.rs` | — | `trebuchet_swing_curve.json`, `siege_engines.json`, `siege_fx.json` |
| Servants des engins | `battle/siege/crew.rs` (mode `Pick`) | `battle/units/skinned.wesl` | `siege_engines.json` |
| Chevaux attachés du camp | `battle/decor/camp_horse.rs` | `battle/decor/camp_horse.wesl`, `camp_horse_pose.wesl`, `camp_horse_shadow.wesl` | `camp_horse_motion.json`, `animal_motion.json` |
| Bêtes, charrettes, troupeaux de la carte vivante | `campaign/life/anim.rs`, `campaign/life/motion.rs` | `campaign/life/anim.wesl`, `anim_pose.wesl`, `anim_shadow.wesl` | `animal_motion.json`, `animal_motion_measured.json` |
| Armées sur la carte (cadence des figurines) | `campaign/armies/skinned.rs`, `campaign/armies/standard.rs` | — | `campaign_army_walk.json` |
| Drapeaux, voiles | `campaign/armies/` | `campaign/armies/shaders/flag.wesl`, `sail.wesl` | — |
| Feu, flipbooks | `fx/` | `fx/flipbook.wesl` | `fire_flipbooks.json`, `map_fire_wind.json` |
| Vent, arbres, herbe | `battle/decor/wind.rs`, `campaign/vegetation/` | `battle/decor/wind.wesl`, `campaign/vegetation/billboard.wesl` | `battle_decor_look.json`, `understory.json` |

Les commentaires `//!` de plusieurs modules citent encore le script Godot d'origine (« port de … »)
et la liste des parties « non portées ». Ils servent de trace de provenance, pas d'état du code.

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
7. Fusion : `uv run --project tools python tools/cent_ans_tools/bake_skinned_manifest.py` régénère
   `game/assets/models/battle_skinned/manifest_merged.json` (mêlée + clips FA3 `default` + figurines
   GA3), chargé tel quel par `battle/units/figures.rs` ; à relancer après tout recuit de `melee/`,
   `fa3_anim/` ou des manifestes fins.

Essai AS8a : les vidéos de reconstitution Commons donnent des pieds qui glissent (0,36–0,47 m,
caméra à la main, jambes cachées par l'armure). Clips `vf_` non versionnés ; ils se recuisent hors
dépôt avec `-- bake-vf` (table `CLIPS_AS8A`). Conclusion : pour les humains, tourner soi-même.

Autres sources humaines : CC0 retargeté (`fa3_anim_retarget.py`, clips `default` de la table
fusionnés, les autres restent cuits mais inutilisés). Les essais CMU (`--mocap-trial`) et vidéo
(`--video-trial`) ont été supprimés (SC bt6, ADR 0239).

### Chevaux de bataille (AS3 → AS8b) — trot et galop mesurés sur Muybridge
1. Images : GIF/disques Muybridge (domaine public) dans `video/free/horses/`, image par image.
2. Points posés à la main (racines des membres, 4 sabots) sur ~12 images d'une foulée :
   `tools/video_mocap/data/horse_keypoints.json`. Aucun modèle de pose animale.
3. Ajustement d'une foulée (séries de Fourier) :
   `uv run tools/video_mocap/track_quadruped.py fit KEYPOINTS.json GAIT OUT.json`.
4. Table versionnée : `tools/video_mocap/export_horse_gaits.py` →
   `tools/blender_scripts/data/horse_gaits_free.json` (+ `SOURCE.md`).
5. Clips : `tools/blender_scripts/battle_skinned_gaits.py` (`horse_trot_free`, `gallop_free` ;
   réglages `FREE_*`) ; recuit fin `battle_fine.py -- rigs`, grossier `as8b_rebake_coarse.py`.
   `AS8B_LEGACY=1` rétablit les clips AS3.
6. Mesures : `blender -b --factory-startup --python tools/blender_scripts/as8b_measure.py -- c_trot c_gallop`.
7. Cadences de lecture : `data/fx/battle_gore.json` ; seuils d'allure : `data/fx/battle_animation.json`
   (`cavalry_gaits`, lus par `battle/units/gaits.rs`).

Reste keyframé : pas (`c_walk`), cheval sans cavalier (`c_fall`).

### Bêtes et charrettes de campagne, chevaux du camp (AS1 → AS8c) — shader + mesures
- Rendu : `campaign/life/anim.wesl` (sommets, pas et cahots lus dans `animal_motion.json`) ;
  chevaux attachés : `battle/decor/camp_horse.wesl`.
- Mesures : `uv run tools/video_mocap/measure_motion.py sheet|track|analyse …` (suivi optique de
  points posés), chaîne complète `uv run tools/video_mocap/as8c_measure.py` →
  `data/fx/animal_motion_measured.json` et `data/fx/camp_horse_motion.json` (valeurs Rama, CC BY-SA).
- Réglages : `data/fx/animal_motion.json` (tables de pas `swing_lut`/`lift_lut`, raies de cahot
  `jolt_lines`, bloc `source` qui distingue mesuré et supposé ; `enabled` coupe l'effet).

### Engins, feu, herbe, drapeaux (AS4/AS5 → AS8d)
- Mesures : `uv run tools/video_mocap/measure_motion_fx.py trebuchet|winch|swing-lut|cannon|grass|flag|flame …`
  (détail et limites : `docs/research/as8d-mesures.md`).
- Trébuchet : courbe du bras `data/fx/trebuchet_swing_curve.json`, appliquée aux nœuds nommés des
  modèles par `battle/siege/engines.rs` (repli sur la courbe procédurale si absente). Servants :
  `battle/siege/crew.rs`, réglages `data/fx/siege_engines.json`.
- Feu : `fx/flipbook.wesl`, cadence `data/fx/fire_flipbooks.json` ; planche optionnelle
  `game/assets/textures/fx/flame_flipbook_video.png`.
- Bannières, arbres, vent : `campaign/markers/banner.wesl`, `battle/decor/wind.wesl`,
  `campaign/vegetation/billboard.wesl`.

### Armées sur la carte (AS2)
Cadence des figurines liée à la vitesse : `campaign/armies/skinned.rs`, `data/fx/campaign_army_walk.json`.

## Vérifier
- Tests unitaires du client (mapping état → clips, allures, servants, bêtes) :
  `cd client && source sccache-env.sh && cargo test --features dev` (sans `CENT_ANS_MUTE`, le test
  audio échoue muet ; voir `CLAUDE.md`).
- Smoke : `CENT_ANS_MUTE=1 client/tests/smoke.sh`.
- Schémas des réglages `data/fx/` et scripts Python : `uv run --project tools pytest`.
- Captures et dumps : **uniquement** via `client/run_bg.sh` avec `CENT_ANS_MUTE=1` et `--offscreen`
  (fenêtre masquée, muet). Options `--dump-state` et `--shot` : `client/README.md`. Jamais le binaire
  lancé en direct avec une fenêtre.

## Ce qui manque encore
- Aucune vidéo libre exploitable pour la bombarde, le mangonneau, les porteurs de pierres, le cheval
  attaché, ni pour le pas du cheval (`c_walk`) et la chute sans cavalier (`c_fall`) : procédural ou
  tournage du joueur (AS6).
- Pas de test qui vérifie, pour chaque famille et chaque état, qu'un clip existe dans le manifeste
  cuit : un nom absent retombe silencieusement sur le repli.
- Effets au sol et aux impacts non portés (mottes sous les sabots, étincelles sur armure) : voir
  `docs/wip/nuit-critiques/animation-vfx.md`.
