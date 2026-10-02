# TB3 — colonies et bâtiments qui poussent

Branche `feat/tb3`, worktree `../gp-tb3`. Plan : `docs/design/2026-10-02-campagne-tob.md` § 3.
Sans service payant (ADR 0152). ADR du lot : `docs/decisions/0153-…` (choix des maquettes).

## État
- [ ] 1. `data/map/building_models.json` + schéma + test pytest
- [ ] 2. Maquettes des 3 niveaux (8 familles) sans génération payante
- [ ] 3. Couche de rendu des bâtiments hors les murs
- [ ] 4. Croissance de la ville 1:1 (faubourgs selon population, enceinte selon fortification)
- [ ] 5. Suie par ville (saccage, assaut)
- [ ] 6. Chantier visible (échafaudage + tas de pierres)
- [ ] Tests `tb3_growth_test.gd`, `tb3_shot.gd`, bench `ss_shot.gd --bench` à 90 et 400

## Relevé de départ
- Pont : `get_province_city(id)` donne `buildings[{id, name, category, upkeep}]`,
  `fortification_level`, `resources`, `construction` ; `get_provinces_snapshot` donne
  `population_total`, `devastation`, `besieged`, `constructing`.
- Aucun état « ville saccagée » n'est conservé par `core/` : le saccage ajoute de la dévastation
  à la province (`capture.rs`). La suie par ville se lit donc sur la dévastation et le siège.
- Les bâtiments de `data/buildings/` n'ont pas de niveau propre : ils ont un `tier` et une chaîne
  `upgrades_from`. Le niveau de maquette vient donc de la correspondance.

## Prochaine étape
Point 1 : écrire la correspondance et son schéma.
