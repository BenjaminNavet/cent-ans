# 0245 — Victoire féodale générique non passive

Statut : accepté (chantier RX, lot `victory`).

## Contexte
La revue RX a relevé que douze factions jouables gagnaient « premier vassal du royaume » au tour 20 (printemps 1342) sans
aucun ordre : elles partent premier vassal de leur couronne et la série `first_vassal` atteignait `ascension_turns` = 20.
La France avait en outre deux objectifs sur quatre remplis au tour 0 (`obj_fr_crown`, `obj_fr_burgundy`) et gagnait
vers le tour 46 (1348), avant Crécy.

## Décision
- `data/rules/feudal.json` : `generic_victory_min_year` = 1360 (aucune victoire féodale générique avant : objectifs de
  titre, indépendance, premier vassal, couronne), `ascension_turns` 20 -> 60, `ascension_power_ratio` = 0,5 (la série
  ne compte que si la puissance du premier vassal atteint la moitié de celle de son suzerain). Champs documentés dans
  `data/schemas/feudal_rules.schema.json` et `FeudalRules`.
- France : `obj_fr_crown` devient « Réunir la Flandre et l'Artois », `obj_fr_burgundy` devient « Acquérir le Dauphiné »
  (`obj_fr_dauphine`) ; `hold_turns` 20 -> 40. Aucun objectif rempli au départ.
- Tests : `core/crates/sim-campaign/tests/feudal/rx_victory.rs`.

## Conséquences
Une faction passive ne gagne plus avant 1360, et ensuite seulement en rivalisant avec son suzerain pendant 15 ans.
L'écran de victoire n'apparaît plus après 5 ans d'inaction. Non traité : signalement de la victoire aux factions sans
bloc `victory` (constat mineur).
