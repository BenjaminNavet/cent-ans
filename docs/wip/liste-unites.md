# Liste « Mes unités » (touche U)

Branche `feat/unit-roster`, worktree `../gp-roster`.

## Demande
Dérouler la liste de ses unités sur la carte de campagne (armées, agents), avec leur portée de
déplacement ; un clic mène à l'unité.

## Fait
- Cœur (pont) : `get_army` expose `movement_km` = points restants en km de plaine
  (`movement_left / PLAIN_COST × cell_km`).
- `game/scripts/map/unit_roster_controller.gd` (`UnitRosterController`) : panneau haut gauche,
  sections « Armées » et « Agents » repliables (▾/▸), une ligne par unité (titre, effectif, lieu,
  ordre en cours, jauge de portée, texte « Portée ≈ N km de plaine » ou « N colonies à portée »),
  unités épuisées estompées, ligne de l'unité sélectionnée surlignée. Clic : sélection + caméra.
- Action `map_toggle_units` (U), bouton « Unités » dans la barre du haut, fiche des raccourcis.
- Test : `game/tests/unit_roster_test.gd`.

## Prochaine étape
- Fait : build, tests (unit_roster, smoke, ui3, ux2), clippy ; capture `tests/roster_shot.gd`.
- Pistes : survol d'une ligne = aperçu de la bulle de portée ; tri (unités ayant encore du mouvement d'abord).
