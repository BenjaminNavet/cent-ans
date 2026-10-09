# Champs peints lisibles (ADR 0272)

État : fait, en attente de fusion (branche de l'agent, non fusionnée).

- Cause : le splat range presque toute la terre ouverte de Beauce/Bretagne en prairie (`farm_share` ~ 0), donc le parcellaire HB ne choisissait presque jamais de culture ; en plus fondu précoce (0,14/0,38) et haies pâles.
- Correctif : bloc `view` de `data/art/ground_biome_mix.json` -> uniforms `hb_*` (`HbGround.apply_view`) ; `open_farm` 0,8 (part minimale de cultures sur terres ouvertes), `patch_gain` 0,65 (couleur de culture de la parcelle à distance, `season_field`), fondu 0,30/0,80, `cell_scale` 4,2, `far_hedge` 0,4, `cell_jitter` 0,28, `cell_hue` 0,35.
- Reste : Provence (vignes/oliviers) peu visible en vue large car surtout garrigue/relief ; la classe de culture lointaine est tirée par parcelle, pas par matière (oliveraie peut sortir « blé ») ; Bretagne encore un peu « pavage » à rig 20 ; échec `test_schemas[fx/battle_gore.json]` préexistant (hors tâche).
