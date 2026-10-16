## Vague 8 (reprise 15/10 après-midi) — ajouts
- Base : `bevy-migration` HEAD 67bdcc002 (tout fusionné, 1029 tests). Restes par lot : `docs/wip/bm-restes.md` et `docs/wip/bm-<lot>.md` ; lis ceux cités dans ta mission.
- Un agent « showfix » travaille EN PARALLÈLE dans gp-bevy sur le rendu des captures (qualité par défaut, rendu hors écran, chargements). Ne touche pas au pipeline de rendu global (camera HDR/tonemapping, `--offscreen`, préréglages de qualité, client/run_bg.sh, tools/compare_views.sh).
- Pas de capture ni de lecture d'image. Preuve par section de vidage (`--dump-state`) et tests.
- Ton worktree est déjà créé : /Users/jean_hubert/dev/gp-bm-<lot> (branche bm/<lot>). Pas de `client/target` : `source client/sccache-env.sh` puis compile (≈ 5 min à froid).
- Mission transverse : tu peux toucher plusieurs dossiers de client/src SI ta mission les nomme ; rien d'autre. Ne modifie pas main.rs/lib.rs sauf enregistrement minimal d'un plugin (signale-le).
- Après une interruption (limite hebdomadaire) : committe d'abord ton travail en cours qui compile (`wip:`), puis continue.
