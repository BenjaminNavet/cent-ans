# FE6 — Interface de la féodalité (`feat/fe6-ui`, worktree `../game_project-fe6`)

Spec FE § 6 (et § 4) ; plan section F6 ; ADR 0098. `CARGO_TARGET_DIR=core/target-fe6`.

## État
- [ ] Squelette : pont `campaign_sim_feudal.rs`, contrôleur `feudal_controller.gd`, tests désactivés.
- [ ] Cœur : vues `sim_campaign::feudal::view` (fiche, obligations, fil d'Ariane, carte féodale, états des vassaux).
- [ ] Pont : lectures + ordres (commise, concession, révolte, hommage, arbitrage).
- [ ] Godot : choix de faction sur carte, filtre « Féodalité », fil d'Ariane, arbre, obligations,
      « Qui peut entrer en guerre », journal, objectifs, Codex « Vassalité », tutoriel 3 étapes.
- [ ] Tests : `fe_ui_test.gd`, `smoke.gd`, `fe_shot.gd`.

## Prochaine étape
Vues du cœur puis pont.
