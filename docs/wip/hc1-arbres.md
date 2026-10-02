# HC1 — arbres généralisés sur la carte de campagne

Worktree `../gp-hc1`, branche `feat/hc1` (issue de `feat/hc`). ADR 0161. Aucun changement Rust.

## État
- [x] Squelette : `map.tree_style` (JSON + schéma), `MapPropScale.tree_style()` /
      `--tree-style=`, champs `generalised_*` du `.tres`, test désactivé, cette note.
- [ ] Rendu généralisé dans `vegetation.gd` (taille, pas, portée, paliers, ombres).
- [ ] Exclusions élargies (lieux, fleuves, lacs, mer, routes).
- [ ] Probabilités hors forêt (gains `generalised_*_gain`).
- [ ] Test `game/tests/hc_forest_test.gd`, planche `game/tests/hc_shots.gd`.
- [ ] Mesures Paris rig 90 / 300 / 700.

## Prochaine étape
Brancher le style dans `Vegetation` (échelle et pas du semis, portée, forêt dense éteinte).

## Points ouverts
(aucun pour l'instant)
