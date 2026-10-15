# bm-progress

Branche `bm/progress` : `client/src/campaign/progress/` (objectives, ending, traditions, tutorial) + `core/crates/game-api/src/progress.rs`. Détails : `client/README.md` § « Progression de campagne ».

Fait et prouvé (clippy, tests, `--script` + `--dump-state`) : panneau Objectifs + choix court/long, écran de fin, traditions (choix = ordre du cœur), didacticiel 14 étapes avec détection réelle (fin de tour constatée).

Restes :
- didacticiel : flèche / halo sur la cible (`tutorial_controller.gd::resolve_target`), sommaire (`jump_to`), focus caméra par étape, persistance (clés `tutorial/step|done|postponed` à ajouter à `Settings`), encyclopédie ;
- câblage `integ` : bouton Objectifs / entrée de menu → `OpenObjectives`, bouton « Traditions » du bandeau d'ost → `OpenTraditions`, Menu → Tutoriel / F1 → `StartTutorial` ;
- infobulle du score (`campaign_score`), blason de maison des traditions ;
- `game_api::progress::end_year` duplique `victory_set` (privé dans `sim-campaign`) : exposer un accesseur public dans `sim-campaign::victory` puis le remplacer ;
- test du clic « Menu principal » de l'écran de fin non exercé en script.
