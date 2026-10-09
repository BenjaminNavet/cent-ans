# 0239 — Manifeste skinné cuit hors ligne, fin des essais mocap et des modes fa_anim

## Contexte
`BattleSkinned.manifest()` assemblait à chaque lancement quatre sources (kit grossier, rigs et figurines fines, couches de clips mêlée NT14 et FA3, figurines générées GA3), avec des interrupteurs d'essai : `--mocap-trial` (CMU, NT12), `--video-trial` (vidéos du joueur, NT13), `--fa-anim` / mode `FA_ALL`, `--keyframed-melee`. Le défaut (docs/animation.md, ADR 0187-0189, as8.md) : clips de mêlée NT14 (`melee/`) puis clips FA3 marqués `default`, figurines GA3 ; les clips humains AS8a ont été abandonnés.

## Décision
- `tools/cent_ans_tools/bake_skinned_manifest.py` produit `battle_skinned/manifest_merged.json` (état par défaut fusionné). `BattleSkinned` le charge tel quel avec les figurines fines ; `--coarse-figures` lit toujours `manifest.json`. Équivalence avec l'ancien assemblage vérifiée avant suppression (test égalité profonde, 0 différence) ; `bt6_manifest_test.gd` garde les couches contre leurs sources.
- Supprimés : l'assemblage à l'exécution (`_merge_fine`, `_merge_mocap_trial`, `_merge_ga3`), les essais NT12/NT13 (`mocap_trial/`, `video_trial/`, options `--mocap-trial` / `--video-trial`), les modes `fa_anim` (`--fa-anim`, `FA_NONE/DEFAULT/ALL`), `--keyframed-melee`, les tests nt12/nt13/nt14/fa3_anim.
- Conservés : le pipeline vidéo-mocap (`tools/video_mocap`, `nt13_video_trial.py`, `nt12_mocap_trial.py`, `fa3_anim_retarget.py`) ; `melee/` et `fa3_anim/` (sources de la cuisson).

## Conséquences
- Comportement par défaut inchangé ; après tout recuit de `melee/`, `fa3_anim/` ou des manifestes fins, relancer l'outil de cuisson.
- Plus de bascule en jeu vers les clips keyframés de mêlée ni vers les clips d'essai.
