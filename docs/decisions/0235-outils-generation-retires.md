# 0235 — Outils de génération et d'aide à la 3D retirés

Statut : accepté (10-09, lot SC TL2-TL7)

## Contexte
Le chantier de simplification (SC) a revérifié `tools/` : quels outils de génération (images, sons,
3D, mocap) sont encore appelés par le code, un test ou une doc vivante (`docs/animation.md`,
`docs/pipeline-assets-3d.md`, ADR non remplacé) ?

## Décision
- **Retirés** (aucun appel, test ni doc) : planches de contrôle `an1b_planche/render`,
  `fg0_planche`, `fg4_planche/render`, `fa3_anim_board` ; sondes `as8b_probe`, `landmark_preview` ;
  `video_mocap/extract_pose_rtmw.py` (RTMW 2D exclu par défaut, ADR 0187). L'option `--render` de
  `battle_fine_cavalry.py` disparaît avec `fg4_render`. Cinq fonctions mortes de `cent_ans_tools`
  (`frame_inset`, `worldcover_corner`, `point_in_districts`, `projected_bounds_of_extent`,
  `tile_counts`).
- **Conservés** (décrits comme outils actuels ou couverts par un test) : tout `video_mocap` actif,
  les pipelines `openrouter`, `voice_tts/voice_shout`, `portraits`, `material_gen`, `local_art`,
  `ui_ornaments`, `audio_bank`, `ui_sounds`, `sg2_sounds`, `era_music`/`ars_nova`, et les dossiers
  `tools/da5_raw`, `da5b_raw`, `horizon_raw` : ce sont les sources brutes relues par `ink_icons`,
  `entity_icons` et `horizon_panoramas` (régénération sans nouvel appel payant). `tools/map_markers`
  n'existe plus.
- Les citations d'outils déjà supprimés (`ga3_cleanup`, `ga3_siege_rig`, `hb_rock_outcrops`,
  `hdri_sun`, `fire_flipbooks`, `ga3_vegetation`) sont retirées des descriptions de schémas, données
  et commentaires GDScript.
- Les validations de schéma des tests et de `geo/biomes.py` passent par `codex.schema_validator`
  (registre des schémas voisins) au lieu d'un `Draft202012Validator` nu.

## Conséquences
-~1000 lignes d'outils. Retirer un pipeline payant encore câblé (et testé) demande un ADR dédié.
