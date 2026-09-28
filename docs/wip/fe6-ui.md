# FE6 — Interface de la féodalité (`feat/fe6-ui`, worktree `../game_project-fe6`)

Spec FE § 6 (et § 4) ; plan section F6 ; ADR 0098. `CARGO_TARGET_DIR=core/target-fe6`.

## État
- [x] Squelette commité.
- [x] Cœur : `sim_campaign::feudal::view` (fiche, obligations, fil d'Ariane, carte féodale, états
      des vassaux, candidats à l'hommage) + tests `sim-campaign/tests/feudal_view.rs`.
- [x] Pont `campaign_sim_feudal.rs` : `get_feudal_sheet/tree/obligations/map/offers`,
      `get_province_breadcrumb/lieges`, `get_war_escalation_preview`, ordres `feudal_*` ;
      `GameDataStore.get_feudal_start_sheets` (choix de faction).
- [x] Godot : filtre MF1 « Féodalité » (hachures shader `hatch_colors`, écus partis), « Qui peut
      entrer en guerre » (`EscalationPreview` dans la confirmation de guerre et l'infobulle de la
      diplomatie), réponses d'arbitrage (diplomatie), section « Féodalité » du panneau de faction
      (obligations, objectifs), fil d'Ariane de la province, panneau « Arbre féodal »
      (`FeudalController`, SIDE_PANEL), guide de 3 étapes (`feudal_tutorial/done`), Codex
      `cdx_vassalite`, choix de faction sur carte (`FactionMapPicker`, onglet du choix de faction).
- [ ] Tests : `fe_ui_test.gd`, `smoke.gd`, `fe_shot.gd`.

## Prochaine étape
Tests headless (`fe_ui_test.gd`), smoke, captures `fe_shot.gd`.

## Points ouverts
- Bretagne, Flandre, Navarre (départs recommandés de la spec) ne sont pas jouables dans les données
  (ni objectifs, ni présentation) : le menu n'affiche que les recommandées jouables.
