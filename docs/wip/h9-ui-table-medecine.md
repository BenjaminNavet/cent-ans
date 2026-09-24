# WIP — H9 interface « La Table » et Médecine

Conception : `docs/design/2026-09-23-histoire-et-savoir.md` §2.3, §3 ; API : `docs/design/h3-h4-api.md`.

## État
- [x] Squelette `game/scripts/ui/table_section.gd`
- [x] Section Table (régime, sélecteur, infobulles `RichTooltip.diet`, bandeau Carême) + hook
      `province_panel.gd` (`table_section`, ajoutée après `ClassesList` dans l'onglet Ville)
- [x] Budget : ligne « Table des provinces » (`faction_panel.gd`, infobulle par province)
- [x] Libellés `rich_tooltip.gd` / `tech_tree_view.gd` (effets, branche medicine, herbes, note)
- [x] Genres `table` / `medicine` (rapport de saison, lettres, alertes ; clic dans `flow_controller.gd`)
- [x] Herbier (`game/scripts/codex/herbarium.gd`, alerte, sous-section « Herbier » du Codex)
- [x] Smoke `_run_table_medicine` écrit ; script de captures `game/tests/table_screenshot.gd`
- [ ] Smoke vert (import Godot puis smoke, en arrière-plan : machine chargée)
- [ ] Captures `docs/img/table-section.png`, `docs/img/tech-medicine.png`

## Limites connues
- Journal (`map_ui.gd`) et solde du HUD (`map_ui.set_treasury`) non touchés (autre session) :
  les genres `table`/`medicine` y sont en texte simple ; `SeasonReport.KIND_STYLES` donne
  glyphe/couleur à reprendre. Le solde du HUD n'ôte pas encore `table_upkeep`.

## Prochaine étape
Faire passer le smoke, prendre les captures, rapport final.
