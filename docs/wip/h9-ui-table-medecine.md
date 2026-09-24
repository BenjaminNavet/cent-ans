# WIP — H9 interface « La Table » et Médecine

Conception : `docs/design/2026-09-23-histoire-et-savoir.md` §2.3, §3 ; API : `docs/design/h3-h4-api.md`.

## État : terminé (à fusionner)
- [x] `game/scripts/ui/table_section.gd` : régime actuel, description (mots du Codex cliquables),
      bandeau Carême (`is_lent`), sélecteur (`get_diet_options`, infobulle `RichTooltip.diet`),
      ordre `set_diet` soumis directement à `SimFacade.sim`, refus en rouge, « déjà changé ce
      tour », lecture seule hors provinces du joueur. Hook `province_panel.gd` (`table_section`,
      insérée après `ClassesList` dans l'onglet Ville).
- [x] Budget : ligne « Table des provinces » (`faction_panel.gd`, détail `get_table_budget` en infobulle)
- [x] Libellés `rich_tooltip.gd` / `tech_tree_view.gd` : `plague_resistance`, `wound_recovery`,
      `diet_health` ; branche `medicine` ; « Plantes : … » (liens Codex) et note historique
- [x] Genres `table` / `medicine` : rapport de saison (rubrique « Table et santé »), lettres,
      alertes (`AlertsPanel.table_medicine_alerts`) ; clic dans `flow_controller.gd`
- [x] Herbier : `game/scripts/codex/herbarium.gd` (découverte des plantes des techs connues,
      fiches absentes ignorées), alerte « Nouvelle plante dans l'herbier : … », sous-section
      « Herbier » dans les onglets du Codex mêlant fiches et plantes
- [x] Smoke `_run_table_medicine` vert (smoke complet exit 0)
- [x] Captures `docs/img/table-section.png`, `docs/img/tech-medicine.png`
      (`godot --path game --script res://tests/table_screenshot.gd`)

## Limites connues
- Journal (`map_ui.gd`) et solde du HUD (`map_ui.set_treasury`) non touchés (autre session) :
  les genres `table`/`medicine` y sont en texte simple (`SeasonReport.KIND_STYLES` donne
  glyphe/couleur à reprendre) ; le solde du HUD ne retranche pas encore `table_upkeep`.
- Pas d'icônes propres aux régimes (repli `cat_resource`) ; fiches Codex des plantes et
  `cdx_careme` pas encore écrites (liens rendus en texte simple, herbier vide jusque-là).
- Après `set_diet`, la section se rafraîchit seule ; le trésor du HUD attend le prochain rafraîchissement.

## Prochaine étape
Fusion dans `main` par l'orchestrateur.
