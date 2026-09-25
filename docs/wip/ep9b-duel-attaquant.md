# EP9b — Duel de l'attaquant et milice en second échelon

Lot : correction d'EP9 mesurée par SG4. ADR 0056 § EP9b. Branche `ep9b-duel-attaquant`
(worktree `agent-a7bf63ca963f71d60`), partie d'`integration/night` (main + SG4), main fusionné
ensuite (SG4 puis EP6).

## État : terminé, à fusionner par l'orchestrateur

- [x] Reproduction : `sim-battle/tests/ep9b_duel.rs` (60 régiments/camp, plat, sans pieux,
  graines 1-10) ; avant : attaquant 0/10 (armée brisée à ~340 s).
- [x] Règles `data/rules/battle_duel.json` (+ schéma, pytest), `sim-battle/src/duel.rs`
  (`DuelRules` gardées dans `BattleSim`, `set_duel_rules` pour les tests).
- [x] Duel prolongé tant que l'attaquant le gagne : pertes par les traits sur une fenêtre de 60 s
  (`BattleSim::recent_missile_losses`) ; gagné si l'ennemi perd ≥ 0,5 % et ≥ 1,5 × ; borne 300 s ;
  sinon 180 s comme EP9 (`ATTACKER_DUEL_LIMIT` supprimée, en données).
- [x] Milice (moral de base < 50) en second échelon à 35 m pendant la marche d'après duel
  (ennemi entre 320 et 120 m), serre sur la première ligne pour la mêlée.
- [x] Tests actifs : `symmetric_flat_battle_is_open`, `a_won_duel_holds_the_line_then_the_battle_ends`.
- [x] Mesures avant/après (ADR 0056 § EP9b) ; renvoi dans ADR 0046.
- [x] fmt, clippy, cargo test --workspace, build.sh, pytest, smoke Godot (voir rapport final).

## Mesures (10 graines, attaquant / défenseur)

| Cas | avant | après |
|---|---|---|
| plat, sans pieux, champ nu | 0/10 | 7/3 |
| plat, sans pieux, décor EP6 | 5/5 | 6/4 |
| plat, pieux | 3/7 | 3/7 |
| crête, pieux | 0/10 | 6/4 |
| crête, sans pieux | 0/10 | 1/9 |
| plaine générée, pieux / sans | 2/8 ; 5/5 | 2/8 ; 6/4 |
| comme ep1_scale | 4/6 | 6/4 |
| Crécy-like (12 graines, Anglais) | 11/12 | 12/12 |

## Points ouverts

- Crête avec pieux : le défenseur ne gagne plus que 4/10 (contagion de déroute dans sa ligne avant
  la mêlée) ; à reprendre côté R4/SG4.
- Joueur défenseur immobile : batailles plus longues (médianes 519-707 s au lieu de 366-439 s).
- Issue très sensible aux réglages (batailles miroir presque déterministes).
