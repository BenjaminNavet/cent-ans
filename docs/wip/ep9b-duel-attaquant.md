# EP9b — Duel de l'attaquant et milice en second échelon

Lot : correction d'EP9 (ADR 0056 § EP9b) mesurée par SG4. Branche `ep9b-duel-attaquant`
(worktree `agent-a7bf63ca963f71d60`), partie d'`integration/night` (main + SG4), main fusionné
ensuite (SG4 y est arrivé).

## État

- [x] Reproduction : `sim-battle/tests/ep9b_duel.rs` (60 régiments/camp, plat, sans pieux,
  graines 1-10) ; avant : attaquant 0/10 (défaite par armée brisée à ~340 s).
- [x] Règles `data/rules/battle_duel.json` (+ schéma, pytest), `sim-battle/src/duel.rs`
  (`DuelRules`, stockées dans `BattleSim`, `set_duel_rules` pour les tests).
- [x] Duel prolongé tant que l'attaquant le gagne : pertes par les traits sur une fenêtre glissante
  de 60 s (`BattleSim::recent_missile_losses`, relevé à chaque période d'IA dans l'horloge
  d'engagement) ; gagné si l'ennemi perd ≥ 0,5 % et ≥ 1,5 × ce que l'attaquant perd ; borne haute
  420 s ; sinon 180 s comme EP9.
- [x] Milice (moral de base < 50) en second échelon à 35 m quand la ligne marche à l'ennemi après
  le duel (entre 320 et 120 m de l'ennemi) ; elle serre sur la première ligne pour la mêlée.
- [x] Tests actifs : `symmetric_flat_battle_is_open` (3 à 7/10), `a_won_duel_holds_the_line_then_the_battle_ends`.
- [ ] Sonde EP9 complète (`ep9_decisive survey`) après, ADR 0056 § EP9b.
- [ ] Merge main (EP6 annoncé : IA et couverts de villages) puis mesures finales (plat nu + décor).
- [ ] fmt, clippy, cargo test --workspace, build.sh, pytest, smoke Godot.

## Mesures (10 graines, attaquant / défenseur)

| Cas | avant | après |
|---|---|---|
| plat, pieux | 3/7 | 3/7 |
| plat, sans pieux | 0/10 | 7/3 |
| crête, pieux | 0/10 | 6/4 |
| crête, sans pieux | 0/10 | 1/9 |
| plaine générée, pieux | 2/8 | 2/8 |
| plaine générée, sans pieux | 5/5 | 6/4 |
| comme ep1_scale | 2/8 | 1/9 |
| Crécy-like (12 graines, Anglais) | 11/12 | 12/12 |

## Points ouverts

- Crête avec pieux : le défenseur ne gagne plus que 4/10 (0/10 → 6/4 pour l'attaquant). Sa ligne
  s'effondre par contagion de déroute (archers et cavalerie qui refluent à travers elle) avant la
  mêlée ; avant EP9b, la milice attaquante cédait la première. Sans pieux : 1/9 inchangé.
- Issue très sensible (échiquier presque déterministe d'une graine à l'autre).

## Prochaine étape

Sonde EP9, merge main (EP6), mesures finales, ADR.
