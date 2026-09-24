# WIP — H9 interface « La Table » et Médecine

Conception : `docs/design/2026-09-23-histoire-et-savoir.md` §2.3, §3 ; API : `docs/design/h3-h4-api.md`.

## État
- [x] Squelette `game/scripts/ui/table_section.gd`
- [ ] Section Table (régime, sélecteur, infobulles, bandeau Carême) + hook `province_panel.gd`
- [ ] Budget : ligne « Table » (`faction_panel.gd`)
- [ ] Libellés `rich_tooltip.gd` (effets, branche medicine, herbes, note historique)
- [ ] Genres `table` / `medicine` (rapport de saison, lettres, alertes)
- [ ] Herbier (CodexStore + alerte + sous-section Codex)
- [ ] Smoke `_run_table_medicine` ; captures `docs/img/table-section.png`, `docs/img/tech-medicine.png`

## Prochaine étape
Implémenter `table_section.gd`.
