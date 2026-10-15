# bm-progress

Branche `bm/progress2` (suite de `bm/progress`). Détails : `client/README.md` § « Progression de campagne ».

Fait et prouvé (clippy, tests, `--script` + `--dump-state`, scripts `client/src/campaign/progress/proof_script_{a,b}.txt`) : panneau Objectifs, écran de fin, traditions (bannière de maison, infobulles riches), didacticiel 14 étapes avec flèche/halo, sommaire, focus caméra, persistance dans `Settings`, infobulle du score, `end_year` unique côté `sim-campaign`.

Restes :
- câblage `integ` : bouton Objectifs / entrée de menu -> `OpenObjectives`, bouton « Traditions » du bandeau d'ost -> `OpenTraditions`, Menu -> Tutoriel / F1 -> `StartTutorial` ;
- encyclopédie (lot Codex) ;
- le parchemin du guide ne s'écarte pas de la cible (`place_panel` / `set_avoid` de `tutorial.gd`) ;
- test du clic « Menu principal » de l'écran de fin non exercé en script ;
- coup d'œil réel du cadre / de la flèche non fait (pas de capture).
